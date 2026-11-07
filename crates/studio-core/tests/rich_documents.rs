#![expect(
    clippy::unwrap_used,
    reason = "Fixture failures must fail the test immediately"
)]

use craft_editor_model::{Command, Revision, ScalarOffset, TextRange, Transaction};
use serde_json::{Value, json};
use studio_core::{StudioError, Workspace};

fn content(text: &str) -> Value {
    json!({"type":"doc","attrs":{"schemaVersion":4,"vendor":{"null":null,"keep":[1,true]}},"opaqueRoot":{"retained":true},"content":[
        {"type":"heading","attrs":{"level":6,"blockId":"heading","sourceRefs":["stt:source:1-2"]},"content":[{"type":"text","text":"Глава"}]},
        {"type":"paragraph","attrs":{"blockId":"paragraph","sourceRefs":["stt:source:2-4"],"vendor":"kept"},"content":[{"type":"text","text":text,"marks":[{"type":"bold"}]}]},
        {"type":"table","attrs":{"blockId":"table"},"content":[{"type":"tableRow","content":[{"type":"tableCell","attrs":{"colspan":1,"rowspan":1,"colwidth":[180],"align":"right"},"content":[{"type":"paragraph","attrs":{"blockId":"cell","sourceRefs":[]},"content":[{"type":"text","text":"Ячейка"}]}]}]}]},
        {"type":"mathBlock","attrs":{"blockId":"math","latex":"E=mc^2","description":"Energy"}}
    ]})
}

fn workspace() -> (Workspace, studio_core::Project) {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Rich fixture").unwrap();
    (workspace, project)
}

#[test]
fn complete_structured_snapshot_and_unknown_metadata_survive_reopening() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let original = content("Автор 😀 е\u{308}");
    let id = {
        let workspace = Workspace::open(&path).unwrap();
        let project = workspace.create_project("Fixture").unwrap();
        workspace
            .create_rich_document(&project.id, "Document", original.clone())
            .unwrap()
            .id
    };
    let workspace = Workspace::open(path).unwrap();
    let restored = workspace.document(&id).unwrap();
    assert_eq!(restored.rich_json, Some(original.clone()));
    assert_eq!(workspace.rich_revisions(&id).unwrap()[0].content, original);
}

#[test]
fn formatting_only_changes_have_independent_durable_revisions() {
    let (workspace, project) = workspace();
    let original = content("Same text");
    let first = workspace
        .create_rich_document(&project.id, "Doc", original.clone())
        .unwrap();
    let mut formatted = original.clone();
    formatted["content"][1]["content"][0]["marks"] =
        json!([{"type":"italic"},{"type":"underline"}]);
    let changed = workspace
        .save_rich_document(&first.id, 1, formatted.clone())
        .unwrap();
    assert_eq!(changed.body, first.body);
    assert_eq!(changed.revision, 2);
    let restored = workspace.restore_rich_revision(&first.id, 2, 1).unwrap();
    assert_eq!((restored.revision, restored.rich_json), (3, Some(original)));
    let history = workspace.rich_revisions(&first.id).unwrap();
    assert_eq!(history[1].content, formatted);
    assert_eq!(history[0].source, "undo");
}

#[test]
fn legacy_text_save_draft_and_proposal_cannot_flatten_structured_content() {
    let (workspace, project) = workspace();
    let first = workspace
        .create_rich_document(&project.id, "Doc", content("Canonical"))
        .unwrap();
    assert!(workspace.save_document(&first.id, 1, "Flattened").is_err());
    assert!(workspace.save_draft(&first.id, 1, "Flattened").is_err());
    assert!(
        workspace
            .stage_replace(&first.id, 1, "Canonical", "Flattened", "Unsafe")
            .is_err()
    );
    assert_eq!(workspace.document(&first.id).unwrap(), first);
}

#[test]
fn stale_structured_save_preserves_current_tree_and_history() {
    let (workspace, project) = workspace();
    let first = workspace
        .create_rich_document(&project.id, "Doc", content("First"))
        .unwrap();
    let second = workspace
        .save_rich_document(&first.id, 1, content("Second"))
        .unwrap();
    assert!(matches!(
        workspace.save_rich_document(&first.id, 1, content("Stale")),
        Err(StudioError::RevisionConflict {
            expected: 1,
            actual: 2
        })
    ));
    assert_eq!(workspace.document(&first.id).unwrap(), second);
    assert_eq!(workspace.rich_revisions(&first.id).unwrap().len(), 2);
}

