#![expect(
    clippy::unwrap_used,
    reason = "Synthetic compatibility fixtures fail immediately"
)]
use craft_editor_model::Document;
use serde_json::{Value, json};
use studio_core::{AnchorStatus, AnnotationAnchor, LegacyAnchorProjection};
fn atom_document() -> Document {
    Document::from_json(json!({"type":"doc","attrs":{"schemaVersion":4},"content":[{"type":"paragraph","attrs":{"blockId":"p"},"content":[{"type":"text","text":"中😀"},{"type":"inlineMath","attrs":{"latex":"x^2"}},{"type":"hardBreak"},{"type":"text","text":"後の文"}]}]})).unwrap()
}
fn original(id: &str, start: usize, end: usize, exact: &str, prefix: &str, suffix: &str) -> Value {
    json!({"blockId":id,"start":start,"end":end,"exactText":exact,"prefix":prefix,"suffix":suffix,"vendor":{"original":null}})
}
#[test]
fn original_frontend_omitted_atoms_convert_to_exact_native_text_leaf_boundaries() {
    let tree = atom_document();
    let original = original("p", 2, 4, "後の", "中😀", "文");
    let resolved = AnnotationAnchor::adapt_legacy(&tree, &original, None).unwrap();
    assert_eq!(resolved.original, original);
    assert_eq!(
        resolved.matching_projections,
        vec![LegacyAnchorProjection::Frontend]
    );
    let native = resolved.native.unwrap();
    assert_eq!(
        (native.start.0, native.end.0, native.exact_text.as_str()),
        (4, 6, "後の")
    );
    assert_eq!(resolved.anchor_status, AnchorStatus::Attached);
}
#[test]
fn original_backend_math_labels_convert_only_at_whole_native_atom_boundaries() {
    let tree = atom_document();
    let label = "[Формула: x^2]";
    let end = 2 + label.chars().count();
    let original = original("p", 2, end, label, "中😀", "\n後の文");
    let resolved =
        AnnotationAnchor::adapt_legacy(&tree, &original, Some(LegacyAnchorProjection::Backend))
            .unwrap();
    let native = resolved.native.unwrap();
    assert_eq!(
        (native.start.0, native.end.0, native.exact_text.as_str()),
        (2, 3, "\u{fffc}")
    );
    assert_eq!(resolved.original, original);
}
#[test]
fn interior_expanded_math_label_has_no_fabricated_native_caret() {
    let tree = atom_document();
    let text = "中😀[Формула: x^2]\n後の文";
    let prefix: String = text.chars().take(3).collect();
    let exact: String = text.chars().skip(3).take(4).collect();
    let suffix: String = text.chars().skip(7).collect();
    let original = original("p", 3, 7, &exact, &prefix, &suffix);
    let resolved = AnnotationAnchor::adapt_legacy(&tree, &original, None).unwrap();
    assert_eq!(resolved.anchor_status, AnchorStatus::Detached);
    assert!(resolved.native.is_none());
    assert_eq!(resolved.original, original);
}
#[test]
fn identical_legacy_text_with_different_physical_newlines_is_explicitly_ambiguous() {
    let tree=Document::from_json(json!({"type":"doc","attrs":{"schemaVersion":4},"content":[{"type":"blockquote","attrs":{"blockId":"q"},"content":[{"type":"paragraph","attrs":{"blockId":"a"},"content":[{"type":"text","text":"echo"},{"type":"hardBreak"}]},{"type":"paragraph","attrs":{"blockId":"b"},"content":[{"type":"text","text":"echo"}]}]}]})).unwrap();
    let original = original("q", 4, 5, "\n", "echo", "echo");
    let ambiguous = AnnotationAnchor::adapt_legacy(&tree, &original, None).unwrap();
    assert!(ambiguous.native.is_none());
    assert_eq!(ambiguous.matching_projections.len(), 2);
    assert_eq!(ambiguous.original, original);
    let frontend =
        AnnotationAnchor::adapt_legacy(&tree, &original, Some(LegacyAnchorProjection::Frontend))
            .unwrap()
            .native
            .unwrap();
    let backend =
        AnnotationAnchor::adapt_legacy(&tree, &original, Some(LegacyAnchorProjection::Backend))
            .unwrap()
            .native
            .unwrap();
    assert_eq!((frontend.start.0, frontend.end.0), (5, 6));
    assert_eq!((backend.start.0, backend.end.0), (4, 5));
}
#[test]
fn two_paragraph_legacy_quote_uses_intended_parent_projection_not_broken_pm_locator() {
    let tree=Document::from_json(json!({"type":"doc","content":[{"type":"blockquote","attrs":{"blockId":"q"},"content":[{"type":"paragraph","attrs":{"blockId":"a"},"content":[{"type":"text","text":"First α"}]},{"type":"paragraph","attrs":{"blockId":"b"},"content":[{"type":"text","text":"Second 😀"}]}]}]})).unwrap();
    let original = original("q", 3, 14, "st α\nSecond", "Fir", " 😀");
    let resolution = AnnotationAnchor::adapt_legacy(&tree, &original, None).unwrap();
    let anchor = resolution.native.unwrap();
    assert_eq!(anchor.exact_text, "st α\nSecond");
    let (a, b) = anchor.selection(&tree).unwrap();
    assert_eq!((a.block_id.as_str(), a.offset.0), ("a", 3));
    assert_eq!((b.block_id.as_str(), b.offset.0), ("b", 6));
}
#[test]
fn invalid_stored_quote_or_context_does_not_search_a_repeated_occurrence() {
    let tree = Document::plain("p", "echo echo").unwrap();
    for original in [
        original("p", 0, 4, "echo", "wrong", " echo"),
        original("p", 1, 5, "echo", "e", "echo"),
    ] {
        let resolution = AnnotationAnchor::adapt_legacy(&tree, &original, None).unwrap();
        assert!(resolution.native.is_none());
        assert_eq!(resolution.original, original);
    }
}
