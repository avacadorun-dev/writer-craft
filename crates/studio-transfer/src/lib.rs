//! Bounded Writers Studio archive inspection, validated migration and preservation exports.
//!
//! The original inspector remains read-only. Full import uses separately validated project
//! commands and a dedicated database worker; export refuses unsupported loss rather than
//! flattening the original rich documents, histories or owned artifacts.
#![deny(missing_docs)]

mod markdown_render;
mod portable_file;
mod portable_markdown;
mod portable_paths;
mod portable_selection;
mod project_archive;
pub use portable_file::{PortableFileReceipt, export_project_archive_file};
pub use portable_markdown::{
    MarkdownConversion, MarkdownExportProgress, MarkdownExportReceipt, MarkdownExportSelection,
    PortableExportFile, export_markdown_folder, export_markdown_folder_with_progress,
};
pub use project_archive::{ProjectArchive, write_project_archive};

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 40_002;
const MAX_NODES: usize = 20_000;
const MAX_REVISIONS: usize = 100_000;

/// A verified archive's non-editable projection, with its source data retained.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    /// Project title from the original manifest.
    pub title: String,
    /// Container format version, independent of document schema versions.
    pub format_version: u64,
    /// Current document revisions in the original node order.
    pub documents: Vec<ImportedDocument>,
    /// Entire source manifest, including fields the native MVP does not yet understand.
    pub manifest: Value,
}

/// A current document revision for preview; rich data is never converted to plain paragraphs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedDocument {
    /// Original node identifier; a subsequent import must remap it into its own namespace.
    pub id: String,
    /// Original node title.
    pub title: String,
    /// Exported plain-text projection, suitable for a read-only preview.
    pub plain_text: String,
    /// Unmodified JSON value of the current revision's canonical rich document.
    pub rich_content: Value,
}

/// Failures that prevent an archive from being safely previewed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransferError {
    /// The archive could not be read from disk.
    #[error("Cannot read project archive: {0}")]
    Io(#[from] std::io::Error),
    /// ZIP structure, compression or an entry checksum is invalid.
    #[error("Cannot read ZIP project archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// The manifest is not a complete, valid JSON value.
    #[error("Cannot read project.json: {0}")]
    Json(#[from] serde_json::Error),
    /// A portable archive or manifest invariant failed.
    #[error("Invalid project archive: {0}")]
    Invalid(String),
    /// A version outside the deliberately supported preview range was requested.
    #[error("Project format version {0} is unsupported; this preview supports versions 1–8")]
    UnsupportedFormatVersion(u64),
}

/// Result type for archive inspection.
pub type Result<T> = std::result::Result<T, TransferError>;

/// Check an archive and return a read-only preview without extracting or modifying any files.
///
/// Limits are 512 MiB compressed/expanded total, 64 MiB for `project.json`, and 40,002
/// entries. Every declared attachment is streamed through SHA-256 and ZIP CRC verification.
/// This is archive inspection, not full document, media-decoder or project-domain validation.
///
/// # Errors
/// Returns an error for unreadable, corrupt, unsafe, oversized, inconsistent or unsupported
/// archives. An error never leaves an extracted file or partially imported project.
pub fn inspect_archive(path: impl AsRef<Path>) -> Result<ImportPreview> {
    let mut file = File::open(path)?;
    if file.metadata()?.len() > MAX_ARCHIVE_BYTES {
        return invalid("compressed archive exceeds 512 MiB");
    }
    let record_count = preflight_directory(&mut file)?;
    let mut archive = ZipArchive::new(file)?;
    if archive.len() != record_count {
        return invalid("ZIP library resolved duplicate entry names");
    }
    let entries = validate_entries(&mut archive)?;
    let manifest_index = entries
        .get("project.json")
        .copied()
        .ok_or_else(|| TransferError::Invalid("missing project.json".into()))?;
    let manifest = read_manifest(&mut archive, manifest_index)?;
    let preview = preview_manifest(manifest)?;
    validate_attachments(&mut archive, &entries, &preview.manifest)?;
    Ok(preview)
}

fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(TransferError::Invalid(message.into()))
}