#[test]
fn a_failed_structured_batch_rolls_back_json_projection_and_both_histories() {
    let (workspace, project) = workspace();
    let first = workspace
        .create_rich_document(&project.id, "Doc", content("А😀Б"))
        .unwrap();
    let command = Transaction {
        base_revision: Revision(1),
        commands: vec![
            Command::ReplaceText {
                range: TextRange {
                    block_id: "paragraph".into(),
                    start: ScalarOffset(1),
                    end: ScalarOffset(2),
                },
                expected: "😀".into(),
                replacement: "X".into(),
            },
            Command::ReplaceText {
                range: TextRange {
                    block_id: "paragraph".into(),
                    start: ScalarOffset(0),
                    end: ScalarOffset(1),
                },
                expected: "Wrong".into(),
                replacement: "Lost".into(),
            },
        ],
    };
    assert!(
        workspace
            .apply_rich_transaction(&first.id, command)
            .is_err()
    );
    assert_eq!(workspace.document(&first.id).unwrap(), first);
    assert_eq!(workspace.revisions(&first.id).unwrap().len(), 1);
    assert_eq!(workspace.rich_revisions(&first.id).unwrap().len(), 1);
}

#[test]
fn exact_unicode_command_preserves_unrelated_table_and_metadata() {
    let (workspace, project) = workspace();
    let original = content("А😀Б");
    let first = workspace
        .create_rich_document(&project.id, "Doc", original.clone())
        .unwrap();
    let changed = workspace
        .apply_rich_transaction(
            &first.id,
            Transaction {
                base_revision: Revision(1),
                commands: vec![Command::ReplaceText {
                    range: TextRange {
                        block_id: "paragraph".into(),
                        start: ScalarOffset(1),
                        end: ScalarOffset(2),
                    },
                    expected: "😀".into(),
                    replacement: "е\u{308}".into(),
                }],
            },
        )
        .unwrap();
    let current = changed.rich_json.unwrap();
    assert_eq!(current["content"][2], original["content"][2]);
    assert_eq!(current["opaqueRoot"], original["opaqueRoot"]);
    assert_eq!(
        current["content"][1]["attrs"],
        original["content"][1]["attrs"]
    );
    assert!(changed.body.contains("Ае\u{308}Б"));
}

#[test]
fn rich_draft_acknowledgement_never_discards_continued_work() {
    let (workspace, project) = workspace();
    let first = workspace
        .create_rich_document(&project.id, "Doc", content("First"))
        .unwrap();
    workspace
        .save_rich_draft(&first.id, 1, None, content("Continued draft"))
        .unwrap();
    workspace
        .save_rich_document(&first.id, 1, content("Earlier acknowledgement"))
        .unwrap();
    let draft = workspace.rich_draft(&first.id).unwrap().unwrap();
    assert_eq!(
        (draft.base_revision, draft.content),
        (1, content("Continued draft"))
    );
    assert!(matches!(
        workspace.save_rich_draft(&first.id, 2, None, content("New base")),
        Err(StudioError::DraftConflict { .. })
    ));
}

#[test]
fn matching_rich_draft_clears_only_after_exact_canonical_acknowledgement() {
    let (workspace, project) = workspace();
    let first = workspace
        .create_rich_document(&project.id, "Doc", content("First"))
        .unwrap();
    let draft = content("Committed draft");
    workspace
        .save_rich_draft(&first.id, 1, None, draft.clone())
        .unwrap();
    workspace
        .save_rich_document(&first.id, 1, draft.clone())
        .unwrap();
    assert!(workspace.rich_draft(&first.id).unwrap().is_none());
    assert_eq!(
        workspace.document(&first.id).unwrap().rich_json,
        Some(draft)
    );
}

#[test]
fn opaque_imports_and_unresolved_prose_drafts_are_not_eagerly_converted() {
    let (workspace, project) = workspace();
    let source = json!({"unknown":"retain original source"});
    let imported = workspace
        .import_document(&project.id, "Opaque", "Preview", source.clone())
        .unwrap();
    assert!(matches!(
        workspace.enable_rich_document(&imported.id, 1),
        Err(StudioError::ReadOnly { .. })
    ));
    assert_eq!(
        workspace.document(&imported.id).unwrap().source_json,
        Some(source)
    );
    let prose = workspace
        .create_document(&project.id, "Prose", "Original\n\nText")
        .unwrap();
    workspace
        .save_draft(&prose.id, 1, "Pending author work")
        .unwrap();
    assert!(workspace.enable_rich_document(&prose.id, 1).is_err());
    assert_eq!(
        workspace.draft(&prose.id).unwrap().unwrap().body,
        "Pending author work"
    );
    assert!(workspace.document(&prose.id).unwrap().rich_json.is_none());
}

