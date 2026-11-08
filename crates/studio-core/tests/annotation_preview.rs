use craft_editor_model::{
    Command, Document as RichTree, Editor, ReplayProof, Revision, ScalarOffset, TextRange,
    Transaction,
};
use studio_core::{AnchorStatus, AnnotationPatch, AnnotationStatus, Document, Workspace};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture() -> TestResult<(Workspace, Document, Editor, String)> {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Synthetic annotation preview")?;
    let tree = RichTree::plain("p", "Я😀 echo echo")?;
    let document = workspace.create_rich_document(&project.id, "Draft", tree.to_json()?)?;
    let anchor = workspace.capture_annotation_anchor(
        &document.id,
        document.revision,
        "p",
        ScalarOffset(8),
        ScalarOffset(12),
        "echo",
    )?;
    let note = workspace.create_annotation(
        &document.id,
        document.revision,
        anchor,
        "Review the second occurrence",
    )?;
    Ok((workspace, document, Editor::new(tree, Revision(0)), note.id))
}
fn replace(
    editor: &mut Editor,
    start: usize,
    end: usize,
    expected: &str,
    replacement: &str,
) -> TestResult<ReplayProof> {
    Ok(editor
        .apply(Transaction {
            base_revision: editor.revision(),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(start),
                    end: ScalarOffset(end),
                },
                expected: expected.into(),
                replacement: replacement.into(),
            }],
        })?
        .replay_proof())
}

#[test]
fn unsaved_preview_maps_exact_unicode_anchor_without_advancing_storage_or_receipts() -> TestResult {
    let (workspace, document, mut editor, id) = fixture()?;
    let original = workspace.annotation(&id)?;
    let proof = replace(&mut editor, 0, 0, "", "New ")?;
    let preview = workspace.preview_rich_editor_annotations(
        &document.id,
        document.revision,
        std::slice::from_ref(&proof),
        editor.document().to_json()?,
    )?;
    assert_eq!(preview[0].anchor.start, ScalarOffset(12));
    assert_eq!(preview[0].anchor_status, AnchorStatus::Attached);
    assert_eq!(preview[0].revision, original.revision);
    assert_eq!(workspace.annotation(&id)?, original);
    assert_eq!(workspace.document(&document.id)?, document);
    assert_eq!(workspace.annotation_history(&id)?.len(), 1);
    let saved = workspace.save_rich_editor_changes(
        &document.id,
        document.revision,
        &[proof],
        editor.document().to_json()?,
    )?;
    assert_eq!(saved.revision, document.revision + 1);
    assert_eq!(
        workspace.annotation(&id)?.anchor.start,
        preview[0].anchor.start
    );
    Ok(())
}

#[test]
fn unsaved_delete_and_undo_restore_marks_in_preview_without_writing_note_history() -> TestResult {
    let (workspace, document, mut editor, id) = fixture()?;
    let original = workspace.annotation(&id)?;
    let deletion = replace(&mut editor, 8, 12, "echo", "")?;
    let detached = workspace.preview_rich_editor_annotations(
        &document.id,
        document.revision,
        std::slice::from_ref(&deletion),
        editor.document().to_json()?,
    )?;
    assert_eq!(detached[0].anchor_status, AnchorStatus::Detached);
    let undo = editor.undo(editor.revision())?.replay_proof();
    let restored = workspace.preview_rich_editor_annotations(
        &document.id,
        document.revision,
        &[deletion, undo],
        editor.document().to_json()?,
    )?;
    assert_eq!(restored[0], original);
    assert_eq!(workspace.annotation_history(&id)?.len(), 1);
    assert_eq!(workspace.annotation(&id)?, original);
    Ok(())
}

#[test]
fn preview_rejects_unverified_snapshot_and_stale_document_revision_without_mutation() -> TestResult
{
    let (workspace, document, mut editor, id) = fixture()?;
    let original = workspace.annotation(&id)?;
    let proof = replace(&mut editor, 0, 0, "", "X")?;
    assert!(
        workspace
            .preview_rich_editor_annotations(
                &document.id,
                document.revision,
                &[],
                editor.document().to_json()?
            )
            .is_err()
    );
    assert!(
        workspace
            .preview_rich_editor_annotations(
                &document.id,
                document.revision + 1,
                &[proof],
                editor.document().to_json()?
            )
            .is_err()
    );
    assert_eq!(workspace.annotation(&id)?, original);
    assert_eq!(workspace.document(&document.id)?, document);
    Ok(())
}

#[test]
fn preview_undo_preserves_later_note_text_and_resolved_lifecycle() -> TestResult {
    let (workspace, document, mut editor, id) = fixture()?;
    let proof = replace(&mut editor, 0, 0, "", "X")?;
    let saved = workspace.save_rich_editor_changes(
        &document.id,
        document.revision,
        &[proof],
        editor.document().to_json()?,
    )?;
    let note = workspace.annotation(&id)?;
    let updated = workspace.update_annotation(
        &id,
        saved.revision,
        note.revision,
        AnnotationPatch {
            note: Some("Later metadata".into()),
            status: Some(AnnotationStatus::Resolved),
        },
    )?;
    let undo = editor.undo(editor.revision())?.replay_proof();
    let preview = workspace.preview_rich_editor_annotations(
        &document.id,
        saved.revision,
        &[undo],
        editor.document().to_json()?,
    )?;
    assert_eq!(preview[0].anchor.start, ScalarOffset(8));
    assert_eq!(preview[0].note, "Later metadata");
    assert_eq!(preview[0].status, AnnotationStatus::Resolved);
    assert_eq!(workspace.annotation(&id)?, updated);
    Ok(())
}