fn little_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn little_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn preflight_directory(file: &mut File) -> Result<usize> {
    let size = file.metadata()?.len();
    let tail_length = size.min(65_557) as usize;
    if tail_length < 22 {
        return invalid("ZIP end record is missing or truncated");
    }
    file.seek(SeekFrom::End(-(tail_length as i64)))?;
    let mut tail = vec![0; tail_length];
    file.read_exact(&mut tail)?;
    let end = (0..=tail.len() - 22)
        .rev()
        .find(|index| {
            tail[*index..*index + 4] == [0x50, 0x4b, 0x05, 0x06]
                && *index + 22 + usize::from(little_u16(&tail[*index + 20..])) == tail.len()
        })
        .ok_or_else(|| TransferError::Invalid("ZIP end record is missing or truncated".into()))?;
    let footer = &tail[end..end + 22];
    let count = usize::from(little_u16(&footer[10..]));
    if little_u16(&footer[4..]) != 0
        || little_u16(&footer[6..]) != 0
        || little_u16(&footer[8..]) != little_u16(&footer[10..])
    {
        return invalid("multi-disk ZIP archives are unsupported");
    }
    if count == 0 || count > MAX_ENTRIES {
        return invalid("archive must contain 1–40,002 entries; ZIP64 directories are unsupported");
    }
    let directory_size = u64::from(little_u32(&footer[12..]));
    let directory_start = u64::from(little_u32(&footer[16..]));
    let footer_start = size - tail_length as u64 + end as u64;
    if directory_start.checked_add(directory_size) != Some(footer_start) {
        return invalid("ZIP directory bounds are inconsistent; ZIP64 directories are unsupported");
    }
    file.seek(SeekFrom::Start(directory_start))?;
    let mut consumed = 0_u64;
    let mut names = HashSet::with_capacity(count);
    for _ in 0..count {
        if consumed + 46 > directory_size {
            return invalid("ZIP central directory is truncated");
        }
        let mut header = [0_u8; 46];
        file.read_exact(&mut header)?;
        if header[..4] != [0x50, 0x4b, 0x01, 0x02] {
            return invalid("ZIP central directory contains an invalid record");
        }
        let name_length = usize::from(little_u16(&header[28..]));
        let extra_length = u64::from(little_u16(&header[30..]));
        let comment_length = u64::from(little_u16(&header[32..]));
        consumed += 46 + name_length as u64 + extra_length + comment_length;
        if name_length == 0 || name_length > 4096 || consumed > directory_size {
            return invalid("ZIP entry name or directory length exceeds supported bounds");
        }
        let mut bytes = vec![0; name_length];
        file.read_exact(&mut bytes)?;
        let name = std::str::from_utf8(&bytes)
            .map_err(|_| TransferError::Invalid("entry name is not UTF-8".into()))?;
        if !safe_path(name) || !names.insert(name.to_lowercase()) {
            return invalid(format!(
                "unsafe, duplicate or case-colliding entry: {name:?}"
            ));
        }
        file.seek(SeekFrom::Current((extra_length + comment_length) as i64))?;
    }
    if consumed != directory_size {
        return invalid("ZIP directory contains undeclared records");
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(count)
}

fn validate_entries<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<HashMap<String, usize>> {
    if archive.is_empty() || archive.len() > MAX_ENTRIES {
        return invalid("archive must contain 1–40,002 entries");
    }
    let mut entries = HashMap::with_capacity(archive.len());
    let mut portable_names = HashSet::with_capacity(archive.len());
    let mut expanded = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = std::str::from_utf8(entry.name_raw())
            .map_err(|_| TransferError::Invalid("entry name is not UTF-8".into()))?;
        if entry.is_dir() || !safe_path(name) {
            return invalid(format!("unsafe entry path: {name:?}"));
        }
        if let Some(mode) = entry.unix_mode() {
            let kind = mode & 0o170_000;
            if kind != 0 && kind != 0o100_000 {
                return invalid(format!("entry is not a regular file: {name:?}"));
            }
        }
        if entries.insert(name.to_owned(), index).is_some()
            || !portable_names.insert(name.to_lowercase())
        {
            return invalid(format!("duplicate or case-colliding entry: {name:?}"));
        }
        expanded = expanded.checked_add(entry.size()).ok_or_else(|| {
            TransferError::Invalid("expanded archive size overflows its limit".into())
        })?;
        if expanded > MAX_ARCHIVE_BYTES {
            return invalid("expanded archive exceeds 512 MiB");
        }
    }
    Ok(entries)
}

