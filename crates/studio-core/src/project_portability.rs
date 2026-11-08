//! Complete project identities and immutable source preservation.
//!
//! Import workers own a separate database connection. External recovery metadata
//! remains quarantined evidence and cannot populate trusted replay checkpoints.
mod canvas_commands;
mod canvas_validation;
mod evidence;
mod json;
mod portable_format;
mod snapshot;
mod validation;

pub use canvas_commands::{
    ProjectCanvasDraft, ProjectCanvasInfo, ProjectCanvasRevision, ProjectCanvasRevisionInfo,
    ProjectCanvasSnapshot,
};
pub use evidence::{
    EvidenceReferenceInput, EvidenceReferenceQuery, EvidenceReferenceSummary,
    MAX_EVIDENCE_METADATA_BYTES, MAX_EVIDENCE_OBJECT_BYTES, ProjectEvidenceSnapshot,
    ValidatedEvidenceObject, ValidatedEvidenceReference,
};
pub use portable_format::{
    MAX_PORTABLE_MATERIALIZED_BYTES, MAX_PORTABLE_MEMBERS, MAX_PORTABLE_RECORD_BYTES,
    MAX_PORTABLE_RECORDS, MAX_PORTABLE_ROOT_BYTES, MAX_PORTABLE_SHARD_BYTES,
    MAX_PORTABLE_SHARD_RECORDS, MAX_PORTABLE_SHARDS, MAX_PORTABLE_TRANSPORT_BYTES,
    PortableObjectReference, PortableProjectHeader, PortableProjectScope, PortableResourceBudget,
    PortableShardDescriptor, VerifiedPortableRecord, visit_portable_shard,
};
pub use snapshot::{ProjectAssetSource, ProjectExportSnapshot};

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::{MAIN_DB, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

use super::{Result, StudioError, Workspace, require_project};
use json::invalid;

/// Maximum compressed/expanded project bytes, matching the original transfer boundary.
pub const MAX_PROJECT_TRANSFER_BYTES: u64 = 512 * 1024 * 1024;
/// Maximum original JSON manifest bytes before any parsing or allocation.
pub const MAX_PROJECT_MANIFEST_BYTES: usize = 64 * 1024 * 1024;

/// Caller-owned cancellation shared with an import/export worker.
#[derive(Clone, Default)]
pub struct ProjectTransferCancellation(Arc<AtomicBool>);
impl ProjectTransferCancellation {
    /// Request cancellation; the command rolls back before reporting completion.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    pub(super) fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            return invalid("operation cancelled; existing data is unchanged");
        }
        Ok(())
    }
}

/// One validated declared binary entry. Identity alone never authorizes a read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectAttachment {
    /// Source collection: media or original dictation audio.
    pub resource_kind: String,
    /// Project-scoped original media/session identity.
    pub logical_id: String,
    /// Safe relative member name inside the pinned archive.
    pub path: String,
    /// Exact original encoded SHA-256.
    pub sha256: String,
    /// Exact encoded length.
    pub byte_size: u64,
    /// Declared source MIME type, with byte-header verification at commit.
    pub media_type: String,
    /// Original display basename; never opened as a filesystem path.
    pub original_name: String,
    /// Optional source intrinsic image width.
    pub width: Option<u32>,
    /// Optional source intrinsic image height.
    pub height: Option<u32>,
}

/// Bounded byte access owned by the caller's already-opened source archive.
pub trait ImportAssetSource {
    /// Open this exact validated descriptor. Core checks length and digest while streaming.
    fn open<'a>(
        &'a mut self,
        attachment: &ProjectAttachment,
    ) -> std::io::Result<Box<dyn Read + 'a>>;
}

