#![expect(
    clippy::unwrap_used,
    reason = "Fixture failures must fail the test immediately"
)]

use rusqlite::Connection;
use serde_json::json;
use studio_core::{ProposalStatus, StudioError, Workspace};

fn document(body: &str) -> (Workspace, studio_core::Document) {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Мой роман").unwrap();
    let document = workspace
        .create_document(&project.id, "Глава первая", body)
        .unwrap();
    (workspace, document)
}

#[test]
fn saved_documents_and_revisions_survive_reopening() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nested/workspace.sqlite3");
    let document_id;
    {
        let workspace = Workspace::open(&path).unwrap();
        let project = workspace.create_project("Роман").unwrap();
        let document = workspace
            .create_document(&project.id, "Сцена", "Первый текст")
            .unwrap();
        document_id = document.id;
        workspace
            .save_document(&document_id, 1, "Второй текст 🦀")
            .unwrap();
    }
    let reopened = Workspace::open(&path).unwrap();
    let current = reopened.document(&document_id).unwrap();
    assert_eq!(
        (current.body.as_str(), current.revision),
        ("Второй текст 🦀", 2)
    );
    assert_eq!(reopened.revisions(&document_id).unwrap().len(), 2);
}

#[test]
fn staging_does_not_mutate_canonical_text_or_history() {
    let (workspace, document) = document("Автор выбирает слова");
    let proposal = workspace
        .stage_replace(&document.id, 1, "слова", "образы", "Уточнить образ")
        .unwrap();
    assert_eq!(proposal.status, ProposalStatus::Staged);
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.revisions(&document.id).unwrap().len(), 1);
}

#[test]
fn anchors_use_unicode_scalars_and_apply_exact_replacement() {
    let (workspace, document) = document("Я 🦀 пишу e\u{301} роман");
    let proposal = workspace
        .stage_replace(&document.id, 1, "пишу", "создаю", "Точная замена")
        .unwrap();
    assert_eq!((proposal.anchor.start, proposal.anchor.end), (4, 8));
    let applied = workspace.apply_proposal(&proposal.id).unwrap();
    assert_eq!(applied.body, "Я 🦀 создаю e\u{301} роман");
    assert_eq!(applied.revision, 2);
}

#[test]
fn stale_apply_changes_neither_document_history_nor_proposal() {
    let (workspace, document) = document("Старый текст");
    let proposal = workspace
        .stage_replace(&document.id, 1, "Старый", "Новый", "Улучшение")
        .unwrap();
    let manual = workspace
        .save_document(&document.id, 1, "Авторская новая версия")
        .unwrap();
    assert!(matches!(
        workspace.apply_proposal(&proposal.id),
        Err(StudioError::StaleProposal {
            expected: 1,
            actual: 2
        })
    ));
    assert_eq!(workspace.document(&document.id).unwrap(), manual);
    assert_eq!(workspace.revisions(&document.id).unwrap().len(), 2);
    assert_eq!(
        workspace.proposals(&document.id).unwrap()[0].status,
        ProposalStatus::Staged
    );
}

#[test]
fn undo_appends_a_compensating_revision_and_preserves_history() {
    let (workspace, document) = document("До замены");
    let proposal = workspace
        .stage_replace(&document.id, 1, "До", "После", "Замена")
        .unwrap();
    workspace.apply_proposal(&proposal.id).unwrap();
    let undone = workspace.undo_proposal(&proposal.id).unwrap();
    assert_eq!((undone.body.as_str(), undone.revision), ("До замены", 3));
    assert_eq!(
        workspace
            .revisions(&document.id)
            .unwrap()
            .iter()
            .map(|revision| revision.source.as_str())
            .collect::<Vec<_>>(),
        vec!["undo", "proposal", "manual"]
    );
    assert_eq!(
        workspace.proposals(&document.id).unwrap()[0].status,
        ProposalStatus::Undone
    );
}

