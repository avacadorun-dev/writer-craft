//! Semantic activation consumes only the verified private index and commits one fresh namespace.
use std::io::{Cursor, Read};

use craft_editor_model::Document as RichTree;
use rusqlite::{Connection, MAIN_DB, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{
    ImportAssetSource, ImportedProject, PortableObjectReference, PortableProjectScope,
    ProjectAttachment, ProjectImportWorker, ProjectTransferCancellation, Result, StudioError,
    ValidatedProjectImport, VerifiedPortableProjectStream, import_in_transaction, insert_alias,
    json::invalid, read_unsigned, sqlite_integer, validation,
};

impl ProjectImportWorker {
    /// Restore a preflighted format-2 project in one transaction with independent scoped handles.
    /// Every saved semantic projection is validated before commit; external recovery/review data
    /// remains ordinary evidence. Failure/cancellation leaves no partial visible project.
    pub fn restore_stream(
        &mut self,
        stream: &VerifiedPortableProjectStream,
        cancel: &ProjectTransferCancellation,
    ) -> Result<ImportedProject> {
        cancel.check()?;
        if stream.header.scope() != PortableProjectScope::CompleteProject {
            return invalid("Selected native format-2 restoration requires its scoped adapter");
        }
        let mut project_record = None;
        records(stream, "project", cancel, &mut |_, record| {
            if project_record.replace(record.clone()).is_some() {
                return invalid("Native project has repeated project metadata");
            }
            Ok(())
        })?;
        let project_record = project_record
            .ok_or_else(|| StudioError::Validation("Native project metadata is missing".into()))?;
        let logical = id(&project_record, "logicalId")?;
        let title = text(&project_record, "title", 120)?;
        let raw = match stream.header.source_original() {
            Some(reference) => String::from_utf8(load(
                stream,
                reference,
                super::MAX_PROJECT_MANIFEST_BYTES as u64,
            )?)
            .map_err(|_| StudioError::Validation("Native original manifest is not UTF-8".into()))?,
            None => serde_json::to_string(&super::snapshot::native_manifest(logical, title))?,
        };
        let source = native_source(raw)?;
        if source.logical_project_id() != logical {
            return invalid("Native project identity differs from its immutable original");
        }
        stream.index.execute_batch("CREATE TABLE IF NOT EXISTS restore_heads(logical_id TEXT PRIMARY KEY,native_id TEXT,head INTEGER NOT NULL,title TEXT NOT NULL,baseline INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS restore_old_documents(old_id TEXT PRIMARY KEY,logical_id TEXT NOT NULL,native_id TEXT NOT NULL); CREATE TABLE IF NOT EXISTS restore_notes(old_id TEXT PRIMARY KEY,native_id TEXT NOT NULL,logical_document TEXT NOT NULL); CREATE TABLE IF NOT EXISTS restore_canvases(logical_id TEXT PRIMARY KEY,head INTEGER NOT NULL); DELETE FROM restore_heads; DELETE FROM restore_old_documents; DELETE FROM restore_notes; DELETE FROM restore_canvases;")?;
        let mut connection = self.workspace.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut assets = IndexedAssets(stream);
        let mut imported = import_in_transaction(&transaction, &source, &mut assets, cancel)?;
        drop(source); // Release the bounded source graph before native phases; never build nativeState.
        transaction.execute(
            "UPDATE projects SET title=?1 WHERE id=?2",
            params![title, imported.project_id],
        )?;
        restore_nodes(&transaction, &imported.project_id, stream, cancel)?;
        restore_revisions(&transaction, &imported.project_id, stream, cancel)?;
        restore_assets(&transaction, &imported.project_id, stream, cancel)?;
        restore_canvases(&transaction, &imported.project_id, stream, cancel)?;
        restore_notes(&transaction, &imported.project_id, stream, cancel)?;
        restore_opaque(&transaction, &imported.project_id, stream, cancel)?;
        restore_preservation(&transaction, &imported.project_id, stream, cancel)?;
        imported.node_count = transaction.query_row(
            "SELECT count(*) FROM project_nodes WHERE project_id=?1",
            [&imported.project_id],
            |row| row.get(0),
        )?;
        imported.document_revision_count=transaction.query_row("SELECT count(*) FROM document_revisions r JOIN documents d ON d.id=r.document_id WHERE d.project_id=?1",[&imported.project_id],|row|row.get(0))?;
        imported.canvas_count = transaction.query_row(
            "SELECT count(*) FROM project_canvases WHERE project_id=?1",
            [&imported.project_id],
            |row| row.get(0),
        )?;
        imported.attachment_count = transaction.query_row(
            "SELECT count(*) FROM project_assets WHERE project_id=?1",
            [&imported.project_id],
            |row| row.get(0),
        )?;
        cancel.check()?;
        transaction.commit()?;
        Ok(imported)
    }
}
fn native_source(raw: String) -> Result<ValidatedProjectImport> {
    if raw.len() > super::MAX_PROJECT_MANIFEST_BYTES {
        return invalid("Native source exceeds the 64 MiB original-manifest bound");
    }
    super::json::reject_duplicate_keys(raw.as_bytes())?;
    let manifest: Value = serde_json::from_str(&raw)?;
    let attachments = validation::validate_native(&manifest)?;
    Ok(ValidatedProjectImport {
        format_version: validation::number(&manifest, "formatVersion")?,
        logical_project_id: validation::string(&manifest["project"], "id")?.to_owned(),
        title: validation::string(&manifest["project"], "title")?.to_owned(),
        raw_manifest: raw,
        manifest,
        attachments,
    })
}
struct IndexedAssets<'a>(&'a VerifiedPortableProjectStream);
impl ImportAssetSource for IndexedAssets<'_> {
    fn open<'a>(
        &'a mut self,
        attachment: &ProjectAttachment,
    ) -> std::io::Result<Box<dyn Read + 'a>> {
        let reference = PortableObjectReference::new(
            &attachment.sha256.to_ascii_lowercase(),
            attachment.byte_size,
        )
        .map_err(std::io::Error::other)?;
        let rowid = object_row(self.0, &reference).map_err(std::io::Error::other)?;
        Ok(Box::new(
            self.0
                .index
                .blob_open(MAIN_DB, "objects", "data", rowid, true)
                .map_err(std::io::Error::other)?,
        ))
    }
}
fn records(
    stream: &VerifiedPortableProjectStream,
    domain: &str,
    cancel: &ProjectTransferCancellation,
    visitor: &mut dyn FnMut(u64, &Value) -> Result<()>,
) -> Result<()> {
    let mut statement = stream
        .index
        .prepare("SELECT ordinal,raw_json FROM records WHERE domain=?1 ORDER BY ordinal")?;
    let mut rows = statement.query([domain])?;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let raw: String = row.get(1)?;
        let value = if let Some(known) = known_fields(domain) {
            let fields: std::collections::BTreeMap<String, Box<serde_json::value::RawValue>> =
                serde_json::from_str(&raw)?;
            let fields = fields
                .into_iter()
                .filter(|(key, _)| known.contains(&key.as_str()))
                .collect::<std::collections::BTreeMap<_, _>>();
            serde_json::from_slice(&serde_json::to_vec(&fields)?)?
        } else {
            serde_json::from_str(&raw)?
        };
        visitor(read_unsigned(row, 0)?, &value)?;
    }
    Ok(())
}
fn id<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    let id = validation::string(value, key)?;
    if id.trim().is_empty() || id.len() > 512 || id.chars().any(char::is_control) {
        return invalid(format!(
            "Native {key} has an empty, oversized or unsafe identity"
        ));
    }
    Ok(id)
}
fn text<'a>(value: &'a Value, key: &str, max: usize) -> Result<&'a str> {
    let value = validation::string(value, key)?;
    if value.trim().is_empty() || value.chars().count() > max {
        return invalid(format!("Native {key} is empty or too long"));
    }
    Ok(value)
}
fn optional_string<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        _ => invalid(format!("Native optional {key} must be text or null")),
    }
}
fn optional_unsigned(value: &Value, key: &str) -> Result<Option<u64>> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or_else(|| {
            StudioError::Validation(format!(
                "Native optional {key} must be a bounded integer or null"
            ))
        }),
    }
}
fn positive(value: &Value, key: &str) -> Result<u64> {
    let n = validation::number(value, key)?;
    if n == 0 {
        return invalid(format!("Native {key} must be positive"));
    }
    Ok(n)
}
fn reference(value: &Value) -> Result<PortableObjectReference> {
    PortableObjectReference::new(
        validation::string(value, "sha256")?,
        validation::number(value, "byteSize")?,
    )
}
fn object_row(
    stream: &VerifiedPortableProjectStream,
    reference: &PortableObjectReference,
) -> Result<i64> {
    let row: Option<(i64, u64)> = stream
        .index
        .query_row(
            "SELECT rowid,byte_size FROM objects WHERE sha256=?1",
            [reference.sha256()],
            |row| Ok((row.get(0)?, read_unsigned(row, 1)?)),
        )
        .optional()?;
    let (rowid, size) = row.ok_or_else(|| {
        StudioError::Validation("Native semantic payload is outside verified object storage".into())
    })?;
    if size != reference.byte_size() {
        return invalid("Native payload length conflicts with its verified descriptor");
    }
    Ok(rowid)
}
fn load(
    stream: &VerifiedPortableProjectStream,
    reference: &PortableObjectReference,
    max: u64,
) -> Result<Vec<u8>> {
    if reference.byte_size() > max {
        return invalid("Native semantic payload exceeds its explicit domain bound");
    }
    let rowid = object_row(stream, reference)?;
    let mut blob = stream
        .index
        .blob_open(MAIN_DB, "objects", "data", rowid, true)?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(reference.byte_size() as usize)
        .map_err(|_| {
            StudioError::Validation("Not enough memory for this bounded native payload".into())
        })?;
    blob.read_to_end(&mut result)?;
    if result.len() as u64 != reference.byte_size()
        || format!("{:x}", Sha256::digest(&result)) != reference.sha256()
    {
        return invalid("Verified native payload changed before semantic activation");
    }
    Ok(result)
}
fn json_object(stream: &VerifiedPortableProjectStream, value: &Value, max: u64) -> Result<Value> {
    let bytes = load(stream, &reference(value)?, max)?;
    super::json::reject_duplicate_keys(&bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}
fn document(transaction: &Connection, project: &str, logical: &str) -> Result<String> {
    transaction.query_row("SELECT native_document_id FROM project_nodes WHERE project_id=?1 AND logical_id=?2 AND native_document_id IS NOT NULL",params![project,logical],|row|row.get(0)).optional()?.ok_or_else(||StudioError::Validation("Native record refers outside its owning project's writing".into()))
}
fn restore_nodes(
    transaction: &Transaction<'_>,
    project: &str,
    stream: &VerifiedPortableProjectStream,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    let structure_raw:String=transaction.query_row("SELECT json_extract(raw_manifest,'$.project.structure') FROM project_portability WHERE project_id=?1",[project],|row|row.get(0))?;
    let structure: Value = serde_json::from_str(&structure_raw)?;
    let default_kind = validation::array(&structure, "nodeTypes")?
        .iter()
        .find(|kind| {
            kind["contentBearing"].as_bool() == Some(true)
                && kind["canBeRoot"].as_bool() == Some(true)
        })
        .and_then(|kind| kind["key"].as_str())
        .map(str::to_owned);
    records(stream, "nodes", cancel, &mut |ordinal, record| {
        let logical = id(record, "logicalId")?;
        let mut metadata = if record.get("metadata").is_some_and(|value| !value.is_null()) {
            json_object(
                stream,
                &record["metadata"],
                super::MAX_PROJECT_MANIFEST_BYTES as u64,
            )?
        } else {
            json!({"id":logical,"kind":default_kind.as_ref().ok_or_else(||StudioError::Validation("Native writing has no compatible root content type in the retained structure".into()))?,"title":record["title"],"status":"draft","position":ordinal,"currentRevision":record["currentRevision"]})
        };
        if metadata["id"].as_str() != Some(logical) {
            return invalid("Native node metadata changes its scoped logical identity");
        }
        if let Some(title) = optional_string(record, "title")? {
            metadata["title"] = json!(title);
        }
        let title = text(&metadata, "title", 200)?.to_owned();
        let head = optional_unsigned(record, "currentRevision")?
            .unwrap_or(validation::number(&metadata, "currentRevision")?);
        sqlite_integer(head)?;
        let kind = id(&metadata, "kind")?;
        let node_kind = validation::array(&structure, "nodeTypes")?
            .iter()
            .find(|item| item["key"].as_str() == Some(kind))
            .ok_or_else(|| {
                StudioError::Validation(
                    "Native node kind is absent from the retained structure".into(),
                )
            })?;
        if head > 0 && node_kind["contentBearing"].as_bool() != Some(true) {
            return invalid("Native container cannot receive manuscript revisions");
        }
        let existing:Option<(Option<String>,u64)>=transaction.query_row("SELECT n.native_document_id,coalesce(d.revision,0) FROM project_nodes n LEFT JOIN documents d ON d.id=n.native_document_id AND d.project_id=n.project_id WHERE n.project_id=?1 AND n.logical_id=?2",params![project,logical],|row|Ok((row.get(0)?,read_unsigned(row,1)?))).optional()?;
        let baseline = existing
            .as_ref()
            .map(|(_, revision)| *revision)
            .unwrap_or(0);
        if head < baseline {
            return invalid("Native node head would discard retained saved history");
        }
        let native = match existing.as_ref().and_then(|(native, _)| native.clone()) {
            Some(native) => Some(native),
            None if head > 0 => {
                let native = uuid::Uuid::new_v4().to_string();
                transaction.execute("INSERT INTO documents(id,project_id,title,body,revision,read_only,source_json) VALUES(?1,?2,?3,'',?4,0,NULL)",params![native,project,title,sqlite_integer(head)?])?;
                Some(native)
            }
            None => None,
        };
        metadata["currentRevision"] = json!(head);
        let parent = metadata["parentId"].as_str().map(str::to_owned);
        let position = metadata["position"]
            .as_f64()
            .filter(|value| value.is_finite() && value.abs() <= 1e12)
            .ok_or_else(|| StudioError::Validation("Native node position is invalid".into()))?;
        let _ = position;
        if !matches!(
            metadata["status"].as_str(),
            Some("idea" | "draft" | "editing" | "ready")
        ) {
            return invalid("Native node status is invalid");
        }
        let ordinal = record["ordinal"].as_u64().unwrap_or(ordinal);
        transaction.execute("INSERT INTO project_nodes(project_id,logical_id,native_document_id,parent_logical_id,ordinal,raw_json) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(project_id,logical_id) DO UPDATE SET native_document_id=excluded.native_document_id,parent_logical_id=excluded.parent_logical_id,ordinal=excluded.ordinal,raw_json=excluded.raw_json",params![project,logical,native,parent,sqlite_integer(ordinal)?,serde_json::to_string(&metadata)?])?;
        if existing.is_none() {
            let alias = native
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            insert_alias(transaction, project, "node", logical, &alias)?;
            insert_alias(transaction, project, "document", logical, &alias)?;
        }
        stream.index.execute("INSERT INTO restore_heads(logical_id,native_id,head,title,baseline) VALUES(?1,?2,?3,?4,?5)",params![logical,native,sqlite_integer(head)?,title,sqlite_integer(baseline)?])?;
        if let Some(old) = optional_string(record, "nativeHandle")? {
            if !bounded_id(old) {
                return invalid("Captured native document handle is invalid");
            }
            let native = native.ok_or_else(|| {
                StudioError::Validation("A captured writing handle names a contentless node".into())
            })?;
            stream.index.execute(
                "INSERT INTO restore_old_documents(old_id,logical_id,native_id) VALUES(?1,?2,?3)",
                params![old, logical, native],
            )?;
        }
        Ok(())
    })?;
    let mut statement =
        transaction.prepare("SELECT logical_id FROM project_nodes WHERE project_id=?1")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let logical: String = row.get(0)?;
        let mut seen = std::collections::HashSet::new();
        let mut cursor = Some(logical);
        while let Some(logical) = cursor {
            if !seen.insert(logical.clone()) || seen.len() > 512 {
                return invalid("Native project tree is cyclic or exceeds 512 levels");
            }
            cursor=transaction.query_row("SELECT parent_logical_id FROM project_nodes WHERE project_id=?1 AND logical_id=?2",params![project,logical],|row|row.get::<_,Option<String>>(0)).optional()?.ok_or_else(||StudioError::Validation("Native project parent is outside its project namespace".into()))?;
        }
    }
    Ok(())
}
fn bounded_id(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= 512 && !id.chars().any(char::is_control)
}
fn restore_revisions(
    transaction: &Transaction<'_>,
    project: &str,
    stream: &VerifiedPortableProjectStream,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    records(stream, "documentRevisions", cancel, &mut |_, record| {
        let logical = id(record, "logicalDocumentId")?;
        let native = document(transaction, project, logical)?;
        let revision = positive(record, "revision")?;
        let source = validation::string(record, "source")?;
        if !matches!(source, "manual" | "import" | "proposal" | "undo") {
            return invalid("Native saved revision has an unsupported provenance");
        }
        let body = String::from_utf8(load(
            stream,
            &reference(&record["body"])?,
            16 * 1024 * 1024,
        )?)
        .map_err(|_| StudioError::Validation("Native saved prose must be UTF-8".into()))?;
        let rich = if record["richContent"].is_null() {
            None
        } else {
            let rich = json_object(stream, &record["richContent"], 16 * 1024 * 1024)?;
            let (_, projection, encoded) = crate::rich::validate_tree(rich)?;
            if projection != body {
                return invalid("Native rich saved content disagrees with its prose projection");
            }
            Some(encoded)
        };
        transaction.execute(
            "INSERT INTO document_revisions(document_id,revision,body,source) VALUES(?1,?2,?3,?4)",
            params![native, sqlite_integer(revision)?, body, source],
        )?;
        if let Some(rich) = rich {
            transaction.execute("INSERT INTO rich_document_revisions(document_id,revision,content_json) VALUES(?1,?2,?3)",params![native,sqlite_integer(revision)?,rich])?;
        }
        Ok(())
    })?;
    let mut statement = stream.index.prepare(
        "SELECT native_id,head,title,baseline FROM restore_heads WHERE native_id IS NOT NULL",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let native: String = row.get(0)?;
        let head = read_unsigned(row, 1)?;
        let title: String = row.get(2)?;
        let baseline = read_unsigned(row, 3)?;
        let latest: Option<u64> = transaction
            .query_row(
                "SELECT max(revision) FROM document_revisions WHERE document_id=?1",
                [&native],
                |row| row.get::<_, Option<i64>>(0),
            )?
            .map(|value| u64::try_from(value).map_err(std::io::Error::other))
            .transpose()?;
        if latest != Some(head) {
            return invalid("Native document head does not match its complete saved history");
        }
        let gaps:i64=transaction.query_row("SELECT count(*) FROM document_revisions r WHERE r.document_id=?1 AND r.revision>?2 AND r.revision>1 AND NOT EXISTS(SELECT 1 FROM document_revisions b WHERE b.document_id=r.document_id AND b.revision=r.revision-1)",params![native,sqlite_integer(baseline)?],|row|row.get(0))?;
        if gaps != 0 {
            return invalid("Native revision delta has a missing saved base");
        }
        let (body,rich):(String,Option<String>)=transaction.query_row("SELECT r.body,j.content_json FROM document_revisions r LEFT JOIN rich_document_revisions j ON j.document_id=r.document_id AND j.revision=r.revision WHERE r.document_id=?1 AND r.revision=?2",params![native,sqlite_integer(head)?],|row|Ok((row.get(0)?,row.get(1)?)))?;
        transaction.execute(
            "UPDATE documents SET title=?1,revision=?2,body=?3 WHERE id=?4 AND project_id=?5",
            params![title, sqlite_integer(head)?, body, native, project],
        )?;
        if let Some(rich) = rich {
            transaction.execute("INSERT INTO rich_documents(document_id,content_json) VALUES(?1,?2) ON CONFLICT(document_id) DO UPDATE SET content_json=excluded.content_json",params![native,rich])?;
        } else {
            transaction.execute("DELETE FROM rich_documents WHERE document_id=?1", [native])?;
        }
    }
    Ok(())
}
fn restore_assets(
    transaction: &Transaction<'_>,
    project: &str,
    stream: &VerifiedPortableProjectStream,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    let mut total = 0_u64;
    records(stream, "assets", cancel, &mut |_, record| {
        let logical = id(record, "logicalId")?;
        let kind = validation::string(record, "resourceKind")?;
        if !matches!(kind, "media" | "audio") {
            return invalid("Native artifact resource kind is unsupported");
        }
        let object = reference(&record["object"])?;
        total = total
            .checked_add(object.byte_size())
            .filter(|total| *total <= super::MAX_PROJECT_TRANSFER_BYTES)
            .ok_or_else(|| {
                StudioError::Validation(
                    "Native original artifacts exceed 512 MiB logical bytes".into(),
                )
            })?;
        let mime = text(record, "mediaType", 255)?;
        let name = text(record, "originalName", 512)?;
        let existing:Option<(String,String,u64,String)>=transaction.query_row("SELECT a.id,a.sha256,a.byte_size,a.media_type FROM project_asset_metadata m JOIN project_assets a ON a.id=m.native_asset_id AND a.project_id=m.project_id WHERE m.project_id=?1 AND m.resource_kind=?2 AND m.logical_id=?3",params![project,kind,logical],|row|Ok((row.get(0)?,row.get(1)?,read_unsigned(row,2)?,row.get(3)?))).optional()?;
        if let Some((_, sha, size, stored_mime)) = existing {
            if sha != object.sha256() || size != object.byte_size() || stored_mime != mime {
                return invalid("Native artifact conflicts with its retained immutable original");
            }
            return Ok(());
        }
        if kind == "audio" {
            let session:bool=transaction.query_row("SELECT EXISTS(SELECT 1 FROM project_source_records WHERE project_id=?1 AND domain='dictationSessions' AND json_extract(raw_json,'$.id')=?2)",params![project,logical],|row|row.get(0))?;
            if !session {
                return invalid("Native original audio has no captured source session");
            }
        }
        let path = record["archivePath"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| {
                format!("{kind}/native-{:x}.bin", Sha256::digest(logical.as_bytes()))
            });
        if !validation::safe_path(&path) || !path.starts_with(&format!("{kind}/")) {
            return invalid("Native original artifact path is unsafe");
        }
        let width = optional_unsigned(record, "width")?
            .map(|value| u32::try_from(value).map_err(std::io::Error::other))
            .transpose()?;
        let height = optional_unsigned(record, "height")?
            .map(|value| u32::try_from(value).map_err(std::io::Error::other))
            .transpose()?;
        let mut metadata = if record["sourceMetadata"].is_null() {
            json!({})
        } else {
            json_object(
                stream,
                &record["sourceMetadata"],
                super::MAX_PROJECT_MANIFEST_BYTES as u64,
            )?
        };
        if !metadata.is_object() {
            return invalid("Native original artifact metadata must be an object");
        }
        metadata[if kind == "audio" { "sessionId" } else { "id" }] = json!(logical);
        metadata["path"] = json!(path);
        metadata["sha256"] = json!(object.sha256());
        metadata["byteSize"] = json!(object.byte_size());
        metadata["mediaType"] = json!(mime);
        metadata["originalName"] = json!(name);
        metadata["width"] = json!(width);
        metadata["height"] = json!(height);
        let attachment = ProjectAttachment {
            resource_kind: kind.into(),
            logical_id: logical.into(),
            path,
            sha256: object.sha256().into(),
            byte_size: object.byte_size(),
            media_type: mime.into(),
            original_name: name.into(),
            width,
            height,
        };
        let mut manifest = json!({});
        manifest[kind] = json!([metadata]);
        super::import_attachment(
            transaction,
            project,
            &manifest,
            &attachment,
            &mut IndexedAssets(stream),
            cancel,
        )
    })
}
fn restore_canvases(
    transaction: &Transaction<'_>,
    project: &str,
    stream: &VerifiedPortableProjectStream,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    records(stream, "canvases", cancel, &mut |_, record| {
        let payload = json_object(stream, &record["payload"], super::MAX_EVIDENCE_OBJECT_BYTES)?;
        let logical = id(&payload, "logicalId")?;
        let head = positive(&payload, "currentRevision")?;
        let raw = validation::string(&payload, "metadataRaw")?;
        super::json::reject_duplicate_keys(raw.as_bytes())?;
        let metadata: Value = serde_json::from_str(raw)?;
        if metadata["id"].as_str() != Some(logical)
            || metadata["currentRevision"].as_u64() != Some(head)
        {
            return invalid("Native Canvas metadata disagrees with its saved identity/head");
        }
        text(&metadata, "title", 160)?;
        if !metadata["position"]
            .as_f64()
            .is_some_and(|value| value.is_finite() && value.abs() <= 1e12)
        {
            return invalid("Native Canvas position is invalid");
        }
        let existing:Option<u64>=transaction.query_row("SELECT current_revision FROM project_canvases WHERE project_id=?1 AND logical_id=?2",params![project,logical],|row|read_unsigned(row,0)).optional()?;
        if existing.is_some_and(|base| head < base) {
            return invalid("Native Canvas head would discard retained history");
        }
        transaction.execute("INSERT INTO project_canvases(project_id,logical_id,current_revision,raw_json) VALUES(?1,?2,?3,?4) ON CONFLICT(project_id,logical_id) DO UPDATE SET current_revision=excluded.current_revision,raw_json=excluded.raw_json",params![project,logical,sqlite_integer(head)?,raw])?;
        if existing.is_none() {
            insert_alias(
                transaction,
                project,
                "canvas",
                logical,
                &uuid::Uuid::new_v4().to_string(),
            )?;
        }
        stream.index.execute(
            "INSERT INTO restore_canvases(logical_id,head) VALUES(?1,?2)",
            params![logical, sqlite_integer(head)?],
        )?;
        Ok(())
    })?;
    records(stream, "canvasRevisions", cancel, &mut |_, record| {
        let logical = id(record, "logicalCanvasId")?;
        let revision = positive(record, "revision")?;
        let base = validation::number(record, "baseRevision")?;
        let schema = positive(record, "schemaVersion")?;
        if base >= revision || !(1..=2).contains(&schema) {
            return invalid("Native Canvas revision base or schema is invalid");
        }
        let mut metadata = json_object(
            stream,
            &record["metadata"],
            super::MAX_PROJECT_MANIFEST_BYTES as u64,
        )?;
        let content = json_object(stream, &record["content"], 12 * 1024 * 1024)?;
        if !metadata.is_object()
            || metadata["canvasId"].as_str() != Some(logical)
            || metadata["revision"].as_u64() != Some(revision)
            || metadata["baseRevision"].as_u64() != Some(base)
            || metadata["schemaVersion"].as_u64() != Some(schema)
        {
            return invalid("Native Canvas revision metadata disagrees with its descriptor");
        }
        if content["x-writers-studio"]["schemaVersion"]
            .as_u64()
            .unwrap_or(1)
            != schema
        {
            return invalid("Native Canvas schema disagrees with its actual content");
        }
        super::canvas_commands::validate_scoped_content(transaction, project, &content)?;
        metadata["content"] = content;
        transaction.execute("INSERT INTO project_canvas_revisions(project_id,canvas_id,revision,base_revision,schema_version,content_json,raw_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![project,logical,sqlite_integer(revision)?,sqlite_integer(base)?,sqlite_integer(schema)?,serde_json::to_string(&metadata["content"])?,serde_json::to_string(&metadata)?])?;
        Ok(())
    })?;
    let mut statement = stream
        .index
        .prepare("SELECT logical_id,head FROM restore_canvases")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let logical: String = row.get(0)?;
        let head = read_unsigned(row, 1)?;
        let latest:Option<i64>=transaction.query_row("SELECT max(revision) FROM project_canvas_revisions WHERE project_id=?1 AND canvas_id=?2",params![project,logical],|row|row.get(0))?;
        if latest != Some(sqlite_integer(head)?) {
            return invalid("Native Canvas head does not match its complete saved history");
        }
        let missing:i64=transaction.query_row("SELECT count(*) FROM project_canvas_revisions r WHERE r.project_id=?1 AND r.canvas_id=?2 AND r.base_revision>0 AND NOT EXISTS(SELECT 1 FROM project_canvas_revisions b WHERE b.project_id=r.project_id AND b.canvas_id=r.canvas_id AND b.revision=r.base_revision)",params![project,logical],|row|row.get(0))?;
        if missing != 0 {
            return invalid("Native Canvas revision has a missing saved base");
        }
    }
    Ok(())
}
fn validated_note(
    transaction: &Transaction<'_>,
    project: &str,
    logical_document: &str,
    old_note: &str,
    old_document: &str,
    value: &Value,
    current: bool,
) -> Result<crate::PassageAnnotation> {
    let mut state: crate::PassageAnnotation = serde_json::from_value(value.clone())?;
    if state.id != old_note
        || state.document_id != old_document
        || state.revision <= 0
        || state.base_revision <= 0
        || !bounded_id(&state.anchor.block_id)
        || state.anchor.start >= state.anchor.end
        || state.anchor.exact_text.is_empty()
        || state.anchor.exact_text.len() > 16 * 1024 * 1024
        || state.anchor.prefix.chars().count() > 32
        || state.anchor.suffix.chars().count() > 32
        || state.note.chars().count() > 10_000
    {
        return invalid("Native note has invalid scoped identity, revisions, quote or text bounds");
    }
    let native = document(transaction, project, logical_document)?;
    let row:Option<(i64,Option<String>)>=transaction.query_row("SELECT d.revision,j.content_json FROM documents d JOIN document_revisions r ON r.document_id=d.id AND r.revision=?1 LEFT JOIN rich_document_revisions j ON j.document_id=r.document_id AND j.revision=r.revision WHERE d.id=?2 AND d.project_id=?3",params![state.base_revision,native,project],|row|Ok((row.get(0)?,row.get(1)?))).optional()?;
    let (head, rich) = row.ok_or_else(|| {
        StudioError::Validation(
            "Native note base is absent from its supplied document history".into(),
        )
    })?;
    let exact = rich
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| RichTree::from_json(value).ok())
        .is_some_and(|tree| state.anchor.selection(&tree).is_ok());
    if !exact || (current && state.base_revision != head) {
        state.anchor_status = crate::AnchorStatus::Detached;
    }
    Ok(state)
}
fn restore_notes(
    transaction: &Transaction<'_>,
    project: &str,
    stream: &VerifiedPortableProjectStream,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    records(stream, "nativeAnnotations", cancel, &mut |_, record| {
        let old = id(record, "nativeHandle")?;
        let logical = id(record, "logicalDocumentId")?;
        let native_document = document(transaction, project, logical)?;
        let raw_state = json_object(stream, &record["state"], super::MAX_EVIDENCE_OBJECT_BYTES)?;
        let old_document = validation::string(&raw_state, "document_id")?;
        let captured: Option<String> = stream
            .index
            .query_row(
                "SELECT logical_id FROM restore_old_documents WHERE old_id=?1",
                [old_document],
                |row| row.get(0),
            )
            .optional()?;
        if captured.as_deref() != Some(logical) {
            return invalid("Native note old handle is outside its captured document aliases");
        }
        let mut state = validated_note(
            transaction,
            project,
            logical,
            old,
            old_document,
            &raw_state,
            true,
        )?;
        let source_logical = optional_string(record, "logicalSourceId")?;
        let existing: Option<String> = if let Some(source_logical) = source_logical {
            if !bounded_id(source_logical) {
                return invalid("Native source-note identity is invalid");
            }
            let row:Option<(Option<String>,String)>=transaction.query_row("SELECT native_annotation_id,raw_json FROM project_annotation_projections WHERE project_id=?1 AND logical_id=?2 AND document_id=?3",params![project,source_logical,native_document],|row|Ok((row.get(0)?,row.get(1)?))).optional()?;
            let (native, raw) = row.ok_or_else(|| {
                StudioError::Validation(
                    "Native note overlay has no matching source note in its own document".into(),
                )
            })?;
            if serde_json::from_str::<Value>(&raw)?["kind"].as_str() != Some("note") {
                return invalid(
                    "A native author note cannot replace another source annotation kind",
                );
            }
            native
        } else {
            None
        };
        let native = existing
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        state.id = native.clone();
        state.document_id = native_document.clone();
        if existing.is_some() {
            transaction.execute(
                "DELETE FROM document_annotation_history WHERE annotation_id=?1",
                [&native],
            )?;
            transaction.execute(
                "DELETE FROM document_annotation_snapshots WHERE annotation_id=?1",
                [&native],
            )?;
            transaction.execute("UPDATE document_annotations SET revision=?1,deleted=?2,annotation_json=?3 WHERE id=?4 AND document_id=?5",params![state.revision,state.deleted,serde_json::to_string(&state)?,native,native_document])?;
        } else {
            transaction.execute("INSERT INTO document_annotations(id,document_id,revision,deleted,annotation_json) VALUES(?1,?2,?3,?4,?5)",params![native,native_document,state.revision,state.deleted,serde_json::to_string(&state)?])?;
        }
        if let Some(source_logical) = source_logical {
            transaction.execute("UPDATE project_annotation_projections SET native_annotation_id=?1,native_anchor_json=?2,anchor_status=?3 WHERE project_id=?4 AND logical_id=?5 AND document_id=?6",params![native,serde_json::to_string(&state.anchor)?,if state.anchor_status==crate::AnchorStatus::Attached{"attached"}else{"detached"},project,source_logical,native_document])?;
            transaction.execute("UPDATE project_resource_aliases SET native_id=?1 WHERE project_id=?2 AND resource_kind='annotation' AND logical_id=?3",params![native,project,source_logical])?;
        } else {
            let stable = id(record, "logicalNativeId")?;
            insert_alias(transaction, project, "native-annotation", stable, &native)?;
        }
        stream.index.execute(
            "INSERT INTO restore_notes(old_id,native_id,logical_document) VALUES(?1,?2,?3)",
            params![old, native, logical],
        )?;
        if state.anchor_status == crate::AnchorStatus::Attached {
            transaction.execute("INSERT INTO document_annotation_snapshots(document_id,document_revision,annotation_id,attachment_json) VALUES(?1,?2,?3,?4)",params![native_document,state.base_revision,native,serde_json::to_string(&json!({"anchor":state.anchor,"status":state.anchor_status}))?])?;
        }
        Ok(())
    })?;
    records(
        stream,
        "nativeAnnotationHistory",
        cancel,
        &mut |_, record| {
            let old = id(record, "nativeHandle")?;
            let logical = id(record, "logicalDocumentId")?;
            let note: Option<(String, String)> = stream
                .index
                .query_row(
                    "SELECT native_id,logical_document FROM restore_notes WHERE old_id=?1",
                    [old],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let (native, document_logical) = note.ok_or_else(|| {
                StudioError::Validation("Native note history has no supplied current note".into())
            })?;
            if document_logical != logical {
                return invalid("Native note history crosses document scope");
            }
            let document = document(transaction, project, logical)?;
            let revision = positive(record, "revision")?;
            let document_revision = positive(record, "documentRevision")?;
            let source = validation::string(record, "source")?;
            if !matches!(
                source,
                "create" | "update" | "reattach" | "delete" | "mapping" | "restore"
            ) {
                return invalid("Native note event has unsupported provenance");
            }
            let after = json_object(stream, &record["after"], super::MAX_EVIDENCE_OBJECT_BYTES)?;
            let old_document = validation::string(&after, "document_id")?;
            let captured: Option<String> = stream
                .index
                .query_row(
                    "SELECT logical_id FROM restore_old_documents WHERE old_id=?1",
                    [old_document],
                    |row| row.get(0),
                )
                .optional()?;
            if captured.as_deref() != Some(logical) {
                return invalid("Native history old handle is outside captured document aliases");
            }
            let mut after = validated_note(
                transaction,
                project,
                logical,
                old,
                old_document,
                &after,
                false,
            )?;
            let current_revision: i64 = transaction.query_row(
                "SELECT revision FROM document_annotations WHERE id=?1 AND document_id=?2",
                params![native, document],
                |row| row.get(0),
            )?;
            if after.revision != sqlite_integer(revision)?
                || after.base_revision != sqlite_integer(document_revision)?
                || after.revision > current_revision
            {
                return invalid("Native note event revisions disagree with their states");
            }
            let before = if record["before"].is_null() {
                None
            } else {
                let value =
                    json_object(stream, &record["before"], super::MAX_EVIDENCE_OBJECT_BYTES)?;
                let mut before = validated_note(
                    transaction,
                    project,
                    logical,
                    old,
                    old_document,
                    &value,
                    false,
                )?;
                if before.revision >= after.revision {
                    return invalid("Native note event moves backwards");
                }
                before.id = native.clone();
                before.document_id = document.clone();
                Some(before)
            };
            after.id = native.clone();
            after.document_id = document;
            transaction.execute("INSERT INTO document_annotation_history(annotation_id,annotation_revision,document_revision,source,before_json,after_json) VALUES(?1,?2,?3,?4,?5,?6)",params![native,sqlite_integer(revision)?,sqlite_integer(document_revision)?,source,before.map(|value|serde_json::to_string(&value)).transpose()?,serde_json::to_string(&after)?])?;
            Ok(())
        },
    )
}
fn restore_opaque(
    transaction: &Transaction<'_>,
    project: &str,
    stream: &VerifiedPortableProjectStream,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    records(stream, "opaqueDocuments", cancel, &mut |_, record| {
        let logical = id(record, "logicalDocumentId")?;
        let native = document(transaction, project, logical)?;
        if record["readOnly"].as_bool() != Some(true) {
            return invalid("Opaque original must remain read-only");
        }
        let raw = load(
            stream,
            &reference(&record["original"])?,
            super::MAX_PROJECT_MANIFEST_BYTES as u64,
        )?;
        super::json::reject_duplicate_keys(&raw)?;
        let raw = String::from_utf8(raw)
            .map_err(|_| StudioError::Validation("Opaque source must be UTF-8 JSON".into()))?;
        transaction.execute(
            "UPDATE documents SET read_only=1,source_json=?1 WHERE id=?2 AND project_id=?3",
            params![raw, native, project],
        )?;
        Ok(())
    })
}
fn restore_preservation(
    transaction: &Transaction<'_>,
    project: &str,
    stream: &VerifiedPortableProjectStream,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    // Existing evidence passes through by its original scoped identity. Never wrap prior bundles.
    records(stream, "evidence", cancel, &mut |_, record| {
        let object = reference(&record["object"])?;
        let metadata = load(
            stream,
            &reference(&record["metadata"])?,
            super::MAX_EVIDENCE_METADATA_BYTES as u64,
        )?;
        let metadata = String::from_utf8(metadata)
            .map_err(|_| StudioError::Validation("Evidence metadata must be UTF-8".into()))?;
        let descriptor = super::ValidatedEvidenceObject::new(
            object.sha256(),
            object.byte_size(),
            text(record, "mediaType", 255)?,
        )?;
        let reference = super::ValidatedEvidenceReference::new(
            &descriptor,
            super::EvidenceReferenceInput {
                domain: text(record, "domain", 128)?.into(),
                logical_id: text(record, "logicalId", 1024)?.into(),
                logical_document_id: record["logicalDocumentId"].as_str().map(str::to_owned),
                logical_canvas_id: record["logicalCanvasId"].as_str().map(str::to_owned),
                metadata_json: metadata,
            },
        )?;
        let mut blob = stream.index.blob_open(
            MAIN_DB,
            "objects",
            "data",
            object_row(stream, &object)?,
            true,
        )?;
        super::evidence::store_evidence_in_transaction(
            transaction,
            project,
            &descriptor,
            &reference,
            &mut blob,
            cancel,
        )
    })?;
    // Preserve only unknown fields in known metadata. Regenerated native handles must not cause a
    // complete prior root/state to be nested on each round trip.
    let root: std::collections::BTreeMap<String, Box<serde_json::value::RawValue>> =
        serde_json::from_slice(&stream.root)?;
    preserve_unknown_fields(
        transaction,
        project,
        "root",
        "project",
        &root,
        &[
            "format",
            "formatVersion",
            "scope",
            "sourceOriginal",
            "components",
        ],
        stream,
        cancel,
    )?;
    if let Some(raw) = root.get("sourceOriginal").filter(|raw| raw.get() != "null") {
        let fields = serde_json::from_str(raw.get())?;
        preserve_unknown_fields(
            transaction,
            project,
            "source-original",
            "project",
            &fields,
            &["sha256", "byteSize"],
            stream,
            cancel,
        )?;
    }
    if let Some(raw) = root.get("components") {
        let components: Vec<Box<serde_json::value::RawValue>> = serde_json::from_str(raw.get())?;
        for raw in components {
            let fields: std::collections::BTreeMap<String, Box<serde_json::value::RawValue>> =
                serde_json::from_str(raw.get())?;
            let domain = fields
                .get("domain")
                .map(|raw| serde_json::from_str::<String>(raw.get()))
                .transpose()?
                .unwrap_or_else(|| "unknown".into());
            preserve_unknown_fields(
                transaction,
                project,
                "component",
                &domain,
                &fields,
                &["domain", "sha256", "byteSize", "recordCount"],
                stream,
                cancel,
            )?;
        }
    }
    let mut statement = stream
        .index
        .prepare("SELECT domain,ordinal,raw_json FROM records ORDER BY domain,ordinal")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let domain: String = row.get(0)?;
        let ordinal = read_unsigned(row, 1)?;
        let raw: String = row.get(2)?;
        if domain == "evidence" {
            continue;
        }
        let fields: std::collections::BTreeMap<String, Box<serde_json::value::RawValue>> =
            serde_json::from_str(&raw)?;
        let Some(known) = known_fields(&domain) else {
            store_bytes_evidence(
                transaction,
                project,
                "portable.unknown-record",
                &format!("{domain}:{ordinal}"),
                raw.as_bytes(),
                "application/json",
                None,
                cancel,
            )?;
            continue;
        };
        let interpreted = fields
            .iter()
            .filter(|(key, _)| known.contains(&key.as_str()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let value: Value = serde_json::from_slice(&serde_json::to_vec(&interpreted)?)?;
        let owner = semantic_owner(&domain, &value, ordinal);
        preserve_unknown_fields(
            transaction,
            project,
            &domain,
            &owner,
            &fields,
            known,
            stream,
            cancel,
        )?;
        match domain.as_str() {
            "nativeAnnotations" => preserve_note_extensions(
                transaction,
                project,
                stream,
                &value["state"],
                &owner,
                "state",
                cancel,
            )?,
            "nativeAnnotationHistory" => {
                preserve_note_extensions(
                    transaction,
                    project,
                    stream,
                    &value["after"],
                    &owner,
                    "after",
                    cancel,
                )?;
                if !value["before"].is_null() {
                    preserve_note_extensions(
                        transaction,
                        project,
                        stream,
                        &value["before"],
                        &owner,
                        "before",
                        cancel,
                    )?;
                }
            }
            "attachmentEvidence" => {
                if !restore_attachment_snapshot(transaction, project, stream, &value, cancel)? {
                    store_record_evidence(
                        transaction,
                        project,
                        stream,
                        &domain,
                        &owner,
                        &raw,
                        &value,
                        cancel,
                    )?;
                }
            }
            "proseDraftEvidence"
            | "richDraftEvidence"
            | "canvasDraftEvidence"
            | "reviewEvidence"
            | "richReviewEvidence" => store_record_evidence(
                transaction,
                project,
                stream,
                &domain,
                &owner,
                &raw,
                &value,
                cancel,
            )?,
            _ => {}
        }
    }
    // Unsupported-domain objects are preserved once by content hash. Known original artifacts
    // are already scoped assets; a giant unknown opaque payload must obey the 64 MiB guard.
    let mut statement = stream
        .index
        .prepare("SELECT sha256,byte_size FROM objects WHERE referenced=0 ORDER BY sha256")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let object =
            PortableObjectReference::new(&row.get::<_, String>(0)?, read_unsigned(row, 1)?)?;
        store_object_evidence(transaction, project, stream, &object, cancel)?;
    }
    Ok(())
}
fn known_fields(domain: &str) -> Option<&'static [&'static str]> {
    Some(match domain {
        "project" => &["logicalId", "title"],
        "nodes" => &[
            "logicalId",
            "nativeHandle",
            "metadata",
            "ordinal",
            "title",
            "currentRevision",
            "nativeCreated",
        ],
        "documentRevisions" => &[
            "logicalDocumentId",
            "revision",
            "source",
            "body",
            "richContent",
        ],
        "opaqueDocuments" => &["logicalDocumentId", "original", "readOnly"],
        "nativeAnnotations" => &[
            "nativeHandle",
            "logicalSourceId",
            "logicalNativeId",
            "logicalDocumentId",
            "state",
        ],
        "nativeAnnotationHistory" => &[
            "nativeHandle",
            "logicalDocumentId",
            "revision",
            "documentRevision",
            "source",
            "before",
            "after",
        ],
        "canvases" | "attachmentEvidence" | "reviewEvidence" => {
            &["ordinal", "payload", "authority"]
        }
        "canvasRevisions" => &[
            "logicalCanvasId",
            "revision",
            "baseRevision",
            "schemaVersion",
            "content",
            "metadata",
        ],
        "proseDraftEvidence" => &["logicalDocumentId", "baseRevision", "body", "authority"],
        "richDraftEvidence" => &[
            "logicalDocumentId",
            "baseRevision",
            "version",
            "content",
            "checkpointEvidence",
            "authority",
        ],
        "canvasDraftEvidence" => &[
            "logicalCanvasId",
            "baseRevision",
            "version",
            "content",
            "authority",
        ],
        "richReviewEvidence" => &[
            "logicalDocumentId",
            "logicalId",
            "baseRevision",
            "status",
            "before",
            "after",
            "command",
            "reason",
            "appliedRevision",
            "undoneRevision",
            "authority",
        ],
        "assets" => &[
            "logicalId",
            "resourceKind",
            "mediaType",
            "originalName",
            "width",
            "height",
            "archivePath",
            "sourceMetadata",
            "object",
        ],
        "evidence" => &[
            "domain",
            "logicalId",
            "logicalDocumentId",
            "logicalCanvasId",
            "metadata",
            "object",
            "mediaType",
        ],
        _ => return None,
    })
}
fn semantic_owner(domain: &str, value: &Value, ordinal: u64) -> String {
    match domain {
        "nativeAnnotations" => format!(
            "{}:{}:{}",
            value["logicalDocumentId"].as_str().unwrap_or(""),
            value["logicalSourceId"]
                .as_str()
                .map(|id| format!("source:{id}"))
                .unwrap_or_else(|| format!(
                    "native:{}",
                    value["logicalNativeId"].as_str().unwrap_or("")
                )),
            "state"
        ),
        "nativeAnnotationHistory" => format!(
            "{}:{}",
            value["logicalDocumentId"].as_str().unwrap_or(""),
            value["revision"]
        ),
        _ => format!(
            "{}:{}:{}",
            value["logicalDocumentId"]
                .as_str()
                .or_else(|| value["logicalCanvasId"].as_str())
                .or_else(|| value["logicalId"].as_str())
                .unwrap_or(domain),
            value["revision"],
            ordinal
        ),
    }
}
fn preserve_unknown_fields(
    transaction: &Transaction<'_>,
    project: &str,
    domain: &str,
    owner: &str,
    fields: &std::collections::BTreeMap<String, Box<serde_json::value::RawValue>>,
    known: &[&str],
    _stream: &VerifiedPortableProjectStream,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    for (key, value) in fields {
        if !known.contains(&key.as_str()) {
            #[derive(serde::Serialize)]
            struct Field<'a> {
                domain: &'a str,
                owner: &'a str,
                path: &'a str,
                value: &'a serde_json::value::RawValue,
            }
            let raw = serde_json::to_vec(&Field {
                domain,
                owner,
                path: key,
                value,
            })?;
            store_bytes_evidence(
                transaction,
                project,
                "portable.extension",
                domain,
                &raw,
                "application/json",
                None,
                cancel,
            )?;
        }
    }
    Ok(())
}
fn preserve_note_extensions(
    transaction: &Transaction<'_>,
    project: &str,
    stream: &VerifiedPortableProjectStream,
    payload: &Value,
    owner: &str,
    path: &str,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    let raw = load(
        stream,
        &reference(payload)?,
        super::MAX_EVIDENCE_OBJECT_BYTES,
    )?;
    let fields: std::collections::BTreeMap<String, Box<serde_json::value::RawValue>> =
        serde_json::from_slice(&raw)?;
    preserve_unknown_fields(
        transaction,
        project,
        &format!("note.{path}"),
        owner,
        &fields,
        &[
            "id",
            "document_id",
            "revision",
            "base_revision",
            "anchor",
            "note",
            "status",
            "anchor_status",
            "deleted",
        ],
        stream,
        cancel,
    )?;
    if let Some(raw) = fields.get("anchor") {
        let fields = serde_json::from_str(raw.get())?;
        preserve_unknown_fields(
            transaction,
            project,
            &format!("note.{path}.anchor"),
            owner,
            &fields,
            &[
                "block_id",
                "blockId",
                "start",
                "end",
                "exact_text",
                "exactText",
                "prefix",
                "suffix",
            ],
            stream,
            cancel,
        )?;
    }
    Ok(())
}
fn store_record_evidence(
    transaction: &Transaction<'_>,
    project: &str,
    stream: &VerifiedPortableProjectStream,
    domain: &str,
    owner: &str,
    raw: &str,
    value: &Value,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    store_bytes_evidence(
        transaction,
        project,
        &format!("portable.record.{domain}"),
        owner,
        raw.as_bytes(),
        "application/json",
        None,
        cancel,
    )?;
    for key in match domain {
        "proseDraftEvidence" => &["body"][..],
        "richDraftEvidence" => &["content", "checkpointEvidence"][..],
        "canvasDraftEvidence" => &["content"][..],
        "richReviewEvidence" => &["before", "after", "command"][..],
        _ => &["payload"][..],
    } {
        if let Some(value) = value.get(*key).filter(|value| !value.is_null()) {
            store_object_evidence(transaction, project, stream, &reference(value)?, cancel)?;
        }
    }
    Ok(())
}
fn store_bytes_evidence(
    transaction: &Transaction<'_>,
    project: &str,
    domain: &str,
    owner: &str,
    raw: &[u8],
    mime: &str,
    logical_document: Option<&str>,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    let sha = format!("{:x}", Sha256::digest(raw));
    let logical_id = format!("{:x}", Sha256::digest(format!("{domain}\0{owner}\0{sha}")));
    let object = super::ValidatedEvidenceObject::new(&sha, raw.len() as u64, mime)?;
    let reference = super::ValidatedEvidenceReference::new(
        &object,
        super::EvidenceReferenceInput {
            domain: domain.into(),
            logical_id,
            logical_document_id: logical_document.map(str::to_owned),
            logical_canvas_id: None,
            metadata_json: serde_json::to_string(
                &json!({"originalDomain":domain,"logicalOwner":owner}),
            )?,
        },
    )?;
    super::evidence::store_evidence_in_transaction(
        transaction,
        project,
        &object,
        &reference,
        &mut Cursor::new(raw),
        cancel,
    )
}
fn store_object_evidence(
    transaction: &Transaction<'_>,
    project: &str,
    stream: &VerifiedPortableProjectStream,
    object: &PortableObjectReference,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    let mime: Option<String> = transaction
        .query_row(
            "SELECT media_type FROM project_evidence_objects WHERE project_id=?1 AND sha256=?2",
            params![project, object.sha256()],
            |row| row.get(0),
        )
        .optional()?;
    let descriptor = super::ValidatedEvidenceObject::new(
        object.sha256(),
        object.byte_size(),
        mime.as_deref().unwrap_or("application/octet-stream"),
    )?;
    let reference = super::ValidatedEvidenceReference::new(
        &descriptor,
        super::EvidenceReferenceInput {
            domain: "portable.payload".into(),
            logical_id: object.sha256().into(),
            logical_document_id: None,
            logical_canvas_id: None,
            metadata_json: "{}".into(),
        },
    )?;
    let mut blob = stream.index.blob_open(
        MAIN_DB,
        "objects",
        "data",
        object_row(stream, object)?,
        true,
    )?;
    super::evidence::store_evidence_in_transaction(
        transaction,
        project,
        &descriptor,
        &reference,
        &mut blob,
        cancel,
    )
}
fn restore_attachment_snapshot(
    transaction: &Transaction<'_>,
    project: &str,
    stream: &VerifiedPortableProjectStream,
    record: &Value,
    cancel: &ProjectTransferCancellation,
) -> Result<bool> {
    cancel.check()?;
    let payload = json_object(stream, &record["payload"], super::MAX_EVIDENCE_OBJECT_BYTES)?;
    let old_document = id(&payload, "documentId")?;
    let old_note = id(&payload, "annotationId")?;
    let revision = positive(&payload, "documentRevision")?;
    let owning: Option<(String, String)> = stream
        .index
        .query_row(
            "SELECT logical_id,native_id FROM restore_old_documents WHERE old_id=?1",
            [old_document],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (logical_document, native_document) = owning.ok_or_else(|| {
        StudioError::Validation("Attachment snapshot refers outside captured writing".into())
    })?;
    let note: Option<(String, String)> = stream
        .index
        .query_row(
            "SELECT native_id,logical_document FROM restore_notes WHERE old_id=?1",
            [old_note],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (native_note, note_document) = note.ok_or_else(|| {
        StudioError::Validation("Attachment snapshot refers outside supplied notes".into())
    })?;
    if logical_document != note_document {
        return invalid("Attachment snapshot crosses document scope");
    }
    let raw = validation::string(&payload, "attachmentRaw")?;
    super::json::reject_duplicate_keys(raw.as_bytes())?;
    let attachment: Value = serde_json::from_str(raw)?;
    let anchor: crate::AnnotationAnchor = serde_json::from_value(attachment["anchor"].clone())?;
    let status: crate::AnchorStatus = serde_json::from_value(attachment["status"].clone())?;
    let rich: Option<String> = transaction
        .query_row(
            "SELECT content_json FROM rich_document_revisions WHERE document_id=?1 AND revision=?2",
            params![native_document, sqlite_integer(revision)?],
            |row| row.get(0),
        )
        .optional()?;
    let exact = rich
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| RichTree::from_json(value).ok())
        .is_some_and(|tree| anchor.selection(&tree).is_ok());
    if !exact {
        return Ok(false);
    }
    // A validated current snapshot may already have been derived from the current saved note.
    let existing:Option<String>=transaction.query_row("SELECT attachment_json FROM document_annotation_snapshots WHERE document_id=?1 AND document_revision=?2 AND annotation_id=?3",params![native_document,sqlite_integer(revision)?,native_note],|row|row.get(0)).optional()?;
    let known = json!({"anchor":anchor,"status":status});
    if let Some(existing) = existing {
        if serde_json::from_str::<Value>(&existing)? != known {
            return invalid("Attachment snapshot disagrees with the validated current saved note");
        }
    } else {
        transaction.execute("INSERT INTO document_annotation_snapshots(document_id,document_revision,annotation_id,attachment_json) VALUES(?1,?2,?3,?4)",params![native_document,sqlite_integer(revision)?,native_note,serde_json::to_string(&known)?])?;
    }
    // Unknown snapshot properties remain ordinary evidence, separately from native attachment data.
    let fields = serde_json::from_str(raw)?;
    preserve_unknown_fields(
        transaction,
        project,
        "attachment-snapshot",
        &format!("{logical_document}:{revision}"),
        &fields,
        &["anchor", "status"],
        stream,
        cancel,
    )?;
    Ok(true)
}
