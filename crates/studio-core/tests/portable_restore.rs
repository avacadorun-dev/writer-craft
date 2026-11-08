#![expect(
    clippy::unwrap_used,
    reason = "Synthetic canonical restore contracts fail immediately"
)]
//! Historical compatibility restores saved author data, never external replay or Apply authority.
use std::io::Read;

use craft_editor_model::{Document as RichTree, ScalarOffset};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use studio_core::{
    AnchorStatus, AnnotationPatch, AnnotationStatus, EvidenceReferenceQuery, ImportAssetSource,
    ProjectAttachment, ProjectImportWorker, ProjectTransferCancellation,
    ValidatedPortableProjectImport, ValidatedProjectImport, Workspace,
};

struct NoAssets;
impl ImportAssetSource for NoAssets {
    fn open<'a>(&'a mut self, _: &ProjectAttachment) -> std::io::Result<Box<dyn Read + 'a>> {
        Err(std::io::Error::other(
            "No assets declared by this synthetic fixture",
        ))
    }
}
fn fixture() -> (Workspace, String, String, String, Vec<u8>) {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Canonical 作者 😀").unwrap();
    let rich = RichTree::plain("p", "echo 😀 echo").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Native", rich.to_json().unwrap())
        .unwrap();
    let anchor = workspace
        .capture_annotation_anchor(
            &document.id,
            1,
            "p",
            ScalarOffset(7),
            ScalarOffset(11),
            "echo",
        )
        .unwrap();
    let note = workspace
        .create_annotation(&document.id, 1, anchor, "Keep the second occurrence")
        .unwrap();
    workspace
        .update_annotation(
            &note.id,
            1,
            1,
            AnnotationPatch {
                note: None,
                status: Some(AnnotationStatus::NeedsAttention),
            },
        )
        .unwrap();
    let bytes = workspace
        .project_export_snapshot(&project.id)
        .unwrap()
        .canonical_manifest_bytes()
        .unwrap();
    (workspace, project.id, document.id, note.id, bytes)
}
fn parse(value: &Value) -> studio_core::Result<ValidatedPortableProjectImport> {
    ValidatedPortableProjectImport::parse_v1(&serde_json::to_vec(value).unwrap())
}
#[test]
fn native_notes_should_restore_exact_repeated_unicode_occurrence_and_history_in_independent_namespaces()
 {
    let (_source, _project, old_document, old_note, bytes) = fixture();
    let validated = ValidatedPortableProjectImport::parse_v1(&bytes).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let first = worker
        .restore_portable(
            &validated,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let second = worker
        .restore_portable(
            &validated,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    assert_ne!(first.project_id, second.project_id);
    let a = worker
        .workspace()
        .resolve_project_resource(&first.project_id, "document", &old_document)
        .unwrap();
    let b = worker
        .workspace()
        .resolve_project_resource(&second.project_id, "document", &old_document)
        .unwrap();
    assert_ne!(a, b);
    assert_ne!(a, old_document);
    let notes = worker.workspace().annotations(&a).unwrap();
    assert_eq!(notes.len(), 1);
    assert_ne!(notes[0].id, old_note);
    assert_eq!(notes[0].document_id, a);
    assert_eq!(notes[0].anchor.start, ScalarOffset(7));
    assert_eq!(notes[0].anchor.end, ScalarOffset(11));
    assert_eq!(notes[0].anchor_status, AnchorStatus::Attached);
    assert_eq!(notes[0].status, AnnotationStatus::NeedsAttention);
    let events = worker.workspace().annotation_history(&notes[0].id).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].after, notes[0]);
    assert!(events.iter().all(|event| event.after.document_id == a));
    assert_eq!(
        worker.workspace().document(&a).unwrap().body,
        "echo 😀 echo"
    );
    assert!(
        worker
            .workspace()
            .resolve_project_resource(&second.project_id, "native-annotation", &notes[0].id)
            .is_err()
    );
}
#[test]
fn invalid_or_declared_detached_anchor_should_never_reattach_by_quote_search() {
    let (_, _, _, _, bytes) = fixture();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    for (key, replacement) in [("start", json!(0)), ("exact_text", json!("missing"))] {
        let mut changed = value.clone();
        changed["nativeState"]["annotations"][0]["state"]["anchor"][key] = replacement;
        // Retain raw historic states independently, while the current note remains readable detached.
        let parsed = parse(&changed).unwrap();
        let mut worker = ProjectImportWorker::memory().unwrap();
        let imported = worker
            .restore_portable(
                &parsed,
                &mut NoAssets,
                &ProjectTransferCancellation::default(),
            )
            .unwrap();
        let document = worker
            .workspace()
            .list_documents(&imported.project_id)
            .unwrap()
            .remove(0);
        assert_eq!(
            worker.workspace().annotations(&document.id).unwrap()[0].anchor_status,
            AnchorStatus::Detached
        );
    }
    let mut changed = value;
    changed["nativeState"]["annotations"][0]["state"]["anchor_status"] = json!("detached");
    let parsed = parse(&changed).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let imported = worker
        .restore_portable(
            &parsed,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let document = worker
        .workspace()
        .list_documents(&imported.project_id)
        .unwrap()
        .remove(0);
    assert_eq!(
        worker.workspace().annotations(&document.id).unwrap()[0].anchor_status,
        AnchorStatus::Detached
    );
}
#[test]
fn forged_alias_missing_base_duplicate_note_or_mismatched_source_should_fail_before_writes() {
    let (_, _, old_document, _, bytes) = fixture();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    let mut bad_alias = value.clone();
    bad_alias["nativeState"]["documentAliases"][&old_document] = json!("foreign-document");
    assert!(parse(&bad_alias).is_err());
    let mut missing_base = value.clone();
    missing_base["nativeState"]["annotations"][0]["state"]["base_revision"] = json!(99);
    assert!(parse(&missing_base).is_err());
    let mut repeated = value.clone();
    let note = repeated["nativeState"]["annotations"][0].clone();
    repeated["nativeState"]["annotations"]
        .as_array_mut()
        .unwrap()
        .push(note);
    assert!(parse(&repeated).is_err());
    let mut bad_source = value;
    bad_source["sourceManifest"]["project"]["title"] = json!("Different source");
    assert!(parse(&bad_source).is_err());
}
#[test]
fn saved_tombstones_should_keep_original_metadata_and_supplied_history() {
    let (workspace, project, _, note, _) = fixture();
    workspace.delete_annotation(&note, 1, 2).unwrap();
    let bytes = workspace
        .project_export_snapshot(&project)
        .unwrap()
        .canonical_manifest_bytes()
        .unwrap();
    let validated = ValidatedPortableProjectImport::parse_v1(&bytes).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let imported = worker
        .restore_portable(
            &validated,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let document = worker
        .workspace()
        .list_documents(&imported.project_id)
        .unwrap()
        .remove(0);
    assert!(
        worker
            .workspace()
            .annotations(&document.id)
            .unwrap()
            .is_empty()
    );
    let snapshot = worker
        .workspace()
        .project_export_snapshot(&imported.project_id)
        .unwrap();
    assert_eq!(
        snapshot.native_state()["annotations"][0]["state"]["deleted"],
        true
    );
    assert_eq!(
        snapshot.native_state()["annotations"][0]["state"]["revision"],
        3
    );
    assert_eq!(
        snapshot.native_state()["annotationHistory"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}
#[test]
fn forged_checkpoint_proposal_and_unknown_values_should_remain_flat_readable_evidence() {
    let (_, _, _, _, bytes) = fixture();
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["nativeState"]["reviewEvidence"] = json!([{"status":"applied","before":"old","after":"replace all","checkpoint":{"sealed":true}}]);
    value["recoveryEvidence"]["richDrafts"] =
        json!([{"content":{"type":"doc"},"checkpointEvidence":{"replay":"trusted"}}]);
    value["futureVendor"] = json!({"keep":[1,null,"unknown"]});
    let parsed = parse(&value).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let imported = worker
        .restore_portable(
            &parsed,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let reference = worker
        .workspace()
        .project_evidence(&imported.project_id, "portable.native", "reviewEvidence")
        .unwrap();
    let raw = reference
        .load(1024, &ProjectTransferCancellation::default())
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&raw).unwrap(),
        value["nativeState"]["reviewEvidence"]
    );
    let document = worker
        .workspace()
        .list_documents(&imported.project_id)
        .unwrap()
        .remove(0);
    assert_eq!(document.body, "echo 😀 echo");
    assert!(
        worker
            .workspace()
            .rich_draft(&document.id)
            .unwrap()
            .is_none()
    );
    let references = worker
        .workspace()
        .project_evidence_references(&imported.project_id, &EvidenceReferenceQuery::default())
        .unwrap();
    assert!(
        references
            .iter()
            .any(|record| record.domain == "portable.extension"
                && record.logical_id == "futureVendor")
    );
}
#[test]
fn native_empty_project_should_restore_without_weakening_original_archive_validation() {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Empty").unwrap();
    let snapshot = workspace.project_export_snapshot(&project.id).unwrap();
    assert!(ValidatedProjectImport::parse(snapshot.manifest_bytes()).is_err());
    let parsed =
        ValidatedPortableProjectImport::parse_v1(&snapshot.canonical_manifest_bytes().unwrap())
            .unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let imported = worker
        .restore_portable(
            &parsed,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    assert_eq!(imported.node_count, 0);
    assert!(
        worker
            .workspace()
            .list_documents(&imported.project_id)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn cancellation_duplicate_keys_or_corrupt_raw_hash_should_leave_existing_projects_unchanged() {
    let (_, _, _, _, bytes) = fixture();
    let parsed = ValidatedPortableProjectImport::parse_v1(&bytes).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    worker.workspace().create_project("Existing").unwrap();
    let cancel = ProjectTransferCancellation::default();
    cancel.cancel();
    assert!(
        worker
            .restore_portable(&parsed, &mut NoAssets, &cancel)
            .is_err()
    );
    assert_eq!(worker.workspace().list_projects().unwrap().len(), 1);
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    let raw = value["sourceManifestRaw"].as_str().unwrap().to_owned();
    value["sourceManifestRaw"] = json!(format!("  {raw}\n"));
    assert!(parse(&value).is_err());
    value["sourceManifestSha256"] = json!(format!(
        "{:x}",
        Sha256::digest(value["sourceManifestRaw"].as_str().unwrap().as_bytes())
    ));
    let parsed = parse(&value).unwrap();
    assert_eq!(
        parsed.source().original_manifest(),
        value["sourceManifestRaw"].as_str().unwrap()
    );
    assert!(
        ValidatedPortableProjectImport::parse_v1(
            br#"{"format":"writercraft-portable-project","formatVersion":1,"formatVersion":1}"#
        )
        .is_err()
    );
}
#[test]
fn imported_note_should_preserve_the_source_24000_character_bound_and_refuse_24001() {
    let (_, _, _, _, bytes) = fixture();
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["nativeState"]["annotationHistory"] = json!([]);
    value["nativeState"]["annotations"][0]["state"]["note"] =
        json!("Я".repeat(studio_core::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS));
    let parsed = parse(&value).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let imported = worker
        .restore_portable(
            &parsed,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let document = worker
        .workspace()
        .list_documents(&imported.project_id)
        .unwrap()
        .remove(0);
    let note = worker
        .workspace()
        .annotations(&document.id)
        .unwrap()
        .remove(0);
    assert_eq!(note.note, "Я".repeat(24_000));
    value["nativeState"]["annotations"][0]["state"]["note"] =
        json!("Я".repeat(studio_core::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS + 1));
    assert!(parse(&value).is_err());
}