/// Semantically validated original project. No deserialization or public fields grant authority.
pub struct ValidatedProjectImport {
    raw_manifest: String,
    manifest: Value,
    format_version: u64,
    logical_project_id: String,
    title: String,
    attachments: Vec<ProjectAttachment>,
}
impl ValidatedProjectImport {
    /// Parse bounded original bytes, reject duplicate keys, and validate every known domain.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > MAX_PROJECT_MANIFEST_BYTES {
            return invalid("manifest must contain nonempty JSON of at most 64 MiB");
        }
        json::reject_duplicate_keys(bytes)?;
        let manifest: Value = serde_json::from_slice(bytes)?;
        let attachments = validation::validate(&manifest)?;
        let format_version = validation::number(&manifest, "formatVersion")?;
        let project = &manifest["project"];
        let logical_project_id = validation::string(project, "id")?.to_owned();
        let title = validation::string(project, "title")?.to_owned();
        Ok(Self {
            raw_manifest: std::str::from_utf8(bytes)
                .map_err(|_| StudioError::Validation("Project manifest must be UTF-8".into()))?
                .to_owned(),
            manifest,
            format_version,
            logical_project_id,
            title,
            attachments,
        })
    }
    /// Entire original JSON text, including untouched number lexemes and unknown fields.
    pub fn original_manifest(&self) -> &str {
        &self.raw_manifest
    }
    /// Read-only complete parsed semantic representation, retaining arbitrary precision.
    pub fn manifest(&self) -> &Value {
        &self.manifest
    }
    /// Original supported envelope version, independent of editor/Canvas schemas.
    pub fn format_version(&self) -> u64 {
        self.format_version
    }
    /// Original project identity; independent copies retain it in separate namespaces.
    pub fn logical_project_id(&self) -> &str {
        &self.logical_project_id
    }
    /// Original title, without content translation or normalization.
    pub fn title(&self) -> &str {
        &self.title
    }
    /// Complete declared media/audio descriptors in source order.
    pub fn attachments(&self) -> &[ProjectAttachment] {
        &self.attachments
    }
}

/// Result of one fully committed independent project migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportedProject {
    /// Native project namespace used for all ownership checks.
    pub project_id: String,
    /// Preserved source project identity.
    pub logical_project_id: String,
    /// Number of all project nodes, including containers with no document revision.
    pub node_count: usize,
    /// Exact number of imported document revisions.
    pub document_revision_count: usize,
    /// Number of complete imported Canvas boards.
    pub canvas_count: usize,
    /// Number of verified immutable attachments.
    pub attachment_count: usize,
}

/// An importer with its own SQLite connection, suitable for a dedicated worker thread.
///
/// The desktop must flush pending writes and present a cancellable import modal before starting
/// its worker. WAL readers on the desktop's other Workspace remain independent of this mutex.
pub struct ProjectImportWorker {
    workspace: Workspace,
}
impl ProjectImportWorker {
    /// Open the explicitly selected workspace on the import worker, never infer an ambient path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            workspace: Workspace::open(path)?,
        })
    }
    /// Create an isolated worker workspace for synthetic contracts and headless use.
    pub fn memory() -> Result<Self> {
        Ok(Self {
            workspace: Workspace::memory()?,
        })
    }
    /// Access this worker's command service; do not share it with a render-thread save loop.
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }
    /// Atomically import all rows and streamed asset bytes, or roll everything back.
    pub fn import(
        &mut self,
        project: &ValidatedProjectImport,
        source: &mut dyn ImportAssetSource,
        cancel: &ProjectTransferCancellation,
    ) -> Result<ImportedProject> {
        cancel.check()?;
        let mut connection = self.workspace.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let project_id = uuid::Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO projects(id,title) VALUES(?1,?2)",
            params![project_id, project.title],
        )?;
        transaction.execute("INSERT INTO project_portability(project_id,logical_project_id,format_version,raw_manifest,manifest_sha256) VALUES(?1,?2,?3,?4,?5)",
            params![project_id,project.logical_project_id,sqlite_integer(project.format_version)?,project.raw_manifest.as_str(),format!("{:x}",Sha256::digest(project.raw_manifest.as_bytes()))])?;
        insert_alias(
            &transaction,
            &project_id,
            "project",
            &project.logical_project_id,
            &project_id,
        )?;
        let node_map = import_documents(&transaction, &project_id, project, cancel)?;
        import_source_records(&transaction, &project_id, project)?;
        import_canvases(&transaction, &project_id, &project.manifest, cancel)?;
        for attachment in &project.attachments {
            cancel.check()?;
            import_attachment(
                &transaction,
                &project_id,
                &project.manifest,
                attachment,
                source,
                cancel,
            )?;
        }
        import_notes(&transaction, &project_id, &project.manifest, &node_map)?;
        cancel.check()?;
        let result = ImportedProject {
            project_id,
            logical_project_id: project.logical_project_id.clone(),
            node_count: validation::array(&project.manifest, "nodes")?.len(),
            document_revision_count: validation::array(&project.manifest, "documentRevisions")?
                .len(),
            canvas_count: validation::array(&project.manifest, "canvases")?.len(),
            attachment_count: project.attachments.len(),
        };
        transaction.commit()?;
        Ok(result)
    }
}

