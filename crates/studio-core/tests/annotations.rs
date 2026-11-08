#![expect(
    clippy::unwrap_used,
    reason = "Synthetic annotation fixtures must fail immediately"
)]

use craft_editor_model::{Command, Revision, ScalarOffset, TextRange, Transaction};
use serde_json::{Value, json};
use studio_core::{
    AnchorStatus, AnnotationPatch, AnnotationStatus, Document, PassageAnnotation, StudioError,
    Workspace,
};

fn content(text: &str) -> Value {
    json!({"type":"doc","attrs":{"schemaVersion":4,"vendor":null},"content":[{"type":"paragraph","attrs":{"blockId":"p","sourceRefs":["recording"],"vendor":[1,null]},"content":[{"type":"text","text":text,"marks":[{"type":"bold"}]}]},{"type":"paragraph","attrs":{"blockId":"other"},"content":[{"type":"text","text":"Unchanged"}]}]})
}

fn fixture(text: &str) -> (Workspace, Document) {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Annotations").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Notes", content(text))
        .unwrap();
    (workspace, document)
}

fn note(
    workspace: &Workspace,
    document: &Document,
    start: usize,
    end: usize,
    exact: &str,
) -> PassageAnnotation {
    let anchor = workspace
        .capture_annotation_anchor(
            &document.id,
            document.revision,
            "p",
            ScalarOffset(start),
            ScalarOffset(end),
            exact,
        )
        .unwrap();
    workspace
        .create_annotation(
            &document.id,
            document.revision,
            anchor,
            "  Rework this passage  ",
        )
        .unwrap()
}

fn edit(
    workspace: &Workspace,
    document: &Document,
    start: usize,
    end: usize,
    expected: &str,
    replacement: &str,
) -> Document {
    workspace
        .apply_rich_transaction(
            &document.id,
            Transaction {
                base_revision: Revision(document.revision as u64),
                commands: vec![Command::ReplaceText {
                    range: TextRange {
                        block_id: "p".into(),
                        start: ScalarOffset(start),
                        end: ScalarOffset(end),
                    },
                    expected: expected.into(),
                    replacement: replacement.into(),
                }],
            },
        )
        .unwrap()
}

#[test]
fn persistent_second_occurrence_note_is_noncanonical_and_survives_reopening() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let (document, annotation) = {
        let workspace = Workspace::open(&path).unwrap();
        let project = workspace.create_project("Reopen").unwrap();
        let document = workspace
            .create_rich_document(&project.id, "Doc", content("Я😀 echo echo"))
            .unwrap();
        let annotation = note(&workspace, &document, 8, 12, "echo");
        assert_eq!(workspace.document(&document.id).unwrap(), document);
        assert_eq!(annotation.note, "Rework this passage");
        (document, annotation)
    };
    let workspace = Workspace::open(path).unwrap();
    assert_eq!(workspace.annotation(&annotation.id).unwrap(), annotation);
    assert_eq!(
        workspace.annotations(&document.id).unwrap(),
        vec![annotation]
    );
    assert_eq!(
        workspace.document(&document.id).unwrap().rich_json,
        document.rich_json
    );
}

#[test]
fn known_edits_before_and_after_shift_only_the_selected_unicode_occurrence() {
    let (workspace, document) = fixture("Я😀 echo echo fin");
    let annotation = note(&workspace, &document, 8, 12, "echo");
    let changed = edit(&workspace, &document, 0, 1, "Я", "Автор");
    let mapped = workspace.annotation(&annotation.id).unwrap();
    assert_eq!(
        (mapped.anchor.start, mapped.anchor.end),
        (ScalarOffset(12), ScalarOffset(16))
    );
    assert_eq!(mapped.anchor_status, AnchorStatus::Attached);
    edit(&workspace, &changed, 17, 20, "fin", "конец");
    let after = workspace.annotation(&annotation.id).unwrap();
    assert_eq!(
        (after.anchor.start, after.anchor.end),
        (mapped.anchor.start, mapped.anchor.end)
    );
    assert_eq!(after.anchor.exact_text, "echo");
    assert_eq!(after.base_revision, 3);
}

