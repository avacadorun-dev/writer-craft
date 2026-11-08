//! Format-2 content-addressed export: a fixed read transaction, bounded records and streamed objects.
use std::fs::File;
use std::io::{Read, Write};

use rusqlite::{Connection, MAIN_DB, OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use super::{
    MAX_PORTABLE_RECORD_BYTES, MAX_PORTABLE_SHARD_BYTES, MAX_PORTABLE_SHARD_RECORDS,
    PortableObjectReference, PortableProjectHeader, PortableProjectScope, PortableResourceBudget,
    PortableShardDescriptor, ProjectTransferCancellation, Result, StudioError, Workspace,
    json::invalid, read_unsigned, require_project,
};

/// Dedicated owner of an export database connection. Never borrows the desktop save mutex.
pub struct ProjectPortableExportWorker {
    workspace: Workspace,
}
impl ProjectPortableExportWorker {
    /// Open the explicitly chosen workspace on an export worker.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Ok(Self {
            workspace: Workspace::open(path)?,
        })
    }
    /// Prepare a complete project in one revision-consistent read transaction.
    ///
    /// Metadata pages and content-addressed objects are job-owned temporary files. Only the source
    /// phase and one known payload/row at a time are loaded; unknown evidence and artifacts stream.
    /// This method neither publishes output nor mutates saved project content.
    pub fn prepare(
        &mut self,
        project: &str,
        cancel: &ProjectTransferCancellation,
    ) -> Result<PreparedPortableProject> {
        let mut connection = self.workspace.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        require_project(&transaction, project)?;
        let mut output = Builder::new(cancel.clone())?;
        let original: Option<(i64, u64)> = transaction.query_row(
            "SELECT rowid,length(CAST(raw_manifest AS BLOB)) FROM project_portability WHERE project_id=?1",
            [project], |row| Ok((row.get(0)?, read_unsigned(row, 1)?)),
        ).optional()?;
        let original = original
            .map(|(rowid, size)| {
                let mut blob = transaction.blob_open(
                    MAIN_DB,
                    "project_portability",
                    "raw_manifest",
                    rowid,
                    true,
                )?;
                output.object(&mut blob, size, super::MAX_PROJECT_MANIFEST_BYTES as u64)
            })
            .transpose()?;
        export_project(&transaction, project, &mut output)?;
        export_nodes(&transaction, project, &mut output)?;
        export_revisions(&transaction, project, &mut output)?;
        export_notes(&transaction, project, &mut output)?;
        export_canvases(&transaction, project, &mut output)?;
        export_recovery(&transaction, project, &mut output)?;
        export_assets_and_evidence(&transaction, project, &mut output)?;
        cancel.check()?;
        super::portable_reading::capture_reading(&transaction, &output.index, project, cancel)?;
        let prepared = output.finish(original)?;
        transaction.commit()?;
        Ok(prepared)
    }
}