fn insert_alias(
    transaction: &Transaction<'_>,
    project: &str,
    kind: &str,
    logical: &str,
    native: &str,
) -> Result<()> {
    transaction.execute("INSERT INTO project_resource_aliases(project_id,resource_kind,logical_id,native_id) VALUES(?1,?2,?3,?4)", params![project,kind,logical,native])?;
    Ok(())
}

fn import_documents(
    transaction: &Transaction<'_>,
    project_id: &str,
    project: &ValidatedProjectImport,
    cancel: &ProjectTransferCancellation,
) -> Result<HashMap<String, String>> {
    let revisions = validation::array(&project.manifest, "documentRevisions")?;
    let mut by_node: HashMap<&str, Vec<&Value>> = HashMap::new();
    for revision in revisions {
        by_node
            .entry(validation::string(revision, "nodeId")?)
            .or_default()
            .push(revision);
    }
    let mut handles = HashMap::new();
    for (ordinal, node) in validation::array(&project.manifest, "nodes")?
        .iter()
        .enumerate()
    {
        cancel.check()?;
        let logical = validation::string(node, "id")?;
        let current = validation::number(node, "currentRevision")?;
        let native = if current > 0 {
            let current_revision = by_node
                .get(logical)
                .and_then(|items| {
                    items
                        .iter()
                        .find(|revision| revision["revision"].as_u64() == Some(current))
                })
                .ok_or_else(|| {
                    StudioError::CorruptData("Validated current revision is missing".into())
                })?;
            let id = uuid::Uuid::new_v4().to_string();
            transaction.execute("INSERT INTO documents(id,project_id,title,body,revision,read_only,source_json) VALUES(?1,?2,?3,?4,?5,0,NULL)", params![id,project_id,validation::string(node,"title")?,validation::string(current_revision,"plainText")?,sqlite_integer(current)?])?;
            if let Some(items) = by_node.get(logical) {
                for revision in items {
                    transaction.execute("INSERT INTO document_revisions(document_id,revision,body,source) VALUES(?1,?2,?3,'import')",params![id,sqlite_integer(validation::number(revision,"revision")?)?,validation::string(revision,"plainText")?])?;
                    transaction.execute("INSERT INTO rich_document_revisions(document_id,revision,content_json) VALUES(?1,?2,?3)",params![id,sqlite_integer(validation::number(revision,"revision")?)?,serde_json::to_string(&revision["content"])?])?;
                }
            }
            transaction.execute(
                "INSERT INTO rich_documents(document_id,content_json) VALUES(?1,?2)",
                params![id, serde_json::to_string(&current_revision["content"])?],
            )?;
            handles.insert(logical.to_owned(), id.clone());
            Some(id)
        } else {
            None
        };
        let alias = native
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        insert_alias(transaction, project_id, "node", logical, &alias)?;
        insert_alias(transaction, project_id, "document", logical, &alias)?;
        transaction.execute("INSERT INTO project_nodes(project_id,logical_id,native_document_id,parent_logical_id,ordinal,raw_json) VALUES(?1,?2,?3,?4,?5,?6)",params![project_id,logical,native,node["parentId"].as_str(),sqlite_integer(ordinal)?,serde_json::to_string(node)?])?;
    }
    Ok(handles)
}

