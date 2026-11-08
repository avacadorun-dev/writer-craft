#![expect(
    clippy::unwrap_used,
    reason = "Synthetic acceptance fixtures must fail immediately"
)]
use craft_editor_model::{Command, Mark, MarkMode, Revision, ScalarOffset, TextRange, Transaction};
use serde_json::{Value, json};
use studio_core::{
    AnchorStatus, AnnotationPatch, AnnotationStatus, ProposalStatus, StudioError, Workspace,
};

fn content() -> Value {
    json!({"type":"doc","attrs":{"schemaVersion":4,"vendor":{"keep":[null,1,true]}},"content":[
        {"type":"paragraph","attrs":{"blockId":"p","sourceRefs":["source-original"],"vendor":"unchanged"},"content":[{"type":"text","text":"Я😀 echo echo","marks":[{"type":"bold"}]}]},
        {"type":"paragraph","attrs":{"blockId":"q"},"content":[{"type":"text","text":"Other writing"}]}
    ]})
}
fn edit(base: i64) -> Transaction {
    Transaction {
        base_revision: Revision(base as u64),
        commands: vec![Command::ReplaceText {
            range: TextRange {
                block_id: "p".into(),
                start: ScalarOffset(8),
                end: ScalarOffset(12),
            },
            expected: "echo".into(),
            replacement: "changed".into(),
        }],
    }
}
fn fixture() -> (Workspace, studio_core::Project, studio_core::Document) {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Synthetic review").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Writing", content())
        .unwrap();
    (workspace, project, document)
}
#[test]
fn staging_rejection_and_metadata_paging_should_never_change_canonical_writing() {
    let (workspace, project, document) = fixture();
    let staged = workspace
        .stage_rich_proposal(&project.id, &document.id, edit(1), "Local author edit")
        .unwrap();
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.rich_revisions(&document.id).unwrap().len(), 1);
    assert_eq!(staged.before, content());
    assert_eq!(
        staged.after["content"][0]["attrs"],
        content()["content"][0]["attrs"]
    );
    assert_eq!(
        staged.after["content"][0]["content"][0]["text"],
        "Я😀 echo changed"
    );
    let rows = workspace
        .rich_proposals(&project.id, &document.id, None, 1)
        .unwrap();
    assert_eq!(rows, vec![staged.summary.clone()]);
    assert!(
        workspace
            .rich_proposals(&project.id, &document.id, Some(rows[0].cursor), 1)
            .unwrap()
            .is_empty()
    );
    workspace
        .reject_rich_proposal(&project.id, &staged.summary.id)
        .unwrap();
    assert_eq!(
        workspace
            .rich_proposal(&project.id, &staged.summary.id)
            .unwrap()
            .summary
            .status,
        ProposalStatus::Rejected
    );
    assert!(
        workspace
            .apply_rich_proposal(&project.id, &staged.summary.id)
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), document);
}
#[test]
fn apply_and_compensating_undo_should_preserve_second_occurrence_notes_and_current_note_text() {
    let (workspace, project, document) = fixture();
    let anchor = workspace
        .capture_annotation_anchor(
            &document.id,
            1,
            "p",
            ScalarOffset(8),
            ScalarOffset(12),
            "echo",
        )
        .unwrap();
    let note = workspace
        .create_annotation(&document.id, 1, anchor.clone(), "Original note")
        .unwrap();
    let proposal = workspace
        .stage_rich_proposal(&project.id, &document.id, edit(1), "Local edit")
        .unwrap();
    let applied = workspace
        .apply_rich_proposal(&project.id, &proposal.summary.id)
        .unwrap();
    assert_eq!(
        (applied.revision, applied.body.as_str()),
        (2, "Я😀 echo changed\n\nOther writing")
    );
    let after = workspace.annotation(&note.id).unwrap();
    // A sealed exact whole-passage replacement maps the attachment to its replacement.
    assert_eq!(after.anchor_status, AnchorStatus::Attached);
    assert_eq!(
        (
            after.anchor.start.0,
            after.anchor.end.0,
            after.anchor.exact_text.as_str()
        ),
        (8, 15, "changed")
    );
    let modified = workspace
        .update_annotation(
            &note.id,
            2,
            after.revision,
            AnnotationPatch {
                note: Some("New note after Apply".into()),
                status: Some(AnnotationStatus::Resolved),
            },
        )
        .unwrap();
    let undone = workspace
        .undo_rich_proposal(&project.id, &proposal.summary.id)
        .unwrap();
    assert_eq!((undone.revision, undone.rich_json), (3, Some(content())));
    let restored = workspace.annotation(&note.id).unwrap();
    assert_eq!(
        (restored.anchor_status, restored.anchor),
        (AnchorStatus::Attached, anchor)
    );
    assert_eq!(
        (restored.note, restored.status),
        (modified.note, AnnotationStatus::Resolved)
    );
    assert_eq!(
        workspace
            .rich_proposal(&project.id, &proposal.summary.id)
            .unwrap()
            .summary
            .status,
        ProposalStatus::Undone
    );
    assert!(
        workspace
            .undo_rich_proposal(&project.id, &proposal.summary.id)
            .is_err()
    );
    assert_eq!(workspace.rich_revisions(&document.id).unwrap().len(), 3);
}
#[test]
fn formatting_only_intervening_changes_should_refuse_stale_apply_without_publishing_anything() {
    let (workspace, project, document) = fixture();
    let proposal = workspace
        .stage_rich_proposal(&project.id, &document.id, edit(1), "Edit")
        .unwrap();
    let current = workspace
        .apply_rich_transaction(
            &document.id,
            Transaction {
                base_revision: Revision(1),
                commands: vec![Command::ApplyMarks {
                    range: TextRange {
                        block_id: "p".into(),
                        start: ScalarOffset(0),
                        end: ScalarOffset(1),
                    },
                    marks: vec![Mark::new("italic")],
                    mode: MarkMode::Add,
                }],
            },
        )
        .unwrap();
    assert_eq!(current.body, document.body);
    assert!(matches!(
        workspace.apply_rich_proposal(&project.id, &proposal.summary.id),
        Err(StudioError::StaleProposal {
            expected: 1,
            actual: 2
        })
    ));
    assert_eq!(workspace.document(&document.id).unwrap(), current);
    assert_eq!(
        workspace
            .rich_proposal(&project.id, &proposal.summary.id)
            .unwrap()
            .summary
            .status,
        ProposalStatus::Staged
    );
    assert_eq!(workspace.rich_revisions(&document.id).unwrap().len(), 2);
}
#[test]
fn pending_drafts_should_block_apply_and_undo_without_deleting_recovery_content() {
    let (workspace, project, document) = fixture();
    let proposal = workspace
        .stage_rich_proposal(&project.id, &document.id, edit(1), "Edit")
        .unwrap();
    let draft = workspace
        .save_rich_draft(&document.id, 1, None, content())
        .unwrap();
    assert!(
        workspace
            .apply_rich_proposal(&project.id, &proposal.summary.id)
            .is_err()
    );
    assert_eq!(workspace.rich_draft(&document.id).unwrap(), Some(draft));
    workspace
        .save_rich_document(&document.id, 1, content())
        .unwrap();
    let applied = workspace
        .apply_rich_proposal(&project.id, &proposal.summary.id)
        .unwrap();
    let draft = workspace
        .save_rich_draft(&document.id, 2, None, applied.rich_json.clone().unwrap())
        .unwrap();
    assert!(
        workspace
            .undo_rich_proposal(&project.id, &proposal.summary.id)
            .is_err()
    );
    assert_eq!(workspace.rich_draft(&document.id).unwrap(), Some(draft));
    assert_eq!(workspace.document(&document.id).unwrap(), applied);
}
#[test]
fn cross_project_ids_and_oversized_pages_should_never_grant_proposal_authority() {
    let (workspace, project, document) = fixture();
    let proposal = workspace
        .stage_rich_proposal(&project.id, &document.id, edit(1), "Edit")
        .unwrap();
    let other = workspace.create_project("Other").unwrap();
    assert!(
        workspace
            .rich_proposal(&other.id, &proposal.summary.id)
            .is_err()
    );
    assert!(
        workspace
            .rich_proposals(&other.id, &document.id, None, 10)
            .is_err()
    );
    assert!(
        workspace
            .stage_rich_proposal(&other.id, &document.id, edit(1), "Edit")
            .is_err()
    );
    assert!(
        workspace
            .apply_rich_proposal(&other.id, &proposal.summary.id)
            .is_err()
    );
    assert!(
        workspace
            .reject_rich_proposal(&other.id, &proposal.summary.id)
            .is_err()
    );
    assert!(
        workspace
            .undo_rich_proposal(&other.id, &proposal.summary.id)
            .is_err()
    );
    assert!(
        workspace
            .rich_proposals(&project.id, &document.id, None, 0)
            .is_err()
    );
    assert!(
        workspace
            .rich_proposals(&project.id, &document.id, None, 129)
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), document);
}
#[test]
fn structured_batch_failure_or_noop_should_not_create_a_proposal() {
    let (workspace, project, document) = fixture();
    let mut command = edit(1);
    command.commands.push(Command::ReplaceText {
        range: TextRange {
            block_id: "q".into(),
            start: ScalarOffset(0),
            end: ScalarOffset(1),
        },
        expected: "wrong".into(),
        replacement: "X".into(),
    });
    assert!(
        workspace
            .stage_rich_proposal(&project.id, &document.id, command, "Fail atomically")
            .is_err()
    );
    assert!(
        workspace
            .stage_rich_proposal(
                &project.id,
                &document.id,
                Transaction {
                    base_revision: Revision(1),
                    commands: vec![]
                },
                "Empty"
            )
            .is_err()
    );
    let mut noop = edit(1);
    if let Command::ReplaceText { replacement, .. } = &mut noop.commands[0] {
        *replacement = "echo".into();
    }
    assert!(
        workspace
            .stage_rich_proposal(&project.id, &document.id, noop, "No-op")
            .is_err()
    );
    assert!(
        workspace
            .rich_proposals(&project.id, &document.id, None, 10)
            .unwrap()
            .is_empty()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), document);
}
#[test]
fn later_canonical_edits_should_block_compensating_undo() {
    let (workspace, project, document) = fixture();
    let proposal = workspace
        .stage_rich_proposal(&project.id, &document.id, edit(1), "Edit")
        .unwrap();
    let applied = workspace
        .apply_rich_proposal(&project.id, &proposal.summary.id)
        .unwrap();
    let current = workspace
        .save_rich_document(&document.id, 2, content())
        .unwrap();
    assert!(matches!(
        workspace.undo_rich_proposal(&project.id, &proposal.summary.id),
        Err(StudioError::UndoConflict {
            expected: 2,
            actual: 3
        })
    ));
    assert_eq!(workspace.document(&document.id).unwrap(), current);
    assert_ne!(current.revision, applied.revision);
    assert_eq!(
        workspace
            .rich_proposal(&project.id, &proposal.summary.id)
            .unwrap()
            .summary
            .status,
        ProposalStatus::Applied
    );
}
#[test]
fn restart_should_retain_prepared_commands_without_auto_applying() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("review.sqlite");
    let (project, document, proposal) = {
        let workspace = Workspace::open(&path).unwrap();
        let project = workspace.create_project("Restart").unwrap();
        let document = workspace
            .create_rich_document(&project.id, "Doc", content())
            .unwrap();
        let proposal = workspace
            .stage_rich_proposal(&project.id, &document.id, edit(1), "Later")
            .unwrap();
        (project, document, proposal)
    };
    let reopened = Workspace::open(path).unwrap();
    assert_eq!(
        reopened
            .rich_proposal(&project.id, &proposal.summary.id)
            .unwrap(),
        proposal
    );
    assert_eq!(reopened.document(&document.id).unwrap(), document);
    reopened
        .apply_rich_proposal(&project.id, &proposal.summary.id)
        .unwrap();
}
#[test]
fn unavailable_image_or_over_capacity_table_should_refuse_without_changes() {
    let (workspace, project, document) = fixture();
    for node in [
        json!({"type":"imageBlock","attrs":{"blockId":"image","assetId":"foreign","src":"asset:foreign","alt":"Synthetic","caption":""}}),
        json!({"type":"table","attrs":{"blockId":"wide"},"content":[{"type":"tableRow","content":[{"type":"tableCell","attrs":{"colspan":64,"rowspan":1},"content":[{"type":"paragraph","attrs":{"blockId":"cell"}}]}]}]}),
    ] {
        let node = serde_json::from_value(node).unwrap();
        let command = Transaction {
            base_revision: Revision(1),
            commands: vec![Command::InsertBlocks {
                index: 1,
                blocks: vec![node],
            }],
        };
        assert!(
            workspace
                .stage_rich_proposal(&project.id, &document.id, command, "Unavailable resource")
                .is_err()
        );
    }
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert!(
        workspace
            .rich_proposals(&project.id, &document.id, None, 10)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn failed_publication_should_roll_back_document_notes_history_and_status() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("atomic.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    let project = workspace.create_project("Atomic").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Doc", content())
        .unwrap();
    let anchor = workspace
        .capture_annotation_anchor(
            &document.id,
            1,
            "p",
            ScalarOffset(8),
            ScalarOffset(12),
            "echo",
        )
        .unwrap();
    let note = workspace
        .create_annotation(&document.id, 1, anchor, "Keep")
        .unwrap();
    let proposal = workspace
        .stage_rich_proposal(&project.id, &document.id, edit(1), "Edit")
        .unwrap();
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER refuse_review_publication BEFORE UPDATE OF status ON rich_proposals WHEN NEW.status='applied' BEGIN SELECT RAISE(ABORT,'Synthetic failure'); END;").unwrap();
    assert!(
        workspace
            .apply_rich_proposal(&project.id, &proposal.summary.id)
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.annotation(&note.id).unwrap(), note);
    assert_eq!(workspace.rich_revisions(&document.id).unwrap().len(), 1);
    assert_eq!(
        workspace
            .rich_proposal(&project.id, &proposal.summary.id)
            .unwrap(),
        proposal
    );
}
#[test]
fn altered_persisted_preview_should_not_replace_the_replayed_result() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tamper.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    let project = workspace.create_project("Tamper").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Doc", content())
        .unwrap();
    let proposal = workspace
        .stage_rich_proposal(&project.id, &document.id, edit(1), "Edit")
        .unwrap();
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE rich_proposals SET after_json=?2 WHERE id=?1",
        rusqlite::params![
            proposal.summary.id,
            serde_json::to_string(&content()).unwrap()
        ],
    )
    .unwrap();
    assert!(matches!(
        workspace.apply_rich_proposal(&project.id, &proposal.summary.id),
        Err(StudioError::CorruptData(_))
    ));
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(workspace.rich_revisions(&document.id).unwrap().len(), 1);
}
#[test]
fn schema_eight_upgrade_should_preserve_canonical_data_and_create_an_empty_review_store() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("upgrade.sqlite");
    let document = {
        let workspace = Workspace::open(&path).unwrap();
        let project = workspace.create_project("Migration").unwrap();
        workspace
            .create_rich_document(&project.id, "Doc", content())
            .unwrap()
    };
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch("DROP TABLE rich_proposals; PRAGMA user_version=8;")
            .unwrap();
    }
    let workspace = Workspace::open(path).unwrap();
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert!(
        workspace
            .rich_proposals(&document.project_id, &document.id, None, 10)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn selected_multi_block_review_should_preserve_omitted_tables_and_exact_duplicate_occurrences() {
    use craft_editor_model::TextPoint;
    let (workspace, project, document) = fixture();
    let mut original = content();
    let table = json!({"type":"table","attrs":{"blockId":"table","vendor":"keep"},"content":[{"type":"tableRow","content":[{"type":"tableCell","attrs":{"colspan":1,"rowspan":1,"colwidth":[180]},"content":[{"type":"paragraph","attrs":{"blockId":"cell"},"content":[{"type":"text","text":"Not selected"}]}]}]}]});
    original["content"]
        .as_array_mut()
        .unwrap()
        .insert(1, table.clone());
    let document = workspace
        .save_rich_document(&document.id, 1, original.clone())
        .unwrap();
    let selection = workspace
        .capture_assistant_selection(
            &project.id,
            &document.id,
            2,
            &TextPoint {
                block_id: "p".into(),
                offset: ScalarOffset(8),
            },
            &TextPoint {
                block_id: "q".into(),
                offset: ScalarOffset(5),
            },
        )
        .unwrap();
    assert_eq!(selection.ranges.len(), 2);
    let proposal = workspace
        .stage_assistant_selection_replacements(
            &project.id,
            &selection,
            &["changed".into(), "Fresh".into()],
            "Two exact ranges",
        )
        .unwrap();
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    let applied = workspace
        .apply_rich_proposal(&project.id, &proposal.summary.id)
        .unwrap();
    let tree = applied.rich_json.unwrap();
    assert_eq!(tree["content"][1], table);
    assert_eq!(tree["content"][0]["content"][0]["text"], "Я😀 echo changed");
    assert_eq!(
        tree["content"][0]["content"][0]["marks"],
        original["content"][0]["content"][0]["marks"]
    );
    assert_eq!(tree["content"][2]["content"][0]["text"], "Fresh writing");
}
#[test]
fn changed_attachment_ranges_or_replacement_count_should_refuse_without_new_authority() {
    use craft_editor_model::TextPoint;
    let (workspace, project, document) = fixture();
    let selection = workspace
        .capture_assistant_selection(
            &project.id,
            &document.id,
            1,
            &TextPoint {
                block_id: "p".into(),
                offset: ScalarOffset(8),
            },
            &TextPoint {
                block_id: "p".into(),
                offset: ScalarOffset(12),
            },
        )
        .unwrap();
    assert!(
        workspace
            .stage_assistant_selection_replacements(&project.id, &selection, &[], "Wrong count")
            .is_err()
    );
    let mut forged = selection;
    forged.ranges[0].start = ScalarOffset(3);
    assert!(
        workspace
            .stage_assistant_selection_replacements(
                &project.id,
                &forged,
                &["changed".into()],
                "Altered range"
            )
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert!(
        workspace
            .rich_proposals(&project.id, &document.id, None, 10)
            .unwrap()
            .is_empty()
    );
}
