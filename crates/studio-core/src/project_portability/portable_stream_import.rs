//! Pinned format-2 preflight: verified object bytes and bounded records live in a private disk index.
use std::io::{Read, Write};

use rusqlite::{Connection, MAIN_DB, OptionalExtension, params};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use super::{
    PortableObjectReference, PortableProjectHeader, PortableResourceBudget,
    ProjectTransferCancellation, Result, StudioError, json::invalid, read_unsigned, sqlite_integer,
    visit_portable_shard,
};

/// Authorized, pinned transport bytes. Names/URLs in JSON never implement this capability.
///
/// Archive/folder adapters must inventory all physical members, reject unsafe/duplicate paths and
/// enforce their encoded/expanded/member limits before handing the capability to core. Core also
/// checks every enumerated object, root, shard, reference and logical materialization budget.
pub trait PortableObjectSource {
    /// Verify actual encoded/expanded/path/member constraints of this pinned transport.
    fn verify_transport(&mut self) -> std::io::Result<()>;
    /// Enumerate every object once, including objects in domains this application cannot interpret.
    fn visit_objects(
        &mut self,
        visitor: &mut dyn FnMut(PortableObjectReference) -> Result<()>,
    ) -> Result<()>;
    /// Open only an exact inventoried descriptor; the core independently rechecks all bytes.
    fn open<'a>(
        &'a mut self,
        object: &PortableObjectReference,
    ) -> std::io::Result<Box<dyn Read + 'a>>;
}