#[test]
fn proven_whole_and_partial_replacement_regenerate_only_the_exact_mapped_passage() {
    let (workspace, document) = fixture("echo echo");
    let annotation = note(&workspace, &document, 5, 9, "echo");
    let replaced = edit(&workspace, &document, 5, 9, "echo", "новое😀");
    let mapped = workspace.annotation(&annotation.id).unwrap();
    assert_eq!(mapped.anchor.exact_text, "новое😀");
    assert_eq!(mapped.anchor.end, ScalarOffset(11));
    assert_eq!(mapped.anchor_status, AnchorStatus::Attached);
    edit(&workspace, &replaced, 6, 7, "о", "X");
    let partial = workspace.annotation(&annotation.id).unwrap();
    assert_eq!(partial.anchor_status, AnchorStatus::Attached);
    assert_eq!(partial.anchor.exact_text, "нXвое😀");
    assert_eq!(partial.anchor.start, mapped.anchor.start);
    assert_eq!(partial.anchor.end, mapped.anchor.end);
    assert_eq!(partial.note, annotation.note);
    assert_eq!(partial.status, AnnotationStatus::Open);
}

#[test]
fn deletion_does_not_reattach_to_an_identical_quote_and_preserves_resolved_lifecycle() {
    let (workspace, document) = fixture("echo echo");
    let annotation = note(&workspace, &document, 5, 9, "echo");
    let resolved = workspace
        .update_annotation(
            &annotation.id,
            1,
            1,
            AnnotationPatch {
                note: None,
                status: Some(AnnotationStatus::Resolved),
            },
        )
        .unwrap();
    let deleted = edit(&workspace, &document, 5, 9, "echo", "");
    let detached = workspace.annotation(&annotation.id).unwrap();
    assert_eq!(detached.status, AnnotationStatus::Resolved);
    assert_eq!(detached.anchor_status, AnchorStatus::Detached);
    assert_eq!(detached.anchor, resolved.anchor);
    let anchor = workspace
        .capture_annotation_anchor(
            &document.id,
            deleted.revision,
            "p",
            ScalarOffset(0),
            ScalarOffset(4),
            "echo",
        )
        .unwrap();
    let reattached = workspace
        .reattach_annotation(&annotation.id, deleted.revision, detached.revision, anchor)
        .unwrap();
    assert_eq!(reattached.anchor_status, AnchorStatus::Attached);
    assert_eq!(reattached.status, AnnotationStatus::Resolved);
    assert_eq!(reattached.anchor.start, ScalarOffset(0));
}

#[test]
fn stale_note_document_and_quote_updates_preserve_all_current_metadata() {
    let (workspace, document) = fixture("Я😀 echo echo");
    let annotation = note(&workspace, &document, 8, 12, "echo");
    let updated = workspace
        .update_annotation(
            &annotation.id,
            1,
            1,
            AnnotationPatch {
                note: Some("New note".into()),
                status: None,
            },
        )
        .unwrap();
    assert!(matches!(
        workspace.update_annotation(
            &annotation.id,
            1,
            1,
            AnnotationPatch {
                note: Some("Stale".into()),
                status: None
            }
        ),
        Err(StudioError::AnnotationConflict {
            expected: 1,
            actual: 2
        })
    ));
    assert!(
        workspace
            .capture_annotation_anchor(
                &document.id,
                1,
                "p",
                ScalarOffset(8),
                ScalarOffset(12),
                "wrong"
            )
            .is_err()
    );
    edit(&workspace, &document, 0, 1, "Я", "X");
    assert!(matches!(
        workspace.delete_annotation(&annotation.id, 1, updated.revision),
        Err(StudioError::RevisionConflict {
            expected: 1,
            actual: 2
        })
    ));
    assert_eq!(
        workspace.annotation(&annotation.id).unwrap().note,
        "New note"
    );
}