#[test]
fn undo_refuses_to_overwrite_a_later_manual_edit() {
    let (workspace, document) = document("До замены");
    let proposal = workspace
        .stage_replace(&document.id, 1, "До", "После", "Замена")
        .unwrap();
    workspace.apply_proposal(&proposal.id).unwrap();
    let manual = workspace
        .save_document(&document.id, 2, "Поздняя авторская правка")
        .unwrap();
    assert!(matches!(
        workspace.undo_proposal(&proposal.id),
        Err(StudioError::UndoConflict {
            expected: 2,
            actual: 3
        })
    ));
    assert_eq!(workspace.document(&document.id).unwrap(), manual);
    assert_eq!(workspace.revisions(&document.id).unwrap().len(), 3);
}

#[test]
fn overlapping_exact_matches_are_ambiguous() {
    let (workspace, document) = document("banana");
    assert!(matches!(
        workspace.stage_replace(&document.id, 1, "ana", "x", "Replace"),
        Err(StudioError::AmbiguousMatch)
    ));
}

#[test]
fn rejected_proposal_cannot_be_applied() {
    let (workspace, document) = document("First draft");
    let proposal = workspace
        .stage_replace(&document.id, 1, "First", "Second", "Edit")
        .unwrap();
    workspace.reject_proposal(&proposal.id).unwrap();
    assert!(matches!(
        workspace.apply_proposal(&proposal.id),
        Err(StudioError::InvalidProposalState {
            actual: ProposalStatus::Rejected,
            ..
        })
    ));
    assert_eq!(
        workspace.document(&document.id).unwrap().body,
        "First draft"
    );
}

#[test]
fn stale_save_from_a_second_connection_cannot_overwrite_text() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite3");
    let first = Workspace::open(&path).unwrap();
    let project = first.create_project("Novel").unwrap();
    let document = first
        .create_document(&project.id, "Scene", "Original")
        .unwrap();
    let second = Workspace::open(&path).unwrap();
    first.save_document(&document.id, 1, "Newer").unwrap();
    assert!(matches!(
        second.save_document(&document.id, 1, "Outdated"),
        Err(StudioError::RevisionConflict {
            expected: 1,
            actual: 2
        })
    ));
    assert_eq!(second.document(&document.id).unwrap().body, "Newer");
}

#[test]
fn imported_rich_content_is_preserved_and_read_only_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite3");
    let source = json!({"formatVersion": 8,"unknownFutureField":{"keep":[1,2,3]},"content":{"type":"doc","attrs":{"schemaVersion":4},"content":[{"type":"mathBlock","attrs":{"latex":"E=mc^2"}}]}});
    let id;
    {
        let workspace = Workspace::open(&path).unwrap();
        let project = workspace.create_project("Imported").unwrap();
        id = workspace
            .import_document(
                &project.id,
                "Rich document",
                "Formula preview",
                source.clone(),
            )
            .unwrap()
            .id;
    }
    let workspace = Workspace::open(&path).unwrap();
    let current = workspace.document(&id).unwrap();
    assert_eq!(current.source_json, Some(source));
    assert!(current.read_only);
    assert!(matches!(
        workspace.save_document(&id, 1, "Flattened"),
        Err(StudioError::ReadOnly { .. })
    ));
    assert!(matches!(
        workspace.stage_replace(&id, 1, "Formula", "Changed", "Edit"),
        Err(StudioError::ReadOnly { .. })
    ));
}

#[test]
fn identical_save_does_not_create_spurious_revision() {
    let (workspace, document) = document("Unchanged");
    let current = workspace
        .save_document(&document.id, 1, "Unchanged")
        .unwrap();
    assert_eq!(current.revision, 1);
    assert_eq!(workspace.revisions(&document.id).unwrap().len(), 1);
}

