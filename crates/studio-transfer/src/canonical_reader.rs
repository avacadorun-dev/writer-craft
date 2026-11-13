//! Pinned native canonical ZIPs are distinct from the unchanged source `.wstudio` reader.
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use studio_core::{
    ImportedProject, MAX_PORTABLE_MEMBERS, MAX_PORTABLE_ROOT_BYTES, MAX_PORTABLE_TRANSPORT_BYTES,
    PortableObjectReference, PortableObjectSource, PortableProjectHeader, ProjectImportWorker,
    ProjectTransferCancellation, VerifiedPortableProjectStream,
};
use zip::ZipArchive;

use super::{Result, TransferError, invalid};

/// An explicitly native canonical archive pinned to its opened file, not a path reopened after review.
/// Opening checks structure/root only. `preflight` verifies every object byte before semantic restore.
pub struct CanonicalProjectArchive {
    archive: ZipArchive<File>,
    pinned: File,
    entries: HashMap<String, (usize, u64)>,
    root: Vec<u8>,
    encoded_size: u64,
    cancel: ProjectTransferCancellation,
}
impl CanonicalProjectArchive {
    /// Open a bounded native ZIP without extracting files or modifying the source/workspace.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_cancellation(path, &ProjectTransferCancellation::default())
    }
    /// Open on a worker while observing the owner's cancellation during bounded directory work.
    pub fn open_with_cancellation(
        path: impl AsRef<Path>,
        cancel: &ProjectTransferCancellation,
    ) -> Result<Self> {
        check_cancel(cancel)?;
        let mut file = File::open(path)?;
        let encoded_size = file.metadata()?.len();
        if encoded_size > MAX_PORTABLE_TRANSPORT_BYTES {
            return invalid("Native encoded archive exceeds 1 GiB");
        }
        let directory = directory(&mut file, cancel)?;
        let pinned = file.try_clone()?;
        let mut archive = ZipArchive::new(file)?;
        let entries = checked_entries(&mut archive, &directory, cancel)?;
        let root = read_root(&mut archive, &entries)?;
        PortableProjectHeader::parse(&root).map_err(core_error)?;
        Ok(Self {
            archive,
            pinned,
            entries,
            root,
            encoded_size,
            cancel: cancel.clone(),
        })
    }
    /// Original bounded native descriptor bytes. This is not a semantic import acceptance claim.
    pub fn root_bytes(&self) -> &[u8] {
        &self.root
    }
    /// Freeze all checked objects/shards in core's private index before any workspace write.
    pub fn preflight(
        &mut self,
        cancel: &ProjectTransferCancellation,
    ) -> Result<VerifiedPortableProjectStream> {
        self.cancel = cancel.clone();
        let root = self.root.clone();
        VerifiedPortableProjectStream::preflight(&root, self, cancel).map_err(core_error)
    }
    /// Restore this exact pinned native file through the same fully scoped core transaction.
    pub fn restore(
        &mut self,
        worker: &mut ProjectImportWorker,
        cancel: &ProjectTransferCancellation,
    ) -> Result<ImportedProject> {
        let stream = self.preflight(cancel)?;
        worker.restore_stream(&stream, cancel).map_err(core_error)
    }
}
impl PortableObjectSource for CanonicalProjectArchive {
    fn verify_transport(&mut self) -> std::io::Result<()> {
        let mut verify = || -> Result<()> {
            check_cancel(&self.cancel)?;
            if self.pinned.metadata()?.len() != self.encoded_size {
                return invalid("Native archive size changed after opening");
            }
            let current = directory(&mut self.pinned, &self.cancel)?;
            if current.len() != self.entries.len()
                || current.iter().any(|(name, size)| {
                    self.entries.get(name).map(|(_, captured)| captured) != Some(size)
                })
            {
                return invalid("Native archive inventory changed after opening");
            }
            let mut fresh = ZipArchive::new(self.pinned.try_clone()?)?;
            let entries = checked_entries(&mut fresh, &current, &self.cancel)?;
            if read_root(&mut fresh, &entries)? != self.root {
                return invalid("Native root changed after opening");
            }
            self.archive = fresh;
            self.entries = entries;
            Ok(())
        };
        verify().map_err(std::io::Error::other)
    }
    fn visit_objects(
        &mut self,
        visitor: &mut dyn FnMut(PortableObjectReference) -> studio_core::Result<()>,
    ) -> studio_core::Result<()> {
        for (name, (_, size)) in &self.entries {
            if self.cancel.is_cancelled() {
                return Err(studio_core::StudioError::Validation(
                    "Native archive preflight cancelled".into(),
                ));
            }
            if let Some(sha) = name.strip_prefix("objects/") {
                visitor(PortableObjectReference::new(sha, *size)?)?;
            }
        }
        Ok(())
    }
    fn open<'a>(
        &'a mut self,
        object: &PortableObjectReference,
    ) -> std::io::Result<Box<dyn Read + 'a>> {
        let path = format!("objects/{}", object.sha256());
        let (index, size) =
            self.entries.get(&path).copied().ok_or_else(|| {
                std::io::Error::other("Native object is outside captured inventory")
            })?;
        if size != object.byte_size() {
            return Err(std::io::Error::other(
                "Native object length differs from captured inventory",
            ));
        }
        let entry = self
            .archive
            .by_index(index)
            .map_err(std::io::Error::other)?;
        if entry.name_raw() != path.as_bytes() || entry.size() != size {
            return Err(std::io::Error::other("Native object identity changed"));
        }
        Ok(Box::new(entry))
    }
}
fn checked_entries(
    archive: &mut ZipArchive<File>,
    directory: &HashMap<String, u64>,
    cancel: &ProjectTransferCancellation,
) -> Result<HashMap<String, (usize, u64)>> {
    if archive.len() != directory.len() {
        return invalid("ZIP library resolved a duplicate native member");
    }
    let mut entries = HashMap::new();
    let mut expanded = 0_u64;
    for index in 0..archive.len() {
        check_cancel(cancel)?;
        let entry = archive.by_index(index)?;
        let name = std::str::from_utf8(entry.name_raw())
            .map_err(|_| TransferError::Invalid("Native member name must be UTF-8".into()))?;
        if !member_name(name) || entry.is_dir() {
            return invalid("Native archive contains an unknown or unsafe member path");
        }
        if let Some(mode) = entry.unix_mode() {
            let kind = mode & 0o170_000;
            if kind != 0 && kind != 0o100_000 {
                return invalid("Native archive members must be regular files");
            }
        }
        expanded = expanded
            .checked_add(entry.size())
            .filter(|size| *size <= MAX_PORTABLE_TRANSPORT_BYTES)
            .ok_or_else(|| {
                TransferError::Invalid("Native expanded archive exceeds 1 GiB".into())
            })?;
        if directory.get(name) != Some(&entry.size())
            || entries
                .insert(name.to_owned(), (index, entry.size()))
                .is_some()
        {
            return invalid("Native directory/member identity disagrees");
        }
    }
    Ok(entries)
}
fn read_root(
    archive: &mut ZipArchive<File>,
    entries: &HashMap<String, (usize, u64)>,
) -> Result<Vec<u8>> {
    let (root_index, size) = entries
        .get("project.json")
        .copied()
        .ok_or_else(|| TransferError::Invalid("Native root project.json is missing".into()))?;
    if size > MAX_PORTABLE_ROOT_BYTES as u64 {
        return invalid("Native root exceeds 1 MiB");
    }
    let mut root = Vec::new();
    archive
        .by_index(root_index)?
        .take(MAX_PORTABLE_ROOT_BYTES as u64 + 1)
        .read_to_end(&mut root)?;
    if root.len() as u64 != size {
        return invalid("Native root length differs from the ZIP directory");
    }
    Ok(root)
}

