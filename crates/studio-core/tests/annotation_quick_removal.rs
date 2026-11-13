//! Local correction removal remains atomic, recoverable and separate from project vocabulary.
use craft_editor_model::{Document as RichTree, ScalarOffset};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use studio_core::{
    AnchorStatus, AnnotationPatch, Document, PassageAnnotation, ProjectLabelColor, Workspace,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn fixture(
    workspace: &Workspace,
    body: &str,
    tags: &[&str],
) -> TestResult<(Document, PassageAnnotation)> {
    let project = workspace.create_project("The Last Light")?;
    let document = workspace.create_rich_document(
        &project.id,
        "Night watch",
        RichTree::plain("p", "The light remains.")?.to_json()?,
    )?;
    let anchor = workspace.capture_annotation_anchor(
        &document.id,
        1,
        "p",
        ScalarOffset(4),
        ScalarOffset(9),
        "light",
    )?;
    let tags = tags.iter().map(|tag| (*tag).to_owned()).collect::<Vec<_>>();
    let note = workspace.create_tagged_annotation(&document.id, 1, anchor, body, &tags)?;
    Ok((document, note))
}

#[test]
fn deleting_with_a_pending_body_records_one_recoverable_revision_and_ignores_unrelated_catalog_change()
-> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("quick-removal.sqlite");
    let workspace = Workspace::open(&path)?;
    let (document, note) = fixture(&workspace, "Initial thought", &["Asya"])?;
    let stale_catalog = workspace.project_labels(&document.project_id)?;
    workspace.create_project_label(&stale_catalog, "Unused", ProjectLabelColor::Gray)?;
    let raw = Connection::open(&path)?;
    let mut state = serde_json::to_value(&note)?;
    state["vendor"] = json!({"keep":"unknown note metadata"});
    state["anchor"]["vendor"] = json!([1, null, "context"]);
    raw.execute(
        "UPDATE document_annotations SET annotation_json=?1 WHERE id=?2",
        params![state.to_string(), note.id],
    )?;
    let history_before = workspace.annotation_history(&note.id)?;
    let removed = workspace.remove_annotation_with_draft(
        &note.id,
        1,
        1,
        Some("She keeps the log to protect Lev."),
        None,
        Some(&stale_catalog),
    )?;
    assert!(removed.deleted);
    assert_eq!(removed.revision, 2);
    assert_eq!(removed.note, "She keeps the log to protect Lev.");
    assert_eq!(removed.anchor, note.anchor);
    let history = workspace.annotation_history(&note.id)?;
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].source, "delete");
    assert_eq!(history[1].after, history_before[0].after);
    assert_eq!(workspace.document(&document.id)?, document);
    let stored: Value = serde_json::from_str(&raw.query_row(
        "SELECT annotation_json FROM document_annotations WHERE id=?1",
        [&note.id],
        |row| row.get::<_, String>(0),
    )?)?;
    assert_eq!(stored["vendor"], state["vendor"]);
    assert_eq!(stored["anchor"]["vendor"], state["anchor"]["vendor"]);
    assert!(
        workspace
            .remove_annotation_with_draft(&note.id, 1, 2, None, None, None)
            .is_err()
    );
    drop(workspace);
    let workspace = Workspace::open(&path)?;
    assert_eq!(
        workspace.project_annotations(&document.project_id, true)?[0].annotation,
        removed
    );
    let restored = workspace.restore_annotation_with_labels(
        &removed.id,
        1,
        removed.revision,
        &workspace.project_labels(&document.project_id)?,
    )?;
    assert_eq!(restored.id, note.id);
    assert_eq!(restored.note, removed.note);
    assert_eq!(restored.tags, note.tags);
    Ok(())
}

#[test]
fn stale_or_failed_draft_deletion_never_partially_saves_pending_metadata() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("refused.sqlite");
    let workspace = Workspace::open(&path)?;
    let (document, note) = fixture(&workspace, "Original note", &["Asya"])?;
    let catalog = workspace.create_project_label(
        &workspace.project_labels(&document.project_id)?,
        "Chronology",
        ProjectLabelColor::Amber,
    )?;
    let tags = vec!["Chronology".into()];
    assert!(
        workspace
            .remove_annotation_with_draft(&note.id, 1, 1, Some("Pending body"), Some(&tags), None)
            .is_err()
    );
    workspace.set_project_label_color(&catalog, "Chronology", ProjectLabelColor::Violet)?;
    assert!(
        workspace
            .remove_annotation_with_draft(
                &note.id,
                1,
                1,
                Some("Pending body"),
                Some(&tags),
                Some(&catalog)
            )
            .is_err()
    );
    let current = workspace.project_labels(&document.project_id)?;
    assert!(
        workspace
            .remove_annotation_with_draft(
                &note.id,
                2,
                1,
                Some("Pending body"),
                Some(&tags),
                Some(&current)
            )
            .is_err()
    );
    assert!(
        workspace
            .remove_annotation_with_draft(
                &note.id,
                1,
                2,
                Some("Pending body"),
                Some(&tags),
                Some(&current)
            )
            .is_err()
    );
    let raw = Connection::open(&path)?;
    raw.execute_batch("CREATE TRIGGER reject_quick_remove BEFORE UPDATE ON document_annotations BEGIN SELECT RAISE(ABORT,'synthetic annotation write failure'); END;")?;
    assert!(
        workspace
            .remove_annotation_with_draft(
                &note.id,
                1,
                1,
                Some("Pending body"),
                Some(&tags),
                Some(&current)
            )
            .is_err()
    );
    assert_eq!(workspace.annotation(&note.id)?, note);
    assert_eq!(workspace.annotation_history(&note.id)?.len(), 1);
    assert_eq!(workspace.project_labels(&document.project_id)?, current);
    assert_eq!(workspace.document(&document.id)?, document);
    Ok(())
}

