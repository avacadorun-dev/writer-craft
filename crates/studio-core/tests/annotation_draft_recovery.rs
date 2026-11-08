#![expect(
    clippy::unwrap_used,
    reason = "Synthetic durable recovery fixtures fail immediately"
)]
use craft_editor_model::{
    Command, Document as RichTree, Editor, ReplayProof, Revision, ScalarOffset, TextRange,
    Transaction,
};
use studio_core::{
    AnchorStatus, AnnotationPatch, AnnotationStatus, Document, PassageAnnotation, Workspace,
};
fn fixture(workspace: &Workspace) -> (Document, PassageAnnotation, Editor) {
    let project = workspace.create_project("Draft annotations").unwrap();
    let tree = RichTree::plain("p", "Я😀 echo echo tail").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Notes", tree.to_json().unwrap())
        .unwrap();
    let anchor = workspace
        .capture_annotation_anchor(
            &document.id,
            document.revision,
            "p",
            ScalarOffset(8),
            ScalarOffset(12),
            "echo",
        )
        .unwrap();
    let note = workspace
        .create_annotation(
            &document.id,
            document.revision,
            anchor,
            "Keep the second occurrence",
        )
        .unwrap();
    (document, note, Editor::new(tree, Revision(1)))
}
fn replace(editor: &mut Editor, start: usize, end: usize, replacement: &str) -> ReplayProof {
    let text = editor.document().block_text("p").unwrap();
    editor
        .apply(Transaction {
            base_revision: editor.revision(),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(start),
                    end: ScalarOffset(end),
                },
                expected: text.chars().skip(start).take(end - start).collect(),
                replacement: replacement.into(),
            }],
        })
        .unwrap()
        .replay_proof()
}
#[test]
fn proof_derived_checkpoint_survives_restart_and_recovers_exact_unicode_mark_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let (id, note_id, version, expected) = {
        let workspace = Workspace::open(&path).unwrap();
        let (document, note, mut editor) = fixture(&workspace);
        let proof = replace(&mut editor, 10, 10, "東京");
        let expected = editor.document().to_json().unwrap();
        let draft = workspace
            .save_rich_editor_draft(
                &document.id,
                document.revision,
                None,
                &[proof],
                expected.clone(),
            )
            .unwrap();
        assert!(draft.has_annotation_checkpoint);
        assert_eq!(workspace.annotation(&note.id).unwrap(), note);
        assert_eq!(workspace.document(&document.id).unwrap(), document);
        (document.id, note.id, draft.version, expected)
    };
    let workspace = Workspace::open(&path).unwrap();
    assert!(
        workspace
            .rich_draft(&id)
            .unwrap()
            .unwrap()
            .has_annotation_checkpoint
    );
    let recovered = workspace
        .recover_rich_editor_draft(&id, 1, &version)
        .unwrap();
    assert_eq!(recovered.rich_json, Some(expected));
    assert_eq!(recovered.revision, 2);
    let note = workspace.annotation(&note_id).unwrap();
    assert_eq!(note.anchor_status, AnchorStatus::Attached);
    assert_eq!(note.anchor.exact_text, "ec東京ho");
    assert_eq!((note.anchor.start.0, note.anchor.end.0), (8, 14));
    assert_eq!(note.base_revision, 2);
    assert!(workspace.rich_draft(&id).unwrap().is_none());
    assert_eq!(workspace.annotation_history(&note_id).unwrap().len(), 2);
}
#[test]
fn replay_and_optimistic_draft_failures_cannot_replace_an_existing_checkpoint() {
    let workspace = Workspace::memory().unwrap();
    let (document, note, mut editor) = fixture(&workspace);
    let proof = replace(&mut editor, 0, 0, "New ");
    let draft = workspace
        .save_rich_editor_draft(
            &document.id,
            1,
            None,
            std::slice::from_ref(&proof),
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    assert!(
        workspace
            .save_rich_editor_draft(
                &document.id,
                1,
                None,
                std::slice::from_ref(&proof),
                editor.document().to_json().unwrap()
            )
            .is_err()
    );
    assert!(
        workspace
            .save_rich_editor_draft(
                &document.id,
                1,
                Some(&draft.version),
                &[],
                editor.document().to_json().unwrap()
            )
            .is_err()
    );
    assert_eq!(workspace.rich_draft(&document.id).unwrap(), Some(draft));
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.annotation(&note.id).unwrap(), note);
}
#[test]
fn authored_note_and_resolved_status_changed_after_checkpoint_survive_recovery() {
    let workspace = Workspace::memory().unwrap();
    let (document, note, mut editor) = fixture(&workspace);
    let proof = replace(&mut editor, 0, 0, "New ");
    let draft = workspace
        .save_rich_editor_draft(
            &document.id,
            1,
            None,
            &[proof],
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    workspace
        .update_annotation(
            &note.id,
            1,
            note.revision,
            AnnotationPatch {
                note: Some("A newer authored note".into()),
                status: Some(AnnotationStatus::Resolved),
            },
        )
        .unwrap();
    workspace
        .recover_rich_editor_draft(&document.id, 1, &draft.version)
        .unwrap();
    let after = workspace.annotation(&note.id).unwrap();
    assert_eq!(after.note, "A newer authored note");
    assert_eq!(after.status, AnnotationStatus::Resolved);
    assert_eq!(after.anchor_status, AnchorStatus::Attached);
    assert_eq!(after.anchor.start.0, 12);
}
#[test]
fn explicit_reattachment_even_back_to_same_range_refuses_stale_checkpoint() {
    let workspace = Workspace::memory().unwrap();
    let (document, note, mut editor) = fixture(&workspace);
    let proof = replace(&mut editor, 0, 0, "New ");
    let draft = workspace
        .save_rich_editor_draft(
            &document.id,
            1,
            None,
            &[proof],
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    let first = workspace
        .capture_annotation_anchor(
            &document.id,
            1,
            "p",
            ScalarOffset(3),
            ScalarOffset(7),
            "echo",
        )
        .unwrap();
    let reattached = workspace
        .reattach_annotation(&note.id, 1, note.revision, first)
        .unwrap();
    let back = workspace
        .reattach_annotation(&note.id, 1, reattached.revision, note.anchor.clone())
        .unwrap();
    assert!(
        workspace
            .recover_rich_editor_draft(&document.id, 1, &draft.version)
            .is_err()
    );
    assert_eq!(workspace.annotation(&note.id).unwrap(), back);
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.rich_draft(&document.id).unwrap(), Some(draft));
}
#[test]
fn later_note_creation_preserves_both_notes_and_the_unrecovered_draft() {
    let workspace = Workspace::memory().unwrap();
    let (document, note, mut editor) = fixture(&workspace);
    let proof = replace(&mut editor, 0, 0, "New ");
    let draft = workspace
        .save_rich_editor_draft(
            &document.id,
            1,
            None,
            &[proof],
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    let added = workspace
        .create_annotation(&document.id, 1, note.anchor.clone(), "Later note")
        .unwrap();
    assert!(
        workspace
            .recover_rich_editor_draft(&document.id, 1, &draft.version)
            .is_err()
    );
    assert_eq!(workspace.annotations(&document.id).unwrap().len(), 2);
    assert_eq!(workspace.annotation(&added.id).unwrap(), added);
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.rich_draft(&document.id).unwrap(), Some(draft));
}
#[test]
fn unproven_draft_replacement_removes_authority_and_recovers_conservatively() {
    let workspace = Workspace::memory().unwrap();
    let (document, note, mut editor) = fixture(&workspace);
    let proof = replace(&mut editor, 0, 0, "New ");
    let verified = workspace
        .save_rich_editor_draft(
            &document.id,
            1,
            None,
            &[proof],
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    let draft = workspace
        .save_rich_draft(
            &document.id,
            1,
            Some(&verified.version),
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    assert!(!draft.has_annotation_checkpoint);
    workspace
        .recover_rich_editor_draft(&document.id, 1, &draft.version)
        .unwrap();
    let note = workspace.annotation(&note.id).unwrap();
    assert_eq!(note.anchor_status, AnchorStatus::Detached);
    assert_eq!(note.anchor.exact_text, "echo");
}
#[test]
fn deleted_passage_undo_checkpoint_restores_exact_attachments_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("undo.sqlite");
    let (workspace, document, note, mut editor) = {
        let workspace = Workspace::open(&path).unwrap();
        let (document, note, editor) = fixture(&workspace);
        (workspace, document, note, editor)
    };
    let deletion = replace(&mut editor, 8, 12, "");
    let undo = editor.undo(editor.revision()).unwrap().replay_proof();
    let draft = workspace
        .save_rich_editor_draft(
            &document.id,
            1,
            None,
            &[deletion, undo],
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    drop(workspace);
    drop(editor);
    let workspace = Workspace::open(&path).unwrap();
    let recovered = workspace
        .recover_rich_editor_draft(&document.id, 1, &draft.version)
        .unwrap();
    assert_eq!(recovered.rich_json, document.rich_json);
    let after = workspace.annotation(&note.id).unwrap();
    assert_eq!(after.anchor, note.anchor);
    assert_eq!(after.anchor_status, AnchorStatus::Attached);
}
#[test]
fn corrupted_draft_content_binding_cannot_publish_notes_or_history() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("corrupt.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    let (document, note, mut editor) = fixture(&workspace);
    let proof = replace(&mut editor, 0, 0, "New ");
    let draft = workspace
        .save_rich_editor_draft(
            &document.id,
            1,
            None,
            &[proof],
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    let raw = rusqlite::Connection::open(&path).unwrap();
    let changed = RichTree::plain("p", "Different text")
        .unwrap()
        .to_json()
        .unwrap();
    raw.execute(
        "UPDATE rich_working_drafts SET content_json=?2 WHERE document_id=?1",
        rusqlite::params![document.id, serde_json::to_string(&changed).unwrap()],
    )
    .unwrap();
    assert!(
        workspace
            .recover_rich_editor_draft(&document.id, 1, &draft.version)
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.annotation(&note.id).unwrap(), note);
    assert_eq!(workspace.annotation_history(&note.id).unwrap().len(), 1);
    assert_eq!(
        workspace.rich_draft(&document.id).unwrap().unwrap().content,
        changed
    );
}
#[test]
fn schema_five_migrates_without_recreating_existing_asset_tables() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("upgrade.sqlite");
    let id = {
        let workspace = Workspace::open(&path).unwrap();
        let (document, _, _) = fixture(&workspace);
        document.id
    };
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute_batch("DROP TABLE rich_proposals; DROP TABLE project_evidence_references; DROP TABLE project_evidence_objects; DROP TABLE project_annotation_projections; DROP TABLE project_quarantined_recovery; DROP TABLE project_canvas_drafts; DROP TABLE project_canvas_revisions; DROP TABLE project_canvases; DROP TABLE project_asset_metadata; DROP TABLE project_source_records; DROP TABLE project_nodes; DROP TABLE project_resource_aliases; DROP TABLE project_portability; DROP TABLE rich_draft_annotations; PRAGMA user_version=5;")
        .unwrap();
    drop(raw);
    let workspace = Workspace::open(&path).unwrap();
    assert_eq!(workspace.document(&id).unwrap().title, "Notes");
    let raw = rusqlite::Connection::open(&path).unwrap();
    let version: i64 = raw
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 9);
    let tables:i64=raw.query_row("SELECT count(*) FROM sqlite_master WHERE name IN ('project_assets','rich_draft_annotations')",[],|row|row.get(0)).unwrap();
    assert_eq!(tables, 2);
}

#[test]
fn modified_then_deleted_passage_retains_its_last_safe_quote_and_context_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("detached.sqlite");
    let (id, note_id, version, expected) = {
        let workspace = Workspace::open(&path).unwrap();
        let (document, note, mut editor) = fixture(&workspace);
        let insert = replace(&mut editor, 10, 10, "東京");
        let delete = replace(&mut editor, 8, 14, "");
        let proofs = [insert, delete];
        let preview = workspace
            .annotation_preview_snapshot(&document.id, document.revision)
            .unwrap()
            .preview(&proofs, editor.document().to_json().unwrap())
            .unwrap();
        assert_eq!(preview[0].anchor_status, AnchorStatus::Detached);
        assert_eq!(preview[0].anchor.exact_text, "ec東京ho");
        let draft = workspace
            .save_rich_editor_draft(
                &document.id,
                document.revision,
                None,
                &proofs,
                editor.document().to_json().unwrap(),
            )
            .unwrap();
        (document.id, note.id, draft.version, preview[0].clone())
    };
    let workspace = Workspace::open(&path).unwrap();
    workspace
        .recover_rich_editor_draft(&id, 1, &version)
        .unwrap();
    let recovered = workspace.annotation(&note_id).unwrap();
    assert_eq!(recovered.anchor_status, expected.anchor_status);
    assert_eq!(recovered.anchor, expected.anchor);
    assert_eq!(recovered.note, expected.note);
    assert_eq!(recovered.status, expected.status);
}