fn member_name(name: &str) -> bool {
    name == "project.json"
        || name.strip_prefix("objects/").is_some_and(|sha| {
            sha.len() == 64
                && sha
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}
fn u16le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}
fn u32le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}
fn u64le(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}
/// Inspect the raw directory before ZipArchive can collapse duplicate names. The native container
/// supports bounded ZIP64 entry counts; the legacy reader continues rejecting ZIP64 unchanged.
fn directory(
    file: &mut File,
    cancel: &ProjectTransferCancellation,
) -> Result<HashMap<String, u64>> {
    let size = file.metadata()?.len();
    if size > MAX_PORTABLE_TRANSPORT_BYTES {
        return invalid("Native encoded archive exceeds 1 GiB");
    }
    let tail_length = size.min(65_557) as usize;
    if tail_length < 22 {
        return invalid("Native ZIP footer is truncated");
    }
    file.seek(SeekFrom::End(-(tail_length as i64)))?;
    let mut tail = vec![0_u8; tail_length];
    file.read_exact(&mut tail)?;
    let offset = (0..=tail.len() - 22)
        .rev()
        .find(|offset| {
            tail[*offset..*offset + 4] == [0x50, 0x4b, 0x05, 0x06]
                && *offset + 22 + usize::from(u16le(&tail[*offset + 20..])) == tail.len()
        })
        .ok_or_else(|| TransferError::Invalid("Native ZIP footer is missing".into()))?;
    let footer = &tail[offset..offset + 22];
    if u16le(&footer[4..]) != 0
        || u16le(&footer[6..]) != 0
        || u16le(&footer[8..]) != u16le(&footer[10..])
    {
        return invalid("Multi-disk native ZIPs are unsupported");
    }
    let footer_start = size - tail_length as u64 + offset as u64;
    let mut count = u64::from(u16le(&footer[10..]));
    let mut directory_size = u64::from(u32le(&footer[12..]));
    let mut directory_start = u64::from(u32le(&footer[16..]));
    let mut directory_end = footer_start;
    if footer_start >= 20 {
        file.seek(SeekFrom::Start(footer_start - 20))?;
        let mut locator = [0_u8; 20];
        file.read_exact(&mut locator)?;
        if locator[..4] == [0x50, 0x4b, 0x06, 0x07] {
            if u32le(&locator[4..]) != 0 || u32le(&locator[16..]) != 1 {
                return invalid("Multi-disk native ZIP64 is unsupported");
            }
            let zip64_start = u64le(&locator[8..]);
            if zip64_start
                .checked_add(56)
                .is_none_or(|end| end > footer_start - 20)
            {
                return invalid("Native ZIP64 directory bounds are invalid");
            }
            file.seek(SeekFrom::Start(zip64_start))?;
            let mut record = [0_u8; 56];
            file.read_exact(&mut record)?;
            let extension = u64le(&record[4..]);
            if record[..4] != [0x50, 0x4b, 0x06, 0x06]
                || !(44..=65_536).contains(&extension)
                || zip64_start
                    .checked_add(12)
                    .and_then(|start| start.checked_add(extension))
                    != Some(footer_start - 20)
                || u32le(&record[16..]) != 0
                || u32le(&record[20..]) != 0
                || u64le(&record[24..]) != u64le(&record[32..])
            {
                return invalid("Native ZIP64 footer fields are inconsistent");
            }
            count = u64le(&record[32..]);
            directory_size = u64le(&record[40..]);
            directory_start = u64le(&record[48..]);
            directory_end = zip64_start;
        }
    }
    if count == 0 || count > MAX_PORTABLE_MEMBERS {
        return invalid("Native archive must contain 1–65536 members");
    }
    if directory_start.checked_add(directory_size) != Some(directory_end) {
        return invalid("Native directory bounds are inconsistent");
    }
    file.seek(SeekFrom::Start(directory_start))?;
    let mut consumed = 0_u64;
    let mut names = HashSet::new();
    let mut entries = HashMap::new();
    let mut expanded = 0_u64;
    for _ in 0..count {
        check_cancel(cancel)?;
        if consumed + 46 > directory_size {
            return invalid("Native central directory is truncated");
        }
        let mut header = [0_u8; 46];
        file.read_exact(&mut header)?;
        if header[..4] != [0x50, 0x4b, 0x01, 0x02] {
            return invalid("Native central directory record is invalid");
        }
        let name_length = usize::from(u16le(&header[28..]));
        let extra = u64::from(u16le(&header[30..]));
        let comment = u64::from(u16le(&header[32..]));
        consumed += 46 + name_length as u64 + extra + comment;
        if name_length == 0 || name_length > 72 || consumed > directory_size {
            return invalid("Native member name/directory length is invalid");
        }
        let mut raw_name = vec![0_u8; name_length];
        file.read_exact(&mut raw_name)?;
        let name = std::str::from_utf8(&raw_name)
            .map_err(|_| TransferError::Invalid("Native directory name must be UTF-8".into()))?;
        if !member_name(name) || !names.insert(name.to_owned()) {
            return invalid("Native archive has an unsafe, noncanonical or duplicate member");
        }
        let mut size = u64::from(u32le(&header[24..]));
        let mut extra_bytes = vec![0_u8; extra as usize];
        file.read_exact(&mut extra_bytes)?;
        let mut extra_offset = 0_usize;
        let mut zip64_seen = false;
        while extra_offset < extra_bytes.len() {
            if extra_offset + 4 > extra_bytes.len() {
                return invalid("Native ZIP extra field is truncated");
            }
            let kind = u16le(&extra_bytes[extra_offset..]);
            let length = usize::from(u16le(&extra_bytes[extra_offset + 2..]));
            let start = extra_offset + 4;
            let end = start
                .checked_add(length)
                .filter(|end| *end <= extra_bytes.len())
                .ok_or_else(|| {
                    TransferError::Invalid("Native ZIP extra field length is invalid".into())
                })?;
            if kind == 1 {
                if zip64_seen {
                    return invalid("Native member repeats its ZIP64 size field");
                }
                zip64_seen = true;
                if size == u64::from(u32::MAX) {
                    if length < 8 {
                        return invalid("Native ZIP64 member size is truncated");
                    }
                    size = u64le(&extra_bytes[start..]);
                }
            }
            extra_offset = end;
        }
        if size == u64::from(u32::MAX) || size > MAX_PORTABLE_TRANSPORT_BYTES {
            return invalid("Native ZIP64 member exceeds the supported transport bound");
        }
        expanded = expanded
            .checked_add(size)
            .filter(|total| *total <= MAX_PORTABLE_TRANSPORT_BYTES)
            .ok_or_else(|| {
                TransferError::Invalid("Native expanded archive exceeds 1 GiB".into())
            })?;
        entries.insert(name.to_owned(), size);
        file.seek(SeekFrom::Current(comment as i64))?;
    }
    if consumed != directory_size {
        return invalid("Native directory has undeclared records");
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(entries)
}
fn check_cancel(cancel: &ProjectTransferCancellation) -> Result<()> {
    if cancel.is_cancelled() {
        invalid("Native canonical import cancelled")
    } else {
        Ok(())
    }
}
fn core_error(error: studio_core::StudioError) -> TransferError {
    TransferError::Invalid(error.to_string())
}
