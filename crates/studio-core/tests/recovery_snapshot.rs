#![expect(
    clippy::unwrap_used,
    reason = "Synthetic read-only recovery fixtures fail immediately"
)]
use craft_editor_model::{
    Command, Document as RichTree, Editor, Revision, ScalarOffset, TextRange, Transaction,
};
use serde_json::{Value, json};
use studio_core::{AnnotationPatch, Workspace};
fn fixture(
    workspace: &Workspace,
) -> (
    studio_core::Document,
    studio_core::PassageAnnotation,
    studio_core::RichDraft,
) {
    let project = workspace.create_project("Preservation").unwrap();
    let tree = RichTree::plain("p", "Я😀 echo echo tail").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Saved copy", tree.to_json().unwrap())
        .unwrap();
    let anchor = workspace
        .capture_annotation_anchor(
            &document.id,
            1,
            "p",
            ScalarOffset(8),
            ScalarOffset(12),
            "echo",
        )
        .unwrap();
    let note = workspace
        .create_annotation(&document.id, 1, anchor, "Inspect this occurrence")
        .unwrap();
    let mut editor = Editor::new(tree, Revision(1));
    let insert = editor
        .apply(Transaction {
            base_revision: editor.revision(),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(10),
                    end: ScalarOffset(10),
                },
                expected: "".into(),
                replacement: "東京".into(),
            }],
        })
        .unwrap();
    let delete = editor
        .apply(Transaction {
            base_revision: editor.revision(),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(8),
                    end: ScalarOffset(14),
                },
                expected: "ec東京ho".into(),
                replacement: "".into(),
            }],
        })
        .unwrap();
    let draft = workspace
        .save_rich_editor_draft(
            &document.id,
            1,
            None,
            &[insert.replay_proof(), delete.replay_proof()],
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    (document, note, draft)
}
#[test]
fn frozen_snapshot_preserves_both_copies_note_history_and_last_safe_checkpoint_without_writes() {
    let workspace = Workspace::memory().unwrap();
    let (document, note, draft) = fixture(&workspace);
    let before = workspace.annotation_history(&note.id).unwrap();
    let snapshot = workspace.rich_recovery_snapshot(&document.id).unwrap();
    assert_eq!(snapshot.document(), &document);
    assert_eq!(snapshot.draft(), &draft);
    assert_eq!(snapshot.annotations().len(), 1);
    assert_eq!(snapshot.annotation_history().len(), before.len());
    let checkpoint = snapshot.attachment_checkpoint().unwrap();
    assert!(
        serde_json::to_string(checkpoint)
            .unwrap()
            .contains("ec東京ho")
    );
    assert_eq!(checkpoint["data_only"], true);
    let value = serde_json::to_value(&snapshot).unwrap();
    assert!(value.get("guard").is_none());
    assert_eq!(value["draft"]["content"], draft.content);
    workspace.verify_rich_recovery_snapshot(&snapshot).unwrap();
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.annotation(&note.id).unwrap(), note);
    assert_eq!(workspace.annotation_history(&note.id).unwrap(), before);
}
#[test]
fn frozen_snapshot_refuses_live_note_or_exact_draft_identity_changes() {
    let workspace = Workspace::memory().unwrap();
    let (document, note, _) = fixture(&workspace);
    let snapshot = workspace.rich_recovery_snapshot(&document.id).unwrap();
    workspace
        .update_annotation(
            &note.id,
            1,
            note.revision,
            AnnotationPatch {
                note: Some("A later note".into()),
                status: None,
            },
        )
        .unwrap();
    assert!(workspace.verify_rich_recovery_snapshot(&snapshot).is_err());
    assert_eq!(snapshot.annotations()[0]["note"], note.note);
    let newer = workspace.rich_recovery_snapshot(&document.id).unwrap();
    let draft = workspace.rich_draft(&document.id).unwrap().unwrap();
    workspace
        .save_rich_draft(&document.id, 1, Some(&draft.version), draft.content)
        .unwrap();
    assert!(workspace.verify_rich_recovery_snapshot(&newer).is_err());
}
#[test]
fn raw_unknown_note_history_and_checkpoint_json_survive_preservation_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("snapshot.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    let (document, note, _) = fixture(&workspace);
    let raw = rusqlite::Connection::open(&path).unwrap();
    let original: String = raw
        .query_row(
            "SELECT annotation_json FROM document_annotations WHERE id=?1",
            [&note.id],
            |row| row.get(0),
        )
        .unwrap();
    let mut value: Value = serde_json::from_str(&original).unwrap();
    value["vendor"] = json!({"unknown":[null,"保留",42]});
    raw.execute(
        "UPDATE document_annotations SET annotation_json=?2 WHERE id=?1",
        rusqlite::params![note.id, serde_json::to_string(&value).unwrap()],
    )
    .unwrap();
    let snapshot = workspace.rich_recovery_snapshot(&document.id).unwrap();
    assert_eq!(snapshot.annotations()[0], value);
    workspace.verify_rich_recovery_snapshot(&snapshot).unwrap();
    raw.execute(
        "UPDATE rich_draft_annotations SET checkpoint_json=?2 WHERE document_id=?1",
        rusqlite::params![document.id, "{preserve malformed checkpoint bytes"],
    )
    .unwrap();
    assert!(workspace.verify_rich_recovery_snapshot(&snapshot).is_err());
    let newer = workspace.rich_recovery_snapshot(&document.id).unwrap();
    assert_eq!(
        newer.attachment_checkpoint().unwrap()["checkpoint"],
        "{preserve malformed checkpoint bytes"
    );
}
#[test]
fn deleted_note_body_and_its_complete_audit_history_remain_in_recovery_evidence() {
    let workspace = Workspace::memory().unwrap();
    let (document, note, _) = fixture(&workspace);
    workspace
        .delete_annotation(&note.id, document.revision, note.revision)
        .unwrap();
    assert!(workspace.annotations(&document.id).unwrap().is_empty());
    let snapshot = workspace.rich_recovery_snapshot(&document.id).unwrap();
    assert_eq!(snapshot.annotations().len(), 1);
    assert_eq!(snapshot.annotations()[0]["deleted"], true);
    assert_eq!(snapshot.annotations()[0]["note"], note.note);
    assert_eq!(snapshot.annotation_history().len(), 2);
    assert_eq!(snapshot.annotation_history()[1]["source"], "delete");
    workspace.verify_rich_recovery_snapshot(&snapshot).unwrap();
}
