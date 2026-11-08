#![expect(
    clippy::unwrap_used,
    reason = "Synthetic annotation source-parity fixtures fail immediately"
)]
use craft_editor_model::{
    Command, Document as RichTree, Editor, Revision, ScalarOffset, TextPoint, TextRange,
    Transaction,
};
use serde_json::json;
use studio_core::{AnchorStatus, AnnotationAnchor, Workspace};
#[test]
fn native_endpoint_affinities_match_the_actual_original_plugin_probe() {
    // Actual annotations.ts hash and headless ProseMirror evidence are recorded
    // in NATIVE_ANNOTATION_MAPPING.md; offsets below are Unicode scalars.
    let cases = [
        (10, 10, "X", Some((8, 13, "ecXho"))),
        (9, 10, "", Some((8, 11, "eho"))),
        (9, 10, "XY", Some((8, 13, "eXYho"))),
        (8, 12, "новое😀", Some((8, 14, "новое😀"))),
        (8, 12, "", None),
        (8, 8, "X", Some((9, 13, "echo"))),
        (12, 12, "X", Some((8, 12, "echo"))),
        (8, 9, "", Some((8, 11, "cho"))),
        (11, 12, "", Some((8, 11, "ech"))),
        (7, 10, "", None),
        (10, 13, "", None),
    ];
    for (start, end, replacement, expected) in cases {
        let workspace = Workspace::memory().unwrap();
        let project = workspace.create_project("Source mapping").unwrap();
        let mut editor = Editor::new(
            RichTree::plain("p", "Я😀 echo echo tail").unwrap(),
            Revision(1),
        );
        let document = workspace
            .create_rich_document(&project.id, "Notes", editor.document().to_json().unwrap())
            .unwrap();
        let anchor = workspace
            .capture_annotation_anchor(
                &document.id,
                document.revision,
                "p",
                ScalarOffset(8),
                ScalarOffset(12),
                "echo",
            )
            .unwrap();
        let note = workspace
            .create_annotation(
                &document.id,
                document.revision,
                anchor,
                "Keep this occurrence",
            )
            .unwrap();
        let original = note.anchor.clone();
        let before = editor.document().block_text("p").unwrap();
        let change = editor
            .apply(Transaction {
                base_revision: editor.revision(),
                commands: vec![Command::ReplaceText {
                    range: TextRange {
                        block_id: "p".into(),
                        start: ScalarOffset(start),
                        end: ScalarOffset(end),
                    },
                    expected: before.chars().skip(start).take(end - start).collect(),
                    replacement: replacement.into(),
                }],
            })
            .unwrap();
        let proofs = [change.replay_proof()];
        let preview = workspace
            .annotation_preview_snapshot(&document.id, document.revision)
            .unwrap()
            .preview(&proofs, editor.document().to_json().unwrap())
            .unwrap();
        workspace
            .save_rich_editor_changes(
                &document.id,
                document.revision,
                &proofs,
                editor.document().to_json().unwrap(),
            )
            .unwrap();
        let mapped = workspace.annotation(&note.id).unwrap();
        assert_eq!(preview[0].anchor, mapped.anchor);
        if let Some((a, b, text)) = expected {
            assert_eq!(
                mapped.anchor_status,
                AnchorStatus::Attached,
                "{start}..{end}"
            );
            assert_eq!(
                (
                    mapped.anchor.start.0,
                    mapped.anchor.end.0,
                    mapped.anchor.exact_text.as_str()
                ),
                (a, b, text)
            );
        } else {
            assert_eq!(
                mapped.anchor_status,
                AnchorStatus::Detached,
                "{start}..{end}"
            );
            assert_eq!(mapped.anchor, original);
        }
    }
}
fn quote() -> RichTree {
    RichTree::from_json(json!({"type":"doc","content":[{"type":"blockquote","attrs":{"blockId":"q"},"content":[{"type":"paragraph","attrs":{"blockId":"a"},"content":[{"type":"text","text":"先😀 α"}]},{"type":"paragraph","attrs":{"blockId":"b"},"content":[{"type":"text","text":"次の文"}]}]}]})).unwrap()
}
#[test]
fn quoted_selection_spans_two_paragraphs_and_replays_proven_leaf_edits() {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Quotes").unwrap();
    let tree = quote();
    let first = TextPoint {
        block_id: "a".into(),
        offset: ScalarOffset(1),
    };
    let last = TextPoint {
        block_id: "b".into(),
        offset: ScalarOffset(2),
    };
    let anchor = AnnotationAnchor::from_selection(&tree, &last, &first).unwrap();
    assert_eq!(anchor.block_id, "q");
    assert_eq!(anchor.exact_text, "😀 α\n次の");
    assert_eq!(anchor.selection(&tree).unwrap(), (first, last));
    let document = workspace
        .create_rich_document(&project.id, "Quote", tree.to_json().unwrap())
        .unwrap();
    let note = workspace
        .create_annotation(
            &document.id,
            document.revision,
            anchor,
            "Revise this quoted passage",
        )
        .unwrap();
    let mut editor = Editor::new(tree, Revision(1));
    let change = editor
        .apply(Transaction {
            base_revision: editor.revision(),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "a".into(),
                    start: ScalarOffset(2),
                    end: ScalarOffset(3),
                },
                expected: " ".into(),
                replacement: "東京".into(),
            }],
        })
        .unwrap();
    workspace
        .save_rich_editor_changes(
            &document.id,
            document.revision,
            &[change.replay_proof()],
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    let mapped = workspace.annotation(&note.id).unwrap();
    assert_eq!(mapped.anchor_status, AnchorStatus::Attached);
    assert_eq!(mapped.anchor.exact_text, "😀東京α\n次の");
    let (a, b) = mapped.anchor.selection(editor.document()).unwrap();
    assert_eq!(a.block_id, "a");
    assert_eq!(a.offset, ScalarOffset(1));
    assert_eq!(b.block_id, "b");
    assert_eq!(b.offset, ScalarOffset(2));
}
#[test]
fn arbitrary_cross_block_selection_still_requires_one_exact_supported_parent() {
    let tree=RichTree::from_json(json!({"type":"doc","content":[{"type":"paragraph","attrs":{"blockId":"a"},"content":[{"type":"text","text":"echo"}]},{"type":"paragraph","attrs":{"blockId":"b"},"content":[{"type":"text","text":"echo"}]}]})).unwrap();
    assert!(
        AnnotationAnchor::from_selection(
            &tree,
            &TextPoint {
                block_id: "a".into(),
                offset: ScalarOffset(1)
            },
            &TextPoint {
                block_id: "b".into(),
                offset: ScalarOffset(2)
            }
        )
        .is_err()
    );
}
