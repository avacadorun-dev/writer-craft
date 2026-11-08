//! Scoped immutable evidence bytes are ordinary data, never replay or proposal authority.
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, MAIN_DB, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::Serialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

use super::{
    ProjectTransferCancellation, Result, StudioError, Workspace, json::invalid, read_unsigned,
    require_project, sqlite_integer,
};

/// Maximum raw opaque/evidence object. Large objects stream rather than becoming a native-state Value.
pub const MAX_EVIDENCE_OBJECT_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum encoded metadata for one evidence reference.
pub const MAX_EVIDENCE_METADATA_BYTES: usize = 256 * 1024;

/// Validated descriptor only; storage still verifies the entire supplied byte stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedEvidenceObject {
    sha256: String,
    byte_size: u64,
    media_type: String,
}
impl ValidatedEvidenceObject {
    /// Validate a bounded digest/size/type descriptor without granting a read or replay capability.
    pub fn new(sha256: &str, byte_size: u64, media_type: &str) -> Result<Self> {
        if sha256.len() != 64
            || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || byte_size > MAX_EVIDENCE_OBJECT_BYTES
            || media_type.is_empty()
            || media_type.len() > 255
            || media_type.chars().any(char::is_control)
        {
            return invalid("Evidence object descriptor exceeds digest, size or media-type bounds");
        }
        Ok(Self {
            sha256: sha256.to_ascii_lowercase(),
            byte_size,
            media_type: media_type.to_owned(),
        })
    }
    /// Exact encoded-byte digest.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    /// Exact encoded length.
    pub fn byte_size(&self) -> u64 {
        self.byte_size
    }
    /// Descriptive data type; this never authorizes decoding or Apply.
    pub fn media_type(&self) -> &str {
        &self.media_type
    }
}
/// Raw caller input; only the checked constructor creates a validated reference.
pub struct EvidenceReferenceInput {
    /// Bounded semantic or unknown evidence domain.
    pub domain: String,
    /// Identity within that domain, independent of its content digest.
    pub logical_id: String,
    /// Optional original/native logical document identity in the same project.
    pub logical_document_id: Option<String>,
    /// Optional logical board identity in the same project.
    pub logical_canvas_id: Option<String>,
    /// Exact small original JSON object; large payloads belong in the referenced object.
    pub metadata_json: String,
}
/// Validated reference metadata. Scope ownership is rechecked inside the write transaction.
pub struct ValidatedEvidenceReference {
    domain: String,
    logical_id: String,
    document: Option<String>,
    canvas: Option<String>,
    object_sha256: String,
    metadata: String,
}
impl ValidatedEvidenceReference {
    /// Validate shape and bounded encoding; no deserialization grants this authority.
    pub fn new(object: &ValidatedEvidenceObject, input: EvidenceReferenceInput) -> Result<Self> {
        bounded_id(&input.domain, 128)?;
        bounded_id(&input.logical_id, 1024)?;
        for id in [&input.logical_document_id, &input.logical_canvas_id]
            .into_iter()
            .flatten()
        {
            bounded_id(id, 512)?;
        }
        if input.metadata_json.len() > MAX_EVIDENCE_METADATA_BYTES {
            return invalid("Evidence metadata exceeds 256 KiB; use an object reference");
        }
        super::json::reject_duplicate_keys(input.metadata_json.as_bytes())?;
        let raw: Box<RawValue> = serde_json::from_str(&input.metadata_json)?;
        if !raw.get().starts_with('{') {
            return invalid("Evidence metadata must be a JSON object");
        }
        Ok(Self {
            domain: input.domain,
            logical_id: input.logical_id,
            document: input.logical_document_id,
            canvas: input.logical_canvas_id,
            object_sha256: object.sha256.clone(),
            metadata: input.metadata_json,
        })
    }
}
/// One metadata page filter. IDs only filter inside the mandatory fixed project scope.
#[derive(Debug, Clone)]
pub struct EvidenceReferenceQuery {
    /// Restrict to one domain, or list every evidence domain.
    pub domain: Option<String>,
    /// Restrict to one logical document.
    pub logical_document_id: Option<String>,
    /// Restrict to one logical board.
    pub logical_canvas_id: Option<String>,
    /// Exclusive (domain, logical identity) cursor from the prior page.
    pub after: Option<(String, String)>,
    /// Page size from 1 through 256.
    pub limit: usize,
}
impl Default for EvidenceReferenceQuery {
    fn default() -> Self {
        Self {
            domain: None,
            logical_document_id: None,
            logical_canvas_id: None,
            after: None,
            limit: 64,
        }
    }
}
/// Bounded list metadata; large bytes and per-record metadata load only on explicit selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceReferenceSummary {
    /// Domain label, not an executable provider.
    pub domain: String,
    /// Original/domain-scoped identity.
    pub logical_id: String,
    /// Optional same-project logical document.
    pub logical_document_id: Option<String>,
    /// Optional same-project logical board.
    pub logical_canvas_id: Option<String>,
    /// Encoded object digest.
    pub sha256: String,
    /// Encoded object size.
    pub byte_size: u64,
    /// Descriptive stored type, never Apply or decoder authority.
    pub media_type: String,
    /// Encoded metadata size, bounded independently of object bytes.
    pub metadata_byte_size: u64,
}
/// A selected reference's fixed-project, lazily verified evidence capability.
#[derive(Clone)]
pub struct ProjectEvidenceSnapshot {
    connection: Arc<Mutex<Connection>>,
    project_id: String,
    rowid: i64,
    summary: EvidenceReferenceSummary,
    metadata: String,
}
impl ProjectEvidenceSnapshot {
    /// Captured immutable object/reference metadata.
    pub fn summary(&self) -> &EvidenceReferenceSummary {
        &self.summary
    }
    /// One bounded original metadata object. Imported authority labels remain untrusted data.
    pub fn metadata_json(&self) -> &str {
        &self.metadata
    }
    /// Copy only the captured same-project object, verifying current descriptor, size and digest.
    pub fn copy_to(
        &self,
        writer: &mut dyn Write,
        cancel: &ProjectTransferCancellation,
    ) -> Result<()> {
        cancel.check()?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StudioError::LockUnavailable)?;
        let current:(String,u64,String)=connection.query_row("SELECT sha256,byte_size,media_type FROM project_evidence_objects WHERE rowid=?1 AND project_id=?2",params![self.rowid,self.project_id],|row|Ok((row.get(0)?,read_unsigned(row,1)?,row.get(2)?)))?;
        if current
            != (
                self.summary.sha256.clone(),
                self.summary.byte_size,
                self.summary.media_type.clone(),
            )
        {
            return Err(StudioError::CorruptData(
                "Evidence metadata changed after capture".into(),
            ));
        }
        let mut blob = connection.blob_open(
            MAIN_DB,
            "project_evidence_objects",
            "data",
            self.rowid,
            true,
        )?;
        if u64::try_from(blob.len())
            .map_err(|_| StudioError::CorruptData("Evidence length exceeds capacity".into()))?
            != self.summary.byte_size
        {
            return Err(StudioError::CorruptData(
                "Evidence length differs from its descriptor".into(),
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
        if format!("{:x}", digest.finalize()) != self.summary.sha256 {
            return Err(StudioError::CorruptData(
                "Evidence checksum differs from its descriptor".into(),
            ));
        }
        Ok(())
    }
    /// Load only when an explicit preview/validator byte budget allows it.
    /// A refused preview never truncates or replaces the preserved object.
    pub fn load(&self, max_bytes: usize, cancel: &ProjectTransferCancellation) -> Result<Vec<u8>> {
        let size = usize::try_from(self.summary.byte_size).map_err(|_| {
            StudioError::Validation("Evidence exceeds addressable preview capacity".into())
        })?;
        if size > max_bytes {
            return invalid(
                "Evidence exceeds this preview budget; its original remains available for export",
            );
        }
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(size).map_err(|_| {
            StudioError::Validation(
                "Not enough memory for this evidence preview; export remains available".into(),
            )
        })?;
        self.copy_to(&mut bytes, cancel)?;
        Ok(bytes)
    }
}
impl Workspace {
    /// Store one exact untrusted object/reference atomically. Run large copies on a dedicated worker.
    /// This command never creates writing, trusted draft checkpoints or active proposals.
    pub fn import_project_evidence(
        &self,
        project: &str,
        object: &ValidatedEvidenceObject,
        reference: &ValidatedEvidenceReference,
        reader: &mut dyn Read,
        cancel: &ProjectTransferCancellation,
    ) -> Result<EvidenceReferenceSummary> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        store_evidence_in_transaction(&transaction, project, object, reference, reader, cancel)?;
        let (summary, _, _) = read_reference(
            &transaction,
            project,
            &reference.domain,
            &reference.logical_id,
        )?;
        transaction.commit()?;
        Ok(summary)
    }
    /// List at most 256 lightweight reference summaries, never every payload/native-state Value.
    pub fn project_evidence_references(
        &self,
        project: &str,
        query: &EvidenceReferenceQuery,
    ) -> Result<Vec<EvidenceReferenceSummary>> {
        if !(1..=256).contains(&query.limit) {
            return invalid("Evidence page size must be between 1 and 256");
        }
        if let Some(domain) = &query.domain {
            bounded_id(domain, 128)?;
        }
        for id in [&query.logical_document_id, &query.logical_canvas_id]
            .into_iter()
            .flatten()
        {
            bounded_id(id, 512)?;
        }
        if let Some((domain, id)) = &query.after {
            bounded_id(domain, 128)?;
            bounded_id(id, 1024)?;
        }
        let connection = self.connection()?;
        require_project(&connection, project)?;
        let mut statement=connection.prepare("SELECT r.domain,r.logical_id,r.logical_document_id,r.logical_canvas_id,o.sha256,o.byte_size,o.media_type,length(CAST(r.metadata_json AS BLOB)) FROM project_evidence_references r JOIN project_evidence_objects o ON o.project_id=r.project_id AND o.sha256=r.object_sha256 WHERE r.project_id=?1 AND (?2 IS NULL OR r.domain=?2) AND (?3 IS NULL OR r.logical_document_id=?3) AND (?4 IS NULL OR r.logical_canvas_id=?4) AND (?5 IS NULL OR r.domain>?5 OR (r.domain=?5 AND r.logical_id>?6)) ORDER BY r.domain,r.logical_id LIMIT ?7")?;
        let after_domain = query.after.as_ref().map(|(domain, _)| domain.as_str());
        let after_id = query.after.as_ref().map(|(_, id)| id.as_str());
        let summaries = statement
            .query_map(
                params![
                    project,
                    query.domain,
                    query.logical_document_id,
                    query.logical_canvas_id,
                    after_domain,
                    after_id,
                    sqlite_integer(query.limit)?
                ],
                read_summary,
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for summary in &summaries {
            validate_stored_summary(summary)?;
        }
        Ok(summaries)
    }
    /// Capture one selected evidence reference; its identity cannot grant another project's bytes.
    pub fn project_evidence(
        &self,
        project: &str,
        domain: &str,
        logical_id: &str,
    ) -> Result<ProjectEvidenceSnapshot> {
        bounded_id(domain, 128)?;
        bounded_id(logical_id, 1024)?;
        let connection = self.connection()?;
        require_project(&connection, project)?;
        let (summary, rowid, metadata) = read_reference(&connection, project, domain, logical_id)?;
        validate_stored_summary(&summary)?;
        if metadata.len() > MAX_EVIDENCE_METADATA_BYTES {
            return Err(StudioError::CorruptData(
                "Stored evidence metadata exceeds its limit".into(),
            ));
        }
        super::json::reject_duplicate_keys(metadata.as_bytes())
            .map_err(|error| StudioError::CorruptData(error.to_string()))?;
        let raw: Box<RawValue> = serde_json::from_str(&metadata)?;
        if !raw.get().starts_with('{') {
            return Err(StudioError::CorruptData(
                "Stored evidence metadata is not an object".into(),
            ));
        }
        Ok(ProjectEvidenceSnapshot {
            connection: self.connection.clone(),
            project_id: project.into(),
            rowid,
            summary,
            metadata,
        })
    }
}
pub(super) fn store_evidence_in_transaction(
    transaction: &Transaction<'_>,
    project: &str,
    object: &ValidatedEvidenceObject,
    reference: &ValidatedEvidenceReference,
    reader: &mut dyn Read,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    if reference.object_sha256 != object.sha256 {
        return invalid("Evidence reference and object digests disagree");
    }
    require_reference_scope(transaction, project, reference)?;
    cancel.check()?;
    let existing:Option<(i64,u64,String)>=transaction.query_row("SELECT rowid,byte_size,media_type FROM project_evidence_objects WHERE project_id=?1 AND sha256=?2",params![project,object.sha256],|row|Ok((row.get(0)?,read_unsigned(row,1)?,row.get(2)?))).optional()?;
    let rowid = if let Some((rowid, size, mime)) = &existing {
        if *size != object.byte_size || *mime != object.media_type {
            return invalid("Evidence digest has conflicting descriptor metadata");
        }
        *rowid
    } else {
        transaction.execute("INSERT INTO project_evidence_objects(project_id,sha256,byte_size,media_type,data) VALUES(?1,?2,?3,?4,zeroblob(?3))",params![project,object.sha256,sqlite_integer(object.byte_size)?,object.media_type])?;
        transaction.last_insert_rowid()
    };
    let mut blob = if existing.is_none() {
        Some(transaction.blob_open(MAIN_DB, "project_evidence_objects", "data", rowid, false)?)
    } else {
        None
    };
    let mut remaining = object.byte_size;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    while remaining > 0 {
        cancel.check()?;
        let requested = usize::try_from(
            remaining.min(u64::try_from(buffer.len()).map_err(std::io::Error::other)?),
        )
        .map_err(std::io::Error::other)?;
        let count = reader.read(&mut buffer[..requested])?;
        if count == 0 || count > requested {
            return invalid("Evidence ended early or its reader violated the bounded read");
        }
        digest.update(&buffer[..count]);
        if let Some(blob) = blob.as_mut() {
            blob.write_all(&buffer[..count])?;
        }
        remaining -= u64::try_from(count).map_err(std::io::Error::other)?;
    }
    let mut extra = [0_u8; 1];
    if reader.read(&mut extra)? != 0 {
        return invalid("Evidence exceeds its declared length");
    }
    drop(blob);
    if format!("{:x}", digest.finalize()) != object.sha256 {
        return invalid("Evidence source checksum does not match");
    }
    // Even deduplication verifies the incoming stream; bytes supplied by the source are never ignored.
    if existing.is_some() {
        let mut stored =
            transaction.blob_open(MAIN_DB, "project_evidence_objects", "data", rowid, true)?;
        let mut digest = Sha256::new();
        loop {
            cancel.check()?;
            let count = stored.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        if format!("{:x}", digest.finalize()) != object.sha256 {
            return Err(StudioError::CorruptData(
                "Existing evidence object failed checksum verification".into(),
            ));
        }
    }
    cancel.check()?;
    let prior:Option<(Option<String>,Option<String>,String,String)>=transaction.query_row("SELECT logical_document_id,logical_canvas_id,object_sha256,metadata_json FROM project_evidence_references WHERE project_id=?1 AND domain=?2 AND logical_id=?3",params![project,reference.domain,reference.logical_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional()?;
    if let Some(prior) = prior {
        if prior
            != (
                reference.document.clone(),
                reference.canvas.clone(),
                reference.object_sha256.clone(),
                reference.metadata.clone(),
            )
        {
            return invalid("An immutable evidence reference already has different data");
        }
    } else {
        transaction.execute("INSERT INTO project_evidence_references(project_id,domain,logical_id,logical_document_id,logical_canvas_id,object_sha256,metadata_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![project,reference.domain,reference.logical_id,reference.document,reference.canvas,reference.object_sha256,reference.metadata])?;
    }
    Ok(())
}
fn require_reference_scope(
    connection: &Connection,
    project: &str,
    reference: &ValidatedEvidenceReference,
) -> Result<()> {
    if let Some(document) = &reference.document {
        let present:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM project_nodes n JOIN documents d ON d.id=n.native_document_id AND d.project_id=n.project_id WHERE n.project_id=?1 AND n.logical_id=?2)",params![project,document],|row|row.get(0))?;
        if !present {
            return invalid(
                "Evidence document scope is outside this project's content-bearing nodes",
            );
        }
    }
    if let Some(canvas) = &reference.canvas {
        let present: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM project_canvases WHERE project_id=?1 AND logical_id=?2)",
            params![project, canvas],
            |row| row.get(0),
        )?;
        if !present {
            return invalid("Evidence Canvas scope is outside this project");
        }
    }
    Ok(())
}
fn read_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceReferenceSummary> {
    Ok(EvidenceReferenceSummary {
        domain: row.get(0)?,
        logical_id: row.get(1)?,
        logical_document_id: row.get(2)?,
        logical_canvas_id: row.get(3)?,
        sha256: row.get(4)?,
        byte_size: read_unsigned(row, 5)?,
        media_type: row.get(6)?,
        metadata_byte_size: read_unsigned(row, 7)?,
    })
}
fn read_reference(
    connection: &Connection,
    project: &str,
    domain: &str,
    id: &str,
) -> Result<(EvidenceReferenceSummary, i64, String)> {
    connection.query_row("SELECT r.domain,r.logical_id,r.logical_document_id,r.logical_canvas_id,o.sha256,o.byte_size,o.media_type,length(CAST(r.metadata_json AS BLOB)),o.rowid,r.metadata_json FROM project_evidence_references r JOIN project_evidence_objects o ON o.project_id=r.project_id AND o.sha256=r.object_sha256 WHERE r.project_id=?1 AND r.domain=?2 AND r.logical_id=?3",params![project,domain,id],|row|Ok((read_summary(row)?,row.get(8)?,row.get(9)?))).optional()?.ok_or_else(||StudioError::NotFound {kind:"Project evidence",id:format!("{domain}/{id}")})
}
fn bounded_id(id: &str, max: usize) -> Result<()> {
    if id.is_empty() || id.len() > max {
        return invalid("Evidence identity is empty or exceeds its domain limit");
    }
    Ok(())
}

fn validate_stored_summary(summary: &EvidenceReferenceSummary) -> Result<()> {
    ValidatedEvidenceObject::new(&summary.sha256, summary.byte_size, &summary.media_type)
        .map_err(|error| StudioError::CorruptData(error.to_string()))?;
    if summary.metadata_byte_size
        > u64::try_from(MAX_EVIDENCE_METADATA_BYTES).map_err(std::io::Error::other)?
    {
        return Err(StudioError::CorruptData(
            "Stored evidence metadata exceeds its limit".into(),
        ));
    }
    Ok(())
}
