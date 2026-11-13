//! A metadata-only version boundary protects tags from older annotation writers.
use craft_editor_model::{Document as RichTree, ScalarOffset};
use rusqlite::{Connection, params};
use studio_core::Workspace;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn annotation_bytes(connection: &Connection) -> rusqlite::Result<(String, String)> {
    Ok((
        connection.query_row(
            "SELECT annotation_json FROM document_annotations",
            [],
            |row| row.get(0),
        )?,
        connection.query_row(
            "SELECT after_json FROM document_annotation_history",
            [],
            |row| row.get(0),
        )?,
    ))
}

#[test]
fn version_nine_upgrade_preserves_annotation_and_history_bytes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("workspace.sqlite");
    let (document, annotation) = {
        let workspace = Workspace::open(&path)?;
        let project = workspace.create_project("The Last Light")?;
        let document = workspace.create_rich_document(
            &project.id,
            "Arrival",
            RichTree::plain("p", "Light 😀")?.to_json()?,
        )?;
        let anchor = workspace.capture_annotation_anchor(
            &document.id,
            document.revision,
            "p",
            ScalarOffset(0),
            ScalarOffset(5),
            "Light",
        )?;
        let annotation = workspace.create_annotation(
            &document.id,
            document.revision,
            anchor,
            "Keep the original note",
        )?;
        (document, annotation)
    };
    let raw = Connection::open(&path)?;
    let mut old = serde_json::to_value(&annotation)?;
    old.as_object_mut().ok_or("note object")?.remove("tags");
    let legacy = serde_json::to_string_pretty(&old)?;
    raw.execute(
        "UPDATE document_annotations SET annotation_json=?1 WHERE id=?2",
        params![legacy, annotation.id],
    )?;
    raw.execute(
        "UPDATE document_annotation_history SET after_json=?1 WHERE annotation_id=?2",
        params![legacy, annotation.id],
    )?;
    raw.execute_batch("DROP TABLE project_label_catalog")?;
    raw.pragma_update(None, "user_version", 9_i64)?;
    let before = annotation_bytes(&raw)?;
    drop(raw);

    let workspace = Workspace::open(&path)?;
    let restored = workspace.annotation(&annotation.id)?;
    assert!(restored.tags.is_empty());
    assert_eq!(restored.note, annotation.note);
    assert_eq!(workspace.document(&document.id)?, document);
    let raw = Connection::open(&path)?;
    assert_eq!(annotation_bytes(&raw)?, before);
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))?,
        11
    );
    Ok(())
}
