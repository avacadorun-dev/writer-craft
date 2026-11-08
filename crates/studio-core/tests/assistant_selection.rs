//! Exact source-compatible capture/reveal contracts, without provider or GUI side effects.
use craft_editor_model::{ScalarOffset, TextPoint};
use serde_json::{Value, json};
use studio_core::{AssistantSelectionAttachment, Document, StudioError, Workspace};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn point(block: &str, offset: usize) -> TextPoint {
    TextPoint {
        block_id: block.into(),
        offset: ScalarOffset(offset),
    }
}
fn paragraph(id: &str, text: &str) -> Value {
    json!({"type":"paragraph","attrs":{"blockId":id},"content":[{"type":"text","text":text}]})
}
fn setup() -> Result<(Workspace, String, Document), Box<dyn std::error::Error>> {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Synthetic selection")?;
    let content = json!({"type":"doc","attrs":{"schemaVersion":4,"future":{"retain":true}},"content":[
        paragraph("p","one 😀 two"),
        {"type":"heading","attrs":{"blockId":"heading","level":2},"content":[{"type":"text","text":"repeat repeat"}]},
        {"type":"bulletList","content":[{"type":"listItem","content":[paragraph("nested","DO NOT EXPAND")]}]},
        {"type":"blockquote","attrs":{"blockId":"quote"},"content":[paragraph("quote-first","first"),paragraph("quote-second","second😀")]},
        {"type":"paragraph","attrs":{"blockId":"atoms"},"content":[{"type":"text","text":"A😀"},{"type":"inlineMath","attrs":{"latex":"x+y","description":"Not extra context"}},{"type":"hardBreak"},{"type":"text","text":"B"}]}
    ]});
    let document = workspace.create_rich_document(&project.id, "Selected source", content)?;
    Ok((workspace, project.id, document))
}
#[test]
fn reverse_multiblock_unicode_selection_retains_exact_ranges_without_expanding_unsupported_text()
-> TestResult {
    let (workspace, project, document) = setup()?;
    let first = point("p", 4);
    let last = point("quote-second", 6);
    let forward =
        workspace.capture_assistant_selection(&project, &document.id, 1, &first, &last)?;
    let reverse =
        workspace.capture_assistant_selection(&project, &document.id, 1, &last, &first)?;
    assert_eq!(forward, reverse);
    assert_eq!(forward.quote, "😀 two\n\nrepeat repeat\n\nfirst\nsecond");
    assert_eq!(
        forward
            .ranges
            .iter()
            .map(|range| range.block_id.as_str())
            .collect::<Vec<_>>(),
        vec!["p", "heading", "quote"]
    );
    assert_eq!((forward.ranges[0].start.0, forward.ranges[0].end.0), (4, 9));
    assert_eq!(
        (forward.ranges[2].start.0, forward.ranges[2].end.0),
        (0, 12)
    );
    assert_eq!(
        workspace.assistant_selection_boundaries(&project, &forward)?,
        (first, last)
    );
    assert_eq!(workspace.document(&document.id)?, document);
    assert_eq!(workspace.rich_revisions(&document.id)?.len(), 1);
    assert!(workspace.annotations(&document.id)?.is_empty());
    assert!(workspace.proposals(&document.id)?.is_empty());
    Ok(())
}
#[test]
fn inline_atom_omission_and_hard_breaks_have_explicit_source_and_native_coordinates() -> TestResult
{
    let (workspace, project, document) = setup()?;
    let attachment = workspace.capture_assistant_selection(
        &project,
        &document.id,
        1,
        &point("atoms", 1),
        &point("atoms", 5),
    )?;
    assert_eq!(attachment.quote, "😀\nB");
    assert_eq!(
        (attachment.ranges[0].start.0, attachment.ranges[0].end.0),
        (1, 4)
    );
    assert_eq!(
        workspace.assistant_selection_boundaries(&project, &attachment)?,
        (point("atoms", 1), point("atoms", 5))
    );
    assert!(
        workspace
            .capture_assistant_selection(
                &project,
                &document.id,
                1,
                &point("atoms", 2),
                &point("atoms", 3)
            )
            .is_err()
    );
    assert!(!attachment.quote.contains("x+y"));
    assert!(!attachment.quote.contains("Not extra context"));
    Ok(())
}
#[test]
fn unsupported_empty_container_and_out_of_bounds_selections_create_no_attachment_or_history()
-> TestResult {
    let (workspace, project, document) = setup()?;
    for (first, last) in [
        (point("nested", 0), point("nested", 3)),
        (point("p", 3), point("p", 3)),
        (point("quote", 0), point("quote", 3)),
        (point("p", 99), point("p", 100)),
        (point("unknown", 0), point("p", 2)),
    ] {
        assert!(
            workspace
                .capture_assistant_selection(&project, &document.id, 1, &first, &last)
                .is_err()
        );
    }
    assert_eq!(workspace.document(&document.id)?, document);
    assert_eq!(workspace.rich_revisions(&document.id)?.len(), 1);
    Ok(())
}
#[test]
fn untrusted_attachment_cannot_change_quotes_reorder_duplicate_or_omit_interior_supported_ranges()
-> TestResult {
    let (workspace, project, document) = setup()?;
    let original = workspace.capture_assistant_selection(
        &project,
        &document.id,
        1,
        &point("p", 4),
        &point("quote-second", 6),
    )?;
    let serialized = serde_json::to_vec(&original)?;
    let decoded: AssistantSelectionAttachment = serde_json::from_slice(&serialized)?;
    assert_eq!(
        workspace.assistant_selection_boundaries(&project, &decoded)?,
        (point("p", 4), point("quote-second", 6))
    );
    let mut cases = vec![];
    let mut bad = original.clone();
    bad.quote.push('!');
    cases.push(bad);
    let mut bad = original.clone();
    bad.ranges[0].expected_text = "other occurrence".into();
    cases.push(bad);
    let mut bad = original.clone();
    bad.ranges.swap(0, 1);
    cases.push(bad);
    let mut bad = original.clone();
    bad.ranges.insert(1, bad.ranges[0].clone());
    cases.push(bad);
    let mut bad = original.clone();
    bad.ranges.remove(1);
    bad.quote = bad
        .ranges
        .iter()
        .map(|r| r.expected_text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    cases.push(bad);
    let mut bad = original.clone();
    bad.ranges[1].start = ScalarOffset(7);
    bad.ranges[1].expected_text = "repeat".into();
    bad.quote = bad
        .ranges
        .iter()
        .map(|r| r.expected_text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    cases.push(bad);
    let mut bad = original.clone();
    bad.ranges[0].block_id = "nested".into();
    cases.push(bad);
    let mut bad = original.clone();
    bad.project_id = "different".into();
    cases.push(bad);
    for bad in cases {
        assert!(
            workspace
                .assistant_selection_boundaries(&project, &bad)
                .is_err()
        );
    }
    assert_eq!(workspace.document(&document.id)?, document);
    Ok(())
}
#[test]
fn repeated_text_reveal_uses_the_captured_occurrence_and_formatting_only_revision_makes_it_stale()
-> TestResult {
    let (workspace, project, document) = setup()?;
    let attached = workspace.capture_assistant_selection(
        &project,
        &document.id,
        1,
        &point("heading", 7),
        &point("heading", 13),
    )?;
    assert_eq!(attached.quote, "repeat");
    assert_eq!(
        workspace.assistant_selection_boundaries(&project, &attached)?,
        (point("heading", 7), point("heading", 13))
    );
    let mut changed = document.rich_json.clone().ok_or("rich fixture")?;
    changed["content"][1]["content"][0]["marks"] = json!([{"type":"bold"}]);
    let changed = workspace.save_rich_document(&document.id, 1, changed)?;
    assert_eq!(changed.body, document.body);
    assert_eq!(changed.revision, 2);
    assert!(matches!(
        workspace.assistant_selection_boundaries(&project, &attached),
        Err(StudioError::RevisionConflict {
            expected: 1,
            actual: 2
        })
    ));
    Ok(())
}
#[test]
fn unacknowledged_draft_and_foreign_project_are_refused_without_mutating_either_version()
-> TestResult {
    let (workspace, project, document) = setup()?;
    let attached = workspace.capture_assistant_selection(
        &project,
        &document.id,
        1,
        &point("heading", 7),
        &point("heading", 13),
    )?;
    let foreign = workspace.create_project("Other")?;
    assert!(
        workspace
            .capture_assistant_selection(
                &foreign.id,
                &document.id,
                1,
                &point("p", 0),
                &point("p", 3)
            )
            .is_err()
    );
    let content = document.rich_json.clone().ok_or("rich fixture")?;
    workspace.save_rich_draft(&document.id, 1, None, content)?;
    let draft = workspace.rich_draft(&document.id)?;
    assert!(
        workspace
            .capture_assistant_selection(&project, &document.id, 1, &point("p", 0), &point("p", 3))
            .is_err()
    );
    assert!(
        workspace
            .assistant_selection_boundaries(&project, &attached)
            .is_err()
    );
    assert_eq!(workspace.rich_draft(&document.id)?, draft);
    assert_eq!(workspace.document(&document.id)?, document);
    Ok(())
}
