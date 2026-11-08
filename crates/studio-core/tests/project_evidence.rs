#![expect(
    clippy::unwrap_used,
    reason = "Synthetic evidence contracts fail immediately"
)]
//! Raw evidence is retained without importing replay or Apply authority.
use std::io::{Cursor, Read};

use rusqlite::{Connection, params};
use serde_json::json;
use sha2::{Digest, Sha256};
use studio_core::{
    EvidenceReferenceInput, EvidenceReferenceQuery, MAX_EVIDENCE_METADATA_BYTES,
    MAX_EVIDENCE_OBJECT_BYTES, ProjectTransferCancellation, ValidatedEvidenceObject,
    ValidatedEvidenceReference, Workspace,
};

fn descriptor(bytes: &[u8]) -> ValidatedEvidenceObject {
    ValidatedEvidenceObject::new(
        &format!("{:x}", Sha256::digest(bytes)),
        u64::try_from(bytes.len()).unwrap(),
        "application/json",
    )
    .unwrap()
}
fn reference(
    object: &ValidatedEvidenceObject,
    id: &str,
    metadata: &str,
) -> ValidatedEvidenceReference {
    ValidatedEvidenceReference::new(
        object,
        EvidenceReferenceInput {
            domain: "recovery.unknown".into(),
            logical_id: id.into(),
            logical_document_id: None,
            logical_canvas_id: None,
            metadata_json: metadata.into(),
        },
    )
    .unwrap()
}
fn count(connection: &Connection, table: &str) -> i64 {
    assert!(
        [
            "project_evidence_objects",
            "project_evidence_references",
            "rich_draft_annotations",
            "proposals"
        ]
        .contains(&table)
    );
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}
#[test]
fn raw_metadata_and_objects_should_deduplicate_without_gaining_checkpoint_or_apply_authority() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("evidence.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Evidence").unwrap();
    let bytes = b"{\"forgedCheckpoint\":{\"sealed\":true},\"unknown\":[1,null]}";
    let object = descriptor(bytes);
    let metadata = "\n {\"authority\":\"trusted\",\"precise\":1.234567890123456789e+100,\"future\":{\"$serde_json::private::Number\":\"opaque\"}} \r\n";
    let cancel = ProjectTransferCancellation::default();
    for id in ["one", "two"] {
        workspace
            .import_project_evidence(
                &project.id,
                &object,
                &reference(&object, id, metadata),
                &mut Cursor::new(bytes),
                &cancel,
            )
            .unwrap();
    }
    let selected = workspace
        .project_evidence(&project.id, "recovery.unknown", "one")
        .unwrap();
    assert_eq!(selected.metadata_json(), metadata);
    assert_eq!(selected.load(bytes.len(), &cancel).unwrap(), bytes);
    assert!(selected.load(bytes.len() - 1, &cancel).is_err());
    let connection = Connection::open(&database).unwrap();
    assert_eq!(count(&connection, "project_evidence_objects"), 1);
    assert_eq!(count(&connection, "project_evidence_references"), 2);
    assert_eq!(count(&connection, "rich_draft_annotations"), 0);
    assert_eq!(count(&connection, "proposals"), 0);
    let first = workspace
        .project_evidence_references(
            &project.id,
            &EvidenceReferenceQuery {
                limit: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(first.len(), 1);
    let next = workspace
        .project_evidence_references(
            &project.id,
            &EvidenceReferenceQuery {
                after: Some((first[0].domain.clone(), first[0].logical_id.clone())),
                limit: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(next.len(), 1);
    assert_ne!(first[0].logical_id, next[0].logical_id);
}
#[test]
fn bad_source_duplicate_reference_or_stored_corruption_should_never_replace_existing_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("evidence.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Integrity").unwrap();
    let bytes = b"synthetic original";
    let object = descriptor(bytes);
    let cancel = ProjectTransferCancellation::default();
    workspace
        .import_project_evidence(
            &project.id,
            &object,
            &reference(&object, "saved", "{}"),
            &mut Cursor::new(bytes),
            &cancel,
        )
        .unwrap();
    let corrupt = b"synthetic corrupte";
    assert_eq!(bytes.len(), corrupt.len());
    assert!(
        workspace
            .import_project_evidence(
                &project.id,
                &object,
                &reference(&object, "bad-source", "{}"),
                &mut Cursor::new(corrupt),
                &cancel
            )
            .is_err()
    );
    let other = b"different original";
    let other_object = descriptor(other);
    assert!(
        workspace
            .import_project_evidence(
                &project.id,
                &other_object,
                &reference(&other_object, "saved", "{}"),
                &mut Cursor::new(other),
                &cancel
            )
            .is_err()
    );
    let connection = Connection::open(&database).unwrap();
    assert_eq!(count(&connection, "project_evidence_objects"), 1);
    assert_eq!(count(&connection, "project_evidence_references"), 1);
    let captured = workspace
        .project_evidence(&project.id, "recovery.unknown", "saved")
        .unwrap();
    assert_eq!(captured.load(1024, &cancel).unwrap(), bytes);
    connection
        .execute(
            "UPDATE project_evidence_objects SET data=?3 WHERE project_id=?1 AND sha256=?2",
            params![project.id, object.sha256(), corrupt.as_slice()],
        )
        .unwrap();
    assert!(captured.load(1024, &cancel).is_err());
    assert!(
        workspace
            .project_evidence(&project.id, "recovery.unknown", "bad-source")
            .is_err()
    );
}
struct CancelAfterRead {
    reader: Cursor<Vec<u8>>,
    cancel: ProjectTransferCancellation,
}
impl Read for CancelAfterRead {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.reader.read(buffer)?;
        if count > 0 {
            self.cancel.cancel();
        }
        Ok(count)
    }
}
#[test]
fn midstream_cancellation_should_rollback_new_blob_and_reference() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("cancel.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Cancellation").unwrap();
    let bytes = vec![42_u8; 128 * 1024];
    let object = descriptor(&bytes);
    let cancel = ProjectTransferCancellation::default();
    let mut reader = CancelAfterRead {
        reader: Cursor::new(bytes),
        cancel: cancel.clone(),
    };
    assert!(
        workspace
            .import_project_evidence(
                &project.id,
                &object,
                &reference(&object, "cancelled", "{}"),
                &mut reader,
                &cancel
            )
            .is_err()
    );
    let connection = Connection::open(&database).unwrap();
    assert_eq!(count(&connection, "project_evidence_objects"), 0);
    assert_eq!(count(&connection, "project_evidence_references"), 0);
}
#[test]
fn descriptors_raw_metadata_and_paging_should_fail_bounds_before_storage() {
    assert!(
        ValidatedEvidenceObject::new(
            &"0".repeat(64),
            MAX_EVIDENCE_OBJECT_BYTES + 1,
            "application/json"
        )
        .is_err()
    );
    assert!(ValidatedEvidenceObject::new("not-a-sha", 1, "application/json").is_err());
    let object = descriptor(b"small");
    for metadata in [
        "{\"duplicate\":1,\"duplicate\":2}".to_owned(),
        "[]".to_owned(),
        format!(
            "{{\"large\":\"{}\"}}",
            "x".repeat(MAX_EVIDENCE_METADATA_BYTES)
        ),
    ] {
        assert!(
            ValidatedEvidenceReference::new(
                &object,
                EvidenceReferenceInput {
                    domain: "unknown".into(),
                    logical_id: "invalid".into(),
                    logical_document_id: None,
                    logical_canvas_id: None,
                    metadata_json: metadata
                }
            )
            .is_err()
        );
    }
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Page bounds").unwrap();
    for limit in [0, 257] {
        assert!(
            workspace
                .project_evidence_references(
                    &project.id,
                    &EvidenceReferenceQuery {
                        limit,
                        ..Default::default()
                    }
                )
                .is_err()
        );
    }
}
#[test]
fn reference_scopes_should_not_resolve_foreign_documents_canvases_or_objects() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("scope.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let first = workspace.create_project("First").unwrap();
    let other = workspace.create_project("Other").unwrap();
    let document = workspace
        .create_document(&other.id, "Other", "Synthetic")
        .unwrap();
    let canvas = workspace
        .create_project_canvas(
            &other.id,
            "Other board",
            json!({"nodes":[],"edges":[],"x-writers-studio":{"schemaVersion":2}}),
        )
        .unwrap();
    let connection = Connection::open(&database).unwrap();
    let node = json!({"id":"foreign-only","kind":"document","title":"Other","status":"draft","position":0,"currentRevision":1});
    connection.execute("INSERT INTO project_nodes(project_id,logical_id,native_document_id,parent_logical_id,ordinal,raw_json) VALUES(?1,'foreign-only',?2,NULL,0,?3)",params![other.id,document.id,serde_json::to_string(&node).unwrap()]).unwrap();
    let bytes = b"scoped original";
    let object = descriptor(bytes);
    let cancel = ProjectTransferCancellation::default();
    for (document, canvas) in [
        (Some("foreign-only".to_owned()), None),
        (None, Some(canvas.canvas.id)),
    ] {
        let reference = ValidatedEvidenceReference::new(
            &object,
            EvidenceReferenceInput {
                domain: "unknown".into(),
                logical_id: "foreign".into(),
                logical_document_id: document,
                logical_canvas_id: canvas,
                metadata_json: "{}".into(),
            },
        )
        .unwrap();
        assert!(
            workspace
                .import_project_evidence(
                    &first.id,
                    &object,
                    &reference,
                    &mut Cursor::new(bytes),
                    &cancel
                )
                .is_err()
        );
    }
    workspace
        .import_project_evidence(
            &other.id,
            &object,
            &reference(&object, "owned", "{}"),
            &mut Cursor::new(bytes),
            &cancel,
        )
        .unwrap();
    assert!(
        workspace
            .project_evidence(&first.id, "recovery.unknown", "owned")
            .is_err()
    );
    assert!(
        workspace
            .project_evidence_references(&first.id, &Default::default())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        workspace
            .project_evidence(&other.id, "recovery.unknown", "owned")
            .unwrap()
            .load(1024, &cancel)
            .unwrap(),
        bytes
    );
}
