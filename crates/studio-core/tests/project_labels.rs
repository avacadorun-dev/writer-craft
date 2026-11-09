//! Named project labels, guarded author operations, and compatible defaults use authored fixtures.
use craft_editor_model::{Document as RichTree, ScalarOffset};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use studio_core::{AnnotationPatch, Document, PassageAnnotation, ProjectLabelColor, Workspace};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn scene(
    workspace: &Workspace,
    project: &str,
) -> Result<Document, Box<dyn std::error::Error + Send + Sync>> {
    Ok(workspace.create_rich_document(
        project,
        "Night watch",
        RichTree::plain("p", "The light remains.")?.to_json()?,
    )?)
}

fn mark(
    workspace: &Workspace,
    document: &Document,
    label: &str,
) -> studio_core::Result<PassageAnnotation> {
    let anchor = workspace.capture_annotation_anchor(
        &document.id,
        document.revision,
        "p",
        ScalarOffset(4),
        ScalarOffset(9),
        "light",
    )?;
    workspace.create_tagged_annotation(&document.id, document.revision, anchor, "", &[label.into()])
}

fn annotation_bytes(connection: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut statement =
        connection.prepare("SELECT annotation_json FROM document_annotations ORDER BY id")?;
    let mut values = statement
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    let mut statement = connection.prepare("SELECT json_array(annotation_id,annotation_revision,document_revision,source,before_json,after_json) FROM document_annotation_history ORDER BY id")?;
    values.extend(
        statement
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?,
    );
    Ok(values)
}

#[test]
fn unused_catalog_crud_is_guarded_and_color_changes_do_not_touch_writing_or_annotations()
-> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("labels.sqlite");
    let workspace = Workspace::open(&path)?;
    let project = workspace.create_project("The Last Light")?;
    let empty = workspace.project_labels(&project.id)?;
    let first = workspace.create_project_label(&empty, "Promise", ProjectLabelColor::Violet)?;
    assert_eq!(first.labels()[0].name, "Promise");
    assert_eq!(
        workspace
            .project_tag_snapshot(&project.id, "Promise")?
            .count(),
        0
    );
    assert!(
        workspace
            .create_project_label(&empty, "Stale", ProjectLabelColor::Gray)
            .is_err()
    );
    let catalog = workspace.create_project_label(&first, "Question", ProjectLabelColor::Amber)?;
    let unused = workspace.project_tag_snapshot(&project.id, "Question")?;
    assert!(workspace.rename_project_tag(&unused, "Promise").is_err());
    assert_eq!(workspace.rename_project_tag(&unused, "Unanswered")?, 0);
    assert!(
        workspace
            .set_project_label_color(&catalog, "Promise", ProjectLabelColor::Rose)
            .is_err()
    );
    let document = scene(&workspace, &project.id)?;
    let note = mark(&workspace, &document, "Promise")?;
    let raw = Connection::open(&path)?;
    let before = annotation_bytes(&raw)?;
    let history = workspace.rich_revisions(&document.id)?;
    let catalog = workspace.project_labels(&project.id)?;
    let stale_tag = workspace.project_tag_snapshot(&project.id, "Promise")?;
    let colored =
        workspace.set_project_label_color(&catalog, "Promise", ProjectLabelColor::Rose)?;
    assert!(colored.revision() > catalog.revision());
    assert!(workspace.remove_project_tag(&stale_tag).is_err());
    assert_eq!(annotation_bytes(&raw)?, before);
    assert_eq!(workspace.annotation(&note.id)?, note);
    assert_eq!(workspace.document(&document.id)?, document);
    assert_eq!(workspace.rich_revisions(&document.id)?, history);
    drop(workspace);
    let reopened = Workspace::open(&path)?;
    assert_eq!(reopened.project_labels(&project.id)?, colored);
    Ok(())
}

