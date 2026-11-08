//! Native format-2 folder transport: paged reading files plus exact canonical payload objects.
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use cap_std::ambient_authority;
use cap_std::fs::{Dir, File, OpenOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use studio_core::{
    ImportedProject, MAX_PORTABLE_MEMBERS, MAX_PORTABLE_RECORD_BYTES, MAX_PORTABLE_RECORDS,
    MAX_PORTABLE_ROOT_BYTES, MAX_PORTABLE_SHARD_BYTES, MAX_PORTABLE_SHARD_RECORDS,
    MAX_PORTABLE_SHARDS, MAX_PORTABLE_TRANSPORT_BYTES, PortableObjectReference,
    PortableObjectSource, PortableProjectHeader, PreparedPortableProject, ProjectImportWorker,
    ProjectTransferCancellation, VerifiedPortableProjectStream, is_workspace_data_path,
};

use super::canonical_reading::ReadingPlan;
use super::portable_paths::safe_relative;
use super::{Result, TransferError};

const COMPLETION: &str = ".writercraft/export.json";
const CANONICAL: &str = ".writercraft/project.json";
const INCOMPLETE: &str = ".incomplete";

/// A completed, independently verified native folder export; no whole-project file array.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalFolderReceipt {
    /// New user-selected output directory, never merged into an existing folder.
    pub destination: PathBuf,
    /// Captured scope; selected scope cannot silently expand to a full project.
    pub scope: String,
    /// Actual physical members, including bounded control pages and completion root.
    pub file_count: u64,
    /// Actual total physical encoded bytes; shared object bytes are not double-counted magically.
    pub byte_size: u64,
    /// Reported reading-view differences, retained separately from canonical semantics.
    pub conversion_count: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Member {
    path: String,
    sha256: String,
    byte_size: u64,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Page {
    member: Member,
    record_count: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Completion {
    format: String,
    format_version: u64,
    state: String,
    scope: String,
    canonical: Member,
    inventory: Vec<Page>,
    objects: Vec<Page>,
    conversions: Vec<Page>,
    payload_count: u64,
    object_count: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ObjectMember {
    sha256: String,
    byte_size: u64,
    path: String,
}
#[derive(Default)]
struct Budget {
    members: u64,
    bytes: u64,
}
impl Budget {
    fn charge(&mut self, size: u64) -> Result<()> {
        self.members = self
            .members
            .checked_add(1)
            .filter(|n| *n <= MAX_PORTABLE_MEMBERS)
            .ok_or_else(|| {
                TransferError::Invalid("Portable folder has too many physical members".into())
            })?;
        self.bytes = self
            .bytes
            .checked_add(size)
            .filter(|n| *n <= MAX_PORTABLE_TRANSPORT_BYTES)
            .ok_or_else(|| {
                TransferError::Invalid(
                    "Portable folder exceeds its 1 GiB physical byte budget".into(),
                )
            })?;
        Ok(())
    }
}
#[derive(Default)]
struct Pages {
    pending: Vec<u8>,
    count: u64,
    total: u64,
    pages: Vec<Page>,
}
impl Pages {
    fn push(
        &mut self,
        value: &impl Serialize,
        kind: &str,
        output: &mut FolderOutput,
    ) -> Result<()> {
        let mut bytes = serde_json::to_vec(value)?;
        if bytes.len() > MAX_PORTABLE_RECORD_BYTES {
            return invalid("Portable folder control record exceeds 256 KiB");
        }
        bytes.push(b'\n');
        if !self.pending.is_empty()
            && (self.pending.len() + bytes.len() > MAX_PORTABLE_SHARD_BYTES
                || self.count == MAX_PORTABLE_SHARD_RECORDS)
        {
            self.flush(kind, output)?;
        }
        self.total = self
            .total
            .checked_add(1)
            .filter(|n| *n <= MAX_PORTABLE_RECORDS)
            .ok_or_else(|| {
                TransferError::Invalid("Portable folder control record limit exceeded".into())
            })?;
        self.pending.extend_from_slice(&bytes);
        self.count += 1;
        Ok(())
    }
    fn flush(&mut self, kind: &str, output: &mut FolderOutput) -> Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        if self.pages.len() >= MAX_PORTABLE_SHARDS {
            return invalid("Portable folder control page limit exceeded");
        }
        let path = format!(".writercraft/{kind}-{:06}.jsonl", self.pages.len() + 1);
        let member = output.bytes(&path, &self.pending)?;
        self.pages.push(Page {
            member,
            record_count: self.count,
        });
        self.pending.clear();
        self.count = 0;
        Ok(())
    }
}
struct FolderOutput {
    root: Dir,
    budget: Budget,
    paths: HashSet<String>,
    cancel: ProjectTransferCancellation,
}
impl FolderOutput {
    fn create(
        source_database: &Path,
        destination: &Path,
        cancel: &ProjectTransferCancellation,
    ) -> Result<Self> {
        check_cancel(cancel)?;
        if is_workspace_data_path(destination, source_database)? {
            return invalid(
                "Choose a new export folder away from workspace databases, journals and preferences",
            );
        }
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let name = destination
            .file_name()
            .ok_or_else(|| TransferError::Invalid("Choose an absent named export folder".into()))?;
        let parent = Dir::open_ambient_dir(parent, ambient_authority())?;
        parent.create_dir(name)?;
        if !parent.symlink_metadata(name)?.is_dir() {
            return invalid("New export directory was replaced before it could be opened");
        }
        let root = parent.open_dir(name)?;
        if root.entries()?.next().is_some() {
            return invalid("New export directory is no longer empty");
        }
        let mut marker =
            root.open_with(INCOMPLETE, OpenOptions::new().write(true).create_new(true))?;
        marker.write_all(
            b"WriterCraft export is incomplete. Do not restore it as a completed backup.\n",
        )?;
        marker.sync_all()?;
        Ok(Self {
            root,
            budget: Budget::default(),
            paths: HashSet::new(),
            cancel: cancel.clone(),
        })
    }
    fn write(
        &mut self,
        path: &str,
        producer: &mut dyn FnMut(&mut dyn Write) -> Result<()>,
    ) -> Result<Member> {
        check_cancel(&self.cancel)?;
        validate_path(path)?;
        if !self.paths.insert(path.into()) {
            return invalid("Portable output member path is duplicated");
        }
        if let Some(parent) = Path::new(path)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            self.root.create_dir_all(parent)?;
        }
        let file = self
            .root
            .open_with(path, OpenOptions::new().write(true).create_new(true))?;
        let mut writer = HashWriter {
            file,
            digest: Sha256::new(),
            size: 0,
            remaining: MAX_PORTABLE_TRANSPORT_BYTES - self.budget.bytes,
            cancel: self.cancel.clone(),
        };
        producer(&mut writer)?;
        writer.file.sync_all()?;
        check_cancel(&self.cancel)?;
        let member = Member {
            path: path.into(),
            sha256: format!("{:x}", writer.digest.finalize()),
            byte_size: writer.size,
        };
        self.budget.charge(member.byte_size)?;
        verify_member(&self.root, &member, &self.cancel)?;
        Ok(member)
    }
    fn bytes(&mut self, path: &str, bytes: &[u8]) -> Result<Member> {
        self.write(path, &mut |writer| {
            writer.write_all(bytes)?;
            Ok(())
        })
    }
}
struct HashWriter {
    file: File,
    digest: Sha256,
    size: u64,
    remaining: u64,
    cancel: ProjectTransferCancellation,
}
impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.cancel.is_cancelled() {
            return Err(std::io::Error::other("Portable folder export cancelled"));
        }
        if bytes.len() as u64 > self.remaining {
            return Err(std::io::Error::other(
                "Portable folder byte budget exhausted",
            ));
        }
        let written = self.file.write(bytes)?;
        self.digest.update(&bytes[..written]);
        self.size += written as u64;
        self.remaining -= written as u64;
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}
/// Export saved Markdown, JSON Canvas and original artifacts with the native format-2 companion.
///
/// The destination must be absent. Cancellation/failure leaves an owned incomplete folder and
/// never merges, overwrites or recursively deletes unrelated data. Use a dedicated worker.
pub fn export_canonical_project_folder(
    prepared: &PreparedPortableProject,
    source_database: &Path,
    destination: &Path,
    cancel: &ProjectTransferCancellation,
) -> Result<CanonicalFolderReceipt> {
    let plan = ReadingPlan::new(prepared, cancel)?;
    let mut output = FolderOutput::create(source_database, destination, cancel)?;
    let mut inventory = Pages::default();
    let mut objects = Pages::default();
    let mut conversions = Pages::default();
    let mut mapped: HashMap<String, String> = HashMap::new();
    for artifact in &plan.artifacts {
        let member = output.write(&artifact.path, &mut |writer| {
            prepared
                .copy_object(&artifact.object, writer, cancel)
                .map_err(core_error)
        })?;
        inventory.push(&member, "inventory", &mut output)?;
        mapped
            .entry(artifact.object.sha256().into())
            .or_insert_with(|| artifact.path.clone());
    }
    plan.visit_files(prepared, cancel, &mut |path, bytes, changes| {
        let member = output.bytes(path, bytes)?;
        inventory.push(&member, "inventory", &mut output)?;
        for change in changes {
            conversions.push(change, "conversions", &mut output)?;
        }
        Ok(())
    })?;
    for conversion in plan.conversions() {
        conversions.push(conversion, "conversions", &mut output)?;
    }
    let canonical = output.bytes(CANONICAL, prepared.root_bytes())?;
    inventory.push(&canonical, "inventory", &mut output)?;
    // The callback borrows the prepared read-only index; output state belongs to this worker only.
    let mut references = Vec::new();
    prepared
        .visit_objects(&mut |reference| {
            references.push(reference.clone());
            Ok(())
        })
        .map_err(core_error)?;
    for reference in references {
        check_cancel(cancel)?;
        let path = if let Some(path) = mapped.remove(reference.sha256()) {
            path
        } else {
            let path = format!(".writercraft/objects/{}", reference.sha256());
            let member = output.write(&path, &mut |writer| {
                prepared
                    .copy_object(&reference, writer, cancel)
                    .map_err(core_error)
            })?;
            inventory.push(&member, "inventory", &mut output)?;
            path
        };
        objects.push(
            &ObjectMember {
                sha256: reference.sha256().into(),
                byte_size: reference.byte_size(),
                path,
            },
            "objects",
            &mut output,
        )?;
    }
    if !mapped.is_empty() {
        return invalid("Reading artifact is outside the captured canonical object inventory");
    }
    inventory.flush("inventory", &mut output)?;
    objects.flush("objects", &mut output)?;
    conversions.flush("conversions", &mut output)?;
    let conversion_count = conversions.total;
    let completion = Completion {
        format: "writercraft-portable-export".into(),
        format_version: 2,
        state: "complete".into(),
        scope: plan.scope.clone(),
        canonical,
        inventory: inventory.pages,
        objects: objects.pages,
        conversions: conversions.pages,
        payload_count: inventory.total,
        object_count: objects.total,
    };
    let bytes = serde_json::to_vec_pretty(&completion)?;
    if bytes.len() > MAX_PORTABLE_ROOT_BYTES {
        return invalid("Portable folder completion root exceeds 1 MiB");
    }
    output.bytes(COMPLETION, &bytes)?;
    // Verify through the same untrusted-folder reader before removing the incomplete marker.
    let mut reader = CanonicalProjectFolder::from_dir(output.root.try_clone()?, true, cancel)?;
    if reader.completion_raw != bytes || reader.canonical_raw != prepared.root_bytes() {
        return invalid("Staged portable controls changed before publication");
    }
    reader.verify_all(true).map_err(TransferError::Io)?;
    check_cancel(cancel)?;
    output.root.remove_file(INCOMPLETE)?;
    Ok(CanonicalFolderReceipt {
        destination: destination.into(),
        scope: plan.scope,
        file_count: output.budget.members,
        byte_size: output.budget.bytes,
        conversion_count,
    })
}

/// A user-selected native project folder pinned to its directory capability.
///
/// No project/media path from the folder is opened with ambient filesystem authority. All control
/// pages and physical reading files must verify before the private semantic stream can activate.
pub struct CanonicalProjectFolder {
    root: Dir,
    completion_raw: Vec<u8>,
    canonical_raw: Vec<u8>,
    completion: Completion,
    members: HashMap<String, Member>,
    objects: HashMap<String, Member>,
    cancel: ProjectTransferCancellation,
}
impl CanonicalProjectFolder {
    /// Open the explicit user-selected directory. Inner symlinks/special files are refused.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_cancellation(path, &ProjectTransferCancellation::default())
    }
    /// Open and inspect bounded control metadata with cooperative cancellation.
    pub fn open_with_cancellation(
        path: impl AsRef<Path>,
        cancel: &ProjectTransferCancellation,
    ) -> Result<Self> {
        Self::from_dir(
            Dir::open_ambient_dir(path, ambient_authority())?,
            false,
            cancel,
        )
    }
    fn from_dir(
        root: Dir,
        allow_incomplete: bool,
        cancel: &ProjectTransferCancellation,
    ) -> Result<Self> {
        check_cancel(cancel)?;
        if !allow_incomplete && exists(&root, INCOMPLETE)? {
            return invalid(
                "Portable folder is incomplete; do not restore it as a completed backup",
            );
        }
        let completion_raw = read_small(&root, COMPLETION, MAX_PORTABLE_ROOT_BYTES, cancel)?;
        let completion: Completion = serde_json::from_slice(&completion_raw)?;
        if completion.format != "writercraft-portable-export"
            || completion.format_version != 2
            || completion.state != "complete"
            || !matches!(
                completion.scope.as_str(),
                "complete-project" | "selected-documents"
            )
            || completion.canonical.path != CANONICAL
        {
            return invalid("Unsupported portable folder completion contract");
        }
        if completion.payload_count > MAX_PORTABLE_MEMBERS
            || completion.object_count > MAX_PORTABLE_MEMBERS
            || completion.inventory.len() + completion.objects.len() + completion.conversions.len()
                > MAX_PORTABLE_SHARDS
        {
            return invalid("Portable folder declared control counts exceed their bounds");
        }
        let mut members = HashMap::new();
        let mut all_control = HashSet::from([COMPLETION.to_owned()]);
        let mut inventory_count = 0_u64;
        let mut declared_budget = Budget::default();
        for page in &completion.inventory {
            validate_control(page, "inventory", &mut all_control)?;
            declared_budget.charge(page.member.byte_size)?;
            let raw = read_page(&root, page, cancel)?;
            for line in lines(&raw, page.record_count)? {
                let member: Member = serde_json::from_slice(line)?;
                if inventory_count >= completion.payload_count {
                    return invalid("Portable inventory exceeds its declared bounded count");
                }
                validate_member(&member)?;
                declared_budget.charge(member.byte_size)?;
                if member.path == COMPLETION
                    || member.path == INCOMPLETE
                    || members.insert(member.path.clone(), member).is_some()
                {
                    return invalid("Portable folder inventory has a duplicate or reserved path");
                }
                inventory_count += 1;
            }
        }
        if inventory_count != completion.payload_count {
            return invalid("Portable folder inventory count differs from completion");
        }
        let canonical = members.get(CANONICAL).ok_or_else(|| {
            TransferError::Invalid("Canonical project is missing from folder inventory".into())
        })?;
        if canonical.sha256 != completion.canonical.sha256
            || canonical.byte_size != completion.canonical.byte_size
        {
            return invalid("Canonical folder root identity differs from inventory");
        }
        let canonical_raw = read_small(&root, CANONICAL, MAX_PORTABLE_ROOT_BYTES, cancel)?;
        verify_bytes(canonical, &canonical_raw)?;
        let header = PortableProjectHeader::parse(&canonical_raw).map_err(core_error)?;
        if header.scope().as_str() != completion.scope {
            return invalid("Reading and canonical scopes differ");
        }
        let mut objects = HashMap::new();
        let mut object_count = 0_u64;
        for page in &completion.objects {
            validate_control(page, "objects", &mut all_control)?;
            declared_budget.charge(page.member.byte_size)?;
            let raw = read_page(&root, page, cancel)?;
            for line in lines(&raw, page.record_count)? {
                if object_count >= completion.object_count {
                    return invalid("Portable object mapping exceeds its declared bounded count");
                }
                let object: ObjectMember = serde_json::from_slice(line)?;
                let identity = PortableObjectReference::new(&object.sha256, object.byte_size)
                    .map_err(core_error)?;
                let member = members.get(&object.path).ok_or_else(|| {
                    TransferError::Invalid(
                        "Canonical object names an unlisted folder member".into(),
                    )
                })?;
                if member.sha256 != identity.sha256()
                    || member.byte_size != identity.byte_size()
                    || objects
                        .insert(identity.sha256().into(), member.clone())
                        .is_some()
                {
                    return invalid(
                        "Canonical object mapping has conflicting or duplicate identity",
                    );
                }
                object_count += 1;
            }
        }
        if object_count != completion.object_count {
            return invalid("Portable folder object mapping count differs from completion");
        }
        let mut conversion_count = 0_u64;
        for page in &completion.conversions {
            validate_control(page, "conversions", &mut all_control)?;
            declared_budget.charge(page.member.byte_size)?;
            let raw = read_page(&root, page, cancel)?;
            for line in lines(&raw, page.record_count)? {
                let value: serde_json::Value = serde_json::from_slice(line)?;
                if !value["resourceId"].is_string() || !value["feature"].is_string() {
                    return invalid("Portable reading conversion is malformed");
                }
                conversion_count += 1;
                if conversion_count > MAX_PORTABLE_RECORDS {
                    return invalid("Portable reading conversion count exceeds its bound");
                }
            }
        }
        declared_budget.charge(completion_raw.len() as u64)?;
        if members.keys().any(|path| all_control.contains(path)) {
            return invalid("Portable payload inventory overlaps its control pages");
        }
        Ok(Self {
            root,
            completion_raw,
            canonical_raw,
            completion,
            members,
            objects,
            cancel: cancel.clone(),
        })
    }
    /// Captured native canonical descriptor; no whole-project Value or filesystem path authority.
    pub fn root_bytes(&self) -> &[u8] {
        &self.canonical_raw
    }
    /// Verify all folder bytes and freeze exact canonical objects in a private core index.
    pub fn preflight(
        &mut self,
        cancel: &ProjectTransferCancellation,
    ) -> Result<VerifiedPortableProjectStream> {
        self.cancel = cancel.clone();
        let root = self.canonical_raw.clone();
        VerifiedPortableProjectStream::preflight(&root, self, cancel).map_err(core_error)
    }
    /// Restore an independent project only after complete transport and semantic validation.
    pub fn restore(
        &mut self,
        worker: &mut ProjectImportWorker,
        cancel: &ProjectTransferCancellation,
    ) -> Result<ImportedProject> {
        let verified = self.preflight(cancel)?;
        worker.restore_stream(&verified, cancel).map_err(core_error)
    }
    fn verify_all(&mut self, allow_incomplete: bool) -> std::io::Result<()> {
        self.verify_all_inner(allow_incomplete)
            .map_err(std::io::Error::other)
    }
    fn verify_all_inner(&mut self, allow_incomplete: bool) -> Result<()> {
        check_cancel(&self.cancel)?;
        if !allow_incomplete && exists(&self.root, INCOMPLETE)? {
            return invalid("Portable folder is incomplete");
        }
        if read_small(
            &self.root,
            COMPLETION,
            MAX_PORTABLE_ROOT_BYTES,
            &self.cancel,
        )? != self.completion_raw
        {
            return invalid("Portable folder completion changed after opening");
        }
        if read_small(&self.root, CANONICAL, MAX_PORTABLE_ROOT_BYTES, &self.cancel)?
            != self.canonical_raw
        {
            return invalid("Portable canonical root changed after opening");
        }
        let mut expected = self.members.keys().cloned().collect::<HashSet<_>>();
        expected.insert(COMPLETION.into());
        for page in self
            .completion
            .inventory
            .iter()
            .chain(&self.completion.objects)
            .chain(&self.completion.conversions)
        {
            verify_member(&self.root, &page.member, &self.cancel)?;
            expected.insert(page.member.path.clone());
        }
        for member in self.members.values() {
            verify_member(&self.root, member, &self.cancel)?;
        }
        if allow_incomplete {
            expected.insert(INCOMPLETE.into());
        }
        let mut budget = Budget::default();
        let mut found = HashSet::new();
        scan(&self.root, "", 0, &mut budget, &mut found, &self.cancel)?;
        if found != expected {
            return invalid(
                "Portable folder has missing or unexpected files; reading edits are not an unchanged canonical backup",
            );
        }
        Ok(())
    }
}
impl PortableObjectSource for CanonicalProjectFolder {
    fn verify_transport(&mut self) -> std::io::Result<()> {
        self.verify_all(false)
    }
    fn visit_objects(
        &mut self,
        visitor: &mut dyn FnMut(PortableObjectReference) -> studio_core::Result<()>,
    ) -> studio_core::Result<()> {
        for (digest, member) in &self.objects {
            visitor(PortableObjectReference::new(digest, member.byte_size)?)?;
        }
        Ok(())
    }
    fn open<'a>(
        &'a mut self,
        object: &PortableObjectReference,
    ) -> std::io::Result<Box<dyn Read + 'a>> {
        let member = self
            .objects
            .get(object.sha256())
            .filter(|member| member.byte_size == object.byte_size())
            .ok_or_else(|| {
                std::io::Error::other("Object is outside this canonical folder capability")
            })?;
        regular(&self.root, &member.path).map_err(std::io::Error::other)?;
        Ok(Box::new(self.root.open(&member.path)?))
    }
}
fn validate_control(page: &Page, kind: &str, seen: &mut HashSet<String>) -> Result<()> {
    validate_member(&page.member)?;
    let prefix = format!(".writercraft/{kind}-");
    let suffix = page
        .member
        .path
        .strip_prefix(&prefix)
        .and_then(|name| name.strip_suffix(".jsonl"))
        .filter(|n| n.len() == 6 && n.bytes().all(|b| b.is_ascii_digit()));
    if suffix.is_none()
        || page.record_count == 0
        || page.record_count > MAX_PORTABLE_SHARD_RECORDS
        || page.member.byte_size > MAX_PORTABLE_SHARD_BYTES as u64
        || !seen.insert(page.member.path.clone())
    {
        return invalid("Portable folder control page identity is invalid or duplicated");
    }
    Ok(())
}
fn read_page(root: &Dir, page: &Page, cancel: &ProjectTransferCancellation) -> Result<Vec<u8>> {
    let bytes = read_small(root, &page.member.path, MAX_PORTABLE_SHARD_BYTES, cancel)?;
    verify_bytes(&page.member, &bytes)?;
    Ok(bytes)
}
fn lines(bytes: &[u8], count: u64) -> Result<Vec<&[u8]>> {
    if !bytes.ends_with(b"\n") {
        return invalid("Portable folder control page is not terminated JSON Lines");
    }
    let lines = bytes[..bytes.len() - 1]
        .split(|b| *b == b'\n')
        .collect::<Vec<_>>();
    if lines.len() as u64 != count
        || lines
            .iter()
            .any(|line| line.is_empty() || line.len() > MAX_PORTABLE_RECORD_BYTES)
    {
        return invalid("Portable folder control page count or record size differs");
    }
    Ok(lines)
}
fn validate_member(member: &Member) -> Result<()> {
    validate_path(&member.path)?;
    PortableObjectReference::new(&member.sha256, member.byte_size).map_err(core_error)?;
    Ok(())
}
fn validate_path(path: &str) -> Result<()> {
    if !safe_relative(path) || path.len() > 4096 || path.split('/').any(|part| part.len() > 240) {
        return invalid("Portable folder member path is unsafe or unrepresentable");
    }
    Ok(())
}
fn verify_bytes(member: &Member, bytes: &[u8]) -> Result<()> {
    if bytes.len() as u64 != member.byte_size
        || format!("{:x}", Sha256::digest(bytes)) != member.sha256
    {
        return invalid("Portable folder control size or checksum differs");
    }
    Ok(())
}
fn exists(root: &Dir, path: &str) -> Result<bool> {
    match root.symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
fn regular(root: &Dir, path: &str) -> Result<()> {
    validate_path(path)?;
    if !root.symlink_metadata(path)?.is_file() {
        return invalid("Portable folder member is not a regular file");
    }
    Ok(())
}
fn read_small(
    root: &Dir,
    path: &str,
    limit: usize,
    cancel: &ProjectTransferCancellation,
) -> Result<Vec<u8>> {
    regular(root, path)?;
    check_cancel(cancel)?;
    let file = root.open(path)?;
    if file.metadata()?.len() > limit as u64 {
        return invalid("Portable folder control exceeds its byte limit");
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    check_cancel(cancel)?;
    if bytes.len() > limit {
        return invalid("Portable folder control grew beyond its limit");
    }
    Ok(bytes)
}
fn verify_member(root: &Dir, member: &Member, cancel: &ProjectTransferCancellation) -> Result<()> {
    validate_member(member)?;
    regular(root, &member.path)?;
    let mut file = root.open(&member.path)?;
    if file.metadata()?.len() != member.byte_size {
        return invalid("Portable reading or canonical file size changed; restore cancelled");
    }
    let mut remaining = member.byte_size;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        check_cancel(cancel)?;
        let capacity = remaining.min(buffer.len() as u64) as usize;
        let count = file.read(&mut buffer[..capacity])?;
        if count == 0 {
            return invalid("Portable folder member ended early");
        }
        digest.update(&buffer[..count]);
        remaining -= count as u64;
    }
    if file.read(&mut [0_u8; 1])? != 0 || format!("{:x}", digest.finalize()) != member.sha256 {
        return invalid("Portable reading or canonical checksum changed; restore cancelled");
    }
    Ok(())
}
fn scan(
    root: &Dir,
    prefix: &str,
    depth: usize,
    budget: &mut Budget,
    files: &mut HashSet<String>,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    if depth > 512 {
        return invalid("Portable folder nesting exceeds its bound");
    }
    for entry in root.entries()? {
        check_cancel(cancel)?;
        let entry = entry?;
        let name = entry.file_name().into_string().map_err(|_| {
            TransferError::Invalid("Portable folder has a non-UTF-8 filename".into())
        })?;
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        validate_path(&path)?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            budget.charge(0)?;
            scan(
                &root.open_dir(&name)?,
                &path,
                depth + 1,
                budget,
                files,
                cancel,
            )?;
        } else if file_type.is_file() {
            budget.charge(entry.metadata()?.len())?;
            files.insert(path);
        } else {
            return invalid("Portable folder contains an inner symlink or special file");
        }
    }
    Ok(())
}
fn check_cancel(cancel: &ProjectTransferCancellation) -> Result<()> {
    if cancel.is_cancelled() {
        return invalid("Portable folder operation cancelled; saved project data is unchanged");
    }
    Ok(())
}
fn core_error(error: studio_core::StudioError) -> TransferError {
    TransferError::Invalid(error.to_string())
}
fn invalid<T>(message: &str) -> Result<T> {
    Err(TransferError::Invalid(message.into()))
}
