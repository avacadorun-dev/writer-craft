//! Bounded pre-release companion compatibility, with semantic projection and quarantined authority.
//!
//! This adapter is intentionally separate from format-2 framing. It accepts the small historical
//! companion only; large projects require the paged format-2 transport rather than a higher limit.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Cursor;

use craft_editor_model::Document as RichTree;
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::{Value, value::RawValue};
use sha2::{Digest, Sha256};

use super::json::invalid;
use super::{
    EvidenceReferenceInput, ImportAssetSource, ImportedProject, ProjectImportWorker,
    ProjectTransferCancellation, Result, StudioError, ValidatedEvidenceObject,
    ValidatedEvidenceReference, ValidatedProjectImport, evidence, import_in_transaction,
    insert_alias, read_unsigned, validation,
};
use crate::{AnchorStatus, PassageAnnotation};

/// Compatibility-only maximum for the historical monolithic format-1 companion.
/// New output must use format 2; increasing this allocation guard is not a large-project solution.
pub const MAX_PORTABLE_COMPANION_V1_BYTES: usize = 64 * 1024 * 1024;
const MAX_NATIVE_COMPONENT_BYTES: usize = 32 * 1024 * 1024;
const MAX_RECOVERY_COMPONENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_NATIVE_RECORDS: usize = 100_000;

struct NativeNote {
    original_id: String,
    logical_id: Option<String>,
    stable_native_id: String,
    logical_document: String,
    state: PassageAnnotation,
    history: Vec<NativeEvent>,
}
struct NativeEvent {
    revision: i64,
    document_revision: i64,
    source: String,
    before: Option<PassageAnnotation>,
    after: PassageAnnotation,
}
struct OpaqueDocument {
    logical_document: String,
    original: String,
}
struct Quarantine {
    domain: String,
    logical_id: String,
    raw: String,
}