fn import_source_records(
    transaction: &Transaction<'_>,
    project_id: &str,
    project: &ValidatedProjectImport,
) -> Result<()> {
    let source: BTreeMap<String, Box<RawValue>> =
        serde_json::from_str(project.original_manifest())?;
    for (domain, raw) in source {
        if raw.get().trim_start().starts_with('[') {
            let records: Vec<Box<RawValue>> = serde_json::from_str(raw.get())?;
            for (ordinal, record) in records.iter().enumerate() {
                transaction.execute("INSERT INTO project_source_records(project_id,domain,ordinal,raw_json) VALUES(?1,?2,?3,?4)",params![project_id,domain,sqlite_integer(ordinal)?,record.get()])?;
            }
        } else {
            transaction.execute("INSERT INTO project_source_records(project_id,domain,ordinal,raw_json) VALUES(?1,?2,0,?3)",params![project_id,domain,raw.get()])?;
        }
        let folded = domain.to_ascii_lowercase();
        if folded.contains("recovery")
            || folded.contains("checkpoint")
            || folded == "rich_draft_annotations"
        {
            transaction.execute("INSERT INTO project_quarantined_recovery(project_id,source_key,raw_json) VALUES(?1,?2,?3)",params![project_id,domain,raw.get()])?;
        }
    }
    Ok(())
}

