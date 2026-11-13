//! Exact author markup and its lifecycle; these fixtures complement native task acceptance.
use std::io::Read;

use craft_editor_model::{Document as RichTree, ScalarOffset};
use studio_core::{
    AnchorStatus, AnnotationPatch, ImportAssetSource, PassageAnnotation, ProjectAttachment,
    ProjectImportWorker, ProjectTransferCancellation, ValidatedPortableProjectImport, Workspace,
};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn mark(
    workspace: &Workspace,
    document: &studio_core::Document,
    tag: &str,
) -> studio_core::Result<PassageAnnotation> {
    let anchor = workspace.capture_annotation_anchor(
        &document.id,
        document.revision,
        "p",
        ScalarOffset(7),
        ScalarOffset(11),
        "echo",
    )?;
    workspace.create_tagged_annotation(&document.id, document.revision, anchor, "", &[tag.into()])
}

fn scene(
    workspace: &Workspace,
    project: &str,
    title: &str,
) -> Result<studio_core::Document, Box<dyn std::error::Error + Send + Sync>> {
    Ok(workspace.create_rich_document(
        project,
        title,
        RichTree::plain("p", "echo 😀 echo")?.to_json()?,
    )?)
}

#[test]
fn tag_only_markup_preserves_exact_labels_and_second_occurrence_after_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("tags.sqlite");
    let (document, annotation) = {
        let workspace = Workspace::open(&path)?;
        let project = workspace.create_project("The Last Light")?;
        let document = scene(&workspace, &project.id, "Night watch")?;
        let annotation = mark(&workspace, &document, "Ася / Motivation 😀")?;
        let mut old = serde_json::to_value(&annotation)?;
        old.as_object_mut().ok_or("note object")?.remove("tags");
        assert!(
            serde_json::from_value::<PassageAnnotation>(old)?
                .tags
                .is_empty()
        );
        assert_eq!(workspace.document(&document.id)?, document);
        (document, annotation)
    };
    let workspace = Workspace::open(&path)?;
    assert_eq!(workspace.annotation(&annotation.id)?, annotation);
    assert_eq!(annotation.anchor.start, ScalarOffset(7));
    assert_eq!(annotation.tags, ["Ася / Motivation 😀"]);
    assert_eq!(
        workspace.document(&document.id)?.rich_json,
        document.rich_json
    );
    Ok(())
}

#[test]
fn project_tag_rename_is_atomic_and_refuses_a_changed_set_or_implicit_merge() -> TestResult {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Story")?;
    let other = workspace.create_project("Unrelated")?;
    let a = scene(&workspace, &project.id, "Arrival")?;
    let b = scene(&workspace, &project.id, "Departure")?;
    let foreign = scene(&workspace, &other.id, "Arrival")?;
    let first = mark(&workspace, &a, "Motivation")?;
    let foreign_mark = mark(&workspace, &foreign, "Motivation")?;
    let stale = workspace.project_tag_snapshot(&project.id, "Motivation")?;
    let second = mark(&workspace, &b, "Motivation")?;
    assert!(workspace.rename_project_tag(&stale, "Choice").is_err());
    assert_eq!(workspace.annotation(&first.id)?.tags, ["Motivation"]);
    let changed_document = workspace.project_tag_snapshot(&project.id, "Motivation")?;
    let a = workspace.save_rich_document(
        &a.id,
        a.revision,
        RichTree::plain("p", "echo 😀 echo; the choice remains")?.to_json()?,
    )?;
    assert!(
        workspace
            .rename_project_tag(&changed_document, "Choice")
            .is_err()
    );
    assert_eq!(workspace.annotation(&second.id)?.tags, ["Motivation"]);
    let current = workspace.project_tag_snapshot(&project.id, "Motivation")?;
    assert_eq!(workspace.rename_project_tag(&current, "Choice")?, 2);
    assert_eq!(workspace.annotation(&second.id)?.tags, ["Choice"]);
    assert_eq!(workspace.annotation(&foreign_mark.id)?, foreign_mark);
    let existing = mark(&workspace, &a, "choice")?;
    let snapshot = workspace.project_tag_snapshot(&project.id, "Choice")?;
    assert!(workspace.rename_project_tag(&snapshot, "choice").is_err());
    assert_eq!(workspace.annotation(&existing.id)?.tags, ["choice"]);
    assert_eq!(workspace.annotation(&first.id)?.tags, ["Choice"]);
    Ok(())
}

