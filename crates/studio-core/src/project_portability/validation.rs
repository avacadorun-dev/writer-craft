//! Source-backed project envelope and cross-domain validation.
use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::json::invalid;
use super::{MAX_PROJECT_TRANSFER_BYTES, ProjectAttachment, Result, StudioError};

pub(super) fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| StudioError::Validation(format!("Project field {key} must be a string")))
}
pub(super) fn number(value: &Value, key: &str) -> Result<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .filter(|n| *n <= i64::MAX as u64)
        .ok_or_else(|| {
            StudioError::Validation(format!(
                "Project field {key} must be a bounded nonnegative integer"
            ))
        })
}
pub(super) fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value]> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        _ => invalid(format!("{key} must be an array")),
    }
}
fn name(value: &Value, key: &str, max: usize) -> Result<()> {
    let value = string(value, key)?;
    if value.trim().is_empty() || value.chars().count() > max {
        return invalid(format!("{key} is empty or too long"));
    }
    Ok(())
}
fn one_of(value: &Value, key: &str, allowed: &[&str]) -> Result<()> {
    if !allowed.contains(&string(value, key)?) {
        return invalid(format!("unsupported {key}"));
    }
    Ok(())
}
fn positive(value: &Value, key: &str) -> Result<u64> {
    let n = number(value, key)?;
    if n == 0 {
        return invalid(format!("{key} must be positive"));
    }
    Ok(n)
}
fn optional_reference(value: &Value, key: &str, targets: &HashMap<&str, &Value>) -> Result<()> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(()),
        Some(Value::String(id)) if targets.contains_key(id.as_str()) => Ok(()),
        _ => invalid(format!("{key} references a missing project resource")),
    }
}
fn ids<'a>(value: &'a Value, domain: &str, max: usize) -> Result<HashMap<&'a str, &'a Value>> {
    let items = array(value, domain)?;
    if items.len() > max {
        return invalid(format!("{domain} exceeds its item bound"));
    }
    let mut result = HashMap::with_capacity(items.len());
    for item in items {
        let id = string(item, "id")?;
        if id.trim().is_empty() || id.len() > 512 || result.insert(id, item).is_some() {
            return invalid(format!(
                "{domain} has an empty, oversized or duplicate identity"
            ));
        }
    }
    Ok(result)
}
pub(super) fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && !path.starts_with('/')
        && !path.contains(['\\', ':', '*', '?', '"', '<', '>', '|'])
        && !path.chars().any(char::is_control)
        && path.split('/').all(|part| {
            !part.is_empty() && part != "." && part != ".." && !part.ends_with(['.', ' '])
        })
}

pub(super) fn validate(manifest: &Value) -> Result<Vec<ProjectAttachment>> {
    validate_inner(manifest, false)
}

// Canonical native projects may legitimately be empty or Canvas-only. The original source
// archive parser retains its nonempty-node guarantee through validate above.
pub(super) fn validate_native(manifest: &Value) -> Result<Vec<ProjectAttachment>> {
    validate_inner(manifest, true)
}