fn import_canvases(
    transaction: &Transaction<'_>,
    project_id: &str,
    manifest: &Value,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    for canvas in validation::array(manifest, "canvases")? {
        cancel.check()?;
        let id = validation::string(canvas, "id")?;
        insert_alias(
            transaction,
            project_id,
            "canvas",
            id,
            &uuid::Uuid::new_v4().to_string(),
        )?;
        transaction.execute("INSERT INTO project_canvases(project_id,logical_id,current_revision,raw_json) VALUES(?1,?2,?3,?4)",params![project_id,id,sqlite_integer(validation::number(canvas,"currentRevision")?)?,serde_json::to_string(canvas)?])?;
    }
    for revision in validation::array(manifest, "canvasRevisions")? {
        cancel.check()?;
        transaction.execute("INSERT INTO project_canvas_revisions(project_id,canvas_id,revision,base_revision,schema_version,content_json,raw_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![project_id,validation::string(revision,"canvasId")?,sqlite_integer(validation::number(revision,"revision")?)?,sqlite_integer(validation::number(revision,"baseRevision")?)?,sqlite_integer(validation::number(revision,"schemaVersion")?)?,serde_json::to_string(&revision["content"])?,serde_json::to_string(revision)?])?;
    }
    Ok(())
}

fn import_attachment(
    transaction: &Transaction<'_>,
    project: &str,
    manifest: &Value,
    attachment: &ProjectAttachment,
    source: &mut dyn ImportAssetSource,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    let native = uuid::Uuid::new_v4().to_string();
    let size = i32::try_from(attachment.byte_size).map_err(|_| {
        StudioError::Validation("Attachment exceeds SQLite streaming capacity".into())
    })?;
    let record = validation::array(manifest, &attachment.resource_kind)?
        .iter()
        .find(|item| {
            item[if attachment.resource_kind == "audio" {
                "sessionId"
            } else {
                "id"
            }]
            .as_str()
                == Some(attachment.logical_id.as_str())
        })
        .ok_or_else(|| StudioError::CorruptData("Validated attachment record is missing".into()))?;
    transaction.execute("INSERT INTO project_assets(id,project_id,media_type,original_name,sha256,byte_size,width,height,source_json,data) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,zeroblob(?6))",params![native,project,attachment.media_type,attachment.original_name,attachment.sha256,sqlite_integer(attachment.byte_size)?,attachment.width,attachment.height,serde_json::to_string(record)?])?;
    let rowid = transaction.last_insert_rowid();
    let mut blob = transaction.blob_open(MAIN_DB, "project_assets", "data", rowid, false)?;
    if blob.len()
        != usize::try_from(size)
            .map_err(|_| StudioError::Validation("Invalid attachment size".into()))?
    {
        return invalid("SQLite attachment reservation mismatch");
    }
    let mut reader = source.open(attachment)?;
    let mut digest = Sha256::new();
    let mut remaining = attachment.byte_size;
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut header = Vec::with_capacity(64);
    while remaining > 0 {
        cancel.check()?;
        let count = usize::try_from(remaining.min(buffer.len() as u64))
            .map_err(|_| StudioError::Validation("Invalid attachment length".into()))?;
        let read = reader.read(&mut buffer[..count])?;
        if read == 0 {
            return invalid("attachment ended before its declared length");
        }
        if header.len() < 64 {
            header.extend_from_slice(&buffer[..read.min(64 - header.len())]);
        }
        digest.update(&buffer[..read]);
        blob.write_all(&buffer[..read])?;
        remaining -= read as u64;
    }
    let mut extra = [0_u8; 1];
    if reader.read(&mut extra)? != 0 {
        return invalid("attachment exceeds its declared length");
    }
    drop(reader);
    drop(blob);
    if !format!("{:x}", digest.finalize()).eq_ignore_ascii_case(&attachment.sha256) {
        return invalid("attachment SHA-256 mismatch");
    }
    validate_attachment_header(transaction, rowid, attachment, &header)?;
    insert_alias(
        transaction,
        project,
        &attachment.resource_kind,
        &attachment.logical_id,
        &native,
    )?;
    transaction.execute("INSERT INTO project_asset_metadata(project_id,resource_kind,logical_id,native_asset_id,archive_path,raw_json) VALUES(?1,?2,?3,?4,?5,?6)",params![project,attachment.resource_kind,attachment.logical_id,native,attachment.path,serde_json::to_string(record)?])?;
    Ok(())
}

fn validate_attachment_header(
    transaction: &Transaction<'_>,
    rowid: i64,
    attachment: &ProjectAttachment,
    header: &[u8],
) -> Result<()> {
    let image_format = match attachment.media_type.as_str() {
        "image/png" => Some(image::ImageFormat::Png),
        "image/jpeg" => Some(image::ImageFormat::Jpeg),
        "image/gif" => Some(image::ImageFormat::Gif),
        "image/webp" => Some(image::ImageFormat::WebP),
        _ => None,
    };
    if let Some(format) = image_format {
        let blob = transaction.blob_open(MAIN_DB, "project_assets", "data", rowid, true)?;
        let reader = image::ImageReader::new(std::io::BufReader::new(blob))
            .with_guessed_format()
            .map_err(|_| StudioError::Validation("Cannot inspect image attachment".into()))?;
        if reader.format() != Some(format) {
            return invalid("image attachment MIME does not match its bytes");
        }
        let (width, height) = reader.into_dimensions().map_err(|_| {
            StudioError::Validation("Cannot inspect image attachment dimensions".into())
        })?;
        if Some(width) != attachment.width || Some(height) != attachment.height {
            return invalid("image attachment dimensions differ from its manifest");
        }
    } else {
        let valid = match attachment.media_type.as_str() {
            "application/pdf" => header.starts_with(b"%PDF-"),
            "audio/wav" => header.starts_with(b"RIFF") && header.get(8..12) == Some(b"WAVE"),
            "audio/mpeg" => {
                header.starts_with(b"ID3")
                    || (header.len() > 1 && header[0] == 0xff && header[1] & 0xe0 == 0xe0)
            }
            "audio/mp4" | "video/mp4" => header.get(4..8) == Some(b"ftyp"),
            "audio/ogg" => header.starts_with(b"OggS"),
            "video/webm" => header.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]),
            _ => true,
        };
        if !valid {
            return invalid("attachment MIME does not match its byte header");
        }
    }
    Ok(())
}

