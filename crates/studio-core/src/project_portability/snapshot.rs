//! A revision-consistent export owns source records and immutable scoped asset capabilities.
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, MAIN_DB, OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::json::invalid;
use super::validation::{array, number, string};
use super::{
    ProjectAttachment, ProjectTransferCancellation, Result, StudioError, Workspace, require_project,
};
use super::{read_optional_unsigned, read_unsigned};

#[derive(Clone)]
struct CapturedAsset {
    rowid: i64,
    descriptor: ProjectAttachment,
}

/// Scoped immutable originals captured by one project export; no ambient path or URL lookup.
#[derive(Clone)]
pub struct ProjectAssetSource {
    connection: Arc<Mutex<Connection>>,
    project_id: String,
    assets: HashMap<String, CapturedAsset>,
}
impl ProjectAssetSource {
    /// Owning project namespace; source identities cannot change it.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    /// Copy only a captured exact descriptor, checking ownership, length and hash again.
    pub fn copy_to(
        &self,
        attachment: &ProjectAttachment,
        writer: &mut dyn Write,
        cancel: &ProjectTransferCancellation,
    ) -> Result<()> {
        let captured = self
            .assets
            .get(&attachment.path)
            .filter(|asset| asset.descriptor == *attachment)
            .ok_or_else(|| {
                StudioError::Validation(
                    "Attachment is outside this captured project snapshot".into(),
                )
            })?;
        cancel.check()?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StudioError::LockUnavailable)?;
        let metadata:(String,u64,String)=connection.query_row("SELECT sha256,byte_size,media_type FROM project_assets WHERE rowid=?1 AND project_id=?2",params![captured.rowid,self.project_id],|row|Ok((row.get(0)?,read_unsigned(row,1)?,row.get(2)?)))?;
        if metadata
            != (
                attachment.sha256.clone(),
                attachment.byte_size,
                attachment.media_type.clone(),
            )
        {
            return Err(StudioError::CorruptData(
                "Export attachment metadata changed after capture".into(),
            ));
        }
        let mut blob =
            connection.blob_open(MAIN_DB, "project_assets", "data", captured.rowid, true)?;
        if blob.len() as u64 != attachment.byte_size {
            return Err(StudioError::CorruptData(
                "Export attachment length differs from its snapshot".into(),
            ));
        }
        let mut digest = Sha256::new();
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            cancel.check()?;
            let count = blob.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
            writer.write_all(&buffer[..count])?;
        }
        if !format!("{:x}", digest.finalize()).eq_ignore_ascii_case(&attachment.sha256) {
            return Err(StudioError::CorruptData(
                "Export attachment checksum differs from its snapshot".into(),
            ));
        }
        Ok(())
    }
    /// Load one authorized original for a decoder; callers retain explicit decoder/output budgets.
    pub fn load(&self, logical_id: &str, cancel: &ProjectTransferCancellation) -> Result<Vec<u8>> {
        let asset = self
            .assets
            .values()
            .find(|asset| {
                asset.descriptor.resource_kind == "media"
                    && asset.descriptor.logical_id == logical_id
            })
            .ok_or_else(|| StudioError::NotFound {
                kind: "project asset",
                id: logical_id.to_owned(),
            })?;
        let count = usize::try_from(asset.descriptor.byte_size)
            .map_err(|_| StudioError::Validation("Asset exceeds decoder input capacity".into()))?;
        let mut output = Vec::new();
        output.try_reserve_exact(count).map_err(|_| {
            StudioError::Validation(
                "Not enough memory to preview this original; export remains available".into(),
            )
        })?;
        self.copy_to(&asset.descriptor, &mut output, cancel)?;
        Ok(output)
    }
}