#[test]
fn rich_snapshot_restore_compensates_attachments_without_overwriting_newer_authored_notes() {
    let (workspace, document) = fixture("echo echo");
    let annotation = note(&workspace, &document, 5, 9, "echo");
    let deleted = edit(&workspace, &document, 5, 9, "echo", "");
    let detached = workspace.annotation(&annotation.id).unwrap();
    let updated = workspace
        .update_annotation(
            &annotation.id,
            2,
            detached.revision,
            AnnotationPatch {
                note: Some("Keep my later note".into()),
                status: Some(AnnotationStatus::Resolved),
            },
        )
        .unwrap();
    let undo = workspace.restore_rich_revision(&document.id, 2, 1).unwrap();
    let restored = workspace.annotation(&annotation.id).unwrap();
    assert_eq!(restored.anchor, annotation.anchor);
    assert_eq!(restored.anchor_status, AnchorStatus::Attached);
    assert_eq!(restored.note, updated.note);
    assert_eq!(restored.status, AnnotationStatus::Resolved);
    assert_eq!(undo.rich_json, document.rich_json);
    let redo = workspace
        .restore_rich_revision(&document.id, 3, deleted.revision)
        .unwrap();
    assert_eq!(redo.rich_json, deleted.rich_json);
    assert_eq!(
        workspace.annotation(&annotation.id).unwrap().anchor_status,
        AnchorStatus::Detached
    );
    let history = workspace.annotation_history(&annotation.id).unwrap();
    assert_eq!(history[0].source, "restore");
    assert_eq!(history.len(), 5);
}

#[test]
fn unproven_snapshot_changes_detach_while_other_blocks_and_formatting_stay_attached() {
    let (workspace, document) = fixture("echo echo");
    let annotation = note(&workspace, &document, 5, 9, "echo");
    let mut formatting = document.rich_json.clone().unwrap();
    formatting["content"][0]["content"][0]["marks"] = json!([{"type":"italic"}]);
    let formatted = workspace
        .save_rich_document(&document.id, 1, formatting)
        .unwrap();
    assert_eq!(
        workspace.annotation(&annotation.id).unwrap().anchor_status,
        AnchorStatus::Attached
    );
    let mut other = formatted.rich_json.clone().unwrap();
    other["content"][1]["content"][0]["text"] = json!("Different other block");
    workspace
        .save_rich_document(&document.id, 2, other)
        .unwrap();
    assert_eq!(
        workspace.annotation(&annotation.id).unwrap().anchor_status,
        AnchorStatus::Attached
    );
    workspace
        .save_rich_document(&document.id, 3, content("echo echo echo"))
        .unwrap();
    assert_eq!(
        workspace.annotation(&annotation.id).unwrap().anchor_status,
        AnchorStatus::Detached
    );
}

#[test]
fn one_canonical_transaction_maps_multiple_edits_and_rejects_the_entire_invalid_batch() {
    let (workspace, document) = fixture("Я😀 echo echo");
    let annotation = note(&workspace, &document, 8, 12, "echo");
    let before_history = workspace.annotation_history(&annotation.id).unwrap();
    let commands = vec![
        Command::ReplaceText {
            range: TextRange {
                block_id: "p".into(),
                start: ScalarOffset(0),
                end: ScalarOffset(1),
            },
            expected: "Я".into(),
            replacement: "XX".into(),
        },
        Command::ReplaceText {
            range: TextRange {
                block_id: "p".into(),
                start: ScalarOffset(0),
                end: ScalarOffset(1),
            },
            expected: "Wrong".into(),
            replacement: "Lost".into(),
        },
    ];
    assert!(
        workspace
            .apply_rich_transaction(
                &document.id,
                Transaction {
                    base_revision: Revision(1),
                    commands
                }
            )
            .is_err()
    );
    assert_eq!(workspace.annotation(&annotation.id).unwrap(), annotation);
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(
        workspace.annotation_history(&annotation.id).unwrap(),
        before_history
    );
}

