#![expect(
    clippy::unwrap_used,
    reason = "Synthetic migration contracts fail immediately"
)]
//! Schema-8 evidence storage must not alter existing authoring/recovery data or trust imported blobs.
use craft_editor_model::ScalarOffset;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use studio_core::Workspace;

fn content(text: &str) -> Value {
    json!({"type":"doc","attrs":{"schemaVersion":4,"unknown":{"keep":true}},"content":[{"type":"paragraph","attrs":{"blockId":"p"},"content":[{"type":"text","text":text}]}]})
}
fn graph(text: &str) -> Value {
    json!({"nodes":[{"id":"card","type":"text","x":0,"y":0,"width":200,"height":120,"text":text}],"edges":[],"x-writers-studio":{"schemaVersion":2,"unknown":null}})
}
#[test]
fn upgrading_schema_seven_should_preserve_rich_notes_canvas_and_both_draft_versions() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("upgrade.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Synthetic migration").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Writing", content("Hello 😀 author"))
        .unwrap();
    let anchor = workspace
        .capture_annotation_anchor(
            &document.id,
            1,
            "p",
            ScalarOffset(0),
            ScalarOffset(5),
            "Hello",
        )
        .unwrap();
    let note = workspace
        .create_annotation(&document.id, 1, anchor, "Preserve this note")
        .unwrap();
    let draft = workspace
        .save_rich_draft(&document.id, 1, None, content("Unfinished rich writing"))
        .unwrap();
    let board = workspace
        .create_project_canvas(&project.id, "Board", graph("Saved graph"))
        .unwrap();
    let board_draft = workspace
        .save_project_canvas_draft(
            &project.id,
            &board.canvas.id,
            1,
            None,
            graph("Unfinished graph"),
        )
        .unwrap();
    let before = workspace.project_export_snapshot(&project.id).unwrap();
    let saved = before.manifest().clone();
    drop(before);
    drop(workspace);
    {
        let connection = Connection::open(&database).unwrap();
        // Recreate an exact schema-7 predecessor; production migration must create the new tables.
        connection.execute_batch("DROP TABLE rich_proposals; DROP TABLE project_evidence_references; DROP TABLE project_evidence_objects; PRAGMA user_version=7;").unwrap();
    }
    let reopened = Workspace::open(&database).unwrap();
    assert_eq!(reopened.document(&document.id).unwrap(), document);
    assert_eq!(reopened.annotation(&note.id).unwrap(), note);
    let restored_draft = reopened.rich_draft(&document.id).unwrap().unwrap();
    assert_eq!(restored_draft.version, draft.version);
    assert_eq!(restored_draft.content, draft.content);
    let restored_board = reopened
        .project_canvas(&project.id, &board.canvas.id)
        .unwrap();
    assert_eq!(restored_board.revision, board.revision);
    assert_eq!(restored_board.draft.unwrap(), board_draft);
    assert_eq!(
        reopened
            .project_export_snapshot(&project.id)
            .unwrap()
            .manifest(),
        &saved
    );
    let connection = Connection::open(&database).unwrap();
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 10);
    for table in ["project_evidence_objects", "project_evidence_references"] {
        let present: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(present, 1);
    }
}
#[test]
fn evidence_references_should_require_objects_documents_and_boards_in_the_same_project() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("scope.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let first = workspace.create_project("First").unwrap();
    let other = workspace.create_project("Other").unwrap();
    let document = workspace
        .create_rich_document(&other.id, "Other writing", content("Other project"))
        .unwrap();
    let board = workspace
        .create_project_canvas(&other.id, "Other board", graph("Other graph"))
        .unwrap();
    let connection = Connection::open(&database).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    connection.execute("INSERT INTO project_nodes(project_id,logical_id,native_document_id,parent_logical_id,ordinal,raw_json) VALUES(?1,'foreign-only',?2,NULL,0,'{}')",params![other.id,document.id]).unwrap();
    let bytes = b"{\"synthetic\":\"quarantined\"}";
    let digest = format!("{:x}", Sha256::digest(bytes));
    connection.execute("INSERT INTO project_evidence_objects(project_id,sha256,byte_size,media_type,data) VALUES(?1,?2,?3,'application/json',?4)",params![other.id,digest,i64::try_from(bytes.len()).unwrap(),bytes.as_slice()]).unwrap();
    assert!(connection.execute("INSERT INTO project_evidence_references(project_id,domain,logical_id,object_sha256,metadata_json) VALUES(?1,'unknown','foreign-object',?2,'{}')",params![first.id,digest]).is_err());
    connection.execute("INSERT INTO project_evidence_objects(project_id,sha256,byte_size,media_type,data) VALUES(?1,?2,?3,'application/json',?4)",params![first.id,digest,i64::try_from(bytes.len()).unwrap(),bytes.as_slice()]).unwrap();
    assert!(connection.execute("INSERT INTO project_evidence_references(project_id,domain,logical_id,logical_document_id,object_sha256,metadata_json) VALUES(?1,'unknown','foreign-document','foreign-only',?2,'{}')",params![first.id,digest]).is_err());
    assert!(connection.execute("INSERT INTO project_evidence_references(project_id,domain,logical_id,logical_canvas_id,object_sha256,metadata_json) VALUES(?1,'unknown','foreign-board',?2,?3,'{}')",params![first.id,board.canvas.id,digest]).is_err());
    connection.execute("INSERT INTO project_evidence_references(project_id,domain,logical_id,object_sha256,metadata_json) VALUES(?1,'unknown','own-object',?2,'{\"authority\":\"untrusted-on-import\"}')",params![first.id,digest]).unwrap();
    let trusted: i64 = connection
        .query_row("SELECT COUNT(*) FROM rich_draft_annotations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(trusted, 0);
    let proposals: i64 = connection
        .query_row("SELECT COUNT(*) FROM proposals", [], |row| row.get(0))
        .unwrap();
    assert_eq!(proposals, 0);
}
#[test]
fn evidence_storage_should_reject_invalid_digest_length_blob_size_or_large_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("bounds.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Bounds").unwrap();
    let connection = Connection::open(&database).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    for digest in ["short".to_owned(), "g".repeat(64)] {
        assert!(connection.execute("INSERT INTO project_evidence_objects(project_id,sha256,byte_size,media_type,data) VALUES(?1,?2,0,'application/json',zeroblob(0))",params![project.id,digest]).is_err());
    }
    let digest = "0".repeat(64);
    assert!(connection.execute("INSERT INTO project_evidence_objects(project_id,sha256,byte_size,media_type,data) VALUES(?1,?2,67108865,'application/json',zeroblob(0))",params![project.id,digest]).is_err());
    assert!(connection.execute("INSERT INTO project_evidence_objects(project_id,sha256,byte_size,media_type,data) VALUES(?1,?2,1,'application/json',zeroblob(0))",params![project.id,digest]).is_err());
    connection.execute("INSERT INTO project_evidence_objects(project_id,sha256,byte_size,media_type,data) VALUES(?1,?2,0,'application/json',zeroblob(0))",params![project.id,digest]).unwrap();
    for metadata in [
        "not json".to_owned(),
        "null".to_owned(),
        format!("{{\"large\":\"{}\"}}", "x".repeat(262144)),
    ] {
        assert!(connection.execute("INSERT INTO project_evidence_references(project_id,domain,logical_id,object_sha256,metadata_json) VALUES(?1,'unknown','invalid',?2,?3)",params![project.id,digest,metadata]).is_err());
    }
}
