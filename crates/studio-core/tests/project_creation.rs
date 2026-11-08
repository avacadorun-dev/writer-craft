#![expect(
    clippy::unwrap_used,
    reason = "Synthetic aggregate creation regressions fail immediately"
)]
use studio_core::{InitialManuscript, Workspace};

#[test]
fn project_and_first_manuscript_should_survive_reopening_in_each_native_representation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("creation.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    for format in [InitialManuscript::Plain, InitialManuscript::Structured] {
        let created = workspace
            .create_project_with_manuscript("  New 作者 😀  ", "  First chapter  ", format)
            .unwrap();
        assert_eq!(created.project.title, "New 作者 😀");
        assert_eq!(created.manuscript.title, "First chapter");
        assert_eq!(created.manuscript.project_id, created.project.id);
        assert_eq!(created.manuscript.revision, 1);
        assert_eq!(created.manuscript.body, "");
        assert_eq!(
            created.manuscript.rich_json.is_some(),
            format == InitialManuscript::Structured
        );
        let reopened = Workspace::open(&path).unwrap();
        assert_eq!(
            reopened.document(&created.manuscript.id).unwrap(),
            created.manuscript
        );
        assert_eq!(reopened.revisions(&created.manuscript.id).unwrap().len(), 1);
    }
}
#[test]
fn validation_and_late_history_insertion_failure_should_leave_no_partial_project() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rollback.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    let sentinel = workspace.create_project("Unrelated project").unwrap();
    let before = workspace.list_projects().unwrap();
    assert!(
        workspace
            .create_project_with_manuscript("Invalid", "", InitialManuscript::Structured)
            .is_err()
    );
    assert_eq!(workspace.list_projects().unwrap(), before);
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER reject_synthetic_initial_history BEFORE INSERT ON rich_document_revisions BEGIN SELECT RAISE(ABORT,'synthetic history failure'); END;").unwrap();
    assert!(
        workspace
            .create_project_with_manuscript(
                "Must roll back",
                "First",
                InitialManuscript::Structured
            )
            .is_err()
    );
    assert_eq!(workspace.list_projects().unwrap(), before);
    assert!(workspace.list_documents(&sentinel.id).unwrap().is_empty());
    assert_eq!(
        raw.query_row("SELECT count(*) FROM documents", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    raw.execute_batch("DROP TRIGGER reject_synthetic_initial_history;")
        .unwrap();
    let created = workspace
        .create_project_with_manuscript("One retry", "First", InitialManuscript::Structured)
        .unwrap();
    assert_eq!(workspace.list_projects().unwrap().len(), 2);
    assert_eq!(
        workspace.list_documents(&created.project.id).unwrap().len(),
        1
    );
}