/// A bounded prepared format-2 project. Paths and database handles are private capabilities.
/// Dropping it removes only its own temporary directory.
pub struct PreparedPortableProject {
    pub(super) index: Connection,
    directory: TempDir,
    root: Vec<u8>,
    object_count: u64,
    object_bytes: u64,
}
impl PreparedPortableProject {
    /// Small canonical descriptor; at most 1 MiB and independently parseable.
    pub fn root_bytes(&self) -> &[u8] {
        &self.root
    }
    /// Number of unique payload/shard objects, independent of logical record identity.
    pub fn object_count(&self) -> u64 {
        self.object_count
    }
    /// Sum of unique object bytes; actual archive members must also obey transport bounds.
    pub fn object_bytes(&self) -> u64 {
        self.object_bytes
    }
    /// Visit bounded object descriptors in deterministic digest order, without loading payloads.
    pub fn visit_objects(
        &self,
        visitor: &mut dyn FnMut(&PortableObjectReference) -> Result<()>,
    ) -> Result<()> {
        let mut statement = self
            .index
            .prepare("SELECT sha256,byte_size FROM objects ORDER BY sha256")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            visitor(&PortableObjectReference::new(
                &row.get::<_, String>(0)?,
                read_unsigned(row, 1)?,
            )?)?;
        }
        Ok(())
    }
    /// Stream one exact listed object with length and SHA-256 verification. No arbitrary path reads.
    pub fn copy_object(
        &self,
        object: &PortableObjectReference,
        writer: &mut dyn Write,
        cancel: &ProjectTransferCancellation,
    ) -> Result<()> {
        let size: Option<u64> = self
            .index
            .query_row(
                "SELECT byte_size FROM objects WHERE sha256=?1",
                [object.sha256()],
                |row| read_unsigned(row, 0),
            )
            .optional()?;
        if size != Some(object.byte_size()) {
            return invalid("Object is outside this prepared project capability");
        }
        let mut file = File::open(self.directory.path().join(object.sha256()))?;
        copy_checked(
            &mut file,
            writer,
            object.byte_size(),
            object.sha256(),
            cancel,
        )
    }
}
struct Builder {
    index: Connection,
    directory: TempDir,
    cancel: ProjectTransferCancellation,
    budget: PortableResourceBudget,
    shards: Vec<PortableShardDescriptor>,
    domain: Option<String>,
    records: u64,
    page: Vec<u8>,
}
impl Builder {
    fn new(cancel: ProjectTransferCancellation) -> Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("writercraft-portable-")
            .tempdir()?;
        let index = Connection::open(directory.path().join("index.sqlite"))?;
        index.execute_batch("CREATE TABLE objects(sha256 TEXT PRIMARY KEY,byte_size INTEGER NOT NULL); PRAGMA journal_mode=OFF; PRAGMA synchronous=OFF; PRAGMA cache_size=-2048;")?;
        Ok(Self {
            directory,
            index,
            cancel,
            budget: PortableResourceBudget::default(),
            shards: vec![],
            domain: None,
            records: 0,
            page: Vec::new(),
        })
    }
    fn bytes(&mut self, bytes: &[u8], limit: u64) -> Result<PortableObjectReference> {
        self.object(&mut std::io::Cursor::new(bytes), bytes.len() as u64, limit)
    }
    fn object(
        &mut self,
        reader: &mut dyn Read,
        size: u64,
        limit: u64,
    ) -> Result<PortableObjectReference> {
        self.cancel.check()?;
        if size > limit {
            return invalid("Portable payload exceeds its explicit domain size bound");
        }
        let mut temporary = tempfile::NamedTempFile::new_in(self.directory.path())?;
        let mut digest = Sha256::new();
        let mut remaining = size;
        let mut buffer = [0_u8; 64 * 1024];
        while remaining > 0 {
            self.cancel.check()?;
            let capacity = remaining.min(buffer.len() as u64) as usize;
            let count = reader.read(&mut buffer[..capacity])?;
            if count == 0 || count > capacity {
                return invalid("Portable object ended early or violated its bounded reader");
            }
            digest.update(&buffer[..count]);
            temporary.write_all(&buffer[..count])?;
            remaining -= count as u64;
        }
        self.cancel.check()?;
        if reader.read(&mut [0_u8; 1])? != 0 {
            return invalid("Portable object exceeds its declared length");
        }
        let sha = format!("{:x}", digest.finalize());
        let reference = PortableObjectReference::new(&sha, size)?;
        let existing: Option<u64> = self
            .index
            .query_row(
                "SELECT byte_size FROM objects WHERE sha256=?1",
                [&sha],
                |row| read_unsigned(row, 0),
            )
            .optional()?;
        if let Some(existing) = existing {
            if existing != size {
                return invalid("Content digest has conflicting portable object lengths");
            }
        } else {
            self.budget.account_unique_transport(size)?;
            self.budget.account_physical_member(size)?;
            temporary.flush()?;
            temporary
                .persist_noclobber(self.directory.path().join(&sha))
                .map_err(|error| error.error)?;
            self.index.execute(
                "INSERT INTO objects(sha256,byte_size) VALUES(?1,?2)",
                params![sha, super::sqlite_integer(size)?],
            )?;
        }
        Ok(reference)
    }
    fn record(&mut self, domain: &str, value: Value) -> Result<()> {
        self.cancel.check()?;
        let bytes = serde_json::to_vec(&value)?;
        if bytes.len() > MAX_PORTABLE_RECORD_BYTES {
            return invalid("Portable metadata record exceeds 256 KiB");
        }
        if self
            .domain
            .as_deref()
            .is_some_and(|previous| previous != domain)
            || self.page.len() + bytes.len() + 1 > MAX_PORTABLE_SHARD_BYTES
            || self.records >= MAX_PORTABLE_SHARD_RECORDS
        {
            self.flush()?;
        }
        self.domain = Some(domain.into());
        self.budget.account_records(1)?;
        self.page.extend(bytes);
        self.page.push(b'\n');
        self.records += 1;
        Ok(())
    }
    fn flush(&mut self) -> Result<()> {
        if self.records == 0 {
            return Ok(());
        }
        let page = std::mem::take(&mut self.page);
        let reference = self.bytes(&page, MAX_PORTABLE_SHARD_BYTES as u64)?;
        let domain = self
            .domain
            .take()
            .ok_or_else(|| StudioError::CorruptData("Portable page has no domain".into()))?;
        self.shards.push(PortableShardDescriptor::new(
            &domain,
            reference,
            self.records,
        )?);
        self.records = 0;
        if self.shards.len() > super::MAX_PORTABLE_SHARDS {
            return invalid("Portable project exceeds 4096 metadata shards");
        }
        Ok(())
    }
    fn finish(
        mut self,
        original: Option<PortableObjectReference>,
    ) -> Result<PreparedPortableProject> {
        self.flush()?;
        self.cancel.check()?;
        let root = PortableProjectHeader::new(
            PortableProjectScope::CompleteProject,
            original,
            self.shards,
        )?
        .to_bytes()?;
        self.budget.account_unique_transport(root.len() as u64)?;
        self.budget.account_physical_member(root.len() as u64)?;
        let object_count: u64 =
            self.index
                .query_row("SELECT count(*) FROM objects", [], |row| {
                    read_unsigned(row, 0)
                })?;
        let object_bytes: u64 = self.index.query_row(
            "SELECT coalesce(sum(byte_size),0) FROM objects",
            [],
            |row| read_unsigned(row, 0),
        )?;
        Ok(PreparedPortableProject {
            directory: self.directory,
            index: self.index,
            root,
            object_count,
            object_bytes,
        })
    }
}
fn copy_checked(
    reader: &mut dyn Read,
    writer: &mut dyn Write,
    size: u64,
    expected: &str,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    let mut remaining = size;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        cancel.check()?;
        let limit = remaining.min(buffer.len() as u64) as usize;
        let count = reader.read(&mut buffer[..limit])?;
        if count == 0 || count > limit {
            return invalid("Prepared portable object length changed");
        }
        writer.write_all(&buffer[..count])?;
        digest.update(&buffer[..count]);
        remaining -= count as u64;
    }
    cancel.check()?;
    if reader.read(&mut [0_u8; 1])? != 0 || format!("{:x}", digest.finalize()) != expected {
        return invalid("Prepared portable object length or checksum changed");
    }
    Ok(())
}
fn logical_document(connection: &Connection, project: &str, native: &str) -> Result<String> {
    Ok(connection
        .query_row(
            "SELECT logical_id FROM project_nodes WHERE project_id=?1 AND native_document_id=?2",
            params![project, native],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or_else(|| native.to_owned()))
}
fn export_project(connection: &Connection, project: &str, output: &mut Builder) -> Result<()> {
    let title: String =
        connection.query_row("SELECT title FROM projects WHERE id=?1", [project], |row| {
            row.get(0)
        })?;
    let logical: String = connection
        .query_row(
            "SELECT logical_project_id FROM project_portability WHERE project_id=?1",
            [project],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or_else(|| project.to_owned());
    output.record("project", json!({"logicalId":logical,"title":title}))
}
fn export_nodes(connection: &Connection, project: &str, output: &mut Builder) -> Result<()> {
    let mut statement = connection.prepare("SELECT n.logical_id,n.native_document_id,n.raw_json,n.ordinal,d.title,d.revision FROM project_nodes n LEFT JOIN documents d ON d.id=n.native_document_id AND d.project_id=n.project_id WHERE n.project_id=?1 ORDER BY n.ordinal")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        let raw: String = row.get(2)?;
        let metadata = output.bytes(raw.as_bytes(), super::MAX_PROJECT_MANIFEST_BYTES as u64)?;
        output.record("nodes",json!({"logicalId":row.get::<_,String>(0)?,"nativeHandle":row.get::<_,Option<String>>(1)?,"metadata":metadata,"ordinal":read_unsigned(row,3)?,"title":row.get::<_,Option<String>>(4)?,"currentRevision":row.get::<_,Option<i64>>(5)?}))?;
    }
    let mut statement=connection.prepare("SELECT d.id,d.title,d.revision FROM documents d WHERE d.project_id=?1 AND NOT EXISTS(SELECT 1 FROM project_nodes n WHERE n.project_id=d.project_id AND n.native_document_id=d.id) ORDER BY d.rowid")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        output.record("nodes",json!({"logicalId":row.get::<_,String>(0)?,"nativeHandle":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"currentRevision":read_unsigned(row,2)?,"nativeCreated":true}))?;
    }
    Ok(())
}
fn export_revisions(connection: &Connection, project: &str, output: &mut Builder) -> Result<()> {
    let mut statement=connection.prepare("SELECT d.id,r.revision,r.body,r.source,j.content_json FROM documents d JOIN document_revisions r ON r.document_id=d.id LEFT JOIN rich_document_revisions j ON j.document_id=r.document_id AND j.revision=r.revision WHERE d.project_id=?1 AND (r.source<>'import' OR NOT EXISTS(SELECT 1 FROM project_nodes n JOIN project_portability p ON p.project_id=n.project_id WHERE n.project_id=d.project_id AND n.native_document_id=d.id)) ORDER BY d.rowid,r.revision")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        let native: String = row.get(0)?;
        let body: String = row.get(2)?;
        let rich: Option<String> = row.get(4)?;
        let body = output.bytes(body.as_bytes(), 16 * 1024 * 1024)?;
        let rich = rich
            .map(|raw| output.bytes(raw.as_bytes(), 16 * 1024 * 1024))
            .transpose()?;
        output.record("documentRevisions",json!({"logicalDocumentId":logical_document(connection,project,&native)?,"revision":read_unsigned(row,1)?,"source":row.get::<_,String>(3)?,"body":body,"richContent":rich}))?;
    }
    let mut statement = connection.prepare(
        "SELECT id,source_json FROM documents WHERE project_id=?1 AND read_only=1 ORDER BY rowid",
    )?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        let native: String = row.get(0)?;
        let raw: String = row.get(1)?;
        let source = output.bytes(raw.as_bytes(), super::MAX_PROJECT_MANIFEST_BYTES as u64)?;
        output.record("opaqueDocuments",json!({"logicalDocumentId":logical_document(connection,project,&native)?,"original":source,"readOnly":true}))?;
    }
    Ok(())
}
fn export_notes(connection: &Connection, project: &str, output: &mut Builder) -> Result<()> {
    let mut statement=connection.prepare("SELECT a.id,a.document_id,a.annotation_json,p.logical_id,n.logical_id FROM document_annotations a JOIN documents d ON d.id=a.document_id LEFT JOIN project_annotation_projections p ON p.project_id=d.project_id AND p.native_annotation_id=a.id LEFT JOIN project_resource_aliases n ON n.project_id=d.project_id AND n.resource_kind='native-annotation' AND n.native_id=a.id WHERE d.project_id=?1 ORDER BY a.rowid")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        let native: String = row.get(0)?;
        let document: String = row.get(1)?;
        let raw: String = row.get(2)?;
        let state = output.bytes(raw.as_bytes(), super::MAX_PROJECT_MANIFEST_BYTES as u64)?;
        output.record("nativeAnnotations",json!({"nativeHandle":native,"logicalSourceId":row.get::<_,Option<String>>(3)?,"logicalNativeId":row.get::<_,Option<String>>(4)?.unwrap_or(native),"logicalDocumentId":logical_document(connection,project,&document)?,"state":state}))?;
    }
    let mut statement=connection.prepare("SELECT h.annotation_id,a.document_id,h.annotation_revision,h.document_revision,h.source,h.before_json,h.after_json FROM document_annotation_history h JOIN document_annotations a ON a.id=h.annotation_id JOIN documents d ON d.id=a.document_id WHERE d.project_id=?1 ORDER BY h.id")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        let document: String = row.get(1)?;
        let before: Option<String> = row.get(5)?;
        let after: String = row.get(6)?;
        let before = before
            .map(|raw| output.bytes(raw.as_bytes(), super::MAX_PROJECT_MANIFEST_BYTES as u64))
            .transpose()?;
        let after = output.bytes(after.as_bytes(), super::MAX_PROJECT_MANIFEST_BYTES as u64)?;
        output.record("nativeAnnotationHistory",json!({"nativeHandle":row.get::<_,String>(0)?,"logicalDocumentId":logical_document(connection,project,&document)?,"revision":read_unsigned(row,2)?,"documentRevision":read_unsigned(row,3)?,"source":row.get::<_,String>(4)?,"before":before,"after":after}))?;
    }
    // Historical attachment snapshots are retained bytes, not native replay/receipt authority.
    export_json_rows(
        connection,
        project,
        output,
        "attachmentEvidence",
        "SELECT json_object('documentId',s.document_id,'documentRevision',s.document_revision,'annotationId',s.annotation_id,'attachmentRaw',s.attachment_json) FROM document_annotation_snapshots s JOIN documents d ON d.id=s.document_id WHERE d.project_id=?1 ORDER BY s.document_id,s.document_revision,s.annotation_id",
    )
}
fn export_canvases(connection: &Connection, project: &str, output: &mut Builder) -> Result<()> {
    export_json_rows(
        connection,
        project,
        output,
        "canvases",
        "SELECT json_object('logicalId',logical_id,'currentRevision',current_revision,'metadataRaw',raw_json) FROM project_canvases WHERE project_id=?1 ORDER BY rowid",
    )?;
    let mut source_bases = std::collections::HashMap::new();
    let mut source_statement = connection.prepare("SELECT json_extract(raw_json,'$.id'),json_extract(raw_json,'$.currentRevision') FROM project_source_records WHERE project_id=?1 AND domain='canvases'")?;
    let mut source_rows = source_statement.query([project])?;
    while let Some(row) = source_rows.next()? {
        source_bases.insert(row.get::<_, String>(0)?, read_unsigned(row, 1)?);
    }
    let mut statement=connection.prepare("SELECT canvas_id,revision,base_revision,schema_version,content_json,raw_json FROM project_canvas_revisions WHERE project_id=?1 ORDER BY canvas_id,revision")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        let canvas_id: String = row.get(0)?;
        if source_bases
            .get(&canvas_id)
            .is_some_and(|base| read_unsigned(row, 1).is_ok_and(|revision| revision <= *base))
        {
            continue;
        }
        let content: String = row.get(4)?;
        let raw: String = row.get(5)?;
        let content = output.bytes(content.as_bytes(), 12 * 1024 * 1024)?;
        let mut metadata: std::collections::BTreeMap<String, Box<serde_json::value::RawValue>> =
            serde_json::from_str(&raw)?;
        metadata.remove("content");
        let metadata = output.bytes(
            &serde_json::to_vec(&metadata)?,
            super::MAX_PROJECT_MANIFEST_BYTES as u64,
        )?;
        output.record("canvasRevisions",json!({"logicalCanvasId":row.get::<_,String>(0)?,"revision":read_unsigned(row,1)?,"baseRevision":read_unsigned(row,2)?,"schemaVersion":read_unsigned(row,3)?,"content":content,"metadata":metadata}))?;
    }
    Ok(())
}
fn export_recovery(connection: &Connection, project: &str, output: &mut Builder) -> Result<()> {
    let mut statement=connection.prepare("SELECT w.document_id,w.base_revision,w.body FROM working_drafts w JOIN documents d ON d.id=w.document_id WHERE d.project_id=?1 ORDER BY w.document_id")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        let document: String = row.get(0)?;
        let body: String = row.get(2)?;
        let body = output.bytes(body.as_bytes(), 16 * 1024 * 1024)?;
        output.record("proseDraftEvidence",json!({"logicalDocumentId":logical_document(connection,project,&document)?,"baseRevision":read_unsigned(row,1)?,"body":body,"authority":"untrusted-on-import"}))?;
    }
    let mut statement=connection.prepare("SELECT w.document_id,w.base_revision,w.version,w.content_json,a.checkpoint_json FROM rich_working_drafts w JOIN documents d ON d.id=w.document_id LEFT JOIN rich_draft_annotations a ON a.document_id=w.document_id AND a.draft_version=w.version WHERE d.project_id=?1 ORDER BY w.document_id")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        let document: String = row.get(0)?;
        let raw: String = row.get(3)?;
        let checkpoint: Option<String> = row.get(4)?;
        let content = output.bytes(raw.as_bytes(), 16 * 1024 * 1024)?;
        let checkpoint = checkpoint
            .map(|raw| output.bytes(raw.as_bytes(), super::MAX_EVIDENCE_OBJECT_BYTES))
            .transpose()?;
        output.record("richDraftEvidence",json!({"logicalDocumentId":logical_document(connection,project,&document)?,"baseRevision":read_unsigned(row,1)?,"version":row.get::<_,String>(2)?,"content":content,"checkpointEvidence":checkpoint,"authority":"untrusted-on-import"}))?;
    }
    let mut statement=connection.prepare("SELECT canvas_id,base_revision,version,content_json FROM project_canvas_drafts WHERE project_id=?1 ORDER BY rowid")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        let raw: String = row.get(3)?;
        let content = output.bytes(raw.as_bytes(), 12 * 1024 * 1024)?;
        output.record("canvasDraftEvidence",json!({"logicalCanvasId":row.get::<_,String>(0)?,"baseRevision":read_unsigned(row,1)?,"version":row.get::<_,String>(2)?,"content":content,"authority":"untrusted-on-import"}))?;
    }
    export_json_rows(
        connection,
        project,
        output,
        "reviewEvidence",
        "SELECT json_object('id',p.id,'documentId',p.document_id,'baseRevision',p.base_revision,'before',p.before_body,'after',p.after_body,'reason',p.reason,'status',p.status,'appliedRevision',p.applied_revision,'undoneRevision',p.undone_revision,'anchorRaw',p.anchor_json,'beforeHash',p.before_hash,'afterHash',p.after_hash) FROM proposals p JOIN documents d ON d.id=p.document_id WHERE d.project_id=?1 ORDER BY p.rowid",
    )?;
    let mut statement=connection.prepare("SELECT p.id,p.document_id,p.base_revision,p.status,p.before_json,p.after_json,p.command_json,p.reason,p.applied_revision,p.undone_revision FROM rich_proposals p JOIN documents d ON d.id=p.document_id WHERE d.project_id=?1 ORDER BY p.rowid")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        let document: String = row.get(1)?;
        let before: String = row.get(4)?;
        let after: String = row.get(5)?;
        let command: String = row.get(6)?;
        let before = output.bytes(before.as_bytes(), 16 * 1024 * 1024)?;
        let after = output.bytes(after.as_bytes(), 16 * 1024 * 1024)?;
        let command = output.bytes(command.as_bytes(), super::MAX_EVIDENCE_OBJECT_BYTES)?;
        output.record("richReviewEvidence",json!({"logicalDocumentId":logical_document(connection,project,&document)?,"logicalId":row.get::<_,String>(0)?,"baseRevision":read_unsigned(row,2)?,"status":row.get::<_,String>(3)?,"before":before,"after":after,"command":command,"reason":row.get::<_,String>(7)?,"appliedRevision":row.get::<_,Option<i64>>(8)?,"undoneRevision":row.get::<_,Option<i64>>(9)?,"authority":"untrusted-on-import"}))?;
    }
    Ok(())
}