#[test]
fn invalid_creation_does_not_leave_an_orphan_document() {
    let workspace = Workspace::memory().unwrap();
    assert!(matches!(
        workspace.create_document("missing", "Scene", "Text"),
        Err(StudioError::NotFound {
            kind: "Project",
            ..
        })
    ));
    let project = workspace.create_project("Valid").unwrap();
    assert!(workspace.list_documents(&project.id).unwrap().is_empty());
}

#[test]
fn opening_another_database_preserves_its_schema_and_content() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("another.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("CREATE TABLE other(data TEXT); INSERT INTO other VALUES('original');")
        .unwrap();
    assert!(matches!(
        Workspace::open(&path),
        Err(StudioError::NotWorkspaceDatabase)
    ));
    let tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tables, 1);
    assert_eq!(
        connection
            .query_row("SELECT data FROM other", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "original"
    );
}

#[test]
fn proposal_lifecycle_persists_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite3");
    let (document_id, proposal_id);
    {
        let workspace = Workspace::open(&path).unwrap();
        let project = workspace.create_project("Novel").unwrap();
        let document = workspace
            .create_document(&project.id, "Scene", "Old")
            .unwrap();
        let proposal = workspace
            .stage_replace(&document.id, 1, "Old", "New", "Edit")
            .unwrap();
        document_id = document.id;
        proposal_id = proposal.id;
    }
    let workspace = Workspace::open(&path).unwrap();
    workspace.apply_proposal(&proposal_id).unwrap();
    assert_eq!(
        workspace.proposals(&document_id).unwrap()[0].applied_revision,
        Some(2)
    );
}

#[test]
fn working_draft_survives_restart_without_mutating_canonical_history() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite3");
    let id;
    {
        let workspace = Workspace::open(&path).unwrap();
        let project = workspace.create_project("Novel").unwrap();
        id = workspace
            .create_document(&project.id, "Scene", "Saved")
            .unwrap()
            .id;
        workspace
            .save_draft(&id, 1, "Unsaved working thought 🦀")
            .unwrap();
    }
    let workspace = Workspace::open(&path).unwrap();
    assert_eq!(
        workspace.draft(&id).unwrap().unwrap().body,
        "Unsaved working thought 🦀"
    );
    assert_eq!(workspace.document(&id).unwrap().body, "Saved");
    assert_eq!(workspace.revisions(&id).unwrap().len(), 1);
}

#[test]
fn canonical_save_clears_only_its_matching_draft() {
    let (workspace, document) = document("Saved");
    workspace.save_draft(&document.id, 1, "New text").unwrap();
    workspace
        .save_document(&document.id, 1, "New text")
        .unwrap();
    assert!(workspace.draft(&document.id).unwrap().is_none());
}

#[test]
fn canonical_save_does_not_delete_a_newer_differing_working_draft() {
    let (workspace, document) = document("Saved");
    workspace
        .save_draft(&document.id, 1, "Newer unsaved text")
        .unwrap();
    workspace
        .save_document(&document.id, 1, "Earlier autosave text")
        .unwrap();
    assert_eq!(
        workspace.draft(&document.id).unwrap().unwrap().body,
        "Newer unsaved text"
    );
}

#[test]
fn stale_draft_write_does_not_overwrite_recovery_text() {
    let (workspace, document) = document("Saved");
    workspace.save_document(&document.id, 1, "Updated").unwrap();
    workspace
        .save_draft(&document.id, 2, "Current draft")
        .unwrap();
    assert!(matches!(
        workspace.save_draft(&document.id, 1, "Stale draft"),
        Err(StudioError::RevisionConflict { .. })
    ));
    assert_eq!(
        workspace.draft(&document.id).unwrap().unwrap().body,
        "Current draft"
    );
}