fn safe_path(name: &str) -> bool {
    if name.is_empty()
        || name.starts_with('/')
        || name.contains(['\\', ':', '*', '?', '"', '<', '>', '|'])
        || name.chars().any(char::is_control)
    {
        return false;
    }
    name.split('/')
        .all(|part| !part.is_empty() && part != "." && part != ".." && !part.ends_with(['.', ' ']))
}

fn read_manifest<R: Read + Seek>(archive: &mut ZipArchive<R>, index: usize) -> Result<Value> {
    let entry = archive.by_index(index)?;
    if entry.size() > MAX_MANIFEST_BYTES {
        return invalid("project.json exceeds 64 MiB");
    }
    let declared = entry.size();
    let mut bytes = Vec::new();
    entry.take(MAX_MANIFEST_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != declared || bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return invalid("project.json expanded size differs from ZIP metadata");
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value]> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| TransferError::Invalid(format!("{key} must be an array")))
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| TransferError::Invalid(format!("{key} must be a string")))
}

fn number(value: &Value, key: &str) -> Result<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| TransferError::Invalid(format!("{key} must be a nonnegative integer")))
}

fn validate_attachments<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    entries: &HashMap<String, usize>,
    manifest: &Value,
) -> Result<()> {
    let mut declared_paths = HashSet::from(["project.json"]);
    for key in ["audio", "media"] {
        let items = match manifest.get(key) {
            None | Some(Value::Null) => continue,
            Some(_) => array(manifest, key)?,
        };
        for item in items {
            let path = string(item, "path")?;
            if !safe_path(path) || !path.starts_with(&format!("{key}/")) {
                return invalid(format!("{key} entry has an unsafe path: {path:?}"));
            }
            if !declared_paths.insert(path) {
                return invalid(format!("attachment path declared twice: {path:?}"));
            }
            let index = entries
                .get(path)
                .copied()
                .ok_or_else(|| TransferError::Invalid(format!("missing attachment: {path:?}")))?;
            let size = number(item, "byteSize")?;
            let checksum = string(item, "sha256")?;
            if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return invalid(format!("invalid SHA-256 for attachment: {path:?}"));
            }
            let entry = archive.by_index(index)?;
            if entry.size() != size || size > MAX_ARCHIVE_BYTES {
                return invalid(format!("attachment size mismatch: {path:?}"));
            }
            let mut reader = entry.take(size + 1);
            let mut digest = Sha256::new();
            let actual = std::io::copy(&mut reader, &mut digest)?;
            if actual != size || !format!("{:x}", digest.finalize()).eq_ignore_ascii_case(checksum)
            {
                return invalid(format!("attachment size or SHA-256 mismatch: {path:?}"));
            }
        }
    }
    if declared_paths.len() != entries.len() {
        return invalid("archive contains undeclared files");
    }
    Ok(())
}

