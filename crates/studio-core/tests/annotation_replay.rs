#![expect(
    clippy::unwrap_used,
    reason = "Synthetic replay fixtures must fail immediately"
)]

use craft_editor_model::{
    Command, Document as RichTree, Editor, ReplayProof, Revision, ScalarOffset, TextRange,
    Transaction,
};
use serde_json::{Value, json};
use studio_core::{AnchorStatus, AnnotationPatch, AnnotationStatus, Document, Workspace};

fn fixture() -> (Workspace, Document, Editor, String) {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Replay").unwrap();
    let tree = RichTree::plain("p", "Я😀 echo echo").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Doc", tree.to_json().unwrap())
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
    let annotation = workspace
        .create_annotation(&document.id, 1, anchor, "Retain note")
        .unwrap();
    (
        workspace,
        document,
        Editor::new(tree, Revision(0)),
        annotation.id,
    )
}
fn insert(editor: &mut Editor, at: usize, text: &str) -> ReplayProof {
    editor
        .apply(Transaction {
            base_revision: editor.revision(),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(at),
                    end: ScalarOffset(at),
                },
                expected: String::new(),
                replacement: text.into(),
            }],
        })
        .unwrap()
        .replay_proof()
}
fn save(
    workspace: &Workspace,
    document: &Document,
    editor: &Editor,
    proofs: &[ReplayProof],
) -> Document {
    workspace
        .save_rich_editor_changes(
            &document.id,
            document.revision,
            proofs,
            editor.document().to_json().unwrap(),
        )
        .unwrap()
}

#[test]
fn debounced_forward_typing_replays_proofs_and_persists_one_canonical_revision() {
    let (workspace, document, mut editor, id) = fixture();
    let first = insert(&mut editor, 0, "X");
    let second = insert(&mut editor, 1, "Y");
    let saved = save(&workspace, &document, &editor, &[first, second]);
    assert_eq!(saved.revision, 2);
    assert_eq!(workspace.rich_revisions(&document.id).unwrap().len(), 2);
    let note = workspace.annotation(&id).unwrap();
    assert_eq!(note.anchor.start, ScalarOffset(10));
    assert_eq!(note.anchor_status, AnchorStatus::Attached);
}

#[test]
fn native_undo_and_redo_restore_exact_attachments_across_autosave_boundaries() {
    let (workspace, document, mut editor, id) = fixture();
    let forward = insert(&mut editor, 0, "X");
    let saved = save(&workspace, &document, &editor, &[forward]);
    let note = workspace.annotation(&id).unwrap();
    workspace
        .update_annotation(
            &id,
            2,
            note.revision,
            AnnotationPatch {
                note: Some("Later author text".into()),
                status: Some(AnnotationStatus::Resolved),
            },
        )
        .unwrap();
    let undo = editor.undo(editor.revision()).unwrap().replay_proof();
    let undone = save(&workspace, &saved, &editor, &[undo]);
    let note = workspace.annotation(&id).unwrap();
    assert_eq!(note.anchor.start, ScalarOffset(8));
    assert_eq!(note.anchor_status, AnchorStatus::Attached);
    assert_eq!(note.note, "Later author text");
    assert_eq!(note.status, AnnotationStatus::Resolved);
    let redo = editor.redo(editor.revision()).unwrap().replay_proof();
    save(&workspace, &undone, &editor, &[redo]);
    assert_eq!(
        workspace.annotation(&id).unwrap().anchor.start,
        ScalarOffset(9)
    );
}

#[test]
fn unsaved_delete_and_undo_use_receipt_identity_even_inside_one_batch() {
    let (workspace, document, mut editor, id) = fixture();
    let deletion = editor
        .apply(Transaction {
            base_revision: Revision(0),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(8),
                    end: ScalarOffset(12),
                },
                expected: "echo".into(),
                replacement: String::new(),
            }],
        })
        .unwrap()
        .replay_proof();
    let undo = editor.undo(editor.revision()).unwrap().replay_proof();
    save(&workspace, &document, &editor, &[deletion, undo]);
    let note = workspace.annotation(&id).unwrap();
    assert_eq!(note.anchor_status, AnchorStatus::Attached);
    assert_eq!(note.anchor.start, ScalarOffset(8));
    assert_eq!(note.anchor.exact_text, "echo");
}

#[test]
fn forged_public_change_maps_and_snapshots_cannot_retarget_a_note() {
    let (workspace, document, mut editor, id) = fixture();
    let mut change = editor
        .apply(Transaction {
            base_revision: Revision(0),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(0),
                    end: ScalarOffset(0),
                },
                expected: String::new(),
                replacement: "X".into(),
            }],
        })
        .unwrap();
    change.mappings.clear();
    change.before = RichTree::plain("p", "Wrong").unwrap();
    change.after = RichTree::plain("p", "Forged").unwrap();
    save(&workspace, &document, &editor, &[change.replay_proof()]);
    assert_eq!(
        workspace.annotation(&id).unwrap().anchor.start,
        ScalarOffset(9)
    );
}

