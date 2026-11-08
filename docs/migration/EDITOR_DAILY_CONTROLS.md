# Daily editor controls: source audit and implementation

Status: implementation checkpoint for issues #4/#7; native acceptance remains pending. No full editor acceptance or alpha.2 release is claimed.

## Source contract

The original `manuscript-editor.tsx` Find action opens an input dialog and selects only the first case-insensitive occurrence in the current document. The toolbar and overflow invoke the same action; Cmd/Ctrl+F is the keyboard route. There is no editor Replace All requirement. The original searches individual text leaves; the native query fixes its formatting-run gap by searching a whole paragraph/heading/code leaf while preserving paragraph and opaque-atom boundaries.

The title is independently editable metadata: double-click to begin, Enter or blur to commit, Escape to cancel. The normalized value must be nonempty, changed and at most 200 Unicode scalars. `handleUpdateNode` and `UpdateDocumentNode` validate/update title metadata without checking content schema editability or creating a text revision. Native title changes therefore preserve read-only imported content, rich JSON, drafts and immutable content history. Project ownership and the exact captured title guard against the wrong document or a concurrent rename.

The source footer displays words and optional Play source. Its separate statistics panel displays words, Unicode-scalar characters, estimated reading minutes at 180 words per minute, and an optional global word goal. A settings hint mentions footer goal progress, but the inspected footer does not implement that progress display. The existing native interface must not infer an additional progress bar from that hint.

The original preferences support Sofia Sans/Inter/Source Serif 4; 17/19/21 px; line spacing 1.5/1.75/2; paragraph spacing 0.65/0.9/1.1 em; list spacing 0/0.25/0.5 em; manuscript measure 60/70/80 ch; Auto/Expanded/Compact toolbar; spellcheck; and a rounded/clamped 0–1,000,000 word goal. Source autoplay is a persisted integration preference; no caret-driven playback is invented. These settings remain required and are not implemented merely by adding the Find/title controls.

## Native and shared core boundary

`daily_controls.rs` owns the bounded Find modal and inline title state. The host calls `commit_title_edit()` before explicit Save, navigation or close; failures retain the draft and original context. Title focus uses the emitted field ID and composition state, so a retained blurred draft does not capture unrelated shortcuts. New/completed/cancelled title sessions clear native Undo state; failed saves preserve it. Native fields use the shared 40 px component and modal inset/footer geometry. Find changes only the shared semantic selection and scroll/focus request. A document/project/session revision change rejects stale Find resolution.

The shared core exposes pure `find_first_text_match`, `find_first_document_match`, `document_statistics`, and `rich_document_statistics`, plus `Workspace::rename_document`. Case-insensitive matches map lowercase expansions back to whole original Unicode scalars. The rich Find query traverses nested text leaves, including table cells/code, without searching source/annotation attributes. Statistics use an explicit original-editor projection rather than redefining canonical `Document::plain_text` or native caret coordinates.

## Source probe and verification

`probe-legacy-editor-statistics.ts` imports the actual original `editorExtensions` and installed Tiptap `getSchema`, `getText` and schema text serializers. It accepts the original checkout and a separate output path and writes no original files. The synthetic input covers every registered node and all six marks. It never opens a running app, clipboard or manuscript.

```sh
bun scripts/probe-legacy-editor-statistics.ts /path/to/original-checkout /tmp/legacy-statistics.json
```

The 2026-11-08 probe found 21 registered nodes, six marks, and only the `hardBreak` text serializer. The authored compatibility fixtures record these results:

| Fixture | Words | Unicode scalars | Reading minutes |
|---|---:|---:|---:|
| Paragraphs, hard break, inline math and Unicode whitespace | 7 | 33 | 1 |
| Nested quote/lists/callout/table | 6 | 61 | 1 |
| Headings, code and opaque blocks | 6 | 40 | 1 |

`rich_statistics_match_original_registered_tiptap_all_node_probe` compares the native core calculation against those source-derived fixtures and verifies JSON immutability. Other focused core tests cover lowercase expansion/contextual Greek, combining/emoji positions, styled runs, table/code traversal, paragraph/atom boundaries, statistics whitespace, title ownership/staleness/limits and save/reopen preservation. Native module tests cover Find session/revision guards, selection-only behavior, Escape, title draft recovery, IME and Undo preservation.

Static rustfmt checks and the actual original-source Bun probe were run. Cargo build/fetch/test/Clippy were deliberately not run locally under the current disk constraint; hosted Rust checks and actual compiled native interaction/independent designer acceptance remain required.