#[test]
fn local_label_removal_preserves_other_usages_and_undo_retains_latest_authored_body_and_order()
-> TestResult {
    let workspace = Workspace::memory()?;
    let (document, note) = fixture(
        &workspace,
        "Initial note",
        &["Asya", "Chronology", "Question"],
    )?;
    let other = workspace.create_tagged_annotation(
        &document.id,
        1,
        note.anchor.clone(),
        "Other use",
        &note.tags,
    )?;
    let catalog = workspace.project_labels(&document.project_id)?;
    let removed = workspace.remove_annotation_label_with_draft(
        &note.id,
        1,
        1,
        "Asya",
        Some("Latest authored body"),
        None,
        &catalog,
    )?;
    assert!(!removed.deleted);
    assert_eq!(removed.tags, ["Chronology", "Question"]);
    assert_eq!(removed.note, "Latest authored body");
    assert_eq!(workspace.annotation(&other.id)?, other);
    assert_eq!(workspace.project_labels(&document.project_id)?, catalog);
    assert!(
        workspace
            .remove_annotation_label_with_draft(
                &removed.id,
                1,
                removed.revision,
                "Asya",
                None,
                None,
                &catalog
            )
            .is_err()
    );
    let restored = workspace.update_annotation_markup_with_labels(
        &removed.id,
        1,
        removed.revision,
        AnnotationPatch::default(),
        Some(note.tags.clone()),
        &catalog,
    )?;
    assert_eq!(restored.tags, note.tags);
    assert_eq!(restored.note, removed.note);
    assert!(
        workspace
            .update_annotation_markup_with_labels(
                &removed.id,
                1,
                removed.revision,
                AnnotationPatch::default(),
                Some(note.tags),
                &catalog
            )
            .is_err()
    );
    assert_eq!(workspace.document(&document.id)?, document);
    Ok(())
}

#[test]
fn removing_the_last_drafted_label_keeps_its_latest_assignment_for_durable_restore() -> TestResult {
    let workspace = Workspace::memory()?;
    let (document, note) = fixture(&workspace, "", &["Asya"])?;
    let catalog = workspace.create_project_label(
        &workspace.project_labels(&document.project_id)?,
        "Chronology",
        ProjectLabelColor::Amber,
    )?;
    let tags = vec!["Chronology".into()];
    let removed = workspace.remove_annotation_label_with_draft(
        &note.id,
        1,
        1,
        "Chronology",
        Some(""),
        Some(&tags),
        &catalog,
    )?;
    assert!(removed.deleted);
    assert_eq!(removed.tags, tags);
    assert!(workspace.annotations(&document.id)?.is_empty());
    assert_eq!(workspace.project_labels(&document.project_id)?, catalog);
    let restored =
        workspace.restore_annotation_with_labels(&removed.id, 1, removed.revision, &catalog)?;
    assert!(!restored.deleted);
    assert_eq!(restored.tags, ["Chronology"]);
    assert_eq!(restored.anchor, note.anchor);
    assert_eq!(
        workspace.project_labels(&document.project_id)?.labels()[1].color,
        ProjectLabelColor::Amber
    );
    Ok(())
}

#[test]
fn restoring_after_manuscript_revision_preserves_identity_as_detached_without_retargeting()
-> TestResult {
    let workspace = Workspace::memory()?;
    let (document, note) = fixture(&workspace, "Revise the light", &["Asya"])?;
    let catalog = workspace.project_labels(&document.project_id)?;
    let removed = workspace.remove_annotation_with_draft(
        &note.id,
        1,
        note.revision,
        Some("Latest explanation"),
        None,
        None,
    )?;
    let changed = workspace.save_rich_document(
        &document.id,
        1,
        RichTree::plain("p", "Only darkness remains. Later, the light returns.")?.to_json()?,
    )?;
    assert!(
        workspace
            .restore_annotation_with_labels(&removed.id, 1, removed.revision, &catalog)
            .is_err()
    );
    let restored = workspace.restore_annotation_with_labels(
        &removed.id,
        changed.revision,
        removed.revision,
        &catalog,
    )?;
    assert_eq!(restored.id, note.id);
    assert_eq!(restored.note, removed.note);
    assert_eq!(restored.anchor, note.anchor);
    assert_eq!(restored.anchor_status, AnchorStatus::Detached);
    assert!(
        workspace
            .restore_annotation_with_labels(
                &removed.id,
                changed.revision,
                removed.revision,
                &catalog
            )
            .is_err()
    );
    assert_eq!(workspace.annotation(&restored.id)?, restored);
    assert_eq!(workspace.document(&document.id)?, changed);
    Ok(())
}
