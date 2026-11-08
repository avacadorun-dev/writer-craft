#![expect(
    clippy::unwrap_used,
    reason = "Synthetic format-2 stream contracts fail immediately"
)]
//! Object identity, saved snapshot consistency and authority retention use synthetic data only.
use std::collections::HashMap;
use std::io::{Cursor, Read};

use craft_editor_model::Document as RichTree;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use studio_core::{
    EvidenceReferenceInput, ImportAssetSource, PortableObjectReference, PortableProjectHeader,
    PreparedPortableProject, ProjectAttachment, ProjectImportWorker, ProjectPortableExportWorker,
    ProjectTransferCancellation, ValidatedEvidenceObject, ValidatedEvidenceReference,
    ValidatedProjectImport, Workspace, visit_portable_shard,
};
struct NoAssets;
impl ImportAssetSource for NoAssets {
    fn open<'a>(&'a mut self, _: &ProjectAttachment) -> std::io::Result<Box<dyn Read + 'a>> {
        Err(std::io::Error::other("No synthetic assets declared"))
    }
}
fn object(value: &Value) -> PortableObjectReference {
    PortableObjectReference::new(
        value["sha256"].as_str().unwrap(),
        value["byteSize"].as_u64().unwrap(),
    )
    .unwrap()
}
fn bytes(project: &PreparedPortableProject, reference: &PortableObjectReference) -> Vec<u8> {
    let mut bytes = Vec::new();
    project
        .copy_object(
            reference,
            &mut bytes,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    bytes
}
fn records(project: &PreparedPortableProject) -> HashMap<String, Vec<Value>> {
    let root = PortableProjectHeader::parse(project.root_bytes()).unwrap();
    let mut records: HashMap<String, Vec<Value>> = HashMap::new();
    for component in root.components() {
        let shard = bytes(project, component.object());
        visit_portable_shard(
            component,
            &mut Cursor::new(shard),
            &ProjectTransferCancellation::default(),
            &mut |record| {
                records
                    .entry(component.domain().to_owned())
                    .or_default()
                    .push(serde_json::from_str(record.raw_json())?);
                Ok(())
            },
        )
        .unwrap();
    }
    records
}
#[test]
fn prepared_native_project_should_deduplicate_payloads_and_keep_one_consistent_revision_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Native 😀").unwrap();
    let rich = RichTree::plain("p", "same rich source")
        .unwrap()
        .to_json()
        .unwrap();
    let first = workspace
        .create_rich_document(&project.id, "First", rich.clone())
        .unwrap();
    workspace
        .create_rich_document(&project.id, "Second", rich)
        .unwrap();
    let mut worker = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared = worker
        .prepare(&project.id, &ProjectTransferCancellation::default())
        .unwrap();
    workspace
        .save_rich_document(
            &first.id,
            1,
            RichTree::plain("p", "later edit")
                .unwrap()
                .to_json()
                .unwrap(),
        )
        .unwrap();
    let records = records(&prepared);
    let revisions = &records["documentRevisions"];
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0]["richContent"], revisions[1]["richContent"]);
    assert_eq!(revisions[0]["body"], revisions[1]["body"]);
    assert_eq!(
        bytes(&prepared, &object(&revisions[0]["body"])),
        b"same rich source"
    );
    assert_eq!(revisions[0]["revision"], 1);
    let mut descriptors = Vec::new();
    prepared
        .visit_objects(&mut |reference| {
            descriptors.push(reference.clone());
            Ok(())
        })
        .unwrap();
    assert_eq!(prepared.object_count(), descriptors.len() as u64);
    assert_eq!(
        prepared.object_bytes(),
        descriptors
            .iter()
            .map(PortableObjectReference::byte_size)
            .sum()
    );
    let forged =
        PortableObjectReference::new(descriptors[0].sha256(), descriptors[0].byte_size() + 1)
            .unwrap();
    assert!(
        prepared
            .copy_object(
                &forged,
                &mut Vec::new(),
                &ProjectTransferCancellation::default()
            )
            .is_err()
    );
}
#[test]
fn exact_original_source_should_stream_once_and_saved_native_revisions_should_be_deltas() {
    let original = Workspace::memory().unwrap();
    let source_project = original.create_project("Original").unwrap();
    original
        .create_rich_document(
            &source_project.id,
            "Source",
            RichTree::plain("p", "original").unwrap().to_json().unwrap(),
        )
        .unwrap();
    let snapshot = original
        .project_export_snapshot(&source_project.id)
        .unwrap();
    let raw = format!(
        " \n{}\r\n",
        std::str::from_utf8(snapshot.manifest_bytes()).unwrap()
    );
    let validated = ValidatedProjectImport::parse(raw.as_bytes()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let mut importer = ProjectImportWorker::open(&database).unwrap();
    let imported = importer
        .import(
            &validated,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let document = importer
        .workspace()
        .list_documents(&imported.project_id)
        .unwrap()
        .remove(0);
    importer
        .workspace()
        .save_rich_document(
            &document.id,
            1,
            RichTree::plain("p", "native new revision")
                .unwrap()
                .to_json()
                .unwrap(),
        )
        .unwrap();
    let mut exporter = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared = exporter
        .prepare(
            &imported.project_id,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let root = PortableProjectHeader::parse(prepared.root_bytes()).unwrap();
    assert_eq!(
        bytes(&prepared, root.source_original().unwrap()),
        raw.as_bytes()
    );
    let records = records(&prepared);
    assert_eq!(records["documentRevisions"].len(), 1);
    assert_eq!(records["documentRevisions"][0]["revision"], 2);
    assert_eq!(
        bytes(&prepared, &object(&records["documentRevisions"][0]["body"])),
        b"native new revision"
    );
}
#[test]
fn large_unknown_evidence_should_remain_flat_streamed_bytes_and_never_become_a_proposal() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Evidence").unwrap();
    let raw = format!(
        "{{\"checkpoint\":{{\"sealed\":true}},\"opaque\":\"{}\"}}",
        "x".repeat(2 * 1024 * 1024)
    );
    let descriptor = ValidatedEvidenceObject::new(
        &format!("{:x}", Sha256::digest(raw.as_bytes())),
        raw.len() as u64,
        "application/json",
    )
    .unwrap();
    let reference = ValidatedEvidenceReference::new(
        &descriptor,
        EvidenceReferenceInput {
            domain: "unknown.recovery".into(),
            logical_id: "current".into(),
            logical_document_id: None,
            logical_canvas_id: None,
            metadata_json: "{\"authority\":\"trusted\"}".into(),
        },
    )
    .unwrap();
    workspace
        .import_project_evidence(
            &project.id,
            &descriptor,
            &reference,
            &mut Cursor::new(raw.as_bytes()),
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let mut worker = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared = worker
        .prepare(&project.id, &ProjectTransferCancellation::default())
        .unwrap();
    let records = records(&prepared);
    assert_eq!(records["evidence"].len(), 1);
    assert_eq!(records["evidence"][0]["domain"], "unknown.recovery");
    assert_eq!(
        bytes(&prepared, &object(&records["evidence"][0]["object"])),
        raw.as_bytes()
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes(
            &prepared,
            &object(&records["evidence"][0]["metadata"])
        ))
        .unwrap(),
        json!({"authority":"trusted"})
    );
    assert!(!records.contains_key("richReviewEvidence"));
    assert!(workspace.list_documents(&project.id).unwrap().is_empty());
}
#[test]
fn cancelled_preparation_should_leave_saved_workspace_unchanged_and_publish_no_capability() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Cancel").unwrap();
    let cancel = ProjectTransferCancellation::default();
    cancel.cancel();
    let mut worker = ProjectPortableExportWorker::open(&database).unwrap();
    assert!(worker.prepare(&project.id, &cancel).is_err());
    assert_eq!(workspace.list_projects().unwrap().len(), 1);
    assert!(workspace.list_documents(&project.id).unwrap().is_empty());
}