/// A transport-verified private snapshot, not permission to activate record semantics.
///
/// No `Deserialize` or public path/connection exists. All bytes are copied through checksums into
/// its own temporary SQLite index before this value is returned. A later semantic transaction may
/// use only these pinned bytes; mutating the original archive cannot change them.
pub struct VerifiedPortableProjectStream {
    pub(super) index: Connection,
    _directory: TempDir,
    pub(super) header: PortableProjectHeader,
    pub(super) root: Vec<u8>,
    record_count: u64,
}
impl VerifiedPortableProjectStream {
    /// Preflight root/shards/object bytes and known object references without changing a workspace.
    /// Unknown domains and extra objects remain retained ordinary data, never replay/Apply authority.
    pub fn preflight(
        root: &[u8],
        source: &mut dyn PortableObjectSource,
        cancel: &ProjectTransferCancellation,
    ) -> Result<Self> {
        cancel.check()?;
        source.verify_transport()?;
        let header = PortableProjectHeader::parse(root)?;
        let directory = tempfile::Builder::new()
            .prefix("writercraft-restore-")
            .tempdir()?;
        let index = Connection::open(directory.path().join("index.sqlite"))?;
        index.execute_batch("PRAGMA journal_mode=OFF; PRAGMA synchronous=OFF; PRAGMA cache_size=-2048; CREATE TABLE objects(sha256 TEXT PRIMARY KEY,byte_size INTEGER NOT NULL,data BLOB NOT NULL,referenced INTEGER NOT NULL DEFAULT 0); CREATE TABLE records(domain TEXT NOT NULL,ordinal INTEGER NOT NULL,raw_json TEXT NOT NULL,PRIMARY KEY(domain,ordinal));")?;
        let mut budget = PortableResourceBudget::default();
        budget.account_unique_transport(root.len() as u64)?;
        budget.account_physical_member(root.len() as u64)?;
        source.visit_objects(&mut |object| {
            cancel.check()?;
            budget.account_unique_transport(object.byte_size())?;
            budget.account_physical_member(object.byte_size())?;
            if index
                .query_row(
                    "SELECT 1 FROM objects WHERE sha256=?1",
                    [object.sha256()],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .is_some()
            {
                return invalid("Native transport repeats an object digest");
            }
            index.execute(
                "INSERT INTO objects(sha256,byte_size,data) VALUES(?1,?2,zeroblob(?2))",
                params![object.sha256(), sqlite_integer(object.byte_size())?],
            )?;
            Ok(())
        })?;
        {
            let mut statement =
                index.prepare("SELECT rowid,sha256,byte_size FROM objects ORDER BY sha256")?;
            let mut rows = statement.query([])?;
            while let Some(row) = rows.next()? {
                cancel.check()?;
                let object = PortableObjectReference::new(
                    &row.get::<_, String>(1)?,
                    read_unsigned(row, 2)?,
                )?;
                let mut input = source.open(&object)?;
                let mut blob = index.blob_open(MAIN_DB, "objects", "data", row.get(0)?, false)?;
                let mut digest = Sha256::new();
                let mut remaining = object.byte_size();
                let mut buffer = [0_u8; 64 * 1024];
                while remaining > 0 {
                    cancel.check()?;
                    let size = remaining.min(buffer.len() as u64) as usize;
                    let count = input.read(&mut buffer[..size])?;
                    if count == 0 || count > size {
                        return invalid("Native object ended early or violated its bounded reader");
                    }
                    digest.update(&buffer[..count]);
                    blob.write_all(&buffer[..count])?;
                    remaining -= count as u64;
                }
                cancel.check()?;
                if input.read(&mut [0_u8; 1])? != 0
                    || format!("{:x}", digest.finalize()) != object.sha256()
                {
                    return invalid("Native object length or checksum differs from its descriptor");
                }
            }
        }
        if let Some(original) = header.source_original() {
            require_object(&index, original)?;
            budget.account_materialized(original.byte_size())?;
        }
        let mut counts = std::collections::HashMap::<String, u64>::new();
        for component in header.components() {
            cancel.check()?;
            let rowid = require_object(&index, component.object())?;
            let mut blob = index.blob_open(MAIN_DB, "objects", "data", rowid, true)?;
            visit_portable_shard(component, &mut blob, cancel, &mut |record| {
                budget.account_records(1)?;
                budget.account_materialized(record.raw_json().len() as u64)?;
                let value: Value = serde_json::from_str(record.raw_json())?;
                for field in reference_fields(component.domain()) {
                    if let Some(value) = value.get(*field).filter(|value| !value.is_null()) {
                        let object = reference(value)?;
                        require_object(&index, &object)?;
                        budget.account_materialized(object.byte_size())?;
                    }
                }
                let ordinal = counts.entry(component.domain().to_owned()).or_default();
                index.execute(
                    "INSERT INTO records(domain,ordinal,raw_json) VALUES(?1,?2,?3)",
                    params![
                        component.domain(),
                        sqlite_integer(*ordinal)?,
                        record.raw_json()
                    ],
                )?;
                *ordinal += 1;
                Ok(())
            })?;
        }
        cancel.check()?;
        Ok(Self {
            _directory: directory,
            index,
            header,
            root: root.to_vec(),
            record_count: budget.record_count(),
        })
    }
    /// Original bounded descriptor bytes, retaining unknown framing/fields as data.
    pub fn root_bytes(&self) -> &[u8] {
        &self.root
    }
    /// Verified small header. It does not imply source/record semantic validity.
    pub fn header(&self) -> &PortableProjectHeader {
        &self.header
    }
    /// Actual verified count across all component domains.
    pub fn record_count(&self) -> u64 {
        self.record_count
    }
    /// Lightweight per-domain metadata count, without loading bodies or whole native state.
    pub fn domain_record_count(&self, domain: &str) -> Result<u64> {
        Ok(self.index.query_row(
            "SELECT count(*) FROM records WHERE domain=?1",
            [domain],
            |row| read_unsigned(row, 0),
        )?)
    }
}
fn require_object(index: &Connection, object: &PortableObjectReference) -> Result<i64> {
    let row: Option<(i64, u64)> = index
        .query_row(
            "SELECT rowid,byte_size FROM objects WHERE sha256=?1",
            [object.sha256()],
            |row| Ok((row.get(0)?, read_unsigned(row, 1)?)),
        )
        .optional()?;
    let (rowid, size) = row.ok_or_else(|| {
        StudioError::Validation(
            "Canonical record references an object outside its pinned transport".into(),
        )
    })?;
    if size != object.byte_size() {
        return invalid("Canonical object reference length conflicts with its pinned inventory");
    }
    index.execute("UPDATE objects SET referenced=1 WHERE rowid=?1", [rowid])?;
    Ok(rowid)
}
fn reference(value: &Value) -> Result<PortableObjectReference> {
    let sha = value["sha256"].as_str().ok_or_else(|| {
        StudioError::Validation("Canonical object reference has no digest".into())
    })?;
    let size = value["byteSize"].as_u64().ok_or_else(|| {
        StudioError::Validation("Canonical object reference has no bounded length".into())
    })?;
    PortableObjectReference::new(sha, size)
}
fn reference_fields(domain: &str) -> &'static [&'static str] {
    match domain {
        "nodes" => &["metadata"],
        "documentRevisions" => &["body", "richContent"],
        "opaqueDocuments" => &["original"],
        "nativeAnnotations" => &["state"],
        "nativeAnnotationHistory" => &["before", "after"],
        "canvasRevisions" => &["content", "metadata"],
        "proseDraftEvidence" => &["body"],
        "richDraftEvidence" => &["content", "checkpointEvidence"],
        "canvasDraftEvidence" => &["content"],
        "richReviewEvidence" => &["before", "after", "command"],
        "assets" => &["object", "sourceMetadata"],
        "evidence" => &["object", "metadata"],
        "canvases" | "attachmentEvidence" | "reviewEvidence" => &["payload"],
        _ => &[],
    }
}
