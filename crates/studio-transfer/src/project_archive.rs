//! Complete project transfer uses a pinned archive and the core's semantic authority.
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;

use studio_core::{
    ImportAssetSource, ImportedProject, ProjectAttachment, ProjectExportSnapshot,
    ProjectImportWorker, ProjectTransferCancellation, ValidatedProjectImport,
};
use zip::ZipArchive;
use zip::write::SimpleFileOptions;

use super::{
    MAX_ARCHIVE_BYTES, MAX_MANIFEST_BYTES, Result, TransferError, invalid, preflight_directory,
    safe_path, validate_entries,
};

/// An opened source file plus validated source semantics, never a path reopened after preview.
/// Binary integrity is checked while the core streams bytes into its import transaction.
pub struct ProjectArchive {
    archive: ZipArchive<File>,
    entries: HashMap<String, usize>,
    project: ValidatedProjectImport,
}
impl ProjectArchive {
    /// Open a bounded archive, verify its member structure and validate all known project domains.
    /// This does not create a project or label unconsumed binary data verified.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;
        if file.metadata()?.len() > MAX_ARCHIVE_BYTES {
            return invalid("compressed archive exceeds 512 MiB");
        }
        let count = preflight_directory(&mut file)?;
        let mut archive = ZipArchive::new(file)?;
        if archive.len() != count {
            return invalid("ZIP library resolved duplicate entry names");
        }
        let entries = validate_entries(&mut archive)?;
        let index = entries
            .get("project.json")
            .copied()
            .ok_or_else(|| TransferError::Invalid("missing project.json".into()))?;
        let entry = archive.by_index(index)?;
        let size = entry.size();
        if size > MAX_MANIFEST_BYTES {
            return invalid("project.json exceeds 64 MiB");
        }
        let mut bytes = Vec::new();
        entry.take(MAX_MANIFEST_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != size {
            return invalid("project.json size differs from its ZIP directory");
        }
        let project = ValidatedProjectImport::parse(&bytes).map_err(core_error)?;
        let mut declared = HashSet::from(["project.json".to_owned()]);
        for attachment in project.attachments() {
            if !safe_path(&attachment.path) || !declared.insert(attachment.path.clone()) {
                return invalid("unsafe or duplicate declared attachment");
            }
            let index = entries
                .get(&attachment.path)
                .copied()
                .ok_or_else(|| TransferError::Invalid("declared attachment is missing".into()))?;
            if archive.by_index(index)?.size() != attachment.byte_size {
                return invalid("attachment size differs from its ZIP directory");
            }
        }
        if declared.len() != entries.len() {
            return invalid("archive contains undeclared members");
        }
        Ok(Self {
            archive,
            entries,
            project,
        })
    }
    /// Read-only validated manifest and original attachment descriptors.
    pub fn project(&self) -> &ValidatedProjectImport {
        &self.project
    }
    /// Commit through a dedicated core worker; cancellation/error rolls the whole import back.
    pub fn import(
        &mut self,
        worker: &mut ProjectImportWorker,
        cancel: &ProjectTransferCancellation,
    ) -> Result<ImportedProject> {
        let mut source = ZipSource {
            archive: &mut self.archive,
            entries: &self.entries,
        };
        worker
            .import(&self.project, &mut source, cancel)
            .map_err(core_error)
    }
    /// Independently verify every declared binary without extracting or modifying files.
    pub fn verify_integrity(&mut self, cancel: &ProjectTransferCancellation) -> Result<()> {
        use sha2::{Digest, Sha256};
        let mut source = ZipSource {
            archive: &mut self.archive,
            entries: &self.entries,
        };
        for attachment in self.project.attachments() {
            if cancel.is_cancelled() {
                return invalid("archive verification cancelled");
            }
            let mut reader = source.open(attachment)?;
            let mut digest = Sha256::new();
            let mut count = 0_u64;
            let mut buffer = vec![0_u8; 64 * 1024];
            loop {
                if cancel.is_cancelled() {
                    return invalid("archive verification cancelled");
                }
                let read = reader.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                count = count
                    .checked_add(read as u64)
                    .filter(|n| *n <= attachment.byte_size)
                    .ok_or_else(|| {
                        TransferError::Invalid("attachment exceeds declared size".into())
                    })?;
                digest.update(&buffer[..read]);
            }
            if count != attachment.byte_size
                || !format!("{:x}", digest.finalize()).eq_ignore_ascii_case(&attachment.sha256)
            {
                return invalid("attachment size or SHA-256 mismatch");
            }
        }
        Ok(())
    }
}
struct ZipSource<'a> {
    archive: &'a mut ZipArchive<File>,
    entries: &'a HashMap<String, usize>,
}
impl ImportAssetSource for ZipSource<'_> {
    fn open<'a>(
        &'a mut self,
        attachment: &ProjectAttachment,
    ) -> std::io::Result<Box<dyn Read + 'a>> {
        let index = self.entries.get(&attachment.path).copied().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Captured attachment is missing",
            )
        })?;
        let entry = self
            .archive
            .by_index(index)
            .map_err(std::io::Error::other)?;
        if entry.size() != attachment.byte_size || entry.name_raw() != attachment.path.as_bytes() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Captured attachment identity changed",
            ));
        }
        Ok(Box::new(entry))
    }
}

/// Write the saved project view as a .wstudio ZIP with every original artifact byte.
/// Native recovery evidence belongs in the canonical companion, not a legacy format extension.
/// The caller owns staging/no-clobber publication and must not report success before this finishes.
pub fn write_project_archive<W: Write + Seek>(
    snapshot: &ProjectExportSnapshot,
    writer: W,
    cancel: &ProjectTransferCancellation,
) -> Result<W> {
    if cancel.is_cancelled() {
        return invalid("project export cancelled");
    }
    if snapshot.requires_canonical_companion() {
        return invalid(
            "This project contains native notes, recovery or review state; export the canonical companion instead of a legacy-only archive",
        );
    }
    // Never publish an archive which our bounded reader must reject on reopening.
    studio_core::ValidatedProjectImport::parse(snapshot.manifest_bytes()).map_err(core_error)?;
    let expanded = snapshot.attachments().iter().try_fold(
        u64::try_from(snapshot.manifest_bytes().len()).map_err(|_| {
            TransferError::Invalid("Export manifest length exceeds capacity".into())
        })?,
        |total, attachment| {
            total
                .checked_add(attachment.byte_size)
                .filter(|n| *n <= studio_core::MAX_PROJECT_TRANSFER_BYTES)
                .ok_or_else(|| {
                    TransferError::Invalid("Expanded project archive exceeds 512 MiB".into())
                })
        },
    )?;
    if expanded > studio_core::MAX_PROJECT_TRANSFER_BYTES {
        return invalid("Expanded project archive exceeds 512 MiB");
    }
    let mut archive = zip::ZipWriter::new(writer);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    archive.start_file("project.json", options)?;
    archive.write_all(snapshot.manifest_bytes())?;
    let mut written = HashSet::from(["project.json".to_owned()]);
    for attachment in snapshot.attachments() {
        if !safe_path(&attachment.path) || !written.insert(attachment.path.clone()) {
            return invalid("unsafe or duplicate export member");
        }
        archive.start_file(&attachment.path, options)?;
        snapshot
            .assets()
            .copy_to(attachment, &mut archive, cancel)
            .map_err(core_error)?;
    }
    let writer = archive.finish()?;
    if cancel.is_cancelled() {
        return invalid("project export cancelled");
    }
    Ok(writer)
}
fn core_error(error: studio_core::StudioError) -> TransferError {
    TransferError::Invalid(error.to_string())
}
