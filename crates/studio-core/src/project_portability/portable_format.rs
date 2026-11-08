//! Small canonical roots and verified bounded shards do not grant semantic replay/Apply authority.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;

use serde::Serialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

use super::{ProjectTransferCancellation, Result, StudioError, json::invalid};

/// Maximum native root descriptor, separate from source format 1–8 limits.
pub const MAX_PORTABLE_ROOT_BYTES: usize = 1024 * 1024;
/// Maximum verified metadata shard.
pub const MAX_PORTABLE_SHARD_BYTES: usize = 1024 * 1024;
/// Maximum one JSON Lines metadata record.
pub const MAX_PORTABLE_RECORD_BYTES: usize = 256 * 1024;
/// Maximum records in one shard; pages remain bounded even for very small records.
pub const MAX_PORTABLE_SHARD_RECORDS: u64 = 4096;
/// Finite native compressed/expanded transport guard; this is not a universal fit guarantee.
pub const MAX_PORTABLE_TRANSPORT_BYTES: u64 = 1024 * 1024 * 1024;
/// Finite materialized logical-byte guard, counted separately from shared transport objects.
pub const MAX_PORTABLE_MATERIALIZED_BYTES: u64 = 1024 * 1024 * 1024;
/// Maximum native transport members; original source archive limits remain unchanged.
pub const MAX_PORTABLE_MEMBERS: u64 = 65_536;
/// Maximum root component descriptors.
pub const MAX_PORTABLE_SHARDS: usize = 4096;
/// Maximum semantic records across components.
pub const MAX_PORTABLE_RECORDS: u64 = 1_000_000;