#[test]
fn sql_failure_rolls_back_rich_content_revision_and_mapped_annotation_together() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    let project = workspace.create_project("Rollback").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Doc", content("echo echo"))
        .unwrap();
    let annotation = note(&workspace, &document, 5, 9, "echo");
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER reject_annotation_mapping BEFORE INSERT ON document_annotation_history WHEN NEW.source='mapping' BEGIN SELECT RAISE(ABORT,'synthetic write failure'); END;").unwrap();
    assert!(
        workspace
            .apply_rich_transaction(
                &document.id,
                Transaction {
                    base_revision: Revision(1),
                    commands: vec![Command::ReplaceText {
                        range: TextRange {
                            block_id: "p".into(),
                            start: ScalarOffset(0),
                            end: ScalarOffset(0)
                        },
                        expected: String::new(),
                        replacement: "X".into()
                    }]
                }
            )
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.annotation(&annotation.id).unwrap(), annotation);
    assert_eq!(workspace.rich_revisions(&document.id).unwrap().len(), 1);
    assert_eq!(
        workspace.annotation_history(&annotation.id).unwrap().len(),
        1
    );
}

#[test]
fn boundary_insertions_stay_outside_the_exact_marked_passage() {
    let (workspace, document) = fixture("echo echo");
    let annotation = note(&workspace, &document, 5, 9, "echo");
    let before = edit(&workspace, &document, 5, 5, "", "😀");
    let shifted = workspace.annotation(&annotation.id).unwrap();
    assert_eq!(
        (shifted.anchor.start, shifted.anchor.end),
        (ScalarOffset(6), ScalarOffset(10))
    );
    edit(&workspace, &before, 10, 10, "", " fin");
    let after = workspace.annotation(&annotation.id).unwrap();
    assert_eq!(
        (after.anchor.start, after.anchor.end),
        (shifted.anchor.start, shifted.anchor.end)
    );
    assert_eq!(after.anchor.exact_text, "echo");
    assert_eq!(after.anchor_status, AnchorStatus::Attached);
}

#[test]
fn stable_anchor_follows_split_and_join_but_a_crossing_split_detaches() {
    let (workspace, document) = fixture("A echo");
    let annotation = note(&workspace, &document, 2, 6, "echo");
    workspace
        .apply_rich_transaction(
            &document.id,
            Transaction {
                base_revision: Revision(1),
                commands: vec![Command::SplitBlock {
                    block_id: "p".into(),
                    at: ScalarOffset(2),
                    new_block_id: "right".into(),
                }],
            },
        )
        .unwrap();
    let moved = workspace.annotation(&annotation.id).unwrap();
    assert_eq!(moved.anchor.block_id, "right");
    assert_eq!(
        (moved.anchor.start, moved.anchor.end),
        (ScalarOffset(0), ScalarOffset(4))
    );
    workspace
        .apply_rich_transaction(
            &document.id,
            Transaction {
                base_revision: Revision(2),
                commands: vec![Command::JoinBlocks {
                    left_id: "p".into(),
                    right_id: "right".into(),
                }],
            },
        )
        .unwrap();
    assert_eq!(
        workspace.annotation(&annotation.id).unwrap().anchor,
        annotation.anchor
    );
    workspace
        .apply_rich_transaction(
            &document.id,
            Transaction {
                base_revision: Revision(3),
                commands: vec![Command::SplitBlock {
                    block_id: "p".into(),
                    at: ScalarOffset(4),
                    new_block_id: "crossing".into(),
                }],
            },
        )
        .unwrap();
    let detached = workspace.annotation(&annotation.id).unwrap();
    assert_eq!(detached.anchor_status, AnchorStatus::Detached);
    assert_eq!(detached.anchor, annotation.anchor);
}

