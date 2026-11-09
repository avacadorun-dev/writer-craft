//! A selected-document companion never carries unrelated project writing or native evidence.
use std::collections::HashSet;

use serde_json::{Value, json};
use studio_core::ProjectExportSnapshot;

use super::{Result, TransferError};

pub(super) struct SelectedCompanion {
    pub(super) manifest: Value,
    pub(super) bytes: Vec<u8>,
    pub(super) asset_ids: HashSet<String>,
    pub(super) audio_ids: HashSet<String>,
    pub(super) outside_source_references: bool,
}

pub(super) fn selected_companion(
    snapshot: &ProjectExportSnapshot,
    selected: &HashSet<String>,
) -> Result<SelectedCompanion> {
    if snapshot.has_retained_evidence() {
        return Err(TransferError::Invalid("This project retains imported native evidence; use the native canonical export instead of the old selected companion".into()));
    }
    let original = snapshot.manifest();
    let nodes = array(original, "nodes")
        .iter()
        .filter(|node| node["id"].as_str().is_some_and(|id| selected.contains(id)))
        .cloned()
        .collect::<Vec<_>>();
    if nodes.len() != selected.len()
        || nodes
            .iter()
            .any(|node| node["currentRevision"].as_u64().unwrap_or(0) == 0)
    {
        return Err(TransferError::Invalid(
            "Select existing content-bearing documents from this project".into(),
        ));
    }
    let revisions = array(original, "documentRevisions")
        .iter()
        .filter(|record| {
            record["nodeId"]
                .as_str()
                .is_some_and(|id| selected.contains(id))
        })
        .cloned()
        .collect::<Vec<_>>();
    let native = snapshot.native_state();
    let aliases = native["documentAliases"].as_object().ok_or_else(|| {
        TransferError::Invalid("Canonical export is missing captured document aliases".into())
    })?;
    let selected_native: HashSet<String> = aliases
        .iter()
        .filter(|(_, logical)| logical.as_str().is_some_and(|id| selected.contains(id)))
        .map(|(native, _)| native.clone())
        .collect();
    let mut asset_ids = HashSet::new();
    let mut source_refs = HashSet::new();
    for revision in &revisions {
        collect_references(&revision["content"], &mut asset_ids, &mut source_refs);
    }
    for draft in array(snapshot.recovery(), "richDrafts")
        .iter()
        .filter(|record| {
            record["documentId"]
                .as_str()
                .is_some_and(|id| selected_native.contains(id))
        })
    {
        collect_references(&draft["content"], &mut asset_ids, &mut source_refs);
    }
    let annotations = array(original, "documentAnnotations")
        .iter()
        .filter(|record| {
            record["nodeId"]
                .as_str()
                .is_some_and(|id| selected.contains(id))
        })
        .cloned()
        .collect::<Vec<_>>();
    let terms: HashSet<String> = annotations
        .iter()
        .filter_map(|record| record["dictionaryTermId"].as_str())
        .map(str::to_owned)
        .collect();
    let sessions: HashSet<String> = array(original, "dictationSessions")
        .iter()
        .filter(|record| {
            record["destinationNodeId"]
                .as_str()
                .is_some_and(|id| selected.contains(id))
        })
        .filter_map(|record| record["id"].as_str())
        .map(str::to_owned)
        .collect();
    // Selecting writing does not authorize expanding a reference into another manuscript's source.
    let mut authorized_source_ids = HashSet::new();
    for artifact in array(original, "transcriptArtifacts")
        .iter()
        .filter(|record| {
            record["sessionId"]
                .as_str()
                .is_some_and(|id| sessions.contains(id))
        })
    {
        collect_ids(&artifact["segments"], &mut authorized_source_ids);
    }
    let outside_source_references = !source_refs.is_subset(&authorized_source_ids);
    let mut manifest = json!({"format":"writers-studio-project","formatVersion":original["formatVersion"],"project":{},"nodes":nodes,"documentRevisions":revisions,"documentAnnotations":annotations});
    for key in [
        "id",
        "title",
        "kind",
        "language",
        "structureRevision",
        "structure",
    ] {
        if let Some(value) = original["project"].get(key) {
            manifest["project"][key] = value.clone();
        }
    }
    for (domain, key, identities) in [
        ("namedVersions", "nodeId", selected),
        ("dictionaryTerms", "id", &terms),
        ("dictationSessions", "id", &sessions),
        ("transcriptArtifacts", "sessionId", &sessions),
        ("transcriptDrafts", "sessionId", &sessions),
        ("transcriptDraftRevisions", "sessionId", &sessions),
        ("audio", "sessionId", &sessions),
        ("media", "id", &asset_ids),
    ] {
        manifest[domain] = Value::Array(
            array(original, domain)
                .iter()
                .filter(|record| {
                    record[key]
                        .as_str()
                        .is_some_and(|id| identities.contains(id))
                })
                .cloned()
                .collect(),
        );
    }
    manifest["projectMemory"] = json!([]);
    manifest["projectMemoryRevisions"] = json!([]);
    manifest["canvases"] = json!([]);
    manifest["canvasRevisions"] = json!([]);
    let selected_annotations = array(native, "annotations")
        .iter()
        .filter(|record| {
            record["nativeDocumentId"]
                .as_str()
                .is_some_and(|id| selected_native.contains(id))
        })
        .cloned()
        .collect::<Vec<_>>();
    let native_notes: HashSet<String> = selected_annotations
        .iter()
        .filter_map(|record| record["nativeId"].as_str())
        .map(str::to_owned)
        .collect();
    let filtered_aliases: serde_json::Map<String, Value> = aliases
        .iter()
        .filter(|(native, _)| selected_native.contains(*native))
        .map(|(native, logical)| (native.clone(), logical.clone()))
        .collect();
    let opaque_documents = array(native, "opaqueDocuments")
        .iter()
        .filter(|record| {
            record["nativeDocumentId"]
                .as_str()
                .is_some_and(|id| selected_native.contains(id))
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut state = json!({"authority":"untrusted-on-import","documentAliases":filtered_aliases,"annotations":selected_annotations,"opaqueDocuments":opaque_documents,"canvasDrafts":[]});
    for (domain, key, identities) in [
        ("annotationHistory", "nativeAnnotationId", &native_notes),
        ("attachmentSnapshots", "nativeDocumentId", &selected_native),
        ("reviewEvidence", "nativeDocumentId", &selected_native),
    ] {
        state[domain] = Value::Array(
            array(native, domain)
                .iter()
                .filter(|record| {
                    record[key]
                        .as_str()
                        .is_some_and(|id| identities.contains(id))
                })
                .cloned()
                .collect(),
        );
    }
    if let Some(labels) = native.get("projectLabels") {
        state["projectLabels"] = selected_labels(labels, &state)?;
    }
    let mut recovery = json!({"format":"writercraft-recovery-evidence","formatVersion":1,"authority":"untrusted-on-import"});
    for domain in ["proseDrafts", "richDrafts"] {
        recovery[domain] = Value::Array(
            array(snapshot.recovery(), domain)
                .iter()
                .filter(|record| {
                    record["documentId"]
                        .as_str()
                        .is_some_and(|id| selected_native.contains(id))
                })
                .cloned()
                .collect(),
        );
    }
    recovery["unknownRevisionTimes"] = Value::Array(
        array(snapshot.recovery(), "unknownRevisionTimes")
            .iter()
            .filter(|record| {
                record["nodeId"]
                    .as_str()
                    .is_some_and(|id| selected.contains(id))
            })
            .cloned()
            .collect(),
    );
    let context = array(&manifest, "nodes")
        .iter()
        .filter_map(|node| {
            node["parentId"]
                .as_str()
                .filter(|id| !selected.contains(*id))
                .map(|id| json!({"nodeId":node["id"],"outsideParentId":id}))
        })
        .collect::<Vec<_>>();
    let mut selected_ids = selected.iter().collect::<Vec<_>>();
    selected_ids.sort();
    let companion = json!({"format":"writercraft-portable-project","formatVersion":1,"scope":"selected-documents","selectedNodeIds":selected_ids,"outsideSelectionContext":context,"sourceManifest":manifest,"nativeState":state,"recoveryEvidence":recovery});
    let bytes = serde_json::to_vec_pretty(&companion)?;
    Ok(SelectedCompanion {
        manifest: companion["sourceManifest"].clone(),
        bytes,
        asset_ids,
        audio_ids: sessions,
        outside_source_references,
    })
}
fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key].as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn selected_labels(catalog: &Value, state: &Value) -> Result<Value> {
    let fields = catalog.as_object().ok_or_else(|| {
        TransferError::Invalid(
            "Project label catalog is invalid; use complete-project canonical export".into(),
        )
    })?;
    if fields
        .keys()
        .any(|key| !matches!(key.as_str(), "schemaVersion" | "revision" | "labels"))
    {
        return Err(TransferError::Invalid("Project labels contain extension data that cannot be scoped safely; use complete-project canonical export".into()));
    }
    let mut names = HashSet::new();
    let mut collect = |annotation: &Value| {
        for name in array(annotation, "tags") {
            if let Some(name) = name.as_str() {
                names.insert(name.to_owned());
            }
        }
    };
    for note in array(state, "annotations") {
        collect(&note["state"]);
    }
    for event in array(state, "annotationHistory") {
        collect(&event["before"]);
        collect(&event["after"]);
    }
    let mut selected = catalog.clone();
    selected["labels"] = Value::Array(
        array(catalog, "labels")
            .iter()
            .filter(|label| {
                label["name"]
                    .as_str()
                    .is_some_and(|name| names.contains(name))
            })
            .cloned()
            .collect(),
    );
    Ok(selected)
}

fn collect_references(value: &Value, assets: &mut HashSet<String>, sources: &mut HashSet<String>) {
    if let Some(id) = value["attrs"]["assetId"].as_str().filter(|s| !s.is_empty()) {
        assets.insert(id.to_owned());
    }
    for source in array(&value["attrs"], "sourceRefs") {
        if let Some(id) = source.as_str() {
            sources.insert(id.to_owned());
        }
    }
    if let Some(source) = value["attrs"]["src"].as_str()
        && let Some(id) = asset_id(source)
    {
        assets.insert(id.to_owned());
    }
    for mark in array(value, "marks") {
        if let Some(href) = mark["attrs"]["href"].as_str()
            && let Some(id) = asset_id(href)
        {
            assets.insert(id.to_owned());
        }
    }
    for child in array(value, "content") {
        collect_references(child, assets, sources);
    }
}
fn collect_ids(value: &Value, output: &mut HashSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(id) = object.get("id").and_then(Value::as_str) {
                output.insert(id.to_owned());
            }
            for child in object.values() {
                collect_ids(child, output);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_ids(item, output);
            }
        }
        _ => {}
    }
}