/// One transaction-consistent complete project snapshot. It cannot be forged by deserialization.
pub struct ProjectExportSnapshot {
    project_id: String,
    manifest: Value,
    manifest_bytes: Vec<u8>,
    recovery: Value,
    native_state: Value,
    requires_companion: bool,
    retained_evidence: bool,
    attachments: Vec<ProjectAttachment>,
    assets: ProjectAssetSource,
}
impl ProjectExportSnapshot {
    /// Native owning namespace captured by this snapshot.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    /// Complete source-compatible canonical manifest, preserving original logical identities.
    pub fn manifest(&self) -> &Value {
        &self.manifest
    }
    /// Exact original manifest bytes when unchanged; precision-preserving JSON after known edits.
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }
    /// Durable unfinished content/evidence, separate from saved Markdown and never replay authority.
    pub fn recovery(&self) -> &Value {
        &self.recovery
    }
    /// Captured native notes, aliases, history and opaque source; imported JSON is untrusted evidence.
    /// This read-only view avoids serializing or parsing unrelated source writing for scoped exports.
    pub fn native_state(&self) -> &Value {
        &self.native_state
    }
    /// Whether a legacy-only export would omit changed native notes, recovery or review state.
    pub fn requires_canonical_companion(&self) -> bool {
        self.requires_companion
    }
    /// Whether flat imported native/unknown evidence requires the format-2 streaming export.
    pub fn has_retained_evidence(&self) -> bool {
        self.retained_evidence
    }
    /// Historical format-1 companion compatibility; imported flat evidence requires format 2.
    pub fn canonical_manifest_bytes(&self) -> Result<Vec<u8>> {
        if self.retained_evidence {
            return invalid(
                "This project contains retained native evidence; use native canonical format-2 export or a whole-workspace backup",
            );
        }
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Canonical<'a> {
            format: &'static str,
            format_version: u32,
            scope: &'static str,
            source_manifest: &'a serde_json::value::RawValue,
            source_manifest_raw: &'a str,
            source_manifest_sha256: String,
            native_state: &'a Value,
            recovery_evidence: &'a Value,
        }
        let source: Box<serde_json::value::RawValue> =
            serde_json::from_slice(&self.manifest_bytes)?;
        Ok(serde_json::to_vec_pretty(&Canonical {
            format: "writercraft-portable-project",
            format_version: 1,
            scope: "complete-project",
            source_manifest: &source,
            source_manifest_raw: std::str::from_utf8(&self.manifest_bytes)
                .map_err(|_| StudioError::CorruptData("Captured manifest is not UTF-8".into()))?,
            source_manifest_sha256: format!("{:x}", Sha256::digest(&self.manifest_bytes)),
            native_state: &self.native_state,
            recovery_evidence: &self.recovery,
        })?)
    }
    /// Every original or native project artifact, including unattached history-referenced resources.
    pub fn attachments(&self) -> &[ProjectAttachment] {
        &self.attachments
    }
    /// Authorized byte capability for only the captured project and descriptors.
    pub fn assets(&self) -> &ProjectAssetSource {
        &self.assets
    }
}