#[test]
fn version_two_workspace_migration_retains_existing_plain_history() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("old.sqlite");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    connection
        .execute_batch(include_str!("../src/drafts.sql"))
        .unwrap();
    connection.execute_batch("PRAGMA application_id=1465078862; PRAGMA user_version=2; INSERT INTO projects VALUES('project','Legacy'); INSERT INTO documents VALUES('document','project','Prose','Keep exact text',1,0,NULL); INSERT INTO document_revisions VALUES('document',1,'Keep exact text','manual');").unwrap();
    drop(connection);
    let workspace = Workspace::open(&path).unwrap();
    let doc = workspace.document("document").unwrap();
    assert_eq!(
        (doc.body.as_str(), doc.revision, doc.rich_json.is_none()),
        ("Keep exact text", 1, true)
    );
    assert_eq!(
        workspace.revisions("document").unwrap()[0].body,
        "Keep exact text"
    );
    let raw = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        3
    );
}

#[test]
fn exact_command_acknowledgement_consumes_matching_draft_before_continued_typing() {
    let (workspace, project) = workspace();
    let original = content("А😀Б");
    let first = workspace
        .create_rich_document(&project.id, "Doc", original.clone())
        .unwrap();
    let command = Transaction {
        base_revision: Revision(1),
        commands: vec![Command::ReplaceText {
            range: TextRange {
                block_id: "paragraph".into(),
                start: ScalarOffset(1),
                end: ScalarOffset(2),
            },
            expected: "😀".into(),
            replacement: "X".into(),
        }],
    };
    // The view saves its actual canonical tree, including text-node boundaries.
    let mut view = craft_editor_model::Editor::new(
        craft_editor_model::Document::from_json(original).unwrap(),
        Revision(1),
    );
    let target = view
        .apply(command.clone())
        .unwrap()
        .after
        .to_json()
        .unwrap();
    workspace
        .save_rich_draft(&first.id, 1, None, target.clone())
        .unwrap();
    let updated = workspace
        .apply_rich_transaction(&first.id, command)
        .unwrap();
    assert_eq!(updated.rich_json, Some(target));
    assert!(workspace.rich_draft(&first.id).unwrap().is_none());
    workspace
        .save_rich_draft(&first.id, 2, None, content("Continued work"))
        .unwrap();
    assert_eq!(
        workspace
            .rich_draft(&first.id)
            .unwrap()
            .unwrap()
            .base_revision,
        2
    );
}

#[test]
fn same_base_stale_or_reordered_draft_writes_preserve_newer_unsaved_rich_work() {
    let (workspace, project) = workspace();
    let doc = workspace
        .create_rich_document(&project.id, "Doc", content("Canonical"))
        .unwrap();
    let first = workspace
        .save_rich_draft(&doc.id, 1, None, content("First draft"))
        .unwrap();
    let newer = workspace
        .save_rich_draft(
            &doc.id,
            1,
            Some(&first.version),
            content("Newer unsaved work"),
        )
        .unwrap();
    assert!(matches!(
        workspace.save_rich_draft(
            &doc.id,
            1,
            Some(&first.version),
            content("Late old payload")
        ),
        Err(StudioError::DraftChanged)
    ));
    assert!(matches!(
        workspace.save_rich_draft(&doc.id, 1, None, content("Another session")),
        Err(StudioError::DraftChanged)
    ));
    assert_eq!(workspace.rich_draft(&doc.id).unwrap(), Some(newer));
    assert_eq!(workspace.document(&doc.id).unwrap(), doc);
}

#[test]
fn concurrent_openers_serialize_one_additive_rich_migration() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("shared.sqlite");
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    raw.execute_batch(include_str!("../src/drafts.sql"))
        .unwrap();
    raw.execute_batch("PRAGMA application_id=1465078862; PRAGMA user_version=2; PRAGMA journal_mode=WAL; INSERT INTO projects VALUES('project','Keep project'); INSERT INTO documents VALUES('document','project','Doc','Keep prose',1,0,NULL); INSERT INTO document_revisions VALUES('document',1,'Keep prose','manual');").unwrap();
    drop(raw);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let barrier = barrier.clone();
            let path = path.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let workspace = Workspace::open(path)?;
                workspace.document("document")
            })
        })
        .collect();
    for handle in handles {
        let document = handle.join().unwrap().unwrap();
        assert_eq!(
            (document.body.as_str(), document.revision),
            ("Keep prose", 1)
        );
    }
}