fn validate_inner(manifest: &Value, allow_empty: bool) -> Result<Vec<ProjectAttachment>> {
    if string(manifest, "format")? != "writers-studio-project" {
        return invalid("unsupported project envelope");
    }
    let version = number(manifest, "formatVersion")?;
    if !(1..=8).contains(&version) {
        return invalid("unsupported project envelope version; supported versions are 1–8");
    }
    let project = &manifest["project"];
    name(project, "id", 512)?;
    name(project, "title", 120)?;
    one_of(
        project,
        "kind",
        &[
            "novel",
            "story",
            "screenplay",
            "nonfiction",
            "game_design",
            "free",
            "custom",
        ],
    )?;
    one_of(project, "language", &["ru", "en"])?;
    positive(project, "structureRevision")?;
    for (key, max) in [("wordGoal", 10_000_000), ("assistantStepLimit", 1_000_000)] {
        if project.get(key).is_some_and(|v| !v.is_null()) {
            let n = positive(project, key)?;
            if n > max {
                return invalid(format!("{key} exceeds its source bound"));
            }
        }
    }
    let structure = &project["structure"];
    positive(structure, "schemaVersion")?;
    name(structure, "template", 512)?;
    name(structure, "rootLabel", 512)?;
    let mut kinds = HashSet::new();
    let mut has_root = false;
    let mut has_content = false;
    let types = array(structure, "nodeTypes")?;
    if types.is_empty() {
        return invalid("invalid project node type count");
    }
    for kind in types {
        name(kind, "key", 512)?;
        name(kind, "label", 512)?;
        if !kinds.insert(string(kind, "key")?) {
            return invalid("duplicate project node kind");
        }
        has_root |= kind["canBeRoot"].as_bool().unwrap_or(false);
        has_content |= kind["contentBearing"].as_bool().unwrap_or(false);
    }
    if !has_root || !has_content {
        return invalid("project structure needs root and content-bearing kinds");
    }
    for kind in types {
        for child in array(kind, "allowedChildren")? {
            if !child.as_str().is_some_and(|s| kinds.contains(s)) {
                return invalid("project structure references an unknown child kind");
            }
        }
    }
    let nodes = ids(manifest, "nodes", 20_000)?;
    if nodes.is_empty() && !allow_empty {
        return invalid("project must contain at least one node");
    }
    for node in nodes.values() {
        name(node, "title", 200)?;
        if !kinds.contains(string(node, "kind")?) {
            return invalid("node kind is missing from project structure");
        }
        one_of(node, "status", &["idea", "draft", "editing", "ready"])?;
        number(node, "currentRevision")?;
        if !node["position"]
            .as_f64()
            .is_some_and(|p| p.is_finite() && p.abs() <= 1e12)
        {
            return invalid("invalid node position");
        }
        optional_reference(node, "parentId", &nodes)?;
        if let Some(frontmatter) = node.get("sourceFrontmatter")
            && ((version < 5 && frontmatter.as_str() != Some(""))
                || !frontmatter
                    .as_str()
                    .is_some_and(|s| s.len() <= 10 * 1024 * 1024))
        {
            return invalid("invalid source frontmatter");
        }
        let mut seen = HashSet::new();
        let mut current = Some(*node);
        while let Some(item) = current {
            let id = string(item, "id")?;
            if !seen.insert(id) || seen.len() > 512 {
                return invalid("project tree has a cycle or exceeds 512 levels");
            }
            current = item["parentId"]
                .as_str()
                .and_then(|id| nodes.get(id).copied());
        }
    }
    ids(manifest, "documentRevisions", 100_000)?;
    let mut revisions = HashMap::new();
    let mut latest: HashMap<&str, u64> = HashMap::new();
    for revision in array(manifest, "documentRevisions")? {
        let node = string(revision, "nodeId")?;
        let ordinal = positive(revision, "revision")?;
        if !nodes.contains_key(node)
            || number(revision, "baseRevision")? >= ordinal
            || revisions.insert((node, ordinal), revision).is_some()
        {
            return invalid("invalid document revision identity/base");
        }
        let schema = positive(revision, "schemaVersion")?;
        let source_schema = revision["content"]["attrs"]["schemaVersion"]
            .as_u64()
            .or_else(|| {
                revision["content"]["attrs"]["schemaVersion"]
                    .as_f64()
                    .filter(|n| n.is_finite() && n.fract() == 0.0 && (1.0..=4.0).contains(n))
                    .map(|n| n as u64)
            })
            .unwrap_or(1);
        if schema != source_schema || !(1..=4).contains(&schema) {
            return invalid("document schema does not match its content");
        }
        craft_editor_model::Document::from_json(revision["content"].clone())
            .map_err(|e| StudioError::Validation(format!("Invalid imported rich document: {e}")))?;
        if string(revision, "plainText")?.len() > 16 * 1024 * 1024 {
            return invalid("document text exceeds source limits");
        }
        string(revision, "source")?;
        latest
            .entry(node)
            .and_modify(|n| *n = (*n).max(ordinal))
            .or_insert(ordinal);
    }
    for (id, node) in &nodes {
        if number(node, "currentRevision")? != latest.get(id).copied().unwrap_or(0) {
            return invalid("node head does not match revision history");
        }
    }
    for revision in revisions.values() {
        let base = number(revision, "baseRevision")?;
        if base > 0 && !revisions.contains_key(&(string(revision, "nodeId")?, base)) {
            return invalid("document base revision is missing");
        }
    }
    for named in array(manifest, "namedVersions")? {
        name(named, "id", 512)?;
        name(named, "name", 160)?;
        if !revisions.contains_key(&(string(named, "nodeId")?, positive(named, "revision")?)) {
            return invalid("named version refers to missing revision");
        }
    }
    ids(manifest, "namedVersions", 100_000)?;
    let terms = ids(manifest, "dictionaryTerms", 100_000)?;
    for term in terms.values() {
        name(term, "canonical", 12_000)?;
        positive(term, "revision")?;
        one_of(term, "kind", &["character", "place", "term", "other"])?;
        one_of(term, "language", &["ru", "en"])?;
        for variant in array(term, "variants")? {
            if !variant.is_string() {
                return invalid("dictionary variants must be strings");
            }
        }
    }
    let sessions = ids(manifest, "dictationSessions", 20_000)?;
    for session in sessions.values() {
        optional_reference(session, "destinationNodeId", &nodes)?;
        one_of(session, "language", &["ru", "en"])?;
        number(session, "durationMs")?;
        number(session, "recordedDurationMs")?;
        if session.get("manifest").is_none() {
            return invalid("dictation source manifest is missing");
        }
    }
    let artifacts = ids(manifest, "transcriptArtifacts", 100_000)?;
    for artifact in artifacts.values() {
        if !sessions.contains_key(string(artifact, "sessionId")?) {
            return invalid("transcript session is missing");
        }
        one_of(
            artifact,
            "kind",
            &["live_final", "batch_source", "cleaned_suggestion"],
        )?;
        string(artifact, "text")?;
        if artifact.get("segments").is_none() {
            return invalid("transcript segments are missing");
        }
    }
    let mut draft_sessions = HashSet::new();
    for draft in array(manifest, "transcriptDrafts")? {
        let session = string(draft, "sessionId")?;
        if !sessions.contains_key(session) || !draft_sessions.insert(session) {
            return invalid("invalid transcript draft session");
        }
        positive(draft, "revision")?;
        optional_reference(draft, "sourceArtifactId", &artifacts)?;
        if !draft["content"].is_object() {
            return invalid("transcript draft content must be an object");
        }
    }
    let mut draft_revisions = HashSet::new();
    for revision in array(manifest, "transcriptDraftRevisions")? {
        if !sessions.contains_key(string(revision, "sessionId")?)
            || !draft_revisions.insert((
                string(revision, "sessionId")?,
                positive(revision, "revision")?,
            ))
            || !revision["content"].is_object()
        {
            return invalid("invalid transcript draft history");
        }
    }
    let media = ids(manifest, "media", 20_000)?;
    let mut attachments = Vec::new();
    let mut paths = HashSet::new();
    let mut total = 0_u64;
    let mut audio_sessions = HashSet::new();
    for domain in ["audio", "media"] {
        for item in array(manifest, domain)? {
            let identity = string(item, if domain == "audio" { "sessionId" } else { "id" })?;
            if domain == "audio"
                && (!sessions.contains_key(identity) || !audio_sessions.insert(identity))
            {
                return invalid("invalid original audio session");
            }
            let path = string(item, "path")?;
            if !safe_path(path)
                || !path.starts_with(&format!("{domain}/"))
                || !paths.insert(path.to_lowercase())
            {
                return invalid("unsafe or duplicate attachment path");
            }
            let sha = string(item, "sha256")?;
            if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
                return invalid("invalid attachment SHA-256");
            }
            let size = number(item, "byteSize")?;
            total = total
                .checked_add(size)
                .filter(|n| *n <= MAX_PROJECT_TRANSFER_BYTES)
                .ok_or_else(|| {
                    StudioError::Validation("Project attachments exceed 512 MiB".into())
                })?;
            let mime = string(item, "mediaType")?;
            if mime.is_empty() || mime.len() > 255 {
                return invalid("invalid attachment media type");
            }
            let width = dimension(item, "width")?;
            let height = dimension(item, "height")?;
            if domain == "media" {
                if version < 3 || size == 0 {
                    return invalid("media requires format 3 and nonempty bytes");
                }
                match mime {
                    "image/png" | "image/jpeg" | "image/gif" | "image/webp"
                        if size <= 12 * 1024 * 1024 && width.is_some() && height.is_some() => {}
                    "application/pdf"
                        if version >= 7
                            && size <= 25 * 1024 * 1024
                            && width.is_none()
                            && height.is_none() => {}
                    "audio/mpeg" | "audio/mp4" | "audio/ogg" | "audio/wav"
                        if version >= 8
                            && size <= 50 * 1024 * 1024
                            && width.is_none()
                            && height.is_none() => {}
                    "video/mp4" | "video/webm"
                        if version >= 8
                            && size <= 100 * 1024 * 1024
                            && width.is_none()
                            && height.is_none() => {}
                    "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "application/pdf"
                    | "audio/mpeg" | "audio/mp4" | "audio/ogg" | "audio/wav" | "video/mp4"
                    | "video/webm" => return invalid("invalid supported-media bounds or metadata"),
                    _ => {}
                }
                if let Some(source) = item.get("sourcePath").filter(|v| !v.is_null())
                    && (version < 5 || !source.as_str().is_some_and(safe_path))
                {
                    return invalid("unsafe original media source path");
                }
                if item.get("durationMs").is_some_and(|v| !v.is_null()) {
                    number(item, "durationMs")?;
                }
                if item.get("pageCount").is_some_and(|v| !v.is_null()) {
                    positive(item, "pageCount")?;
                }
            } else {
                number(item, "durationMs")?;
            }
            let original_name = if domain == "media" {
                string(item, "originalName")?.to_owned()
            } else {
                path.rsplit('/').next().unwrap_or("audio").to_owned()
            };
            if original_name.chars().count() > 255 {
                return invalid("attachment name exceeds source bound");
            }
            attachments.push(ProjectAttachment {
                resource_kind: domain.to_owned(),
                logical_id: identity.to_owned(),
                path: path.to_owned(),
                sha256: sha.to_ascii_lowercase(),
                byte_size: size,
                media_type: mime.to_owned(),
                original_name,
                width,
                height,
            });
        }
    }
    for revision in revisions.values() {
        validate_document_media(&revision["content"], &media)?;
    }
    if let Some(cover) = project["exportProfile"]["metadata"]["coverAssetId"].as_str()
        && !media.contains_key(cover)
    {
        return invalid("export cover attachment is missing");
    }
    let canvases = ids(manifest, "canvases", 2_000)?;
    if version < 6 && (!canvases.is_empty() || !array(manifest, "canvasRevisions")?.is_empty()) {
        return invalid("Canvas requires format version 6");
    }
    let schema = manifest.get("canvasSchema").filter(|v| !v.is_null());
    if schema.is_some() && version < 8 {
        return invalid("Canvas type schema requires format version 8");
    }
    super::canvas_validation::validate_project(manifest, &nodes, &media, &canvases)?;
    ids(manifest, "documentAnnotations", 100_000)?;
    for annotation in array(manifest, "documentAnnotations")? {
        if version < 2 {
            return invalid("annotations require format version 2");
        }
        one_of(annotation, "kind", &["note", "dictionary_link"])?;
        one_of(
            annotation,
            "status",
            &["open", "resolved", "needs_attention"],
        )?;
        let node = string(annotation, "nodeId")?;
        let base = positive(annotation, "baseRevision")?;
        let revision = revisions.get(&(node, base)).ok_or_else(|| {
            StudioError::Validation("Annotation base document/revision is missing".into())
        })?;
        if annotation
            .get("anchorStatus")
            .is_some_and(|v| !v.is_null() && v.as_str() != Some(""))
        {
            one_of(annotation, "anchorStatus", &["attached", "detached"])?;
        }
        optional_reference(annotation, "dictionaryTermId", &terms)?;
        let anchor: &Value = &annotation["anchor"];
        name(anchor, "blockId", 512)?;
        name(anchor, "exactText", 16 * 1024 * 1024)?;
        if number(anchor, "start")? >= number(anchor, "end")? {
            return invalid("invalid annotation anchor bounds");
        }
        for key in ["prefix", "suffix"] {
            if string(anchor, key)?.chars().count() > 32 {
                return invalid("annotation context exceeds 32 scalars");
            }
        }
        let tree = craft_editor_model::Document::from_json(revision["content"].clone())
            .map_err(|e| StudioError::Validation(format!("Invalid annotation source tree: {e}")))?;
        super::super::AnnotationAnchor::adapt_legacy(&tree, anchor, None)?;
        if annotation["kind"].as_str() == Some("note") {
            name(
                annotation,
                "note",
                super::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS,
            )?;
        }
        if annotation["kind"].as_str() == Some("dictionary_link")
            && annotation["dictionaryTermId"].is_null()
            && annotation["targetSnapshot"]
                .as_object()
                .is_none_or(|m| m.is_empty())
        {
            return invalid("dictionary link has no retained target");
        }
    }
    validate_context(manifest, &nodes)?;
    Ok(attachments)
}
fn dimension(item: &Value, key: &str) -> Result<Option<u32>> {
    match item.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => {
            let value = positive(item, key)?;
            if value > 20_000 {
                return invalid("image axis exceeds 20000");
            }
            Ok(Some(value as u32))
        }
    }
}
fn validate_document_media(content: &Value, media: &HashMap<&str, &Value>) -> Result<()> {
    if content["type"].as_str() == Some("imageBlock")
        && let Some(id) = content["attrs"]["assetId"]
            .as_str()
            .filter(|s| !s.is_empty())
        && !media.contains_key(id)
    {
        return invalid("document image attachment is missing");
    }
    for child in array(content, "content")? {
        validate_document_media(child, media)?;
    }
    Ok(())
}
fn validate_context(manifest: &Value, nodes: &HashMap<&str, &Value>) -> Result<()> {
    if let Some(guide) = manifest.get("assistantGuide").filter(|v| !v.is_null()) {
        let mut ordinals = HashSet::new();
        for revision in array(guide, "revisions")? {
            if !ordinals.insert(positive(revision, "revision")?)
                || string(revision, "content")?.chars().count() > 24_000
            {
                return invalid("invalid assistant guide history");
            }
            one_of(revision, "source", &["manual", "assistant", "restore"])?;
        }
        let current = number(guide, "currentRevision")?;
        if current > 0 && !ordinals.contains(&current) {
            return invalid("assistant guide head is missing");
        }
    }
    let memories = ids(manifest, "projectMemory", 100_000)?;
    for memory in memories.values() {
        name(memory, "statement", 12_000)?;
        positive(memory, "revision")?;
        one_of(
            memory,
            "kind",
            &[
                "fact",
                "character",
                "world_rule",
                "plot",
                "style",
                "constraint",
                "decision",
                "other",
            ],
        )?;
        one_of(memory, "source", &["manual", "assistant"])?;
        match string(memory, "scope")? {
            "project" if memory.get("scopeNodeId").is_none_or(Value::is_null) => {}
            "node" => {
                if !nodes.contains_key(string(memory, "scopeNodeId")?) {
                    return invalid("memory scope node is missing");
                }
            }
            _ => return invalid("invalid memory scope"),
        }
    }
    let mut revisions = HashSet::new();
    for revision in array(manifest, "projectMemoryRevisions")? {
        let id = string(revision, "memoryId")?;
        if !memories.contains_key(id) || !revisions.insert((id, positive(revision, "revision")?)) {
            return invalid("invalid memory history");
        }
        one_of(revision, "source", &["manual", "assistant"])?;
        if !revision["snapshot"].is_object() {
            return invalid("memory historical snapshot is missing");
        }
        optional_reference(&revision["snapshot"], "scopeNodeId", nodes)?;
    }
    Ok(())
}