#[test]
fn additive_schema_three_migration_preserves_documents_and_existing_rich_history() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    raw.execute_batch(include_str!("../src/drafts.sql"))
        .unwrap();
    raw.execute_batch(include_str!("../src/rich.sql")).unwrap();
    raw.execute_batch("PRAGMA application_id=1465078862; PRAGMA user_version=3; INSERT INTO projects VALUES('project','Keep'); INSERT INTO documents VALUES('document','project','Doc','Text',1,0,NULL); INSERT INTO document_revisions VALUES('document',1,'Text','manual');").unwrap();
    let encoded = serde_json::to_string(&content("Text")).unwrap();
    raw.execute(
        "INSERT INTO rich_documents VALUES('document',?1)",
        [&encoded],
    )
    .unwrap();
    raw.execute(
        "INSERT INTO rich_document_revisions VALUES('document',1,?1)",
        [&encoded],
    )
    .unwrap();
    drop(raw);
    let workspace = Workspace::open(&path).unwrap();
    assert_eq!(
        workspace.document("document").unwrap().rich_json,
        Some(content("Text"))
    );
    assert_eq!(workspace.rich_revisions("document").unwrap().len(), 1);
    assert!(workspace.annotations("document").unwrap().is_empty());
    let raw = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        5
    );
}

#[test]
fn nested_stable_text_blocks_and_inline_atom_scalar_positions_use_the_shared_projection() {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Nested").unwrap();
    let content = json!({"type":"doc","attrs":{"schemaVersion":4},"content":[{"type":"blockquote","attrs":{"blockId":"quote"},"content":[{"type":"paragraph","attrs":{"blockId":"nested"},"content":[{"type":"text","text":"Я😀"},{"type":"inlineMath","attrs":{"latex":"x"}},{"type":"hardBreak"},{"type":"text","text":"echo"}]}]}]});
    let document = workspace
        .create_rich_document(&project.id, "Doc", content)
        .unwrap();
    let anchor = workspace
        .capture_annotation_anchor(
            &document.id,
            1,
            "nested",
            ScalarOffset(4),
            ScalarOffset(8),
            "echo",
        )
        .unwrap();
    let annotation = workspace
        .create_annotation(&document.id, 1, anchor, "Nested note")
        .unwrap();
    assert_eq!(annotation.anchor.prefix, "Я😀\u{fffc}\n");
    assert!(
        workspace
            .capture_annotation_anchor(
                &document.id,
                1,
                "nested",
                ScalarOffset(4),
                ScalarOffset(9),
                "echoX"
            )
            .is_err()
    );
}

#[test]
fn explicit_deletion_keeps_audit_and_is_not_reversed_by_document_restore() {
    let (workspace, document) = fixture("echo echo");
    let annotation = note(&workspace, &document, 5, 9, "echo");
    let changed = edit(&workspace, &document, 0, 0, "", "X");
    let current = workspace.annotation(&annotation.id).unwrap();
    workspace
        .delete_annotation(&annotation.id, changed.revision, current.revision)
        .unwrap();
    assert!(workspace.annotation(&annotation.id).is_err());
    workspace
        .restore_rich_revision(&document.id, changed.revision, 1)
        .unwrap();
    assert!(workspace.annotations(&document.id).unwrap().is_empty());
    let history = workspace.annotation_history(&annotation.id).unwrap();
    assert_eq!(history[0].source, "delete");
    assert!(history[0].after.deleted);
    assert_eq!(history.len(), 3);
}

#[test]
fn read_only_document_refuses_every_annotation_metadata_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("notes.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    let project = workspace.create_project("Readonly").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Notes", content("echo"))
        .unwrap();
    let annotation = note(&workspace, &document, 0, 4, "echo");
    let external = rusqlite::Connection::open(&path).unwrap();
    external
        .execute(
            "UPDATE documents SET read_only=1 WHERE id=?1",
            [&document.id],
        )
        .unwrap();
    assert!(matches!(
        workspace.update_annotation(
            &annotation.id,
            document.revision,
            annotation.revision,
            AnnotationPatch {
                note: Some("Changed".into()),
                status: None
            }
        ),
        Err(StudioError::ReadOnly { .. })
    ));
    assert!(matches!(
        workspace.delete_annotation(&annotation.id, document.revision, annotation.revision),
        Err(StudioError::ReadOnly { .. })
    ));
    assert!(matches!(
        workspace.reattach_annotation(
            &annotation.id,
            document.revision,
            annotation.revision,
            annotation.anchor.clone()
        ),
        Err(StudioError::ReadOnly { .. })
    ));
    assert_eq!(workspace.annotation(&annotation.id).unwrap(), annotation);
}