impl Workspace {
    /// Capture only one same-project media resource, without reading manuscripts or other assets.
    ///
    /// Source logical aliases and native-created media handles are scoped to this project. The
    /// returned descriptor is the exact identity accepted by the one-object byte capability.
    /// Open a separate media-worker Workspace before calling this method: subsequent copy/load
    /// operations intentionally hold that worker's own connection while verifying immutable bytes.
    /// Non-image originals retain their existing MIME-specific bounds, never the image 12 MiB cap.
    pub fn project_media_source(
        &self,
        project: &str,
        logical_id: &str,
    ) -> Result<(ProjectAttachment, ProjectAssetSource)> {
        let connection = self.connection()?;
        require_project(&connection, project)?;
        let mut statement = connection.prepare("SELECT a.rowid,a.id,a.sha256,a.byte_size,a.media_type,a.original_name,a.width,a.height,m.logical_id,m.archive_path FROM project_assets a LEFT JOIN project_asset_metadata m ON m.project_id=a.project_id AND m.native_asset_id=a.id WHERE a.project_id=?1 AND (m.resource_kind IS NULL OR m.resource_kind='media') AND (m.logical_id=?2 OR a.id=?2) LIMIT 2")?;
        let rows = statement
            .query_map(params![project, logical_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    read_unsigned(row, 3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<u32>>(6)?,
                    row.get::<_, Option<u32>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if rows.len() > 1 {
            return invalid(
                "Media identity is ambiguous inside this project; select its explicit logical reference",
            );
        }
        let (rowid, native, sha, size, mime, name, width, height, logical, path) = rows
            .into_iter()
            .next()
            .ok_or_else(|| StudioError::NotFound {
                kind: "project media",
                id: logical_id.to_owned(),
            })?;
        if size > super::MAX_PROJECT_TRANSFER_BYTES {
            return invalid("Media original exceeds the explicit 512 MiB resource bound");
        }
        let logical = logical.unwrap_or(native);
        let path = path.unwrap_or_else(|| format!("media/native-{}{}", logical, extension(&mime)));
        let descriptor = ProjectAttachment {
            resource_kind: "media".into(),
            logical_id: logical,
            path: path.clone(),
            sha256: sha,
            byte_size: size,
            media_type: mime,
            original_name: name,
            width,
            height,
        };
        let assets = ProjectAssetSource {
            connection: Arc::clone(&self.connection),
            project_id: project.to_owned(),
            assets: HashMap::from([(
                path,
                CapturedAsset {
                    rowid,
                    descriptor: descriptor.clone(),
                },
            )]),
        };
        Ok((descriptor, assets))
    }
    /// Capture canonical records, metadata, histories and recovery in one read transaction.
    /// Run sizeable captures and subsequent asset copies on a separately opened worker Workspace.
    pub fn project_export_snapshot(&self, project: &str) -> Result<ProjectExportSnapshot> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        require_project(&transaction, project)?;
        let title: String =
            transaction.query_row("SELECT title FROM projects WHERE id=?1", [project], |row| {
                row.get(0)
            })?;
        let original: Option<(String, String)> = transaction
            .query_row(
                "SELECT raw_manifest,manifest_sha256 FROM project_portability WHERE project_id=?1",
                [project],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let original = original
            .map(|(raw, hash)| -> Result<String> {
                if !format!("{:x}", Sha256::digest(raw.as_bytes())).eq_ignore_ascii_case(&hash) {
                    return Err(StudioError::CorruptData(
                        "Retained source manifest checksum changed".into(),
                    ));
                }
                Ok(raw)
            })
            .transpose()?;
        let mut manifest: Value = match &original {
            Some(raw) => serde_json::from_str(raw)?,
            None => native_manifest(project, &title),
        };
        manifest["project"]["title"] = Value::String(title);
        let mut node_rows=transaction.prepare("SELECT logical_id,native_document_id,raw_json FROM project_nodes WHERE project_id=?1 ORDER BY ordinal")?;
        let nodes = node_rows
            .query_map([project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(node_rows);
        let mut node_map = HashMap::new();
        let mut output_nodes = Vec::new();
        for (logical, native, raw) in nodes {
            let mut node: Value = serde_json::from_str(&raw)?;
            if let Some(native) = native {
                let (title, revision): (String, u64) = transaction.query_row(
                    "SELECT title,revision FROM documents WHERE id=?1 AND project_id=?2",
                    params![native, project],
                    |row| Ok((row.get(0)?, read_unsigned(row, 1)?)),
                )?;
                node["title"] = Value::String(title);
                node["currentRevision"] = json!(revision);
                node_map.insert(native, logical);
            }
            output_nodes.push(node);
        }
        let mut document_statement = transaction.prepare(
            "SELECT id,title,revision FROM documents WHERE project_id=?1 ORDER BY rowid",
        )?;
        let documents = document_statement
            .query_map([project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    read_unsigned(row, 2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(document_statement);
        let content_kind = array(&manifest["project"]["structure"], "nodeTypes")?
            .iter()
            .find(|kind| {
                kind["contentBearing"].as_bool() == Some(true)
                    && kind["canBeRoot"].as_bool() == Some(true)
            })
            .and_then(|kind| kind["key"].as_str())
            .unwrap_or("document")
            .to_owned();
        for (native, title, revision) in documents {
            if node_map.contains_key(&native) {
                continue;
            }
            output_nodes.push(json!({"id":native,"kind":content_kind,"title":title,"status":"draft","position":output_nodes.len(),"currentRevision":revision,"createdAt":"0001-01-01T00:00:00Z","updatedAt":"0001-01-01T00:00:00Z"}));
            node_map.insert(native.clone(), native);
        }
        manifest["nodes"] = Value::Array(output_nodes);
        let originals = array(&manifest, "documentRevisions")?;
        let mut revision_map: HashMap<(String, u64), Value> = HashMap::new();
        for revision in originals {
            revision_map.insert(
                (
                    string(revision, "nodeId")?.to_owned(),
                    number(revision, "revision")?,
                ),
                revision.clone(),
            );
        }
        let mut output_revisions = originals.to_vec();
        let mut statement=transaction.prepare("SELECT d.id,r.revision,r.body,r.source,j.content_json FROM documents d JOIN document_revisions r ON r.document_id=d.id LEFT JOIN rich_document_revisions j ON j.document_id=r.document_id AND j.revision=r.revision WHERE d.project_id=?1 ORDER BY d.rowid,r.revision")?;
        let revisions = statement
            .query_map([project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    read_unsigned(row, 1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(statement);
        let mut unknown_revision_times = Vec::new();
        for (native, ordinal, body, source, rich) in revisions {
            let logical = node_map.get(&native).ok_or_else(|| {
                StudioError::CorruptData("Export document alias is missing".into())
            })?;
            let content: Value = match rich {
                Some(raw) => serde_json::from_str(&raw)?,
                None => craft_editor_model::Document::plain(format!("native-{native}"), &body)
                    .map_err(|e| {
                        StudioError::Validation(format!(
                            "Cannot project native prose for export: {e}"
                        ))
                    })?
                    .to_json()
                    .map_err(|e| {
                        StudioError::Validation(format!("Cannot preserve native prose: {e}"))
                    })?,
            };
            if let Some(original) = revision_map.get(&(logical.clone(), ordinal)) {
                if original["content"] != content
                    || original["plainText"].as_str() != Some(body.as_str())
                {
                    return Err(StudioError::CorruptData(
                        "An imported immutable revision differs from its retained source".into(),
                    ));
                }
            } else {
                let id = format!("writercraft:{native}:{ordinal}");
                output_revisions.push(json!({"id":id,"nodeId":logical,"revision":ordinal,"baseRevision":ordinal.saturating_sub(1),"schemaVersion":content["attrs"]["schemaVersion"].as_u64().unwrap_or(1),"content":content,"plainText":body,"source":source,"createdAt":"0001-01-01T00:00:00Z"}));
                unknown_revision_times.push(json!({"nodeId":logical,"revision":ordinal}));
            }
        }
        manifest["documentRevisions"] = Value::Array(output_revisions);
        refresh_canvases(&transaction, project, &mut manifest)?;
        let (attachments, assets) = capture_assets(&transaction, project, &mut manifest)?;
        let mut drafts=transaction.prepare("SELECT d.id,w.base_revision,w.body FROM documents d JOIN working_drafts w ON w.document_id=d.id WHERE d.project_id=?1")?;
        let prose=drafts.query_map([project],|row|Ok(json!({"documentId":row.get::<_,String>(0)?,"baseRevision":read_unsigned(row,1)?,"body":row.get::<_,String>(2)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        drop(drafts);
        let mut drafts=transaction.prepare("SELECT d.id,w.base_revision,w.version,w.content_json,a.checkpoint_json FROM documents d JOIN rich_working_drafts w ON w.document_id=d.id LEFT JOIN rich_draft_annotations a ON a.document_id=w.document_id AND a.draft_version=w.version WHERE d.project_id=?1")?;
        let rich = drafts
            .query_map([project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    read_unsigned(row, 1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(drafts);
        let rich=rich.into_iter().map(|(document,base,version,content,evidence)|->Result<Value>{Ok(json!({"documentId":document,"baseRevision":base,"version":version,"content":serde_json::from_str::<Value>(&content)?,"checkpointEvidence":evidence.map(|raw|serde_json::from_str::<Value>(&raw)).transpose()?}))}).collect::<Result<Vec<_>>>()?;
        let recovery = json!({"format":"writercraft-recovery-evidence","formatVersion":1,"authority":"untrusted-on-import","proseDrafts":&prose,"richDrafts":&rich,"unknownRevisionTimes":unknown_revision_times});
        capture_saved_canvases(&transaction, project, &mut manifest)?;
        let (mut native_state, native_changed) = capture_native_state(&transaction, project)?;
        native_state["documentAliases"] = serde_json::to_value(&node_map)?;
        let retained_evidence: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM project_evidence_references WHERE project_id=?1)",
            [project],
            |row| row.get(0),
        )?;
        let requires_companion =
            native_changed || !prose.is_empty() || !rich.is_empty() || retained_evidence;
        let manifest_bytes = match original {
            Some(raw) => super::json::encode_preserving_manifest(&raw, &manifest)?,
            None => serde_json::to_vec_pretty(&manifest)?,
        };
        transaction.commit()?;
        Ok(ProjectExportSnapshot {
            project_id: project.to_owned(),
            manifest,
            manifest_bytes,
            recovery,
            native_state,
            requires_companion,
            retained_evidence,
            attachments,
            assets: ProjectAssetSource {
                connection: Arc::clone(&self.connection),
                project_id: project.to_owned(),
                assets,
            },
        })
    }
}
pub(super) fn native_manifest(project: &str, title: &str) -> Value {
    json!({"format":"writers-studio-project","formatVersion":8,"exportedAt":"0001-01-01T00:00:00Z","project":{"id":project,"title":title,"kind":"free","language":"ru","synopsis":"","structureRevision":1,"structure":{"schemaVersion":1,"template":"free","rootLabel":"Manuscript","nodeTypes":[{"key":"document","label":"Document","pluralLabel":"Documents","icon":"file-text","contentBearing":true,"allowedChildren":["document"],"canBeRoot":true}]},"createdAt":"0001-01-01T00:00:00Z","updatedAt":"0001-01-01T00:00:00Z"},"nodes":[],"documentRevisions":[],"namedVersions":[],"dictionaryTerms":[],"documentAnnotations":[],"dictationSessions":[],"transcriptArtifacts":[],"transcriptDrafts":[],"transcriptDraftRevisions":[],"audio":[],"media":[],"canvases":[],"canvasRevisions":[],"projectMemory":[],"projectMemoryRevisions":[]})
}
fn capture_assets(
    connection: &Connection,
    project: &str,
    manifest: &mut Value,
) -> Result<(Vec<ProjectAttachment>, HashMap<String, CapturedAsset>)> {
    let mut statement=connection.prepare("SELECT a.rowid,a.id,a.sha256,a.byte_size,a.media_type,a.original_name,a.width,a.height,m.resource_kind,m.logical_id,m.archive_path,m.raw_json FROM project_assets a LEFT JOIN project_asset_metadata m ON m.native_asset_id=a.id AND m.project_id=a.project_id WHERE a.project_id=?1 ORDER BY a.rowid")?;
    let rows = statement
        .query_map([project], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                read_unsigned(row, 3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<u32>>(6)?,
                row.get::<_, Option<u32>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut attachments = Vec::with_capacity(rows.len());
    let mut assets = HashMap::with_capacity(rows.len());
    let mut paths: HashSet<String> = array(manifest, "media")?
        .iter()
        .chain(array(manifest, "audio")?.iter())
        .filter_map(|item| item["path"].as_str())
        .map(str::to_lowercase)
        .collect();
    let mut declared = HashSet::new();
    for (domain, key) in [("media", "id"), ("audio", "sessionId")] {
        for record in array(manifest, domain)? {
            declared.insert((domain.to_owned(), string(record, key)?.to_owned()));
        }
    }
    let mut appended: HashMap<String, Vec<Value>> = HashMap::new();
    for (rowid, native, sha, size, mime, name, width, height, kind, logical, path, raw) in rows {
        let kind = kind.unwrap_or_else(|| "media".into());
        let logical = logical.unwrap_or(native);
        let path = path.unwrap_or_else(|| format!("media/native-{logical}{}", extension(&mime)));
        let original = raw
            .map(|raw| serde_json::from_str::<Value>(&raw))
            .transpose()?;
        if let Some(original) = original.as_ref()
            && (original["sha256"]
                .as_str()
                .is_none_or(|value| !value.eq_ignore_ascii_case(&sha))
                || original["byteSize"].as_u64() != Some(size)
                || original["mediaType"].as_str() != Some(mime.as_str()))
        {
            return Err(StudioError::CorruptData(
                "Original project asset metadata changed".into(),
            ));
        }
        if !declared.contains(&(kind.clone(), logical.clone())) {
            if !paths.insert(path.to_lowercase()) {
                return invalid("native export asset path collides with retained source");
            }
            let mut record=original.unwrap_or_else(||json!({"path":path,"sha256":sha,"byteSize":size,"mediaType":mime,"originalName":name,"width":width,"height":height}));
            let key = match kind.as_str() {
                "media" => "id",
                "audio" => "sessionId",
                _ => return invalid("Unknown captured artifact resource kind"),
            };
            if record
                .get(key)
                .is_some_and(|id| id.as_str() != Some(logical.as_str()))
                || record["path"].as_str() != Some(path.as_str())
            {
                return invalid("Native artifact identity/path differs from its retained metadata");
            }
            record[key] = json!(logical);
            appended.entry(kind.clone()).or_default().push(record);
            manifest["formatVersion"] = json!(8);
        }
        let descriptor = ProjectAttachment {
            resource_kind: kind,
            logical_id: logical,
            path: path.clone(),
            sha256: sha,
            byte_size: size,
            media_type: mime,
            original_name: name,
            width,
            height,
        };
        attachments.push(descriptor.clone());
        if assets
            .insert(path, CapturedAsset { rowid, descriptor })
            .is_some()
        {
            return invalid("duplicate export attachment path");
        }
    }
    for (domain, records) in appended {
        let mut all = array(manifest, &domain)?.to_vec();
        all.extend(records);
        manifest[&domain] = Value::Array(all);
    }
    for domain in ["media", "audio"] {
        for record in array(manifest, domain)? {
            let path = string(record, "path")?;
            let captured = assets.get(path).ok_or_else(|| {
                StudioError::CorruptData(
                    "A declared project artifact is missing from owned storage".into(),
                )
            })?;
            let expected_id = string(record, if domain == "audio" { "sessionId" } else { "id" })?;
            if captured.descriptor.resource_kind != domain
                || captured.descriptor.logical_id != expected_id
                || captured.descriptor.byte_size != number(record, "byteSize")?
                || captured.descriptor.media_type != string(record, "mediaType")?
                || !captured
                    .descriptor
                    .sha256
                    .eq_ignore_ascii_case(string(record, "sha256")?)
            {
                return Err(StudioError::CorruptData(
                    "A declared project artifact differs from captured owned storage".into(),
                ));
            }
        }
    }
    Ok((attachments, assets))
}
fn extension(mime: &str) -> &str {
    match mime {
        "image/png" => ".png",
        "image/jpeg" => ".jpg",
        "image/gif" => ".gif",
        "image/webp" => ".webp",
        "application/pdf" => ".pdf",
        "audio/mpeg" => ".mp3",
        "audio/mp4" => ".m4a",
        "audio/ogg" => ".ogg",
        "audio/wav" => ".wav",
        "video/mp4" => ".mp4",
        "video/webm" => ".webm",
        _ => ".bin",
    }
}
fn refresh_canvases(connection: &Connection, project: &str, manifest: &mut Value) -> Result<()> {
    let mut statement = connection.prepare(
        "SELECT raw_json,current_revision FROM project_canvases WHERE project_id=?1 ORDER BY rowid",
    )?;
    let rows = statement
        .query_map([project], |row| {
            Ok((row.get::<_, String>(0)?, read_unsigned(row, 1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if !rows.is_empty() {
        let canvases = rows
            .into_iter()
            .map(|(raw, revision)| -> Result<Value> {
                let mut value: Value = serde_json::from_str(&raw)?;
                value["currentRevision"] = json!(revision);
                Ok(value)
            })
            .collect::<Result<Vec<_>>>()?;
        manifest["canvases"] = Value::Array(canvases);
    }
    let mut statement = connection.prepare(
        "SELECT raw_json FROM project_canvas_revisions WHERE project_id=?1 ORDER BY rowid",
    )?;
    let rows = statement
        .query_map([project], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if !rows.is_empty() {
        manifest["canvasRevisions"] = Value::Array(
            rows.iter()
                .map(|raw| serde_json::from_str(raw).map_err(StudioError::from))
                .collect::<Result<Vec<Value>>>()?,
        );
    }
    Ok(())
}

fn capture_native_state(connection: &Connection, project: &str) -> Result<(Value, bool)> {
    let mut statement=connection.prepare("SELECT a.id,a.document_id,a.annotation_json,p.logical_id,p.native_projection_json FROM document_annotations a JOIN documents d ON d.id=a.document_id LEFT JOIN project_annotation_projections p ON p.project_id=d.project_id AND p.native_annotation_id=a.id WHERE d.project_id=?1 ORDER BY a.rowid")?;
    let rows = statement
        .query_map([project], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(statement);
    let mut opaque_statement = connection.prepare(
        "SELECT id,source_json FROM documents WHERE project_id=?1 AND read_only=1 ORDER BY rowid",
    )?;
    let opaque_rows = opaque_statement
        .query_map([project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(opaque_statement);
    let mut changed = !opaque_rows.is_empty();
    let opaque_documents=opaque_rows.into_iter().map(|(document,raw)|->Result<Value>{Ok(json!({"nativeDocumentId":document,"readOnly":true,"sourceJsonRaw":raw,"sourceJson":serde_json::from_str::<Value>(&raw)?}))}).collect::<Result<Vec<_>>>()?;
    let mut annotations = Vec::new();
    for (native, document, raw, logical, baseline) in rows {
        let current: Value = serde_json::from_str(&raw)?;
        changed |= match baseline {
            Some(raw) => serde_json::from_str::<Value>(&raw)? != current,
            None => true,
        };
        annotations.push(json!({"nativeId":native,"nativeDocumentId":document,"logicalId":logical,"state":current}));
    }
    let mut statement=connection.prepare("SELECT h.annotation_id,h.annotation_revision,h.document_revision,h.source,h.before_json,h.after_json FROM document_annotation_history h JOIN document_annotations a ON a.id=h.annotation_id JOIN documents d ON d.id=a.document_id WHERE d.project_id=?1 ORDER BY h.id")?;
    let rows = statement
        .query_map([project], |row| {
            Ok((
                row.get::<_, String>(0)?,
                read_unsigned(row, 1)?,
                read_unsigned(row, 2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(statement);
    let history=rows.into_iter().map(|(annotation,revision,document_revision,source,before,after)|->Result<Value>{Ok(json!({"nativeAnnotationId":annotation,"revision":revision,"documentRevision":document_revision,"source":source,"before":before.map(|raw|serde_json::from_str::<Value>(&raw)).transpose()?,"after":serde_json::from_str::<Value>(&after)?}))}).collect::<Result<Vec<_>>>()?;
    let mut statement=connection.prepare("SELECT s.document_id,s.document_revision,s.annotation_id,s.attachment_json FROM document_annotation_snapshots s JOIN documents d ON d.id=s.document_id WHERE d.project_id=?1 ORDER BY s.document_id,s.document_revision,s.annotation_id")?;
    let rows = statement
        .query_map([project], |row| {
            Ok((
                row.get::<_, String>(0)?,
                read_unsigned(row, 1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(statement);
    let attachments=rows.into_iter().map(|(document,revision,annotation,raw)|->Result<Value>{Ok(json!({"nativeDocumentId":document,"documentRevision":revision,"nativeAnnotationId":annotation,"attachmentEvidence":serde_json::from_str::<Value>(&raw)?}))}).collect::<Result<Vec<_>>>()?;
    let mut statement=connection.prepare("SELECT canvas_id,base_revision,version,content_json FROM project_canvas_drafts WHERE project_id=?1 ORDER BY rowid")?;
    let rows = statement
        .query_map([project], |row| {
            Ok((
                row.get::<_, String>(0)?,
                read_unsigned(row, 1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(statement);
    changed |= !rows.is_empty();
    let canvas_drafts=rows.into_iter().map(|(canvas,base,version,raw)|->Result<Value>{Ok(json!({"canvasId":canvas,"baseRevision":base,"version":version,"content":serde_json::from_str::<Value>(&raw)?}))}).collect::<Result<Vec<_>>>()?;
    let mut statement=connection.prepare("SELECT p.id,p.document_id,p.base_revision,p.before_body,p.after_body,p.reason,p.status,p.applied_revision,p.undone_revision,p.anchor_json,p.before_hash,p.after_hash FROM proposals p JOIN documents d ON d.id=p.document_id WHERE d.project_id=?1 ORDER BY p.rowid")?;
    let rows = statement
        .query_map([project], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                read_unsigned(row, 2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                read_optional_unsigned(row, 7)?,
                read_optional_unsigned(row, 8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    changed |= !rows.is_empty();
    let proposals=rows.into_iter().map(|(id,document,base,before,after,reason,status,applied,undone,anchor,before_hash,after_hash)|->Result<Value>{Ok(json!({"nativeId":id,"nativeDocumentId":document,"baseRevision":base,"before":before,"after":after,"reason":reason,"status":status,"appliedRevision":applied,"undoneRevision":undone,"anchor":serde_json::from_str::<Value>(&anchor)?,"beforeHash":before_hash,"afterHash":after_hash}))}).collect::<Result<Vec<_>>>()?;
    Ok((
        json!({"authority":"untrusted-on-import","opaqueDocuments":opaque_documents,"annotations":annotations,"annotationHistory":history,"attachmentSnapshots":attachments,"canvasDrafts":canvas_drafts,"reviewEvidence":proposals}),
        changed,
    ))
}

fn capture_saved_canvases(
    connection: &Connection,
    project: &str,
    manifest: &mut Value,
) -> Result<()> {
    for (domain, query) in [
        (
            "canvases",
            "SELECT raw_json FROM project_canvases WHERE project_id=?1 ORDER BY rowid",
        ),
        (
            "canvasRevisions",
            "SELECT raw_json FROM project_canvas_revisions WHERE project_id=?1 ORDER BY rowid",
        ),
    ] {
        let mut statement = connection.prepare(query)?;
        let rows = statement
            .query_map([project], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        manifest[domain] = Value::Array(
            rows.into_iter()
                .map(|raw| serde_json::from_str(&raw).map_err(StudioError::from))
                .collect::<Result<Vec<_>>>()?,
        );
    }
    Ok(())
}