#[test]
fn unknown_undo_receipt_is_refused_instead_of_matching_identical_json() {
    let (workspace, document, _, id) = fixture();
    let original = workspace.annotation(&id).unwrap();
    let mut outsider = Editor::new(RichTree::plain("p", "Other history").unwrap(), Revision(0));
    let target = outsider
        .document()
        .node_ref(&craft_editor_model::NodePath::root())
        .unwrap();
    outsider
        .apply(Transaction {
            base_revision: Revision(0),
            commands: vec![Command::ReplaceNode {
                target,
                replacement: Box::new(
                    RichTree::from_json(document.rich_json.clone().unwrap())
                        .unwrap()
                        .root()
                        .clone(),
                ),
            }],
        })
        .unwrap();
    let undo = outsider.undo(outsider.revision()).unwrap().replay_proof();
    assert!(
        workspace
            .save_rich_editor_changes(
                &document.id,
                1,
                &[undo],
                outsider.document().to_json().unwrap()
            )
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.annotation(&id).unwrap(), original);
}

#[test]
fn noncontiguous_or_continued_snapshot_journals_leave_drafts_and_notes_unchanged() {
    let (workspace, document, mut editor, id) = fixture();
    let first = insert(&mut editor, 0, "X");
    let _missing = insert(&mut editor, 1, "Y");
    let third = insert(&mut editor, 2, "Z");
    let draft = workspace
        .save_rich_draft(&document.id, 1, None, editor.document().to_json().unwrap())
        .unwrap();
    assert!(
        workspace
            .save_rich_editor_changes(
                &document.id,
                1,
                &[first.clone(), third],
                editor.document().to_json().unwrap()
            )
            .is_err()
    );
    assert!(
        workspace
            .save_rich_editor_changes(
                &document.id,
                1,
                &[first],
                editor.document().to_json().unwrap()
            )
            .is_err()
    );
    assert_eq!(workspace.rich_draft(&document.id).unwrap().unwrap(), draft);
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.annotation(&id).unwrap().revision, 1);
}

#[test]
fn external_revision_changes_invalidate_native_receipts_even_when_json_returns_identically() {
    let (workspace, document, mut editor, _) = fixture();
    let forward = insert(&mut editor, 0, "X");
    let saved = save(&workspace, &document, &editor, &[forward]);
    let original_content = saved.rich_json.clone().unwrap();
    let mut other: Value = original_content.clone();
    other["content"][0]["attrs"]["vendor"] = json!("external");
    workspace
        .save_rich_document(&document.id, 2, other)
        .unwrap();
    workspace
        .save_rich_document(&document.id, 3, original_content)
        .unwrap();
    let undo = editor.undo(editor.revision()).unwrap().replay_proof();
    assert!(
        workspace
            .save_rich_editor_changes(
                &document.id,
                4,
                &[undo],
                editor.document().to_json().unwrap()
            )
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap().revision, 4);
}

#[test]
fn saved_deletion_undo_and_redo_restore_attachment_without_searching_the_other_quote() {
    let (workspace, document, mut editor, id) = fixture();
    let deletion = editor
        .apply(Transaction {
            base_revision: Revision(0),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(8),
                    end: ScalarOffset(12),
                },
                expected: "echo".into(),
                replacement: String::new(),
            }],
        })
        .unwrap()
        .replay_proof();
    let saved = save(&workspace, &document, &editor, &[deletion]);
    assert_eq!(
        workspace.annotation(&id).unwrap().anchor_status,
        AnchorStatus::Detached
    );
    let undo = editor.undo(editor.revision()).unwrap().replay_proof();
    let undone = save(&workspace, &saved, &editor, &[undo]);
    assert_eq!(
        workspace.annotation(&id).unwrap().anchor_status,
        AnchorStatus::Attached
    );
    assert_eq!(
        workspace.annotation(&id).unwrap().anchor.start,
        ScalarOffset(8)
    );
    let redo = editor.redo(editor.revision()).unwrap().replay_proof();
    save(&workspace, &undone, &editor, &[redo]);
    assert_eq!(
        workspace.annotation(&id).unwrap().anchor_status,
        AnchorStatus::Detached
    );
    assert_eq!(
        workspace.annotation(&id).unwrap().anchor.start,
        ScalarOffset(8)
    );
}

#[test]
fn failed_native_commit_does_not_acknowledge_receipts_and_can_retry_the_same_proofs() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    let project = workspace.create_project("Replay rollback").unwrap();
    let tree = RichTree::plain("p", "Я😀 echo echo").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Doc", tree.to_json().unwrap())
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
    let annotation = workspace
        .create_annotation(&document.id, 1, anchor, "Retain")
        .unwrap();
    let mut editor = Editor::new(tree, Revision(0));
    let proof = insert(&mut editor, 0, "X");
    let raw = rusqlite::Connection::open(path).unwrap();
    raw.execute_batch("CREATE TRIGGER reject_native_mapping BEFORE INSERT ON document_annotation_history WHEN NEW.source='mapping' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(
        workspace
            .save_rich_editor_changes(
                &document.id,
                1,
                std::slice::from_ref(&proof),
                editor.document().to_json().unwrap()
            )
            .is_err()
    );
    assert_eq!(workspace.annotation(&annotation.id).unwrap(), annotation);
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    raw.execute_batch("DROP TRIGGER reject_native_mapping;")
        .unwrap();
    let saved = save(&workspace, &document, &editor, &[proof]);
    assert_eq!(saved.revision, 2);
    let undo = editor.undo(editor.revision()).unwrap().replay_proof();
    save(&workspace, &saved, &editor, &[undo]);
    assert_eq!(
        workspace.annotation(&annotation.id).unwrap().anchor.start,
        ScalarOffset(8)
    );
}