fn asset_id(source: &str) -> Option<&str> {
    let source = source.split('#').next().unwrap_or(source);
    source.strip_prefix("asset://").or_else(|| {
        source
            .strip_prefix("/api/media/")
            .and_then(|s| s.strip_suffix("/content"))
    })
}

#[cfg(test)]
mod label_tests {
    use super::*;

    #[test]
    fn selected_catalog_preserves_included_colors_and_history_without_leaking_unused_names()
    -> Result<()> {
        let catalog = json!({"schemaVersion":1,"revision":8,"labels":[
            {"name":"Promise","color":"violet","retired":false,"vendor":{"keep":"included"}},
            {"name":"Earlier motive","color":"amber","retired":true},
            {"name":"Unrelated private vocabulary","color":"rose","retired":false},
            {"name":"Unused private label","color":"gray","retired":true}
        ]});
        let state = json!({"annotations":[{"state":{"tags":["Promise"]}}],"annotationHistory":[
            {"before":{"tags":["Earlier motive","Old name with no retained definition"]},"after":{"tags":["Promise"]}}
        ]});
        let selected = selected_labels(&catalog, &state)?;
        assert_eq!(
            selected["labels"],
            json!([catalog["labels"][0], catalog["labels"][1]])
        );
        assert_eq!(selected["revision"], 8);
        Ok(())
    }

    #[test]
    fn selected_catalog_refuses_unscopable_unknown_project_data() {
        let catalog =
            json!({"schemaVersion":1,"revision":0,"labels":[],"vendor":{"private":"unrelated"}});
        assert!(selected_labels(&catalog, &json!({})).is_err());
    }
}
