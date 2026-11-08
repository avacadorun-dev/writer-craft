#![expect(
    clippy::unwrap_used,
    reason = "Synthetic fixture failures must fail immediately"
)]
use craft_editor_model::{Document as RichDocument, ScalarOffset};
use serde_json::json;
use studio_core::{
    StudioError, Workspace, document_statistics, find_first_document_match, find_first_text_match,
};

fn fixture() -> RichDocument {
    RichDocument::from_json(json!({"type":"doc","attrs":{"schemaVersion":4},"content":[
        {"type":"paragraph","attrs":{"blockId":"intro","sourceRefs":["search is not provenance"]},"content":[
            {"type":"text","text":"Я 😀 é И"},
            {"type":"text","text":"щу текст","marks":[{"type":"bold"}]},
            {"type":"hardBreak"},
            {"type":"inlineMath","attrs":{"latex":"hidden_formula","description":"not reader prose"}},
            {"type":"text","text":" После"}
        ]},
        {"type":"table","attrs":{"blockId":"table"},"content":[{"type":"tableRow","content":[
            {"type":"tableCell","attrs":{"colspan":1,"rowspan":1},"content":[
                {"type":"paragraph","attrs":{"blockId":"cell"},"content":[{"type":"text","text":"Таблица"}]}
            ]}
        ]}]},
        {"type":"codeBlock","attrs":{"blockId":"code","language":"rust"},"content":[{"type":"text","text":"fn Needle() {}"}]},
        {"type":"paragraph","attrs":{"blockId":"later"},"content":[{"type":"text","text":"Ищу текст"}]}
    ]})).unwrap()
}

#[test]
fn find_maps_lowercase_expansion_back_to_whole_original_scalars() {
    assert_eq!(find_first_text_match("😀 İSTANBUL", "i̇st"), Some(2..5));
}

#[test]
fn find_after_expansion_uses_original_scalar_positions() {
    assert_eq!(find_first_text_match("İ 😀 конец", "КОНЕЦ"), Some(4..9));
}

#[test]
fn find_preserves_combining_marks_and_non_bmp_scalar_offsets() {
    assert_eq!(find_first_text_match("Я 👩‍💻 é слово", "É"), Some(6..8));
}

#[test]
fn empty_find_query_has_no_match() {
    assert_eq!(find_first_text_match("unchanged", ""), None);
}

#[test]
fn find_returns_first_occurrence_across_formatting_runs_without_mutating_json() {
    let document = fixture();
    let before = document.to_json().unwrap();
    let found = find_first_document_match(&document, "ищу текст")
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            found.anchor.block_id.as_str(),
            found.anchor.offset,
            found.head.offset
        ),
        ("intro", ScalarOffset(7), ScalarOffset(16))
    );
    assert_eq!(document.to_json().unwrap(), before);
}

#[test]
fn find_visits_table_text_leaves_with_their_exact_native_identity() {
    let found = find_first_document_match(&fixture(), "ТАБЛИЦА")
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            found.anchor.block_id.as_str(),
            found.anchor.offset,
            found.head.offset
        ),
        ("cell", ScalarOffset(0), ScalarOffset(7))
    );
}

#[test]
fn find_visits_code_without_searching_the_language_label() {
    let found = find_first_document_match(&fixture(), "needle")
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            found.anchor.block_id.as_str(),
            found.anchor.offset,
            found.head.offset
        ),
        ("code", ScalarOffset(3), ScalarOffset(9))
    );
    assert!(
        find_first_document_match(&fixture(), "rust")
            .unwrap()
            .is_none()
    );
}

#[test]
fn find_after_inline_atom_keeps_hard_break_and_atom_scalar_positions() {
    let found = find_first_document_match(&fixture(), "ПОСЛЕ")
        .unwrap()
        .unwrap();
    assert_eq!(
        (found.anchor.offset, found.head.offset),
        (ScalarOffset(19), ScalarOffset(24))
    );
}