#[test]
fn rename_retire_and_restore_keep_deleted_names_colors_unknown_fields_and_original_history()
-> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("retained.sqlite");
    let workspace = Workspace::open(&path)?;
    let project = workspace.create_project("The Last Light")?;
    workspace.create_project_label(
        &workspace.project_labels(&project.id)?,
        "Witness",
        ProjectLabelColor::Violet,
    )?;
    let document = scene(&workspace, &project.id)?;
    let active = mark(&workspace, &document, "Witness")?;
    let deleted = mark(&workspace, &document, "Witness")?;
    workspace.delete_annotation(&deleted.id, document.revision, deleted.revision)?;
    let other = workspace.create_project("Other manuscript")?;
    let other_document = scene(&workspace, &other.id)?;
    let other_note = mark(&workspace, &other_document, "Witness")?;
    let raw = Connection::open(&path)?;
    let mut state = serde_json::to_value(&active)?;
    state["vendor"] = json!({"keep":[1,null,"original"]});
    let anchor = state["anchor"].as_object_mut().ok_or("anchor")?;
    let block = anchor.remove("block_id").ok_or("block")?;
    let exact = anchor.remove("exact_text").ok_or("quote")?;
    anchor.insert("blockId".into(), block);
    anchor.insert("exactText".into(), exact);
    anchor.insert("vendor".into(), json!({"context":"retained"}));
    raw.execute(
        "UPDATE document_annotations SET annotation_json=?1 WHERE id=?2",
        params![state.to_string(), active.id],
    )?;
    let mut catalog: Value = serde_json::from_str(&raw.query_row(
        "SELECT raw_json FROM project_label_catalog WHERE project_id=?1",
        [&project.id],
        |row| row.get::<_, String>(0),
    )?)?;
    catalog["vendor"] = json!({"source":"kept"});
    catalog["labels"][0]["vendor"] = json!([1, null, "retained"]);
    raw.execute(
        "UPDATE project_label_catalog SET raw_json=?1 WHERE project_id=?2",
        params![catalog.to_string(), project.id],
    )?;
    let original_events: Vec<String> = raw
        .prepare(
            "SELECT after_json FROM document_annotation_history WHERE annotation_id=?1 ORDER BY id",
        )?
        .query_map([&active.id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let snapshot = workspace.project_tag_snapshot(&project.id, "Witness")?;
    assert_eq!(
        (
            snapshot.count(),
            snapshot.deleted_count(),
            snapshot.tag_only_count()
        ),
        (1, 1, 1)
    );
    assert_eq!(workspace.rename_project_tag(&snapshot, "Testimony")?, 1);
    assert_eq!(workspace.annotation(&active.id)?.tags, ["Testimony"]);
    assert_eq!(workspace.annotation(&other_note.id)?, other_note);
    let deleted = workspace
        .project_annotations(&project.id, true)?
        .into_iter()
        .find(|row| row.annotation.id == deleted.id)
        .ok_or("deleted note")?
        .annotation;
    assert!(deleted.deleted);
    assert_eq!(deleted.tags, ["Testimony"]);
    let updated: Value = serde_json::from_str(&raw.query_row(
        "SELECT annotation_json FROM document_annotations WHERE id=?1",
        [&active.id],
        |row| row.get::<_, String>(0),
    )?)?;
    assert_eq!(updated["vendor"], state["vendor"]);
    assert_eq!(updated["anchor"]["vendor"], state["anchor"]["vendor"]);
    assert!(updated["anchor"].get("blockId").is_none());
    let events: Vec<String> = raw
        .prepare(
            "SELECT after_json FROM document_annotation_history WHERE annotation_id=?1 ORDER BY id",
        )?
        .query_map([&active.id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    assert_eq!(&events[..original_events.len()], original_events);
    workspace.remove_project_tag(&workspace.project_tag_snapshot(&project.id, "Testimony")?)?;
    let retired = workspace.project_labels(&project.id)?;
    assert!(retired.labels().is_empty());
    assert_eq!(retired.retired_labels()[0].color, ProjectLabelColor::Violet);
    assert!(
        workspace
            .create_project_label(&retired, "Testimony", ProjectLabelColor::Gray)
            .is_err()
    );
    let restored = workspace.restore_annotation_with_labels(
        &deleted.id,
        document.revision,
        deleted.revision,
        &retired,
    )?;
    assert_eq!(restored.tags, ["Testimony"]);
    assert_eq!(
        workspace.project_labels(&project.id)?.labels()[0].color,
        ProjectLabelColor::Violet
    );
    let retained: Value = serde_json::from_str(&raw.query_row(
        "SELECT raw_json FROM project_label_catalog WHERE project_id=?1",
        [&project.id],
        |row| row.get::<_, String>(0),
    )?)?;
    assert_eq!(retained["vendor"], catalog["vendor"]);
    assert_eq!(
        retained["labels"][0]["vendor"],
        catalog["labels"][0]["vendor"]
    );
    assert_eq!(workspace.document(&document.id)?, document);
    Ok(())
}

#[test]
fn old_schema_defaults_exact_names_without_rewriting_notes_history_or_materializing_catalog()
-> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("legacy.sqlite");
    let workspace = Workspace::open(&path)?;
    let project = workspace.create_project("Existing notes")?;
    let document = scene(&workspace, &project.id)?;
    for name in ["Ася", "ася", "  Ася  "] {
        mark(&workspace, &document, name)?;
    }
    drop(workspace);
    let raw = Connection::open(&path)?;
    raw.execute_batch("DROP TABLE project_label_catalog; PRAGMA user_version=10;")?;
    let before = annotation_bytes(&raw)?;
    let workspace = Workspace::open(&path)?;
    let labels = workspace.project_labels(&project.id)?;
    assert_eq!(
        labels
            .labels()
            .iter()
            .map(|label| label.name.as_str())
            .collect::<Vec<_>>(),
        ["  Ася  ", "Ася", "ася"]
    );
    assert!(
        labels
            .labels()
            .iter()
            .all(|label| label.color == ProjectLabelColor::Blue)
    );
    assert_eq!(annotation_bytes(&raw)?, before);
    assert_eq!(
        raw.query_row("SELECT count(*) FROM project_label_catalog", [], |row| row
            .get::<_, i64>(
            0
        ))?,
        0
    );
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))?,
        11
    );
    let note = workspace
        .project_annotations(&project.id, false)?
        .into_iter()
        .find(|row| row.annotation.tags == ["Ася"])
        .ok_or("legacy mark")?
        .annotation;
    workspace.update_annotation_markup(
        &note.id,
        document.revision,
        note.revision,
        AnnotationPatch {
            note: Some("Keep the note without its label".into()),
            status: None,
        },
        Some(vec![]),
    )?;
    assert_eq!(workspace.project_labels(&project.id)?, labels);
    Ok(())
}