#[test]
fn schema_one_migration_adds_drafts_without_rewriting_existing_documents() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite3");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(include_str!("../src/schema.sql"))
            .unwrap();
        connection
            .pragma_update(None, "application_id", 0x5753544e_i64)
            .unwrap();
        connection
            .pragma_update(None, "user_version", 1_i64)
            .unwrap();
        connection.execute_batch("INSERT INTO projects(id,title) VALUES('project','Existing'); INSERT INTO documents(id,project_id,title,body,revision) VALUES('document','project','Scene','Preserved',1); INSERT INTO document_revisions(document_id,revision,body,source) VALUES('document',1,'Preserved','manual');").unwrap();
    }
    let workspace = Workspace::open(&path).unwrap();
    assert_eq!(workspace.document("document").unwrap().body, "Preserved");
    workspace
        .save_draft("document", 1, "Recovered thought")
        .unwrap();
    assert_eq!(
        workspace.draft("document").unwrap().unwrap().body,
        "Recovered thought"
    );
    assert_eq!(workspace.revisions("document").unwrap().len(), 1);
}

#[test]
fn newer_workspace_schema_is_refused_without_downgrading() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "application_id", 0x5753544e_i64)
        .unwrap();
    connection
        .pragma_update(None, "user_version", 99_i64)
        .unwrap();
    assert!(matches!(
        Workspace::open(&path),
        Err(StudioError::UnsupportedSchema { found: 99, .. })
    ));
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        99
    );
}

#[test]
fn new_base_draft_cannot_overwrite_older_distinct_recovery_text() {
    let (workspace, document) = document("Initial");
    workspace
        .save_draft(&document.id, 1, "Unfinished thought 🦀")
        .unwrap();
    let current = workspace
        .save_document(&document.id, 1, "Different saved text")
        .unwrap();
    assert!(matches!(
        workspace.save_draft(&document.id, current.revision, "New current thought"),
        Err(StudioError::DraftConflict {
            draft_revision: 1,
            current_revision: 2
        })
    ));
    assert_eq!(
        workspace.draft(&document.id).unwrap().unwrap(),
        studio_core::Draft {
            base_revision: 1,
            body: "Unfinished thought 🦀".into()
        }
    );
    assert_eq!(workspace.document(&document.id).unwrap(), current);
}

#[test]
fn draft_recovery_creates_editable_copy_without_changing_original_and_survives_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite3");
    let (original_id, recovered_id, original_body);
    {
        let workspace = Workspace::open(&path).unwrap();
        let project = workspace.create_project("Novel").unwrap();
        let original = workspace
            .create_document(&project.id, "Сцена", "Original saved text")
            .unwrap();
        workspace
            .save_draft(&original.id, 1, "Ценная незаконченная мысль 🦀")
            .unwrap();
        let updated = workspace
            .save_document(&original.id, 1, "New canonical text")
            .unwrap();
        let recovered = workspace.recover_draft_as_document(&original.id).unwrap();
        assert_eq!(recovered.title, "Сцена · восстановленный черновик");
        assert_eq!(recovered.project_id, project.id);
        assert_eq!(recovered.revision, 1);
        assert!(!recovered.read_only);
        assert!(workspace.draft(&original.id).unwrap().is_none());
        assert_eq!(workspace.document(&original.id).unwrap(), updated);
        assert_eq!(workspace.revisions(&original.id).unwrap().len(), 2);
        original_id = original.id;
        recovered_id = recovered.id;
        original_body = updated.body;
    }
    let workspace = Workspace::open(&path).unwrap();
    let recovered = workspace.document(&recovered_id).unwrap();
    assert_eq!(recovered.body, "Ценная незаконченная мысль 🦀");
    assert_eq!(workspace.revisions(&recovered_id).unwrap().len(), 1);
    assert_eq!(
        workspace.document(&original_id).unwrap().body,
        original_body
    );
    assert!(workspace.draft(&original_id).unwrap().is_none());
    workspace
        .save_document(&recovered_id, 1, "Продолжение мысли")
        .unwrap();
}