fn import_notes(
    transaction: &Transaction<'_>,
    project: &str,
    manifest: &Value,
    node_map: &HashMap<String, String>,
) -> Result<()> {
    for original in validation::array(manifest, "documentAnnotations")? {
        let logical = validation::string(original, "id")?;
        let native = uuid::Uuid::new_v4().to_string();
        insert_alias(transaction, project, "annotation", logical, &native)?;
        let node = validation::string(original, "nodeId")?;
        let document_id = node_map.get(node).ok_or_else(|| {
            StudioError::Validation("Annotation has no content-bearing document".into())
        })?;
        let base = validation::number(original, "baseRevision")?;
        let revision = validation::array(manifest, "documentRevisions")?
            .iter()
            .find(|r| r["nodeId"].as_str() == Some(node) && r["revision"].as_u64() == Some(base))
            .ok_or_else(|| {
                StudioError::CorruptData("Validated annotation revision is missing".into())
            })?;
        let tree = craft_editor_model::Document::from_json(revision["content"].clone())
            .map_err(|e| StudioError::Validation(format!("Cannot preserve imported note: {e}")))?;
        let resolution = super::AnnotationAnchor::adapt_legacy(&tree, &original["anchor"], None)?;
        let current: u64 = transaction.query_row(
            "SELECT revision FROM documents WHERE id=?1",
            [document_id],
            |row| read_unsigned(row, 0),
        )?;
        let attached = resolution.anchor_status == super::AnchorStatus::Attached
            && current == base
            && original["anchorStatus"].as_str() != Some("detached");
        let kind = validation::string(original, "kind")?;
        let mut native_annotation = None;
        let mut native_projection = None;
        // An unmappable legacy/UTF-16-bug range has no native anchor. Keep its
        // readable note and original anchor in a detached projection, never invent offsets.
        if kind == "note"
            && let Some(anchor) = resolution.native.as_ref()
        {
            let status = match original["status"].as_str() {
                Some("resolved") => super::AnnotationStatus::Resolved,
                Some("needs_attention") => super::AnnotationStatus::NeedsAttention,
                _ => super::AnnotationStatus::Open,
            };
            let note = super::PassageAnnotation {
                id: native.clone(),
                document_id: document_id.clone(),
                revision: i64::try_from(original["revision"].as_u64().unwrap_or(1).max(1))
                    .map_err(|_| {
                        StudioError::Validation("Annotation revision exceeds native bounds".into())
                    })?,
                base_revision: i64::try_from(base).map_err(|_| {
                    StudioError::Validation("Annotation base exceeds native bounds".into())
                })?,
                anchor: anchor.clone(),
                note: validation::string(original, "note")?.to_owned(),
                status,
                anchor_status: if attached {
                    super::AnchorStatus::Attached
                } else {
                    super::AnchorStatus::Detached
                },
                deleted: false,
            };
            transaction.execute("INSERT INTO document_annotations(id,document_id,revision,annotation_json) VALUES(?1,?2,?3,?4)",params![note.id,note.document_id,note.revision,serde_json::to_string(&note)?])?;
            transaction.execute("INSERT INTO document_annotation_history(annotation_id,annotation_revision,document_revision,source,before_json,after_json) VALUES(?1,?2,?3,'create',NULL,?4)",params![note.id,note.revision,note.base_revision,serde_json::to_string(&note)?])?;
            transaction.execute("INSERT INTO document_annotation_snapshots(document_id,document_revision,annotation_id,attachment_json) VALUES(?1,?2,?3,?4)",params![note.document_id,note.base_revision,note.id,serde_json::to_string(&serde_json::json!({"anchor":&note.anchor,"status":note.anchor_status}))?])?;
            native_projection = Some(serde_json::to_string(&note)?);
            native_annotation = Some(native);
        }
        transaction.execute("INSERT INTO project_annotation_projections(project_id,logical_id,document_id,native_annotation_id,native_anchor_json,native_projection_json,anchor_status,raw_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![project,logical,document_id,native_annotation,resolution.native.as_ref().map(serde_json::to_string).transpose()?,native_projection,if attached {"attached"}else{"detached"},serde_json::to_string(original)?])?;
    }
    Ok(())
}

/// Readable imported note/dictionary metadata, including legacy anchors without native coordinates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportedProjectAnnotation {
    /// Source identity scoped to the requested project.
    pub logical_id: String,
    /// Existing native document handle.
    pub document_id: String,
    /// Native note handle only when an exact known legacy mapping exists.
    pub native_annotation_id: Option<String>,
    /// Proven exact native range, absent for ambiguous or invalid legacy coordinates.
    pub native_anchor: Option<super::AnnotationAnchor>,
    /// Independent native attachment validity; raw source status remains in metadata.
    pub anchor_status: super::AnchorStatus,
    /// Complete original kind/status/note/anchor/target/history metadata without rewriting.
    pub metadata: Value,
}