/// Scope of the captured project. Selected scope cannot embed a complete original source manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PortableProjectScope {
    /// All captured project data.
    #[serde(rename = "complete-project")]
    CompleteProject,
    /// Explicit selected documents and their authorized related records.
    #[serde(rename = "selected-documents")]
    SelectedDocuments,
}
impl PortableProjectScope {
    /// Stable portable scope name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CompleteProject => "complete-project",
            Self::SelectedDocuments => "selected-documents",
        }
    }
}
/// Byte identity only. It authorizes no filesystem path, project lookup, decoder or semantic write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableObjectReference {
    sha256: String,
    byte_size: u64,
}
impl PortableObjectReference {
    /// Validate canonical lowercase SHA-256 and an individually bounded payload length.
    /// Domain validators enforce the narrower rich/Canvas/opaque/media limits.
    pub fn new(sha256: &str, byte_size: u64) -> Result<Self> {
        if sha256.len() != 64
            || !sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || byte_size > super::MAX_PROJECT_TRANSFER_BYTES
        {
            return invalid("Portable object has an invalid canonical digest or exceeds 512 MiB");
        }
        Ok(Self {
            sha256: sha256.to_owned(),
            byte_size,
        })
    }
    /// Exact encoded-byte fingerprint.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    /// Exact encoded length; actual readers must verify it again.
    pub fn byte_size(&self) -> u64 {
        self.byte_size
    }
}
/// One hash-verified metadata component page; its records still require domain validation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableShardDescriptor {
    domain: String,
    #[serde(flatten)]
    object: PortableObjectReference,
    record_count: u64,
    #[serde(flatten)]
    extensions: BTreeMap<String, Box<RawValue>>,
}
impl PortableShardDescriptor {
    /// Validate page framing and counts without granting record interpretation authority.
    pub fn new(domain: &str, object: PortableObjectReference, record_count: u64) -> Result<Self> {
        if domain.is_empty()
            || domain.len() > 128
            || domain.chars().any(char::is_control)
            || object.byte_size
                > u64::try_from(MAX_PORTABLE_SHARD_BYTES).map_err(std::io::Error::other)?
            || record_count > MAX_PORTABLE_SHARD_RECORDS
        {
            return invalid("Portable shard exceeds domain, byte or record bounds");
        }
        Ok(Self {
            domain: domain.to_owned(),
            object,
            record_count,
            extensions: BTreeMap::new(),
        })
    }
    /// Domain registry key; an unknown domain remains opaque data.
    pub fn domain(&self) -> &str {
        &self.domain
    }
    /// The exact bounded shard byte identity.
    pub fn object(&self) -> &PortableObjectReference {
        &self.object
    }
    /// Declared count, verified before any record visitor is called.
    pub fn record_count(&self) -> u64 {
        self.record_count
    }
}
/// A structurally validated small root, not authority to activate an imported project.
pub struct PortableProjectHeader {
    scope: PortableProjectScope,
    source_original: Option<PortableObjectReference>,
    components: Vec<PortableShardDescriptor>,
    source_extensions: BTreeMap<String, Box<RawValue>>,
    extensions: BTreeMap<String, Box<RawValue>>,
}
impl PortableProjectHeader {
    /// Construct an output root with checked aggregate declared shard budgets.
    pub fn new(
        scope: PortableProjectScope,
        source_original: Option<PortableObjectReference>,
        components: Vec<PortableShardDescriptor>,
    ) -> Result<Self> {
        validate_header(scope, source_original.as_ref(), &components, 0)?;
        let result = Self {
            scope,
            source_original,
            components,
            source_extensions: BTreeMap::new(),
            extensions: BTreeMap::new(),
        };
        result.to_bytes()?;
        Ok(result)
    }
    /// Parse at most 1 MiB with duplicate-key checks. Unknown root/shard fields remain raw.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > MAX_PORTABLE_ROOT_BYTES {
            return invalid("Portable format-2 root must contain at most 1 MiB");
        }
        super::json::reject_duplicate_keys(bytes)?;
        let mut fields: BTreeMap<String, Box<RawValue>> = serde_json::from_slice(bytes)?;
        if take::<String>(&mut fields, "format")? != "writercraft-portable-project"
            || take::<u64>(&mut fields, "formatVersion")? != 2
        {
            return invalid("Unsupported native canonical format/version");
        }
        let scope = match take::<String>(&mut fields, "scope")?.as_str() {
            "complete-project" => PortableProjectScope::CompleteProject,
            "selected-documents" => PortableProjectScope::SelectedDocuments,
            _ => return invalid("Unsupported native canonical scope"),
        };
        let (source_original, source_extensions) = match fields.remove("sourceOriginal") {
            Some(raw) if raw.get() != "null" => {
                let (reference, extra) = parse_object(raw.get())?;
                (Some(reference), extra)
            }
            _ => (None, BTreeMap::new()),
        };
        let raw_components: Vec<Box<RawValue>> = take(&mut fields, "components")?;
        if raw_components.len() > MAX_PORTABLE_SHARDS {
            return invalid("Portable root exceeds 4096 shard descriptors");
        }
        let mut components = Vec::with_capacity(raw_components.len());
        for raw in raw_components {
            let mut fields: BTreeMap<String, Box<RawValue>> = serde_json::from_str(raw.get())?;
            let domain: String = take(&mut fields, "domain")?;
            let sha256: String = take(&mut fields, "sha256")?;
            let byte_size: u64 = take(&mut fields, "byteSize")?;
            let record_count: u64 = take(&mut fields, "recordCount")?;
            let mut descriptor = PortableShardDescriptor::new(
                &domain,
                PortableObjectReference::new(&sha256, byte_size)?,
                record_count,
            )?;
            descriptor.extensions = fields;
            components.push(descriptor);
        }
        validate_header(
            scope,
            source_original.as_ref(),
            &components,
            u64::try_from(bytes.len()).map_err(std::io::Error::other)?,
        )?;
        Ok(Self {
            scope,
            source_original,
            components,
            source_extensions,
            extensions: fields,
        })
    }
    /// Encode the small root while retaining unknown raw fields at their original key paths.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        #[derive(Serialize)]
        struct Original<'a> {
            #[serde(flatten)]
            reference: &'a PortableObjectReference,
            #[serde(flatten)]
            extensions: &'a BTreeMap<String, Box<RawValue>>,
        }
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Root<'a> {
            format: &'static str,
            format_version: u32,
            scope: PortableProjectScope,
            #[serde(skip_serializing_if = "Option::is_none")]
            source_original: Option<Original<'a>>,
            components: &'a [PortableShardDescriptor],
            #[serde(flatten)]
            extensions: &'a BTreeMap<String, Box<RawValue>>,
        }
        let bytes = serde_json::to_vec(&Root {
            format: "writercraft-portable-project",
            format_version: 2,
            scope: self.scope,
            source_original: self.source_original.as_ref().map(|reference| Original {
                reference,
                extensions: &self.source_extensions,
            }),
            components: &self.components,
            extensions: &self.extensions,
        })?;
        if bytes.len() > MAX_PORTABLE_ROOT_BYTES {
            return invalid("Encoded portable root exceeds 1 MiB");
        }
        validate_header(
            self.scope,
            self.source_original.as_ref(),
            &self.components,
            u64::try_from(bytes.len()).map_err(std::io::Error::other)?,
        )?;
        Ok(bytes)
    }
    /// Explicit complete/selected scope.
    pub fn scope(&self) -> PortableProjectScope {
        self.scope
    }
    /// Immutable exact original, available only for complete-project scope.
    pub fn source_original(&self) -> Option<&PortableObjectReference> {
        self.source_original.as_ref()
    }
    /// Bounded component descriptors; payloads are read separately.
    pub fn components(&self) -> &[PortableShardDescriptor] {
        &self.components
    }
}
fn take<T: serde::de::DeserializeOwned>(
    fields: &mut BTreeMap<String, Box<RawValue>>,
    name: &str,
) -> Result<T> {
    let raw = fields
        .remove(name)
        .ok_or_else(|| StudioError::Validation(format!("Portable root is missing {name}")))?;
    Ok(serde_json::from_str(raw.get())?)
}
fn parse_object(raw: &str) -> Result<(PortableObjectReference, BTreeMap<String, Box<RawValue>>)> {
    let mut fields: BTreeMap<String, Box<RawValue>> = serde_json::from_str(raw)?;
    let sha256: String = take(&mut fields, "sha256")?;
    let byte_size: u64 = take(&mut fields, "byteSize")?;
    Ok((PortableObjectReference::new(&sha256, byte_size)?, fields))
}
fn validate_header(
    scope: PortableProjectScope,
    source: Option<&PortableObjectReference>,
    components: &[PortableShardDescriptor],
    root_bytes: u64,
) -> Result<()> {
    if components.len() > MAX_PORTABLE_SHARDS {
        return invalid("Portable root exceeds 4096 shard descriptors");
    }
    if let Some(source) = source {
        if scope == PortableProjectScope::SelectedDocuments {
            return invalid("Selected scope cannot embed a complete original source manifest");
        }
        if source.byte_size
            > u64::try_from(super::MAX_PROJECT_MANIFEST_BYTES).map_err(std::io::Error::other)?
        {
            return invalid("Original source manifest exceeds its unchanged 64 MiB limit");
        }
    }
    let mut domains = HashSet::new();
    let mut sizes = HashMap::new();
    let mut budget = PortableResourceBudget::default();
    if root_bytes > 0 {
        budget.account_unique_transport(root_bytes)?;
    }
    if let Some(source) = source {
        sizes.insert(source.sha256.as_str(), source.byte_size);
        budget.account_unique_transport(source.byte_size)?;
    }
    for component in components {
        if !domains.insert((component.domain.as_str(), component.object.sha256.as_str())) {
            return invalid("Portable root repeats the same shard in one domain");
        }
        budget.account_records(component.record_count)?;
        match sizes.insert(component.object.sha256.as_str(), component.object.byte_size) {
            Some(size) if size != component.object.byte_size => {
                return invalid("Portable digest has conflicting declared lengths");
            }
            None => budget.account_unique_transport(component.object.byte_size)?,
            _ => {}
        }
    }
    Ok(())
}
/// Independent counters: deduplication of transport bytes never exempts materialized logical writes.
#[derive(Debug, Clone, Default)]
pub struct PortableResourceBudget {
    transport_bytes: u64,
    materialized_bytes: u64,
    record_count: u64,
    member_count: u64,
    expanded_member_bytes: u64,
    encoded_container_bytes: Option<u64>,
}
impl PortableResourceBudget {
    /// Account one unique byte object after a private index has established digest uniqueness.
    pub fn account_unique_transport(&mut self, bytes: u64) -> Result<()> {
        self.transport_bytes=self.transport_bytes.checked_add(bytes).filter(|total|*total<=MAX_PORTABLE_TRANSPORT_BYTES).ok_or_else(||StudioError::Validation("Native transport exceeds the explicit 1 GiB budget; use whole-workspace backup".into()))?;
        self.transport_bytes = transport_bytes;
        Ok(())
    }
    /// Count every actual authorized transport path, even when its content hash is shared.
    /// Transfer must include reading views, aliases and control files instead of deduplicating paths.
    pub fn account_physical_member(&mut self, expanded_bytes: u64) -> Result<()> {
        let bytes = self
            .expanded_member_bytes
            .checked_add(expanded_bytes)
            .filter(|total| *total <= MAX_PORTABLE_TRANSPORT_BYTES)
            .ok_or_else(|| {
                StudioError::Validation("Actual expanded transport exceeds 1 GiB".into())
            })?;
        let members = self
            .member_count
            .checked_add(1)
            .filter(|count| *count <= MAX_PORTABLE_MEMBERS)
            .ok_or_else(|| {
                StudioError::Validation("Actual transport exceeds 65536 members".into())
            })?;
        self.expanded_member_bytes = bytes;
        self.member_count = members;
        Ok(())
    }
    /// Record actual pinned/staged container length, including encoded headers and directory bytes.
    /// Calling again detects a changed length; JSON declarations cannot replace this filesystem check.
    pub fn record_encoded_container_size(&mut self, bytes: u64) -> Result<()> {
        if bytes > MAX_PORTABLE_TRANSPORT_BYTES {
            return invalid("Actual encoded native container exceeds 1 GiB");
        }
        if self
            .encoded_container_bytes
            .is_some_and(|previous| previous != bytes)
        {
            return invalid("Encoded native container length changed after capture");
        }
        self.encoded_container_bytes = Some(bytes);
        Ok(())
    }
    /// Account actual projected destination bytes, counting repeated materialization separately.
    pub fn account_materialized(&mut self, bytes: u64) -> Result<()> {
        self.materialized_bytes=self.materialized_bytes.checked_add(bytes).filter(|total|*total<=MAX_PORTABLE_MATERIALIZED_BYTES).ok_or_else(||StudioError::Validation("Restored logical data exceeds the explicit 1 GiB materialization budget; use whole-workspace backup".into()))?;
        Ok(())
    }
    /// Account semantic records even when the containing bytes share one transport digest.
    pub fn account_records(&mut self, count: u64) -> Result<()> {
        self.record_count = self
            .record_count
            .checked_add(count)
            .filter(|count| *count <= MAX_PORTABLE_RECORDS)
            .ok_or_else(|| {
                StudioError::Validation("Native project exceeds 1000000 records".into())
            })?;
        Ok(())
    }
    /// Current independently counted transport bytes.
    pub fn transport_bytes(&self) -> u64 {
        self.transport_bytes
    }
    /// Actual authorized member bytes, including duplicate-content paths.
    pub fn expanded_member_bytes(&self) -> u64 {
        self.expanded_member_bytes
    }
    /// Actual authorized member count, independent of content hashes.
    pub fn member_count(&self) -> u64 {
        self.member_count
    }
    /// Actual encoded length if a transfer adapter has supplied its pinned/staged file check.
    pub fn encoded_container_bytes(&self) -> Option<u64> {
        self.encoded_container_bytes
    }
    /// Current projected destination bytes.
    pub fn materialized_bytes(&self) -> u64 {
        self.materialized_bytes
    }
    /// Current semantic record count.
    pub fn record_count(&self) -> u64 {
        self.record_count
    }
}
/// Verified shard framing only; a domain-specific private constructor still validates semantics.
pub struct VerifiedPortableRecord<'a> {
    raw_json: &'a str,
}
impl VerifiedPortableRecord<'_> {
    /// Exact record bytes excluding its JSON Lines newline delimiter.
    pub fn raw_json(&self) -> &str {
        self.raw_json
    }
}
/// Verify bounded bytes, digest, UTF-8, duplicate keys, object framing and count before any callback.
/// This function never constructs a whole native-state Value or grants replay/Apply authority.
pub fn visit_portable_shard(
    descriptor: &PortableShardDescriptor,
    reader: &mut dyn Read,
    cancel: &ProjectTransferCancellation,
    visitor: &mut dyn for<'a> FnMut(VerifiedPortableRecord<'a>) -> Result<()>,
) -> Result<()> {
    cancel.check()?;
    let size = usize::try_from(descriptor.object.byte_size).map_err(std::io::Error::other)?;
    if size > MAX_PORTABLE_SHARD_BYTES {
        return invalid("Portable shard exceeds 1 MiB");
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(|_| {
        StudioError::Validation("Not enough memory for a bounded metadata shard".into())
    })?;
    bytes.resize(size, 0);
    let mut offset = 0;
    while offset < size {
        cancel.check()?;
        let end = offset.saturating_add(64 * 1024).min(size);
        let count = reader.read(&mut bytes[offset..end])?;
        if count == 0 || count > end - offset {
            return invalid("Portable shard ended early or violated its bounded reader");
        }
        offset += count;
    }
    let mut extra = [0_u8; 1];
    if reader.read(&mut extra)? != 0 {
        return invalid("Portable shard exceeds its declared length");
    }
    if format!("{:x}", Sha256::digest(&bytes)) != descriptor.object.sha256 {
        return invalid("Portable shard checksum does not match");
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| StudioError::Validation("Portable metadata shard must be UTF-8".into()))?;
    let mut ranges = Vec::new();
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        cancel.check()?;
        let line = line.strip_suffix('\n').unwrap_or(line);
        if line.is_empty() || line.len() > MAX_PORTABLE_RECORD_BYTES {
            return invalid("Portable record is empty or exceeds 256 KiB");
        }
        super::json::reject_duplicate_keys(line.as_bytes())?;
        let raw: &RawValue = serde_json::from_str(line)?;
        if !raw.get().starts_with('{') {
            return invalid("Portable metadata records must be JSON objects");
        }
        ranges.push((start, start + line.len()));
        if u64::try_from(ranges.len()).map_err(std::io::Error::other)? > MAX_PORTABLE_SHARD_RECORDS
        {
            return invalid("Portable shard exceeds 4096 records");
        }
        start += line.len() + usize::from(text.as_bytes().get(start + line.len()) == Some(&b'\n'));
    }
    if u64::try_from(ranges.len()).map_err(std::io::Error::other)? != descriptor.record_count {
        return invalid("Portable shard record count disagrees with its descriptor");
    }
    for (start, end) in ranges {
        cancel.check()?;
        visitor(VerifiedPortableRecord {
            raw_json: &text[start..end],
        })?;
    }
    Ok(())
}