/// Checked pre-release companion. Neither deserialization nor external checkpoints create authority.
///
/// Use `ProjectImportWorker::restore_portable` on a dedicated worker connection. Inputs never
/// identify live global native rows; every old handle resolves through this companion's source.
pub struct ValidatedPortableProjectImport {
    source: ValidatedProjectImport,
    notes: Vec<NativeNote>,
    opaque: Vec<OpaqueDocument>,
    quarantine: Vec<Quarantine>,
}
impl ValidatedPortableProjectImport {
    /// Validate a bounded historical format-1 companion before any workspace writes.
    ///
    /// Exact source framing/hash, selected scope, aliases, note base revisions and known history
    /// are checked independently. Unknown and recovery fields remain data-only evidence.
    pub fn parse_v1(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > MAX_PORTABLE_COMPANION_V1_BYTES {
            return invalid(
                "Old canonical companion exceeds 64 MiB; re-export using format 2 or use a whole-workspace backup",
            );
        }
        super::json::reject_duplicate_keys(bytes)?;
        let mut root: BTreeMap<String, Box<RawValue>> = serde_json::from_slice(bytes)?;
        if take::<String>(&mut root, "format")? != "writercraft-portable-project"
            || take::<u64>(&mut root, "formatVersion")? != 1
        {
            return invalid("This compatibility adapter accepts native canonical format 1 only");
        }
        let scope: String = take(&mut root, "scope")?;
        if !matches!(scope.as_str(), "complete-project" | "selected-documents") {
            return invalid("Unsupported canonical project scope");
        }
        let declared = root.remove("sourceManifest").ok_or_else(|| {
            StudioError::Validation("Canonical companion is missing sourceManifest".into())
        })?;
        let mut source_raw = match root.remove("sourceManifestRaw") {
            Some(raw) => {
                let raw: String = serde_json::from_str(raw.get())?;
                let hash: String = take(&mut root, "sourceManifestSha256")?;
                if hash != format!("{:x}", Sha256::digest(raw.as_bytes())) {
                    return invalid(
                        "Canonical source manifest checksum differs from its exact bytes",
                    );
                }
                super::json::reject_duplicate_keys(raw.as_bytes())?;
                if serde_json::from_str::<Value>(&raw)?
                    != serde_json::from_str::<Value>(declared.get())?
                {
                    return invalid(
                        "Canonical source manifest value disagrees with its exact bytes",
                    );
                }
                raw
            }
            None => {
                if root.contains_key("sourceManifestSha256") {
                    return invalid("Canonical source hash has no exact source bytes");
                }
                declared.get().to_owned()
            }
        };
        if source_raw.len() > super::MAX_PROJECT_MANIFEST_BYTES {
            return invalid("Canonical source exceeds the unchanged 64 MiB source-manifest bound");
        }
        let mut manifest: Value = serde_json::from_str(&source_raw)?;
        let mut quarantine = Vec::new();
        if scope == "selected-documents" {
            let ids: Vec<String> = take(&mut root, "selectedNodeIds")?;
            let selected: HashSet<&str> = ids.iter().map(String::as_str).collect();
            let nodes = validation::array(&manifest, "nodes")?;
            if ids.len() != selected.len()
                || ids.len() != nodes.len()
                || nodes.iter().any(|node| {
                    !node["id"].as_str().is_some_and(|id| selected.contains(id))
                        || node["currentRevision"].as_u64().unwrap_or(0) == 0
                })
            {
                return invalid("Selected canonical scope must exactly identify supplied writing");
            }
            for domain in [
                "projectMemory",
                "projectMemoryRevisions",
                "canvases",
                "canvasRevisions",
            ] {
                if !validation::array(&manifest, domain)?.is_empty() {
                    return invalid(
                        "Selected writing cannot include project-wide memory or Canvas bodies",
                    );
                }
            }
            if manifest
                .get("assistantGuide")
                .is_some_and(|value| !value.is_null())
            {
                return invalid("Selected writing cannot include a project-wide assistant guide");
            }
            quarantine.push(Quarantine {
                domain: "portable.selection".into(),
                logical_id: "original-source".into(),
                raw: source_raw,
            });
            for node in manifest["nodes"].as_array_mut().into_iter().flatten() {
                if node["parentId"]
                    .as_str()
                    .is_some_and(|id| !selected.contains(id))
                {
                    node.as_object_mut()
                        .ok_or_else(|| {
                            StudioError::Validation("Selected node must be an object".into())
                        })?
                        .remove("parentId");
                }
            }
            source_raw = serde_json::to_string(&manifest)?;
        }
        let attachments = validation::validate_native(&manifest)?;
        let source = ValidatedProjectImport {
            format_version: validation::number(&manifest, "formatVersion")?,
            logical_project_id: validation::string(&manifest["project"], "id")?.to_owned(),
            title: validation::string(&manifest["project"], "title")?.to_owned(),
            raw_manifest: source_raw,
            manifest,
            attachments,
        };
        let native_raw = root.remove("nativeState");
        let native: Value = parse_component(
            native_raw.as_deref(),
            MAX_NATIVE_COMPONENT_BYTES,
            "native state",
        )?;
        let recovery_raw = root.remove("recoveryEvidence");
        parse_component(
            recovery_raw.as_deref(),
            MAX_RECOVERY_COMPONENT_BYTES,
            "recovery evidence",
        )?;
        let aliases = validate_aliases(&native, &source.manifest)?;
        let notes = validate_notes(&native, &source.manifest, &aliases)?;
        let opaque = validate_opaque(&native, &aliases)?;
        if let Some(raw) = native_raw {
            split_quarantine("portable.native", raw.get(), &mut quarantine)?;
        }
        if let Some(raw) = recovery_raw {
            split_quarantine("portable.recovery", raw.get(), &mut quarantine)?;
        }
        for (key, raw) in root {
            quarantine.push(Quarantine {
                domain: "portable.extension".into(),
                logical_id: key,
                raw: raw.get().to_owned(),
            });
        }
        for record in &quarantine {
            let object = ValidatedEvidenceObject::new(
                &format!("{:x}", Sha256::digest(record.raw.as_bytes())),
                u64::try_from(record.raw.len()).map_err(std::io::Error::other)?,
                "application/json",
            )?;
            ValidatedEvidenceReference::new(
                &object,
                EvidenceReferenceInput {
                    domain: record.domain.clone(),
                    logical_id: record.logical_id.clone(),
                    logical_document_id: None,
                    logical_canvas_id: None,
                    metadata_json: "{}".into(),
                },
            )?;
        }
        Ok(Self {
            source,
            notes,
            opaque,
            quarantine,
        })
    }
    /// The validated saved source representation, with aliases confined to this source namespace.
    pub fn source(&self) -> &ValidatedProjectImport {
        &self.source
    }
    /// Count of current author-note states, including detached and deleted notes.
    pub fn note_count(&self) -> usize {
        self.notes.len()
    }
    /// Count of data-only retained domains; none grant draft, replay or Apply authority.
    pub fn evidence_count(&self) -> usize {
        self.quarantine.len()
    }
}
fn take<T: serde::de::DeserializeOwned>(
    root: &mut BTreeMap<String, Box<RawValue>>,
    key: &str,
) -> Result<T> {
    let raw = root
        .remove(key)
        .ok_or_else(|| StudioError::Validation(format!("Canonical companion is missing {key}")))?;
    Ok(serde_json::from_str(raw.get())?)
}
fn parse_component(raw: Option<&RawValue>, limit: usize, name: &str) -> Result<Value> {
    let Some(raw) = raw else {
        return Ok(serde_json::json!({}));
    };
    if raw.get().len() > limit || !raw.get().starts_with('{') {
        return invalid(format!("Canonical {name} must be a bounded JSON object"));
    }
    Ok(serde_json::from_str(raw.get())?)
}
fn bounded_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}
fn validate_aliases(native: &Value, manifest: &Value) -> Result<HashMap<String, String>> {
    let Some(aliases) = native.get("documentAliases") else {
        if validation::array(native, "annotations")?.is_empty()
            && validation::array(native, "opaqueDocuments")?.is_empty()
        {
            return Ok(HashMap::new());
        }
        return invalid("Native state has no scoped document aliases");
    };
    let aliases = aliases
        .as_object()
        .ok_or_else(|| StudioError::Validation("Native aliases must be an object".into()))?;
    let nodes: HashSet<&str> = validation::array(manifest, "nodes")?
        .iter()
        .filter(|node| node["currentRevision"].as_u64().unwrap_or(0) > 0)
        .filter_map(|node| node["id"].as_str())
        .collect();
    if aliases.len() > nodes.len() {
        return invalid("Native aliases exceed supplied writing");
    }
    let mut mapped = HashSet::new();
    let mut result = HashMap::new();
    for (old, logical) in aliases {
        let logical = logical
            .as_str()
            .filter(|id| nodes.contains(*id))
            .ok_or_else(|| {
                StudioError::Validation("Native alias refers outside supplied writing".into())
            })?;
        if !bounded_id(old) || !mapped.insert(logical) {
            return invalid("Native aliases repeat or exceed identity bounds");
        }
        result.insert(old.clone(), logical.to_owned());
    }
    Ok(result)
}
fn validate_state(
    value: &Value,
    old_id: &str,
    old_document: &str,
    logical: &str,
    manifest: &Value,
    current_state: bool,
) -> Result<PassageAnnotation> {
    let mut state: PassageAnnotation = serde_json::from_value(value.clone())?;
    if state.id != old_id
        || state.document_id != old_document
        || state.revision <= 0
        || state.base_revision <= 0
        || !bounded_id(&state.anchor.block_id)
        || state.anchor.start >= state.anchor.end
        || state.anchor.exact_text.is_empty()
        || state.anchor.exact_text.len() > 16 * 1024 * 1024
        || state.anchor.prefix.chars().count() > 32
        || state.anchor.suffix.chars().count() > 32
        || state.note.chars().count() > 10_000
    {
        return invalid("Native note has invalid identity, revisions, text or anchor bounds");
    }
    let base = u64::try_from(state.base_revision).map_err(std::io::Error::other)?;
    let revision = validation::array(manifest, "documentRevisions")?
        .iter()
        .find(|revision| {
            revision["nodeId"].as_str() == Some(logical)
                && revision["revision"].as_u64() == Some(base)
        })
        .ok_or_else(|| {
            StudioError::Validation(
                "Native note base revision is missing from supplied writing".into(),
            )
        })?;
    let current = validation::array(manifest, "nodes")?
        .iter()
        .find(|node| node["id"].as_str() == Some(logical))
        .and_then(|node| node["currentRevision"].as_u64());
    let exact = RichTree::from_json(revision["content"].clone())
        .ok()
        .is_some_and(|tree| state.anchor.selection(&tree).is_ok());
    if !exact || (current_state && current != Some(base)) {
        state.anchor_status = AnchorStatus::Detached;
    }
    Ok(state)
}
fn validate_notes(
    native: &Value,
    manifest: &Value,
    aliases: &HashMap<String, String>,
) -> Result<Vec<NativeNote>> {
    let records = validation::array(native, "annotations")?;
    if records.len() > MAX_NATIVE_RECORDS {
        return invalid("Native note count exceeds its explicit bound");
    }
    let mut ids = HashSet::new();
    let mut logical_ids = HashSet::new();
    let mut result = Vec::new();
    for record in records {
        let old = validation::string(record, "nativeId")?;
        let old_document = validation::string(record, "nativeDocumentId")?;
        if !bounded_id(old) || !ids.insert(old.to_owned()) {
            return invalid("Native note identity is empty, oversized or repeated");
        }
        let logical_document = aliases.get(old_document).ok_or_else(|| {
            StudioError::Validation("Native note refers outside its captured aliases".into())
        })?;
        let logical_id = match record.get("logicalId") {
            None | Some(Value::Null) => None,
            Some(Value::String(id)) if bounded_id(id) => Some(id.clone()),
            _ => return invalid("Invalid source annotation identity"),
        };
        let stable_native_id = record["logicalNativeId"].as_str().unwrap_or(old).to_owned();
        if !bounded_id(&stable_native_id)
            || !logical_ids.insert((
                logical_id.is_some(),
                logical_id.clone().unwrap_or(stable_native_id.clone()),
            ))
        {
            return invalid("Native notes repeat a stable scoped identity");
        }
        if let Some(id) = &logical_id
            && !validation::array(manifest, "documentAnnotations")?
                .iter()
                .any(|note| {
                    note["id"].as_str() == Some(id)
                        && note["nodeId"].as_str() == Some(logical_document)
                        && note["kind"].as_str() == Some("note")
                })
        {
            return invalid("Native note overlay does not match a source note in its document");
        }
        let state = validate_state(
            &record["state"],
            old,
            old_document,
            logical_document,
            manifest,
            true,
        )?;
        result.push(NativeNote {
            original_id: old.into(),
            logical_id,
            stable_native_id,
            logical_document: logical_document.clone(),
            state,
            history: vec![],
        });
    }
    let indices: HashMap<String, usize> = result
        .iter()
        .enumerate()
        .map(|(index, note)| (note.original_id.clone(), index))
        .collect();
    let mut event_ids = HashSet::new();
    let history = validation::array(native, "annotationHistory")?;
    if history.len() > MAX_NATIVE_RECORDS {
        return invalid("Native annotation history exceeds its explicit bound");
    }
    for event in history {
        let old = validation::string(event, "nativeAnnotationId")?;
        let index = *indices.get(old).ok_or_else(|| {
            StudioError::Validation("Annotation history has no current scoped note".into())
        })?;
        let note = &mut result[index];
        let revision =
            i64::try_from(validation::number(event, "revision")?).map_err(std::io::Error::other)?;
        let document_revision = i64::try_from(validation::number(event, "documentRevision")?)
            .map_err(std::io::Error::other)?;
        let source = validation::string(event, "source")?;
        if revision <= 0
            || revision > note.state.revision
            || document_revision <= 0
            || !event_ids.insert((old, revision))
            || !matches!(
                source,
                "create" | "update" | "reattach" | "delete" | "mapping" | "restore"
            )
        {
            return invalid("Native note event has invalid, duplicated or future revision/source");
        }
        let after = validate_state(
            &event["after"],
            old,
            &note.state.document_id,
            &note.logical_document,
            manifest,
            false,
        )?;
        let before = match event.get("before") {
            None | Some(Value::Null) => None,
            Some(value) => Some(validate_state(
                value,
                old,
                &note.state.document_id,
                &note.logical_document,
                manifest,
                false,
            )?),
        };
        if after.revision != revision
            || after.base_revision != document_revision
            || before
                .as_ref()
                .is_some_and(|state| state.revision >= revision)
        {
            return invalid("Native note event states disagree with their declared revisions");
        }
        note.history.push(NativeEvent {
            revision,
            document_revision,
            source: source.into(),
            before,
            after,
        });
    }
    Ok(result)
}
fn validate_opaque(
    native: &Value,
    aliases: &HashMap<String, String>,
) -> Result<Vec<OpaqueDocument>> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for record in validation::array(native, "opaqueDocuments")? {
        let old = validation::string(record, "nativeDocumentId")?;
        let logical = aliases.get(old).ok_or_else(|| {
            StudioError::Validation("Opaque original refers outside supplied writing".into())
        })?;
        if !seen.insert(logical) || record["readOnly"].as_bool() != Some(true) {
            return invalid("Opaque original identity or read-only state is invalid");
        }
        let raw = validation::string(record, "sourceJsonRaw")?;
        if raw.len() > super::MAX_PROJECT_MANIFEST_BYTES {
            return invalid("Opaque original exceeds 64 MiB");
        }
        super::json::reject_duplicate_keys(raw.as_bytes())?;
        if serde_json::from_str::<Value>(raw)? != record["sourceJson"] {
            return invalid("Opaque original bytes disagree with their declared JSON");
        }
        result.push(OpaqueDocument {
            logical_document: logical.clone(),
            original: raw.to_owned(),
        });
    }
    Ok(result)
}
fn split_quarantine(prefix: &str, raw: &str, result: &mut Vec<Quarantine>) -> Result<()> {
    let fields: BTreeMap<String, Box<RawValue>> = serde_json::from_str(raw)?;
    for (key, raw) in fields {
        result.push(Quarantine {
            domain: prefix.into(),
            logical_id: key,
            raw: raw.get().to_owned(),
        });
    }
    Ok(())
}