fn preview_manifest(manifest: Value) -> Result<ImportPreview> {
    if string(&manifest, "format")? != "writers-studio-project" {
        return invalid("format must be writers-studio-project");
    }
    let format_version = number(&manifest, "formatVersion")?;
    if !(1..=8).contains(&format_version) {
        return Err(TransferError::UnsupportedFormatVersion(format_version));
    }
    let title = string(&manifest["project"], "title")?.to_owned();
    if title.trim().is_empty() || title.chars().count() > 120 {
        return invalid("project title must contain 1–120 characters");
    }
    let nodes = array(&manifest, "nodes")?;
    let revisions = array(&manifest, "documentRevisions")?;
    if nodes.len() > MAX_NODES || revisions.len() > MAX_REVISIONS {
        return invalid("node or document revision count exceeds supported limits");
    }
    let mut node_ids = HashSet::with_capacity(nodes.len());
    for node in nodes {
        let id = string(node, "id")?;
        if id.is_empty() || !node_ids.insert(id) {
            return invalid("node IDs must be nonempty and unique");
        }
    }
    let mut latest: HashMap<&str, (u64, &Value)> = HashMap::new();
    let mut revision_ids = HashSet::with_capacity(revisions.len());
    let mut revision_numbers = HashSet::with_capacity(revisions.len());
    for revision in revisions {
        let id = string(revision, "id")?;
        let node_id = string(revision, "nodeId")?;
        let ordinal = number(revision, "revision")?;
        if id.is_empty()
            || !revision_ids.insert(id)
            || !node_ids.contains(node_id)
            || ordinal == 0
            || !revision_numbers.insert((node_id, ordinal))
        {
            return invalid("revision IDs/numbers must be unique and reference existing nodes");
        }
        if latest
            .get(node_id)
            .is_none_or(|(current, _)| ordinal > *current)
        {
            latest.insert(node_id, (ordinal, revision));
        }
    }
    let mut documents = Vec::with_capacity(latest.len());
    for node in nodes {
        let id = string(node, "id")?;
        let current = number(node, "currentRevision")?;
        let Some((ordinal, revision)) = latest.get(id) else {
            if current != 0 {
                return invalid(format!("node {id:?} has no current revision"));
            }
            continue;
        };
        if *ordinal != current {
            return invalid(format!(
                "node {id:?} currentRevision does not match its latest revision"
            ));
        }
        let rich_content = revision
            .get("content")
            .filter(|value| value.is_object())
            .ok_or_else(|| {
                TransferError::Invalid("revision content must be a JSON object".into())
            })?;
        documents.push(ImportedDocument {
            id: id.to_owned(),
            title: string(node, "title")?.to_owned(),
            plain_text: string(revision, "plainText")?.to_owned(),
            rich_content: rich_content.clone(),
        });
    }
    Ok(ImportPreview {
        title,
        format_version,
        documents,
        manifest,
    })
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "Test fixtures report which setup step failed"
)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn fixture(version: u64) -> Value {
        json!({
            "format":"writers-studio-project", "formatVersion":version,
            "project":{"title":"Книга 🪶"},
            "nodes":[{"id":"node-1","title":"Глава","currentRevision":1}],
            "documentRevisions":[{"id":"rev-1","nodeId":"node-1","revision":1,
                "plainText":"Привет 🪶", "content":{"type":"doc","attrs":{"schemaVersion":4},
                "content":[{"type":"paragraph","attrs":{"blockId":"stable"},
                "content":[{"type":"text","text":"Привет 🪶","marks":[{"type":"bold"}]}]}],
                "futureRichField":{"keep":true}}}],
            "audio":[], "media":[], "futureOpaqueSection":{"keep":[1,2,3]}
        })
    }

    fn archive(manifest: &Value, extras: &[(&str, &[u8])]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("temporary archive");
        {
            let mut writer = zip::ZipWriter::new(file.as_file_mut());
            writer
                .start_file("project.json", SimpleFileOptions::default())
                .expect("manifest entry");
            writer
                .write_all(&serde_json::to_vec(manifest).expect("manifest JSON"))
                .expect("manifest bytes");
            for (name, bytes) in extras {
                writer
                    .start_file(*name, SimpleFileOptions::default())
                    .expect("extra entry");
                writer.write_all(bytes).expect("extra bytes");
            }
            writer.finish().expect("finish archive");
        }
        file
    }

    #[test]
    fn preserves_unknown_rich_data_and_cyrillic_for_versions_one_and_eight() {
        for version in [1, 8] {
            let manifest = fixture(version);
            let file = archive(&manifest, &[]);
            let preview = inspect_archive(file.path()).expect("valid project preview");
            assert_eq!(preview.manifest, manifest);
            assert_eq!(
                preview.documents[0].rich_content,
                manifest["documentRevisions"][0]["content"]
            );
            assert_eq!(preview.documents[0].plain_text, "Привет 🪶");
        }
    }

    #[test]
    fn rejects_future_versions() {
        let file = archive(&fixture(9), &[]);
        assert!(matches!(
            inspect_archive(file.path()),
            Err(TransferError::UnsupportedFormatVersion(9))
        ));
    }

    #[test]
    fn rejects_traversal_and_undeclared_files_without_extracting() {
        for name in [
            "../outside",
            "audio/../outside",
            "C:/outside",
            "audio\\outside",
            "unknown.bin",
        ] {
            let file = archive(&fixture(8), &[(name, b"test")]);
            assert!(
                inspect_archive(file.path()).is_err(),
                "accepted unsafe/undeclared entry {name}"
            );
        }
    }

    #[test]
    fn verifies_attachment_hashes_and_lengths() {
        let mut manifest = fixture(8);
        manifest["audio"] = json!([{"path":"audio/source.webm","byteSize":4,"sha256":format!("{:x}", Sha256::digest(b"test"))}]);
        let valid = archive(&manifest, &[("audio/source.webm", b"test")]);
        assert!(inspect_archive(valid.path()).is_ok());
        let changed = archive(&manifest, &[("audio/source.webm", b"fail")]);
        assert!(inspect_archive(changed.path()).is_err());
        manifest["audio"][0]["byteSize"] = json!(3);
        let wrong_size = archive(&manifest, &[("audio/source.webm", b"test")]);
        assert!(inspect_archive(wrong_size.path()).is_err());
    }

    #[test]
    fn rejects_missing_and_inconsistent_revisions() {
        let mut manifest = fixture(8);
        manifest["nodes"][0]["currentRevision"] = json!(2);
        assert!(inspect_archive(archive(&manifest, &[]).path()).is_err());
        manifest["documentRevisions"] = json!([]);
        assert!(inspect_archive(archive(&manifest, &[]).path()).is_err());
    }

    #[test]
    fn rejects_advertised_oversized_manifest_without_allocating_it() {
        let file = archive(&fixture(8), &[]);
        let mut bytes = std::fs::read(file.path()).expect("fixture bytes");
        let central = bytes
            .windows(4)
            .position(|window| window == [0x50, 0x4b, 0x01, 0x02])
            .expect("central directory record");
        bytes[central + 24..central + 28]
            .copy_from_slice(&((MAX_MANIFEST_BYTES + 1) as u32).to_le_bytes());
        std::fs::write(file.path(), bytes).expect("oversized manifest fixture");
        assert!(inspect_archive(file.path()).is_err());
    }

    #[test]
    fn rejects_advertised_entry_counts_before_zip_allocation() {
        let file = archive(&fixture(8), &[]);
        let mut bytes = std::fs::read(file.path()).expect("fixture bytes");
        let footer = bytes.len() - 22;
        let count = ((MAX_ENTRIES + 1) as u16).to_le_bytes();
        bytes[footer + 8..footer + 10].copy_from_slice(&count);
        bytes[footer + 10..footer + 12].copy_from_slice(&count);
        std::fs::write(file.path(), bytes).expect("oversized entry count fixture");
        assert!(inspect_archive(file.path()).is_err());
    }

    #[test]
    fn rejects_truncated_zip() {
        let file = archive(&fixture(8), &[]);
        file.as_file().set_len(12).expect("truncate fixture");
        assert!(inspect_archive(file.path()).is_err());
    }

    #[test]
    fn rejects_duplicate_names_hidden_by_zip_index() {
        let file = archive(&fixture(8), &[("PROJECT.JSON", b"{}")]);
        let mut bytes = std::fs::read(file.path()).expect("fixture bytes");
        for index in 0..=bytes.len() - 12 {
            if &bytes[index..index + 12] == b"PROJECT.JSON" {
                bytes[index..index + 12].copy_from_slice(b"project.json");
            }
        }
        std::fs::write(file.path(), bytes).expect("duplicate entry fixture");
        assert!(inspect_archive(file.path()).is_err());
    }

    #[test]
    fn rejects_symlink_entries() {
        let mut file = tempfile::NamedTempFile::new().expect("temporary archive");
        {
            let mut writer = zip::ZipWriter::new(file.as_file_mut());
            writer
                .start_file("project.json", SimpleFileOptions::default())
                .expect("manifest entry");
            writer
                .write_all(&serde_json::to_vec(&fixture(8)).expect("manifest JSON"))
                .expect("manifest bytes");
            writer
                .add_symlink("audio/link", "../../private", SimpleFileOptions::default())
                .expect("symlink fixture");
            writer.finish().expect("finish symlink fixture");
        }
        assert!(inspect_archive(file.path()).is_err());
    }

    #[test]
    fn rejects_case_colliding_names() {
        let file = archive(&fixture(8), &[("PROJECT.JSON", b"{}")]);
        assert!(inspect_archive(file.path()).is_err());
    }
}