#[test]
fn find_does_not_search_opaque_formula_source_or_provenance() {
    let document = fixture();
    for query in [
        "hidden_formula",
        "not reader prose",
        "search is not provenance",
        "текст\n\u{fffc} После",
    ] {
        assert!(
            find_first_document_match(&document, query)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn find_does_not_join_separate_paragraphs_into_one_match() {
    let document = RichDocument::from_json(json!({"type":"doc","content":[
        {"type":"paragraph","attrs":{"blockId":"one"},"content":[{"type":"text","text":"first"}]},
        {"type":"paragraph","attrs":{"blockId":"two"},"content":[{"type":"text","text":"second"}]}
    ]}))
    .unwrap();
    assert!(
        find_first_document_match(&document, "first\nsecond")
            .unwrap()
            .is_none()
    );
}

#[test]
fn statistics_match_whitespace_words_scalar_characters_and_180_wpm() {
    let text = " Я 😀 é\n\n日本語 ";
    let stats = document_statistics(text);
    assert_eq!(
        (stats.words, stats.characters, stats.reading_minutes),
        (4, text.chars().count(), 1)
    );
    assert_eq!(document_statistics("").reading_minutes, 0);
    assert_eq!(document_statistics(&"word ".repeat(181)).reading_minutes, 2);
}

#[test]
fn rename_preserves_rich_content_history_and_durable_draft_across_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite3");
    let workspace = Workspace::open(&path).unwrap();
    let project = workspace.create_project("Synthetic project").unwrap();
    let rich = fixture().to_json().unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Old title", rich.clone())
        .unwrap();
    let draft = workspace
        .save_rich_draft(&document.id, document.revision, None, rich)
        .unwrap();
    let history = workspace.rich_revisions(&document.id).unwrap();
    let renamed = workspace
        .rename_document(&project.id, &document.id, "Old title", "  Новая 😀  ")
        .unwrap();
    assert_eq!(renamed.title, "Новая 😀");
    assert_eq!(
        (renamed.revision, &renamed.rich_json, &renamed.body),
        (document.revision, &document.rich_json, &document.body)
    );
    assert_eq!(workspace.rich_revisions(&document.id).unwrap(), history);
    assert_eq!(workspace.rich_draft(&document.id).unwrap(), Some(draft));
    drop(workspace);
    let reopened = Workspace::open(&path).unwrap();
    assert_eq!(reopened.document(&document.id).unwrap(), renamed);
}

#[test]
fn rename_refuses_another_project_and_preserves_the_document() {
    let workspace = Workspace::memory().unwrap();
    let owner = workspace.create_project("Owner").unwrap();
    let other = workspace.create_project("Other").unwrap();
    let document = workspace
        .create_document(&owner.id, "Title", "Synthetic text")
        .unwrap();
    assert!(matches!(
        workspace.rename_document(&other.id, &document.id, "Title", "Changed"),
        Err(StudioError::NotFound { .. })
    ));
    assert_eq!(workspace.document(&document.id).unwrap(), document);
}

#[test]
fn rename_refuses_stale_title_without_overwriting_the_newer_one() {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Project").unwrap();
    let document = workspace
        .create_document(&project.id, "Initial", "Text")
        .unwrap();
    let renamed = workspace
        .rename_document(&project.id, &document.id, "Initial", "First rename")
        .unwrap();
    assert!(
        workspace
            .rename_document(&project.id, &document.id, "Initial", "Stale rename")
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), renamed);
}

#[test]
fn rename_validates_whole_unicode_scalars_without_truncating() {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Project").unwrap();
    let document = workspace
        .create_document(&project.id, "Initial", "Text")
        .unwrap();
    let renamed = workspace
        .rename_document(&project.id, &document.id, "Initial", &"📖".repeat(200))
        .unwrap();
    assert_eq!(renamed.title.chars().count(), 200);
    assert!(
        workspace
            .rename_document(&project.id, &document.id, &renamed.title, &"📖".repeat(201))
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), renamed);
}

#[test]
fn rename_preserves_read_only_imports_while_allowing_title_metadata() {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Project").unwrap();
    let document = workspace
        .import_document(
            &project.id,
            "Original",
            "Preview",
            json!({"opaque":"preserve"}),
        )
        .unwrap();
    let renamed = workspace
        .rename_document(&project.id, &document.id, "Original", "Changed")
        .unwrap();
    assert_eq!(
        (
            &renamed.source_json,
            &renamed.body,
            renamed.read_only,
            renamed.revision
        ),
        (
            &document.source_json,
            &document.body,
            true,
            document.revision
        )
    );
    assert!(
        workspace
            .rename_document(&project.id, &document.id, "Changed", "  ")
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), renamed);
}

#[test]
fn find_matches_contextual_greek_lowercase_without_losing_source_boundaries() {
    assert_eq!(find_first_text_match("😀 ΟΣ", "ος"), Some(2..4));
}

#[test]
fn statistics_use_original_javascript_whitespace_rules() {
    assert_eq!(document_statistics("one\u{feff}two\u{0085}three").words, 2);
}

#[test]
fn rich_statistics_match_original_registered_tiptap_all_node_probe() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/legacy_editor_statistics.json")).unwrap();
    for fixture in fixtures.as_array().unwrap() {
        let document = RichDocument::from_json(fixture["input"].clone()).unwrap();
        let original = document.to_json().unwrap();
        let expected = document_statistics(fixture["text"].as_str().unwrap());
        let actual = studio_core::rich_document_statistics(&document);
        assert_eq!(actual, expected, "{}", fixture["name"]);
        assert_eq!(actual.words, fixture["words"].as_u64().unwrap() as usize);
        assert_eq!(
            actual.characters,
            fixture["characters"].as_u64().unwrap() as usize
        );
        assert_eq!(
            actual.reading_minutes,
            fixture["readingMinutes"].as_u64().unwrap() as usize
        );
        assert_eq!(document.to_json().unwrap(), original);
    }
}
