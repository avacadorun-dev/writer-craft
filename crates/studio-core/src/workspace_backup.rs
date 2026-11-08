//! Consistent full-workspace backups, independent of live UI connections.
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
};
use sha2::{Digest, Sha256};

use super::{APPLICATION_ID, Result, StudioError};

/// Receipt for a complete, verified backup published without replacing an existing file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceBackup {
    /// Explicit destination selected by the caller.
    pub path: PathBuf,
    /// Exact encoded length of the standalone SQLite snapshot.
    pub byte_size: u64,
    /// SHA-256 of the complete published file, including all owned artifacts.
    pub sha256: String,
}

/// Back up the complete workspace, including committed WAL data and recoverable drafts.
///
/// Call on a worker after flushing the active editor and note drafts. This function opens a
/// separate read-only source connection and never shares a render-thread Workspace mutex.
/// Cancellation or failure removes its private staging file; the selected destination must not
/// exist. The caller must not treat this as a portable project or Markdown export.
///
/// # Errors
/// Returns an error for invalid/unreadable workspace data, cancellation, sustained SQLite locks,
/// incomplete snapshots or an existing destination. Neither the source nor existing output is
/// overwritten. The resulting SQLite file is self-contained; copying a live main file is not used.
pub fn backup_workspace(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    cancelled: &AtomicBool,
) -> Result<WorkspaceBackup> {
    fn check(cancelled: &AtomicBool) -> Result<()> {
        if cancelled.load(Ordering::Acquire) {
            return Err(StudioError::Validation(
                "Workspace backup was cancelled".into(),
            ));
        }
        Ok(())
    }
    check(cancelled)?;
    let source = source.as_ref();
    let destination = destination.as_ref();
    if super::is_workspace_data_path(destination, source)? {
        return Err(StudioError::Validation(
            "Choose a backup filename outside the workspace database, sidecars and settings files"
                .into(),
        ));
    }
    // symlink_metadata includes dangling links; final persist_noclobber closes the race.
    match std::fs::symlink_metadata(destination) {
        Ok(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Backup destination already exists",
            )
            .into());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let source = Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    source.busy_timeout(Duration::from_millis(250))?;
    let application_id: i64 =
        source.pragma_query_value(None, "application_id", |row| row.get(0))?;
    if application_id != APPLICATION_ID {
        return Err(StudioError::Validation(
            "Selected source is not a WriterCraft workspace".into(),
        ));
    }
    let staged = tempfile::Builder::new()
        .prefix(".writercraft-backup-")
        .tempfile_in(parent)?;
    let mut target = Connection::open(staged.path())?;
    let started = Instant::now();
    {
        let backup = Backup::new(&source, &mut target)?;
        loop {
            check(cancelled)?;
            if started.elapsed() > Duration::from_secs(120) {
                return Err(StudioError::Validation(
                    "Workspace backup timed out; retry when other workspace writes have finished"
                        .into(),
                ));
            }
            match backup.step(256)? {
                StepResult::Done => break,
                StepResult::More => {}
                StepResult::Busy | StepResult::Locked => {
                    std::thread::sleep(Duration::from_millis(25))
                }
                _ => {
                    return Err(StudioError::Validation(
                        "Unexpected SQLite backup state".into(),
                    ));
                }
            }
        }
    }
    let integrity: String = target.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(StudioError::CorruptData(
            "Workspace backup failed SQLite integrity verification".into(),
        ));
    }
    let copied_id: i64 = target.pragma_query_value(None, "application_id", |row| row.get(0))?;
    if copied_id != application_id {
        return Err(StudioError::CorruptData(
            "Workspace backup identity changed".into(),
        ));
    }
    // A copied WAL-mode header must become a standalone file before the connection closes.
    let mode: String = target.query_row("PRAGMA journal_mode=DELETE", [], |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("delete") {
        return Err(StudioError::CorruptData(
            "Workspace backup could not finalize its standalone journal".into(),
        ));
    }
    target.close().map_err(|(_, error)| error)?;
    staged.as_file().sync_all()?;
    let mut reader = staged.reopen()?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut length = 0_u64;
    loop {
        check(cancelled)?;
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        length += count as u64;
    }
    drop(reader);
    check(cancelled)?;
    staged
        .persist_noclobber(destination)
        .map_err(|error| error.error)?;
    Ok(WorkspaceBackup {
        path: destination.to_path_buf(),
        byte_size: length,
        sha256: format!("{:x}", digest.finalize()),
    })
}