#[test]
fn stale_picker_assignment_and_restore_disclosure_refuse_without_partial_changes() -> TestResult {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Story")?;
    let document = scene(&workspace, &project.id)?;
    let old = workspace.create_project_label(
        &workspace.project_labels(&project.id)?,
        "A",
        ProjectLabelColor::Violet,
    )?;
    let anchor = workspace.capture_annotation_anchor(
        &document.id,
        1,
        "p",
        ScalarOffset(4),
        ScalarOffset(9),
        "light",
    )?;
    let note = workspace.create_annotation(&document.id, 1, anchor.clone(), "Existing thought")?;
    workspace.rename_project_tag(&workspace.project_tag_snapshot(&project.id, "A")?, "B")?;
    assert!(
        workspace
            .create_tagged_annotation_with_labels(
                &document.id,
                1,
                anchor.clone(),
                "",
                &["A".into()],
                &old
            )
            .is_err()
    );
    assert!(
        workspace
            .update_annotation_markup_with_labels(
                &note.id,
                1,
                1,
                AnnotationPatch::default(),
                Some(vec!["A".into()]),
                &old
            )
            .is_err()
    );
    assert_eq!(workspace.annotation(&note.id)?, note);
    let current = workspace.project_labels(&project.id)?;
    assert_eq!(current.labels()[0].name, "B");
    let foreign = workspace.create_project("Foreign")?;
    let foreign = workspace.create_project_label(
        &workspace.project_labels(&foreign.id)?,
        "B",
        ProjectLabelColor::Blue,
    )?;
    assert!(
        workspace
            .create_tagged_annotation_with_labels(
                &document.id,
                1,
                anchor,
                "",
                &["B".into()],
                &foreign
            )
            .is_err()
    );
    let tagged = mark(&workspace, &document, "B")?;
    workspace.delete_annotation(&tagged.id, 1, 1)?;
    let deleted = workspace
        .project_annotations(&project.id, true)?
        .into_iter()
        .find(|row| row.annotation.id == tagged.id)
        .ok_or("deleted")?
        .annotation;
    workspace.remove_project_tag(&workspace.project_tag_snapshot(&project.id, "B")?)?;
    assert!(
        workspace
            .restore_annotation_with_labels(&deleted.id, 1, deleted.revision, &current)
            .is_err()
    );
    let retired = workspace.project_labels(&project.id)?;
    assert_eq!(retired.retired_labels()[0].color, ProjectLabelColor::Violet);
    assert_eq!(workspace.annotation_history(&deleted.id)?.len(), 2);
    let restored = workspace.restore_project_label(&retired, "B")?;
    assert!(restored.retired_labels().is_empty());
    assert!(workspace.annotation(&deleted.id).is_err());
    workspace.restore_annotation_with_labels(&deleted.id, 1, deleted.revision, &restored)?;
    assert_eq!(workspace.annotation(&deleted.id)?.tags, ["B"]);
    Ok(())
}