impl ProjectImportWorker {
    /// Restore one checked historical companion atomically into an independent namespace.
    /// Saved notes are validated ordinary author data. Drafts, attachment checkpoints and proposal
    /// objects remain quarantine and cannot create native replay/Apply authority.
    pub fn restore_portable(
        &mut self,
        project: &ValidatedPortableProjectImport,
        source: &mut dyn ImportAssetSource,
        cancel: &ProjectTransferCancellation,
    ) -> Result<ImportedProject> {
        cancel.check()?;
        let mut connection = self.workspace.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let result = import_in_transaction(&transaction, &project.source, source, cancel)?;
        let mut node_ids = HashMap::new();
        for node in validation::array(&project.source.manifest, "nodes")? {
            let logical = validation::string(node, "id")?;
            let native: Option<String> = transaction.query_row("SELECT native_document_id FROM project_nodes WHERE project_id=?1 AND logical_id=?2", params![result.project_id, logical], |row| row.get(0))?;
            if let Some(native) = native {
                node_ids.insert(logical.to_owned(), native);
            }
        }
        for original in &project.opaque {
            cancel.check()?;
            let document = node_ids.get(&original.logical_document).ok_or_else(|| {
                StudioError::CorruptData("Validated opaque document is missing".into())
            })?;
            transaction.execute(
                "UPDATE documents SET read_only=1,source_json=?1 WHERE id=?2 AND project_id=?3",
                params![original.original, document, result.project_id],
            )?;
        }
        for note in &project.notes {
            restore_note(&transaction, &result.project_id, note, &node_ids, cancel)?;
        }
        for record in &project.quarantine {
            cancel.check()?;
            let object = ValidatedEvidenceObject::new(
                &format!("{:x}", Sha256::digest(record.raw.as_bytes())),
                u64::try_from(record.raw.len()).map_err(std::io::Error::other)?,
                "application/json",
            )?;
            let reference = ValidatedEvidenceReference::new(
                &object,
                EvidenceReferenceInput {
                    domain: record.domain.clone(),
                    logical_id: record.logical_id.clone(),
                    logical_document_id: None,
                    logical_canvas_id: None,
                    metadata_json: "{}".into(),
                },
            )?;
            evidence::store_evidence_in_transaction(
                &transaction,
                &result.project_id,
                &object,
                &reference,
                &mut Cursor::new(record.raw.as_bytes()),
                cancel,
            )?;
        }
        cancel.check()?;
        transaction.commit()?;
        Ok(result)
    }
}
fn restore_note(
    transaction: &Transaction<'_>,
    project: &str,
    note: &NativeNote,
    node_ids: &HashMap<String, String>,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    cancel.check()?;
    let document = node_ids.get(&note.logical_document).ok_or_else(|| {
        StudioError::CorruptData("Validated annotation document is missing".into())
    })?;
    let existing = match &note.logical_id {
        Some(logical) => transaction.query_row("SELECT native_annotation_id FROM project_annotation_projections WHERE project_id=?1 AND logical_id=?2 AND document_id=?3", params![project, logical, document], |row| row.get::<_, Option<String>>(0)).optional()?.flatten(),
        None => None,
    };
    let native = existing
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut state = note.state.clone();
    state.id = native.clone();
    state.document_id = document.clone();
    if existing.is_some() {
        transaction.execute(
            "DELETE FROM document_annotation_history WHERE annotation_id=?1",
            [&native],
        )?;
        transaction.execute(
            "DELETE FROM document_annotation_snapshots WHERE annotation_id=?1",
            [&native],
        )?;
        transaction.execute("UPDATE document_annotations SET revision=?1,deleted=?2,annotation_json=?3 WHERE id=?4 AND document_id=?5", params![state.revision, state.deleted, serde_json::to_string(&state)?, native, document])?;
    } else {
        transaction.execute("INSERT INTO document_annotations(id,document_id,revision,deleted,annotation_json) VALUES(?1,?2,?3,?4,?5)",params![native,document,state.revision,state.deleted,serde_json::to_string(&state)?])?;
    }
    if let Some(logical) = &note.logical_id {
        transaction.execute("UPDATE project_annotation_projections SET native_annotation_id=?1,native_anchor_json=?2,anchor_status=?3 WHERE project_id=?4 AND logical_id=?5 AND document_id=?6", params![native,serde_json::to_string(&state.anchor)?,if state.anchor_status==AnchorStatus::Attached {"attached"} else {"detached"},project,logical,document])?;
        transaction.execute("UPDATE project_resource_aliases SET native_id=?1 WHERE project_id=?2 AND resource_kind='annotation' AND logical_id=?3",params![native,project,logical])?;
    } else {
        insert_alias(
            transaction,
            project,
            "native-annotation",
            &note.stable_native_id,
            &native,
        )?;
    }
    for event in &note.history {
        cancel.check()?;
        let remap = |original: &PassageAnnotation| {
            let mut state = original.clone();
            state.id = native.clone();
            state.document_id = document.clone();
            state
        };
        let after = remap(&event.after);
        let before = event.before.as_ref().map(remap);
        transaction.execute("INSERT INTO document_annotation_history(annotation_id,annotation_revision,document_revision,source,before_json,after_json) VALUES(?1,?2,?3,?4,?5,?6)",params![native,event.revision,event.document_revision,event.source,before.map(|state|serde_json::to_string(&state)).transpose()?,serde_json::to_string(&after)?])?;
    }
    // Only current, exactly validated saved attachment data enters the normal snapshot table.
    // External historical checkpoints remain evidence and never become replay receipts.
    if state.anchor_status == AnchorStatus::Attached {
        let revision: u64 = transaction.query_row(
            "SELECT revision FROM documents WHERE id=?1 AND project_id=?2",
            params![document, project],
            |row| read_unsigned(row, 0),
        )?;
        transaction.execute("INSERT INTO document_annotation_snapshots(document_id,document_revision,annotation_id,attachment_json) VALUES(?1,?2,?3,?4)",params![document,super::sqlite_integer(revision)?,native,serde_json::to_string(&serde_json::json!({"anchor":state.anchor,"status":state.anchor_status}))?])?;
    }
    Ok(())
}
