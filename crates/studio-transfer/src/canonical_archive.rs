//! Native format-2 ZIP publication is distinct from source-compatible `.wstudio` archives.
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use sha2::{Digest, Sha256};
use studio_core::{
    MAX_PORTABLE_TRANSPORT_BYTES, PortableObjectReference, PreparedPortableProject,
    ProjectTransferCancellation, is_workspace_data_path,
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use super::{PortableFileReceipt, Result, TransferError, invalid};

/// Write a prepared native format-2 project and unique objects to a caller-owned staged writer.
///
/// This is a distinct native container; the original Studio reader is not advertised as supporting
/// it. Publication and actual encoded-size verification belong to the file wrapper below.
///
/// # Errors
/// Returns cancellation, storage, ZIP, object-integrity or finite resource-budget failures.
pub fn write_canonical_project_archive<W: Write + Seek>(
    project: &PreparedPortableProject,
    writer: W,
    cancel: &ProjectTransferCancellation,
) -> Result<W> {
    check_cancel(cancel)?;
    let mut archive = ZipWriter::new(writer);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    archive.start_file("project.json", options)?;
    archive.write_all(project.root_bytes())?;
    project
        .visit_objects(&mut |reference| {
            if cancel.is_cancelled() {
                return Err(studio_core::StudioError::Validation(
                    "Canonical export cancelled".into(),
                ));
            }
            archive
                .start_file(object_path(reference), options)
                .map_err(|error| {
                    studio_core::StudioError::Validation(format!(
                        "Cannot write canonical ZIP object: {error}"
                    ))
                })?;
            project.copy_object(reference, &mut archive, cancel)
        })
        .map_err(core_error)?;
    let result = archive.finish()?;
    check_cancel(cancel)?;
    Ok(result)
}

/// Verify and publish a native canonical ZIP only to an absent destination.
///
/// The staged archive is independently reopened, every exact expected member is read through ZIP
/// CRC and SHA-256 checks, and actual encoded size is limited before no-clobber publication. A
/// failed/cancelled operation removes its own sibling temporary file and never changes the source
/// workspace, existing destination or unrelated files. Run on a dedicated worker.
///
/// # Errors
/// Refuses protected data paths, existing output, corruption, cancellation and I/O/ZIP failures.
pub fn export_canonical_project_archive_file(
    project: &PreparedPortableProject,
    source_database: &Path,
    destination: &Path,
    cancel: &ProjectTransferCancellation,
) -> Result<PortableFileReceipt> {
    if is_workspace_data_path(destination, source_database)? {
        return invalid(
            "Choose a canonical destination away from databases, journals and preferences",
        );
    }
    check_cancel(cancel)?;
    if destination.try_exists()? {
        return invalid("The canonical destination already exists; choose a new filename");
    }
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::Builder::new()
        .prefix(".writercraft-canonical-")
        .tempfile_in(parent)?;
    write_canonical_project_archive(project, temporary.as_file_mut(), cancel)?;
    temporary.as_file_mut().flush()?;
    temporary.as_file().sync_all()?;
    let size = temporary.as_file().metadata()?.len();
    if size > MAX_PORTABLE_TRANSPORT_BYTES {
        return invalid("Encoded native canonical ZIP exceeds 1 GiB; use whole-workspace backup");
    }
    verify_staged(project, File::open(temporary.path())?, cancel)?;
    temporary.as_file_mut().seek(SeekFrom::Start(0))?;
    let mut digest = Sha256::new();
    let mut actual = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_cancel(cancel)?;
        let count = temporary.as_file_mut().read(&mut buffer)?;
        if count == 0 {
            break;
        }
        actual = actual
            .checked_add(count as u64)
            .filter(|count| *count <= MAX_PORTABLE_TRANSPORT_BYTES)
            .ok_or_else(|| {
                TransferError::Invalid("Canonical file changed size before publication".into())
            })?;
        digest.update(&buffer[..count]);
    }
    if actual != size {
        return invalid("Canonical file length changed before publication");
    }
    check_cancel(cancel)?;
    let published = temporary
        .persist_noclobber(destination)
        .map_err(|error| TransferError::Io(error.error))?;
    drop(published);
    Ok(PortableFileReceipt {
        path: destination.to_path_buf(),
        byte_size: size,
        sha256: format!("{:x}", digest.finalize()),
    })
}
fn verify_staged(
    project: &PreparedPortableProject,
    file: File,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    let mut archive = ZipArchive::new(file)?;
    let expected = usize::try_from(project.object_count())
        .map_err(std::io::Error::other)?
        .checked_add(1)
        .ok_or_else(|| TransferError::Invalid("Canonical member count overflow".into()))?;
    if archive.len() != expected {
        return invalid("Staged canonical archive has an unexpected member count");
    }
    {
        let mut root = archive.by_name("project.json")?;
        if root.size() != project.root_bytes().len() as u64 {
            return invalid("Staged canonical root length changed");
        }
        let mut bytes = Vec::new();
        root.read_to_end(&mut bytes)?;
        if bytes != project.root_bytes() {
            return invalid("Staged canonical root differs from the prepared descriptor");
        }
    }
    project
        .visit_objects(&mut |object| {
            let mut entry = archive.by_name(&object_path(object)).map_err(|error| {
                studio_core::StudioError::Validation(format!(
                    "Staged canonical object is missing: {error}"
                ))
            })?;
            if entry.size() != object.byte_size() {
                return Err(studio_core::StudioError::Validation(
                    "Staged canonical object length differs".into(),
                ));
            }
            let mut digest = Sha256::new();
            let mut size = 0_u64;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                if cancel.is_cancelled() {
                    return Err(studio_core::StudioError::Validation(
                        "Canonical verification cancelled".into(),
                    ));
                }
                let count = entry.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                size = size
                    .checked_add(count as u64)
                    .filter(|size| *size <= object.byte_size())
                    .ok_or_else(|| {
                        studio_core::StudioError::Validation(
                            "Staged object exceeds its expected length".into(),
                        )
                    })?;
                digest.update(&buffer[..count]);
            }
            if size != object.byte_size() || format!("{:x}", digest.finalize()) != object.sha256() {
                return Err(studio_core::StudioError::Validation(
                    "Staged canonical object checksum differs".into(),
                ));
            }
            Ok(())
        })
        .map_err(core_error)?;
    check_cancel(cancel)
}
fn object_path(reference: &PortableObjectReference) -> String {
    format!("objects/{}", reference.sha256())
}
fn check_cancel(cancel: &ProjectTransferCancellation) -> Result<()> {
    if cancel.is_cancelled() {
        invalid("Canonical project export cancelled before publication")
    } else {
        Ok(())
    }
}
fn core_error(error: studio_core::StudioError) -> TransferError {
    TransferError::Invalid(error.to_string())
}
