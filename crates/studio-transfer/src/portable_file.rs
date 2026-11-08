//! Single-file publication preserves unrelated files and all live workspace paths.
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use studio_core::{ProjectExportSnapshot, ProjectTransferCancellation, is_workspace_data_path};

use super::{Result, TransferError, write_project_archive};

/// A flushed, verified file published to an absent destination without overwriting an existing path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableFileReceipt {
    /// Explicit user-selected output; not embedded in its portable contents.
    pub path: PathBuf,
    /// Actual encoded file length.
    pub byte_size: u64,
    /// SHA-256 of the written file, verified before publication.
    pub sha256: String,
}
/// Publish a source-compatible saved .wstudio ZIP to a new path.
///
/// The actual selected workspace is mandatory for reserved database/journal/preferences checks.
/// An exclusively created sibling temporary file is flushed and verified before no-clobber
/// publication. Cancellation/failure cleans up that owned temporary file only. Projects needing
/// native evidence must use the canonical Markdown/artifact folder instead of this legacy format.
///
/// Run on a dedicated worker. Success means a completed file was published; this protocol does
/// not claim that every filesystem provides universal power-loss durability.
///
/// # Errors
/// Refuses live data paths, existing destinations, unsupported legacy loss, corruption, exhausted
/// limits, cancellation or filesystem errors. It never replaces a destination or removes user paths.
pub fn export_project_archive_file(
    snapshot: &ProjectExportSnapshot,
    source_database: &Path,
    destination: &Path,
    cancel: &ProjectTransferCancellation,
) -> Result<PortableFileReceipt> {
    if is_workspace_data_path(destination, source_database)? {
        return invalid(
            "Choose an archive destination away from workspace databases, journals and preferences",
        );
    }
    check_cancel(cancel)?;
    if snapshot.requires_canonical_companion() {
        return invalid(
            "Native notes, drafts or review evidence require the canonical Markdown and artifact folder",
        );
    }
    if destination.try_exists()? {
        return invalid("The archive destination already exists; choose a new filename");
    }
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::Builder::new()
        .prefix(".writercraft-export-")
        .tempfile_in(parent)?;
    write_project_archive(snapshot, temporary.as_file_mut(), cancel)?;
    temporary.as_file_mut().flush()?;
    temporary.as_file().sync_all()?;
    // Reopen the staged ZIP through the same bounded reader used for imports, including CRC/hash.
    let mut verification = super::ProjectArchive::open(temporary.path())?;
    verification.verify_integrity(cancel)?;
    drop(verification);
    let byte_size = temporary.as_file().metadata()?.len();
    if byte_size > studio_core::MAX_PROJECT_TRANSFER_BYTES {
        return invalid("Encoded project archive exceeds 512 MiB");
    }
    temporary.as_file_mut().seek(SeekFrom::Start(0))?;
    let mut digest = Sha256::new();
    let mut actual = 0_u64;
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        check_cancel(cancel)?;
        let count = temporary.as_file_mut().read(&mut buffer)?;
        if count == 0 {
            break;
        }
        actual = actual
            .checked_add(u64::try_from(count).map_err(std::io::Error::other)?)
            .ok_or_else(|| TransferError::Invalid("Archive size overflow".into()))?;
        digest.update(&buffer[..count]);
    }
    if actual != byte_size {
        return invalid("Archive size changed before publication");
    }
    check_cancel(cancel)?;
    // The parent directory was resolved before publication, and the primitive remains no-clobber
    // even if another application creates the chosen final path after the initial check.
    let published = temporary
        .persist_noclobber(destination)
        .map_err(|error| TransferError::Io(error.error))?;
    drop(published);
    Ok(PortableFileReceipt {
        path: destination.to_path_buf(),
        byte_size,
        sha256: format!("{:x}", digest.finalize()),
    })
}
fn check_cancel(cancel: &ProjectTransferCancellation) -> Result<()> {
    if cancel.is_cancelled() {
        return invalid("Project export cancelled before publication");
    }
    Ok(())
}
fn invalid<T>(message: &str) -> Result<T> {
    Err(TransferError::Invalid(message.into()))
}