fn export_json_rows(
    connection: &Connection,
    project: &str,
    output: &mut Builder,
    domain: &str,
    query: &str,
) -> Result<()> {
    let mut statement = connection.prepare(query)?;
    let mut rows = statement.query([project])?;
    let mut ordinal = 0_u64;
    while let Some(row) = rows.next()? {
        let raw: String = row.get(0)?;
        let payload = output.bytes(raw.as_bytes(), super::MAX_EVIDENCE_OBJECT_BYTES)?;
        output.record(
            domain,
            json!({"ordinal":ordinal,"payload":payload,"authority":"untrusted-on-import"}),
        )?;
        ordinal += 1;
    }
    Ok(())
}
fn export_assets_and_evidence(
    connection: &Connection,
    project: &str,
    output: &mut Builder,
) -> Result<()> {
    let mut statement=connection.prepare("SELECT a.rowid,a.id,a.sha256,a.byte_size,a.media_type,a.original_name,a.width,a.height,m.resource_kind,m.logical_id,m.archive_path,m.raw_json FROM project_assets a LEFT JOIN project_asset_metadata m ON m.native_asset_id=a.id AND m.project_id=a.project_id WHERE a.project_id=?1 ORDER BY a.rowid")?;
    let mut rows = statement.query([project])?;
    let mut aggregate = 0_u64;
    while let Some(row) = rows.next()? {
        let rowid: i64 = row.get(0)?;
        let size = read_unsigned(row, 3)?;
        aggregate = aggregate
            .checked_add(size)
            .filter(|total| *total <= super::MAX_PROJECT_TRANSFER_BYTES)
            .ok_or_else(|| {
                StudioError::Validation(
                    "Portable original artifacts exceed their 512 MiB logical budget".into(),
                )
            })?;
        let mut blob = connection.blob_open(MAIN_DB, "project_assets", "data", rowid, true)?;
        let object = output.object(&mut blob, size, super::MAX_PROJECT_TRANSFER_BYTES)?;
        if object.sha256() != row.get::<_, String>(2)? {
            return invalid("Original artifact checksum differs from saved metadata");
        }
        let native: String = row.get(1)?;
        let source_metadata: Option<String> = row.get(11)?;
        let source_metadata = source_metadata
            .map(|raw| output.bytes(raw.as_bytes(), super::MAX_PROJECT_MANIFEST_BYTES as u64))
            .transpose()?;
        output.record("assets",json!({"logicalId":row.get::<_,Option<String>>(9)?.unwrap_or(native),"resourceKind":row.get::<_,Option<String>>(8)?.unwrap_or_else(||"media".into()),"mediaType":row.get::<_,String>(4)?,"originalName":row.get::<_,String>(5)?,"width":row.get::<_,Option<u32>>(6)?,"height":row.get::<_,Option<u32>>(7)?,"archivePath":row.get::<_,Option<String>>(10)?,"sourceMetadata":source_metadata,"object":object}))?;
    }
    let mut statement=connection.prepare("SELECT r.domain,r.logical_id,r.logical_document_id,r.logical_canvas_id,r.metadata_json,o.rowid,o.sha256,o.byte_size,o.media_type FROM project_evidence_references r JOIN project_evidence_objects o ON o.project_id=r.project_id AND o.sha256=r.object_sha256 WHERE r.project_id=?1 ORDER BY r.domain,r.logical_id")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        let metadata: String = row.get(4)?;
        let metadata = output.bytes(
            metadata.as_bytes(),
            super::MAX_EVIDENCE_METADATA_BYTES as u64,
        )?;
        let mut blob = connection.blob_open(
            MAIN_DB,
            "project_evidence_objects",
            "data",
            row.get(5)?,
            true,
        )?;
        let object = output.object(
            &mut blob,
            read_unsigned(row, 7)?,
            super::MAX_EVIDENCE_OBJECT_BYTES,
        )?;
        if object.sha256() != row.get::<_, String>(6)? {
            return invalid("Retained evidence checksum differs from saved metadata");
        }
        output.record("evidence",json!({"domain":row.get::<_,String>(0)?,"logicalId":row.get::<_,String>(1)?,"logicalDocumentId":row.get::<_,Option<String>>(2)?,"logicalCanvasId":row.get::<_,Option<String>>(3)?,"metadata":metadata,"object":object,"mediaType":row.get::<_,String>(8)?}))?;
    }
    Ok(())
}