#[test]
fn recovering_a_missing_draft_does_not_create_a_document() {
    let (workspace, document) = document("Saved");
    assert!(matches!(
        workspace.recover_draft_as_document(&document.id),
        Err(StudioError::NotFound {
            kind: "Working draft",
            ..
        })
    ));
    assert_eq!(
        workspace
            .list_documents(&document.project_id)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(workspace.document(&document.id).unwrap(), document);
}

#[test]
fn recovered_document_title_preserves_suffix_within_title_limit() {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Novel").unwrap();
    let document = workspace
        .create_document(&project.id, &"Я".repeat(200), "Saved")
        .unwrap();
    workspace.save_draft(&document.id, 1, "Draft").unwrap();
    let recovered = workspace.recover_draft_as_document(&document.id).unwrap();
    assert_eq!(recovered.title.chars().count(), 200);
    assert!(recovered.title.ends_with(" · восстановленный черновик"));
}

#[test]
fn selected_second_repeated_phrase_applies_and_undoes_exact_unicode_range() {
    let (workspace, document) = document("Я 🦀 слово и слово.");
    let anchor = studio_core::TextAnchor {
        start: 12,
        end: 17,
        expected_text: "слово".into(),
    };
    let proposal = workspace
        .stage_replace_range(
            &document.id,
            1,
            &anchor,
            "образ",
            "Clarify the selected occurrence",
        )
        .unwrap();
    assert_eq!(proposal.anchor, anchor);
    assert_eq!(proposal.after, "Я 🦀 слово и образ.");
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    let applied = workspace.apply_proposal(&proposal.id).unwrap();
    assert_eq!(applied.body, "Я 🦀 слово и образ.");
    let undone = workspace.undo_proposal(&proposal.id).unwrap();
    assert_eq!(undone.body, "Я 🦀 слово и слово.");
    assert_eq!(undone.revision, 3);
}

#[test]
fn selected_invalid_scalar_ranges_stage_nothing_and_preserve_history() {
    let (workspace, document) = document("Я 🦀 слово и слово.");
    for (start, end) in [
        (12, 12),
        (17, 12),
        (12, usize::MAX),
        (usize::MAX, usize::MAX),
    ] {
        let anchor = studio_core::TextAnchor {
            start,
            end,
            expected_text: "слово".into(),
        };
        assert!(matches!(
            workspace.stage_replace_range(&document.id, 1, &anchor, "образ", "Edit selection"),
            Err(StudioError::InvalidTextRange { .. })
        ));
    }
    assert!(workspace.proposals(&document.id).unwrap().is_empty());
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.revisions(&document.id).unwrap().len(), 1);
}

#[test]
fn selected_literal_mismatch_stages_nothing_and_preserves_history() {
    let (workspace, document) = document("Я 🦀 слово и слово.");
    let anchor = studio_core::TextAnchor {
        start: 12,
        end: 17,
        expected_text: "друго".into(),
    };
    assert!(matches!(
        workspace.stage_replace_range(&document.id, 1, &anchor, "образ", "Edit selection"),
        Err(StudioError::ExpectedTextMismatch)
    ));
    assert!(workspace.proposals(&document.id).unwrap().is_empty());
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.revisions(&document.id).unwrap().len(), 1);
}

#[test]
fn selected_stale_revision_stages_nothing_and_preserves_newer_work() {
    let (workspace, document) = document("Я 🦀 слово и слово.");
    let anchor = studio_core::TextAnchor {
        start: 12,
        end: 17,
        expected_text: "слово".into(),
    };
    let updated = workspace
        .save_document(&document.id, 1, "Поздняя авторская редакция")
        .unwrap();
    assert!(matches!(
        workspace.stage_replace_range(&document.id, 1, &anchor, "образ", "Edit selection"),
        Err(StudioError::RevisionConflict {
            expected: 1,
            actual: 2
        })
    ));
    assert!(workspace.proposals(&document.id).unwrap().is_empty());
    assert_eq!(workspace.document(&document.id).unwrap(), updated);
    assert_eq!(workspace.revisions(&document.id).unwrap().len(), 2);
}