/// One project-scoped node retaining its exact source metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectTreeNode {
    /// Original node identity, scoped to the returned project.
    pub logical_id: String,
    /// Existing native rich document handle; absent for contentless nodes.
    pub document_id: Option<String>,
    /// Original source order used to break equal position ties.
    pub ordinal: u64,
    /// Complete node metadata including custom kinds, parent, position and unknown fields.
    pub metadata: Value,
}

impl Workspace {
    /// Read all original annotation kinds with explicit detached/no-native-anchor states.
    pub fn imported_project_annotations(
        &self,
        project: &str,
    ) -> Result<Vec<ImportedProjectAnnotation>> {
        let connection = self.connection()?;
        require_project(&connection, project)?;
        let mut statement=connection.prepare("SELECT logical_id,document_id,native_annotation_id,native_anchor_json,anchor_status,raw_json FROM project_annotation_projections WHERE project_id=?1 ORDER BY rowid")?;
        let rows = statement
            .query_map([project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(
                |(logical_id, document_id, native_annotation_id, anchor, status, raw)| {
                    Ok(ImportedProjectAnnotation {
                        logical_id,
                        document_id,
                        native_annotation_id,
                        native_anchor: anchor.map(|raw| serde_json::from_str(&raw)).transpose()?,
                        anchor_status: if status == "attached" {
                            super::AnchorStatus::Attached
                        } else {
                            super::AnchorStatus::Detached
                        },
                        metadata: serde_json::from_str(&raw)?,
                    })
                },
            )
            .collect()
    }
    /// Read imported nodes in source order; parent/position metadata controls tree presentation.
    pub fn project_tree(&self, project: &str) -> Result<Vec<ProjectTreeNode>> {
        let connection = self.connection()?;
        require_project(&connection, project)?;
        let mut statement=connection.prepare("SELECT logical_id,native_document_id,ordinal,raw_json FROM project_nodes WHERE project_id=?1 ORDER BY ordinal")?;
        let rows = statement
            .query_map([project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    read_unsigned(row, 2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(logical_id, document_id, ordinal, raw)| {
                Ok(ProjectTreeNode {
                    logical_id,
                    document_id,
                    ordinal,
                    metadata: serde_json::from_str(&raw)?,
                })
            })
            .collect()
    }
    /// Resolve only inside the specified owning project; unknown or foreign identities fail.
    pub fn resolve_project_resource(
        &self,
        project: &str,
        kind: &str,
        logical: &str,
    ) -> Result<String> {
        let connection = self.connection()?;
        require_project(&connection, project)?;
        connection.query_row("SELECT native_id FROM project_resource_aliases WHERE project_id=?1 AND resource_kind=?2 AND logical_id=?3",params![project,kind,logical],|row|row.get(0)).optional()?.ok_or_else(||StudioError::NotFound {kind:"project resource",id:logical.to_owned()})
    }
}

fn sqlite_integer(value: impl TryInto<i64>) -> Result<i64> {
    value.try_into().map_err(|_| {
        StudioError::Validation("Project integer exceeds SQLite signed integer bounds".into())
    })
}
fn read_unsigned(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(column)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(column, value))
}
fn read_optional_unsigned(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<Option<u64>> {
    let value: Option<i64> = row.get(column)?;
    value
        .map(|value| {
            u64::try_from(value)
                .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(column, value))
        })
        .transpose()
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Synthetic SQLite conversion contracts fail immediately"
)]
mod sqlite_boundaries {
    use super::{read_optional_unsigned, read_unsigned, sqlite_integer};
    #[test]
    fn unsigned_sql_boundaries_should_reject_negative_reads_and_signed_overflow() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        assert!(
            connection
                .query_row("SELECT -1", [], |row| read_unsigned(row, 0))
                .is_err()
        );
        assert!(
            connection
                .query_row("SELECT -1", [], |row| read_optional_unsigned(row, 0))
                .is_err()
        );
        assert_eq!(
            connection
                .query_row("SELECT NULL", [], |row| read_optional_unsigned(row, 0))
                .unwrap(),
            None
        );
        assert!(sqlite_integer(u64::MAX).is_err());
        assert_eq!(
            sqlite_integer(u64::try_from(i64::MAX).unwrap()).unwrap(),
            i64::MAX
        );
    }
}