#[test]
fn invalid_label_edits_leave_authored_state_and_history_unchanged() -> TestResult {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Story")?;
    let document = scene(&workspace, &project.id, "Night watch")?;
    let note = mark(&workspace, &document, "  Ася  ")?;
    for labels in [
        vec![],
        vec!["Question".into(), "Question".into()],
        vec![" ".into()],
        vec!["Question\t".into()],
        vec!["😀".repeat(81)],
    ] {
        assert!(
            workspace
                .update_annotation_markup(
                    &note.id,
                    document.revision,
                    note.revision,
                    AnnotationPatch::default(),
                    Some(labels),
                )
                .is_err()
        );
        assert_eq!(workspace.annotation(&note.id)?, note);
    }
    assert_eq!(workspace.annotation_history(&note.id)?.len(), 1);
    let updated = workspace.update_annotation_markup(
        &note.id,
        document.revision,
        note.revision,
        AnnotationPatch {
            note: Some("An intentional note".into()),
            status: None,
        },
        Some(vec!["  Ася  ".into(), "Ася".into(), "ася".into()]),
    )?;
    assert_eq!(updated.tags, ["  Ася  ", "Ася", "ася"]);
    let unlinked = workspace.update_annotation_markup(
        &note.id,
        document.revision,
        updated.revision,
        AnnotationPatch::default(),
        Some(vec![]),
    )?;
    assert!(unlinked.tags.is_empty());
    assert_eq!(unlinked.note, "An intentional note");
    assert_eq!(workspace.document(&document.id)?, document);
    Ok(())
}

#[test]
fn removing_last_tag_tombstones_empty_markup_and_restore_never_guesses_a_changed_quote()
-> TestResult {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Story")?;
    let document = scene(&workspace, &project.id, "Arrival")?;
    let note = mark(&workspace, &document, "Question")?;
    let snapshot = workspace.project_tag_snapshot(&project.id, "Question")?;
    assert_eq!(snapshot.tag_only_count(), 1);
    assert_eq!(workspace.remove_project_tag(&snapshot)?, 1);
    assert!(workspace.annotations(&document.id)?.is_empty());
    let deleted = workspace
        .project_annotations(&project.id, true)?
        .remove(0)
        .annotation;
    assert!(deleted.deleted);
    let changed = workspace.save_rich_document(
        &document.id,
        document.revision,
        RichTree::plain("p", "Different 😀 echo")?.to_json()?,
    )?;
    let restored = workspace.restore_annotation(&deleted.id, changed.revision, deleted.revision)?;
    assert_eq!(restored.id, note.id);
    assert_eq!(restored.anchor, note.anchor);
    assert_eq!(restored.anchor_status, AnchorStatus::Detached);
    assert!(!restored.deleted);
    // Restoring a removed tag-only mark retains the author label from deletion history.
    assert_eq!(restored.tags, ["Question"]);
    Ok(())
}

struct NoAssets;
impl ImportAssetSource for NoAssets {
    fn open<'a>(&'a mut self, _: &ProjectAttachment) -> std::io::Result<Box<dyn Read + 'a>> {
        Err(std::io::Error::other(
            "No assets in this annotation fixture",
        ))
    }
}

#[test]
fn canonical_restore_preserves_tags_note_history_and_logical_passage_without_legacy_loss()
-> TestResult {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Story")?;
    let document = scene(&workspace, &project.id, "Night watch")?;
    let note = mark(&workspace, &document, "Ася")?;
    let updated = workspace.update_annotation_markup(
        &note.id,
        1,
        1,
        AnnotationPatch {
            note: Some("She conceals the log to protect Lev.".into()),
            status: None,
        },
        Some(vec!["Ася".into(), "motivation".into()]),
    )?;
    let snapshot = workspace.project_export_snapshot(&project.id)?;
    assert!(snapshot.requires_canonical_companion());
    let validated =
        ValidatedPortableProjectImport::parse_v1(&snapshot.canonical_manifest_bytes()?)?;
    let mut worker = ProjectImportWorker::memory()?;
    let restored = worker.restore_portable(
        &validated,
        &mut NoAssets,
        &ProjectTransferCancellation::default(),
    )?;
    let restored_document = worker.workspace().resolve_project_resource(
        &restored.project_id,
        "document",
        &document.id,
    )?;
    let notes = worker.workspace().annotations(&restored_document)?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].tags, updated.tags);
    assert_eq!(notes[0].note, updated.note);
    assert_eq!(notes[0].anchor, updated.anchor);
    let history = worker.workspace().annotation_history(&notes[0].id)?;
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].after.tags, ["Ася"]);
    Ok(())
}
