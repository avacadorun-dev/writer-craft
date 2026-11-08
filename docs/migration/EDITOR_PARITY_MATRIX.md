# Complete legacy editor compatibility matrix

Status: source audit and required acceptance inventory; no editor implementation or passed native parity is claimed. The owner selected **every current original editor capability** for the next editor-focused milestone. A first technical slice is internal evidence only. Plain-text editing, preserved opaque JSON, or a subset of rich blocks cannot pass this gate.

This gate concerns the complete editor, its data guarantees and its existing integration points. It does not imply complete product parity, a full project importer, recording/transcription implementation, provider connectivity or a released MCP transport. Those boundaries must not remove existing editor behavior from acceptance: source playback, annotation/dictionary-link interaction and assistant attachment/review still need working canonical integration.

The companion [editor UX matrix](../design/EDITOR_UX_PARITY.md) owns independent interaction and presentation acceptance. [The editor spike](../research/EDITOR_SPIKE.md) is limited model evidence, not a native editor. The [feature inventory](FEATURE_PARITY.md) remains the complete product migration boundary.

## Baseline and source references

`legacy:` below means a path in the original Writers Studio working checkout. The baseline is the **current file bytes**, including uncommitted owner changes, rather than an assumption that a release tag or older prose describes them. The source identity and SHA-256 inventory at the end pin the audited files without embedding machine paths or manuscript data. Line references apply to that snapshot. Installed upstream Tiptap sources were inspected at the **3.29.2 versions fixed by `bun.lock`**; they are dependency evidence, not vendored native code.

| Ref | Current source and responsibility |
| --- | --- |
| S1 | `legacy:internal/editor/document.go:17–51,94–219,486–576,1080–1171` — allowed nodes/marks, schema gates, attribute validation and URLs |
| S2 | `legacy:src/features/editor/model/extensions.ts:18–33,95–286,297–342` — registered extensions, IDs, source references, schema promotion and source highlighting |
| S3 | `legacy:src/features/editor/model/rich-blocks.tsx:10–326` — callout/image/Mermaid schema, attributes and editable node views |
| S4 | `legacy:src/features/editor/model/math.tsx:12–337` — inline/block formulas, commands, paste, source/description and rendering failure behavior |
| S5 | `legacy:src/features/editor/model/code-block.tsx:145–217,376–572` and `code-languages.ts` — code parsing, migration, language selection, copying and syntax decorations |
| S6 | `legacy:src/features/editor/ui/formatting-toolbar.tsx:80–747` — exact exposed formatting, insertion, table and typography commands |
| S7 | `legacy:src/widgets/manuscript-editor/ui/manuscript-editor.tsx:143–1067` — editing surface, selection menu, drop, save/recovery, source playback, focus, find, statistics and proposal preview |
| S8 | `legacy:src/features/editor/model/annotations.ts:21–214`; `legacy:internal/documentannotation/annotation.go:23–120`; `legacy:internal/store/annotations.go:90–500` — decorations, portable anchors, remapping, metadata and transactional persistence |
| S9 | `legacy:src/pages/studio/ui/studio-page.tsx:106–207,268–425,624–678`; `legacy:src/widgets/manuscript-annotations/ui/manuscript-annotations-panel.tsx` — note/dictionary chooser, lifecycle, docking and deep links |
| S10 | `legacy:src/features/autosave/model/use-editor-autosave.ts:40–168` — local snapshot, debounce, flush, optimistic save and errors |
| S11 | `legacy:src/shared/lib/preferences.ts:1–205`; `legacy:src/shared/styles/globals.css` — typography, spacing, width, spellcheck, themes/palettes and editor styles |
| S12 | `legacy:internal/projectmcp/catalog.go:16–43,135–231,471–541`; `legacy:internal/projectmcp/server.go:50–95` — public tool schemas and the shared backend |
| S13 | `legacy:internal/assistantapp/backend.go:2436–2492,3250–3536,3653–3830,4104–4297,4468–4522` — canonical content normalization, insertion, exact changes, marks and protected ranges |
| S14 | `legacy:internal/editor/document.go:588–674,729–769,835–1065`; `legacy:internal/assistant/model.go:511–550`; `legacy:internal/assistantapp/backend.go:2788–2855` — projections, exact positions, attachment validation and asset ownership |
| S15 | `legacy:internal/projectmedia/image.go:18–76`; `legacy:src/features/editor/model/image-drop.ts:1–25` — image integrity, limits and actual frontend/backend media differences |
| S16 | `legacy:internal/assistantapp/native_code_roundtrip_test.go`; `native_rich_blocks_roundtrip_test.go`; `backend_test.go`; `legacy:src/features/editor/model/*.test.*`; `legacy:src/widgets/manuscript-editor/model/selection-context-menu.test.ts` — existing regression examples to adapt |

## Owner-approved native capacity exception

On 2026-11-08 the owner accepted 63 logical table columns for foreseeable alpha.2 writing scenarios; see F-30 and the UX contract. Do not fork a layout dependency solely to increase this limit. All table operations and all other editor requirements remain required within that capacity. Count logical grid columns, including merged spans, rather than cell count or pixel width.

The original canonical contract remains span 1–100 with no new 63-column parser clamp. Preserve wider existing JSON/history/media exactly, expose a clear read-only capacity state, and reject UI/MCP creation/widening/paste beyond the active limit before mutation. Never truncate, clamp, drop cells or silently downgrade data to meet a renderer limit. This is the single explicitly approved native capacity exception, not permission to reduce unrelated editor functionality. E06 native acceptance and associated corpus cases use this recorded exception; canonical preservation still tests wide structures.

## Canonical node and attribute inventory

The canonical wire shape is `Node { type, attrs?, marks?, text?, content? }`, with arbitrary JSON-valued `attrs`; `Mark { type, attrs? }`. There are **21 allowed node names and six mark names**. This is a tree, not Markdown or a flattened text buffer. Attribute absence, `null`, empty arrays and declared values must be handled deliberately; importing must not rewrite historical snapshots just to materialize defaults.

Global attributes: `doc.attrs.schemaVersion`; `blockId` on paragraph, heading, blockquote, codeBlock, callout, table, imageBlock, mermaidBlock and mathBlock; `sourceRefs` on paragraph, heading and blockquote. The backend accepts a `blockId` string on any node when supplied (maximum 160 bytes); the frontend actively assigns IDs only to the nine identified types. Preserve existing IDs outside that frontend set rather than deleting them. [S1, S2]

| ID | Node | Children / behavior | Specific attributes and preservation cases |
| --- | --- | --- | --- |
| N01 | `doc` | Ordered top-level blocks | `schemaVersion`: absent means v1; supported 1–4 |
| N02 | `paragraph` | Inline content | `blockId`; `sourceRefs` string array, default `[]` |
| N03 | `heading` | Inline content | N02 metadata plus `level` 1–6; toolbar currently exposes H1–H3 while schema supports H1–H6 |
| N04 | `blockquote` | Nested block content | `blockId`, `sourceRefs`; preserve inner paragraph IDs and nesting |
| N05 | `bulletList` | `listItem` children | No additional registered frontend attrs; preserve supplied extension attrs |
| N06 | `orderedList` | `listItem` children | Inherited `start` default 1 and `type` default null; imported nondefault starts/list numbering styles |
| N07 | `listItem` | Paragraph plus following blocks/nested lists | No additional registered attrs; do not flatten nested items |
| N08 | `hardBreak` | Inline leaf | Must retain its positional/text-projection meaning |
| N09 | `horizontalRule` | Block leaf | Distinct from scene break |
| N10 | `sceneBreak` | Selectable block atom | No custom content/attrs; distinct canonical node |
| N11 | `text` | Nonempty literal text | `text` plus zero or more canonical marks; preserve exact whitespace/Unicode |
| N12 | `codeBlock` | Literal text, no prose transformations while inside | `blockId`, optional/null `language`; valid nonempty language matches `[a-zA-Z0-9_+#.-]{1,40}`; unknown language preserves source and uses safe unhighlighted rendering |
| N13 | `table` | `tableRow` children | `blockId`; column resizing is represented by cell widths, not a new text document |
| N14 | `tableRow` | `tableCell` / `tableHeader` | No additional registered attrs |
| N15 | `tableHeader` | Block content, including paragraphs | `colspan`, `rowspan` defaults 1, validated 1–100; `colwidth` default null or width array; `align` default null, left/right/center |
| N16 | `tableCell` | Block content, including paragraphs | Same attrs as N15; retain cell identity/content through table commands |
| N17 | `imageBlock` | Selectable/draggable block atom | `blockId`, `src`, `assetId` default null, `alt`, `caption`, width/height nullable; dimensions 1–20000 when present; alt/caption maximum 1000 bytes in backend |
| N18 | `mermaidBlock` | Selectable/draggable block atom | `blockId`, nonempty `code` ≤100000 bytes, `caption` ≤1000 bytes; source stays canonical even when rendering fails |
| N19 | `callout` | `block+`, defining | `blockId`, `kind` exactly info/idea/warning/todo; retain nested rich blocks |
| N20 | `inlineMath` | Inline atom, no children | `latex` ≤10000 bytes and `description` ≤1000 bytes; source and accessible description both persist |
| N21 | `mathBlock` | Selectable/draggable block atom, no children | N20 attributes plus `blockId` |

Inherited attrs were checked in installed Tiptap 3.29.2 `extension-list/src/ordered-list/ordered-list.ts:115`, `extension-table/src/cell/table-cell.ts:33`, `extension-table/src/header/table-header.ts:33`, and `extension-link/src/link.ts:296`. Width/alignment and extended link attrs are real serialized frontend state even where the Go validator does not validate every field.

| ID | Mark | Canonical attributes / behavior |
| --- | --- | --- |
| M01 | `bold` | No registered custom attrs; preserves other compatible marks |
| M02 | `italic` | No registered custom attrs |
| M03 | `underline` | No registered custom attrs |
| M04 | `strike` | No registered custom attrs |
| M05 | `link` | `href`, `target`, `rel`, `class`, `title`; configured rel is `noopener noreferrer`, inherited target `_blank`, class/title null unless supplied |
| M06 | `code` | No registered custom attrs; literal inline code, delimiters are syntax on input/export rather than stored manuscript characters |

Link validation allows HTTP/HTTPS/mailto and the supported internal `/projects/<uuid>/studio/<uuid>` path (no foreign host/query on the internal path). Images allow HTTPS sources or the asset-bound `/api/media/<uuid>/content` route. A native asset adapter must replace the old server location while retaining actual bytes, asset identity, integrity and reference semantics. It must not fetch arbitrary original HTTP routes or turn missing media into successful migration. [S1, S14]

Schema gates are independent of transfer/archive versions: prose/basic marks/lists start at v1; rich blocks and inline code require v2; heading levels 4–6 require v3; formulas require v4. A plain v1 document opened and saved without new richer content must not be eagerly upgraded. Exact edits, duplication, history and Undo preserve applicable schema and attrs. Unknown supported attrs stay round-trippable; unknown node names, future schema versions and unsupported unsafe structures must be rejected before editing, with the original retained rather than silently serialized away. The existing model spike shows that parser success alone is not schema validation. [S1, S2]

## Full editor behavior gate

Every row below is required. “Preserved”, “rendered”, “editable”, “durable”, “UI verified” and “headless-command verified” are separate evidence columns in eventual implementation records; preservation alone is not a pass for an editable feature. The source behavior is the contract baseline; platform presentation can improve without deleting capability. The owner's explicit removal of green palettes remains the existing native preference decision.

| ID | Required current-editor capability | Acceptance cases / source |
| --- | --- | --- |
| E01 | Continuous manuscript input and selection | Mouse/keyboard selection, cross-paragraph selection, caret travel, cut/copy/paste, deletion, Enter split, Backspace/Delete join, scroll/selection retention; native Cyrillic, Western accents, Chinese/Japanese IME, combining characters and emoji; check actual input rather than only model transforms. S2/S7 |
| E02 | Paragraphs/headings/quotes | Switch P and H1–H3 through UI; preserve/render/edit H4–H6; toggle blockquote, nested quote content and exit behavior; no text or provenance loss. S2/S6 |
| E03 | All six marks/links | Toggle bold/italic/underline/strike/code; set, edit and remove a link; active selection state, collapsed-caret typing marks, mixed selections and compatible mark combinations; clipboard retains structure. S6 |
| E04 | Nested ordered/unordered lists | Create/toggle, Enter split/new item, empty-item exit, indent/outdent/lift, nested lists and nondefault ordered starts/types; change type without losing child blocks. S2/inherited StarterKit |
| E05 | Both separators | Insert/select/delete scene break and horizontal rule independently; Undo restores exact node. S2/S6 |
| E06 | Tables | Create header table, edit cells, add/delete rows and columns, delete table, resize columns, multi-cell and table selection, keyboard movement, merged-cell spans and persisted width/alignment; preserve nested cell content. S2/S6 |
| E07 | Code blocks | Insert/change language (including plain text), visible syntax highlighting, copy exact code, preserve tabs/indentation/newlines, paste fenced code as code, type/paste inline backticks as native code marks, Undo conversion without corrupting text. S5 |
| E08 | Images | Choose and drop at the actual manuscript position, visible drop indicator and upload/error state, durable verified media before node insertion, sizing/selection/move/delete, editable alt/caption, missing image state, valid PNG/JPEG/GIF; handle WebP backend/MCP input explicitly. S3/S7/S15 |
| E09 | Mermaid | Insert/edit source and caption, actual diagram preview, correct theme, select/move/delete, syntax failure and renderer-unavailable fallback retaining editable source; a source-only card cannot be reported as successful diagram rendering. S3 |
| E10 | Four callouts | Insert and edit info/idea/warning/todo with correct distinguishable presentation, nested blocks, selection, split/exit and Undo. S3/S6 |
| E11 | Both formula forms | Insert/edit inline and block LaTeX, editable accessible description, render, move/delete, paste `$...$`/`$$...$$`, preserve source on syntax/renderer failure; reject unsafe executable/HTML input rather than enabling trust. S4 |
| E12 | Clipboard interoperability | Safe rich HTML and plain-text paste, marks/lists/tables/code/image/math input paths, remove scripts/iframes/forms/SVG/event handlers and unsafe URLs; imported/canonical JSON is not passed through an HTML sanitizer and flattened. `sanitize-pasted-html.ts`, S4/S5 |
| E13 | Identity/provenance | Missing/duplicate IDs repaired only by the intended edit policy; split creates independent block ID while retaining sourceRefs; joins union/deduplicate provenance; deletion does not invent source; copies get independent IDs. S2 |
| E14 | Toolbar modes | Auto/expanded/compact, actual overflow commands, active/disabled state, keyboard-focusable named controls, no lost commands at narrow widths; presentation belongs to shared native command surface. S6 |
| E15 | Typography/preferences | Three font roles, sizes 17/19/21, line heights 1.5/1.75/2, paragraph spacing .65/.9/1.1 em, list spacing 0/.25/.5 em, manuscript widths 60/70/80 ch; persisted preferences, spellcheck toggle, light/dark/system and retained non-green palettes. Exact native font-family choice is an explicit migration/design decision, not a silent capability deletion. S11 |
| E16 | Local history | Manual Undo/Redo for text, marks, blocks, table/media changes and source conversions; keep selection/provenance/anchor state consistent; merely reloading a same-node/same-revision view does not reset caret/history. S6/S7/S10 |
| E17 | Durable edits/recovery | Debounced local draft and acknowledged revision, saving/saved/pending/local/conflict states, retry/error recovery, navigation/close flush, restart recovery, concurrent revision rejection and both versions retained. Do not copy legacy asynchronous race behavior as a requirement. S7/S10 |
| E18 | Document title and context | Existing in-editor rename, cancellation/commit behavior and title bounds; document overview, project/type/status context. This editor title affordance is required even though broad tree work is deferred. S7 |
| E19 | Focus/find/statistics | Focus enter/exit and shortcut; in-document first-match find with no-match state and Cmd/Ctrl+F; word count, Unicode-character count, reading time and word-goal progress. This is an existing editor find command, not the deferred project-search workstream. S7 |
| E20 | Source playback | Source references remain discoverable at the active block; bounded original transcript segment playback, stop at end, missing/unavailable source state, active-source highlighting and no forced editor read-only state. Recording/transcription creation is outside this editor gate. S2/S7 |
| E21 | Selection context menu | Nonempty exact selection; right-click preserves selection, Shift+F10/context-menu key, touch long press, correct menu placement/focus/Escape; no permanent toolbar created merely by selecting text. S7/context-menu helper |
| E22 | Persistent notes | Single supported block selection → note; persistent decoration after focus/reopen, exact reveal, editable body, resolve/reopen/delete, detached readability and explicit verified reattachment; no author note leaked into reader text. S8/S9 |
| E23 | Dictionary links | Existing-term search/selection, real term reference and readable deleted-term snapshot, distinct highlight, click/reveal/panel and project-list/deep-link behavior; preserve annotations on duplicate/conflict copies. Creating a complete new dictionary administration product is outside the editor gate. S8/S9 |
| E24 | Assistant selection attachment | Capture one or multiple supported block ranges, exact quote/base revision, flush before capture, visible attachment, reveal/remove/replace; never send silently expanded manuscript scope. S7/S14 |
| E25 | Assistant proposal integration | Exact before/after or structured preview remains separate from canonical editor/autosave/manual Undo; a local edit invalidates preview; staged Apply/Reject/guarded Undo, no canonical writes before Apply. Provider transport need not exist to test the actual shared command/review path. S7/S13 |
| E26 | Annotation/proposal atomicity | Text, exact mapped anchors, lifecycle and history commit together; stale revision, expected-text mismatch, overlap/protected ranges and Undo conflicts cause no partial mutation; resolved and detached states remain independent. S8/S13 |
| E27 | Accessibility/localization | All commands have stable semantics, accessible labels/focus and readable errors, including nontext atoms/cell/header/annotation/source controls; eight native UI languages, legible glyph coverage and correct composition; display labels never become canonical node names. Companion UX matrix |

Browser DOM/React controls need native equivalents, not replication of nested web frames. Implicit inherited editing behavior is still behavior: native keymaps/IME/clipboard and selection need direct verification. No generic `TextEdit(String)` plus syntax decoration can satisfy table, atom and nested-block editing.

## MCP and shared command compatibility

The existing in-process assistant and MCP server both call `ProjectBackend.Call`; migration should preserve that architectural property. UI events and future MCP tools must adapt into the **same validated Rust commands and transactions**, with canonical tree/attrs, revision checks, identity mapping, history and assets owned by the core. Do not introduce separate “MCP document JSON” or mutate a detached renderer model and later flatten it into the core. A network/MCP server is not required for this milestone's headless engine acceptance; the tool-to-command adapter and exact semantics need fixtures now. [S12, S13]

Catalog objects are strict (`additionalProperties: false`), and **every declared property is required**, including explicitly nullable fields. Public `content` is `{ "blocks": [...] }`, a convenience block schema, not arbitrary canonical JSON. The backend also accepts direct raw canonical trees/strings internally; that is a wider internal path and must not be mislabeled as the advertised strict MCP input. Preserve full-fidelity canonical reads separately from convenience writes. [S12/S13]

| ID | Existing operation | Exact relevant input and semantics |
| --- | --- | --- |
| C01 | `tree.create_nodes_batch` | `parentId` nullable, `positionAfterId` nullable, `items[1..50]` each `{kind,title,content}`; initial complete content staged with node creation, array order retained; do not create an empty staged node then append to its nonexistent canonical ID |
| C02 | `document.create_draft`, `note.create` | `{title,content,parentId,positionAfterId}`; complete draft/note creation is staged, not a direct canonical write |
| C03 | `document.insert` | `{nodeId,baseRevision,position,content}`; **position is a zero-based top-level block insertion ordinal**, 0 through block count; it is not a byte/scalar/UTF-16/editor-caret offset |
| C04 | `document.append` | `{nodeId,baseRevision,content}`; append structured blocks, retain existing blocks/attrs; same content normalizer as insert/create |
| C05 | `document.append_chunk` | `{nodeId,baseRevision,sequence:1..16,targetWords:100..20000,finalChunk,text:1..9000 chars}`; sequential bounded accumulator becomes one staged operation, not one canonical revision per chunk; incomplete/error work remains explicit |
| C06 | `document.get`, `document.list_references` | Structured canonical document plus blocks/plain text; preserve nested rich nodes and provenance rather than treating convenience projection as canonical truth |
| C07 | `document.find_exact` / `_batch` | `{nodeId,revision,text,blockId:null|string}` or `{nodeId,revision,queries:[{text,blockId}]}`; server-derived half-open **block-local Unicode scalar** anchors; batch backend maximum 50, up to 100 matches/query with truncation flag; repeated literal occurrences retain distinct exact ranges |
| C08 | `document.replace_range` | `{nodeId,baseRevision,edits:[{blockId,start,end,expectedText,replacement}],protectedRanges:[{blockId,start,end,reason}]}`; exact original-revision text, nonoverlap, descending application within block, all document edits one atomic reviewed operation; zero-length insertion can use empty expected text; preserve surrounding nodes/marks/attrs |
| C09 | `document.apply_marks` | `{nodeId,baseRevision,blockId,start,end,marks:[{type,href:null|string}],protectedRanges}`; nonempty exact range. Existing behavior **adds missing mark types**, retains existing marks and does not replace an existing same-type link's attrs; `marks:[]` is not “remove all formatting” |
| C10 | `document.set_title` / `note.update` | Title command `{nodeId,title}`; note update `{nodeId,baseRevision,content}`; preserve distinction between metadata changes, complete content update and range edit |
| C11 | `project.upload_image` | `{filename:1..255,mediaType:PNG/JPEG/GIF/WebP,dataBase64}`; raw base64 bounded for 12MiB decoded bytes, inspect bytes/dimensions, return real asset and ready image block; storing unattached asset is immediate while document insertion remains reviewed |
| C12 | `annotation.propose_fixes` | `items[1..50]` containing annotationId, baseRevision, expectedText, replacement, resolve; server owns offset derivation; group by document; Apply updates text/anchor/lifecycle/history atomically, Undo compensates rather than deleting notes |

Convenience block names and fields: paragraph/heading/blockquote `{type,text}` (heading shortcut currently creates level 2); code `{type,text,language?}`; callout `{type,kind,text}`; mermaid `{type,code,caption}`; image `{type,assetId,alt,caption}`; table `{type,rows:[{cells:[{text,header,colspan:1..100,rowspan:1..100}]}]}` (1–1000 rows, 1–100 cells); bullet_list/ordered_list `{type,items:[text]}` (1–1000 items); scene_break/horizontal_rule `{type}`. Prose backticks become native code marks; code block source stays literal. The backend normalizer assigns new IDs and assistant provenance to appropriate new blocks, and supports canonical schema normalization. [S12/S13]

### Source discrepancies requiring explicit decisions

These are observed differences, not authorization to duplicate bugs or silently change a public coordinate system:

1. **Text projections disagree.** Editor annotation projection includes `hardBreak` as newline and represents inline math as `[Формула: <latex>]`; MCP exact-find/replace projection currently omits hardBreak and spells inline math `$<latex>$`. Frontend ProseMirror `textBetween` has its own atom behavior. One tested core projection/position mapping must define each command; moving offsets between these paths is unsafe. Preserve existing wire units; document any corrected behavior and fixtures. S14; `backend.go:3430–3448`.
2. **Catalog/backend math gap.** The backend supports convenience `math {latex,description}` and inline-dollar parsing, but the strict public content union has no math block variant. Full canonical math support is required; future MCP exposure needs an explicitly versioned/schema-tested correction rather than claiming it was already advertised. S12/S13.
3. **Frontend/backend validation differ.** The Go validator permits many child arrangements/unknown attrs that the frontend schema does not; its recursive allowed-name check is not full structural validation. Conversely table `colwidth`/`align`, ordered-list `type` and extended link attrs are real frontend data. Keep those values; reject malformed editable trees without destroying the original.
4. **Heading presentation and paste differ.** H4–H6 exist canonically, but toolbar style selection and HTML paste allowlist presently expose H1–H3. Native parity must preserve all six levels; improvements must be recorded rather than labeling H4–H6 unsupported.
5. **Image formats differ.** Original chooser/drop allows PNG/JPEG/GIF; backend/MCP also accept WebP. Test all persisted/backend-supported inputs and make the native chooser policy explicit.
6. **Async selection/drop races.** Source captures a drop position before upload completes, and selection anchors before an awaited flush. Native commands need revision-aware mapping or actionable rejection when text changes meanwhile; stale positions cannot silently target different content. Do not reproduce a race for superficial UI fidelity.
7. **Formatting verbs differ.** UI toggles/removes marks and edits link attrs; MCP `apply_marks` is additive. Use distinct command semantics under one engine, not one ambiguous function that changes behavior according to caller.

8. **Annotation styling bug.** `annotations.ts:196–205` checks missing dictionaryTerm for notes as well as links, incorrectly giving ordinary notes a needs-attention class. Native note styling must distinguish the annotation kind.
9. **Leaf-only find bug.** `manuscript-editor.tsx:1069` searches one text leaf at a time, so a literal spanning a mark boundary is missed. Native in-document find must resolve a logical text match back to an exact range without losing marks.
10. **Promised/reachable UI gaps are not invented existing controls.** Current annotation metadata API supports reanchoring, but the docked panel only tells the author to reattach and has no reattach action. Touch long press is documented and relies on browser context-menu delivery rather than a custom handler. Record native equivalents/fixes with an explicit acceptance case; do not claim a legacy toolbar control exists when it does not.

## Compatibility fixtures and required evidence

No production fixture files or editor code were added by this audit. The following fixture families are required additions before full acceptance; derive synthetic examples from S16 and add explicit expected canonical JSON/metadata. A screenshot is not a replacement for a model transaction assertion.

| Fixture family | Required contents and checks |
| --- | --- |
| F01 — schema corpus | v1 absent/explicit version, all base nodes/marks; v2 every rich node and inline code; v3 H1–H6; v4 both math forms; nested quotes/lists/callouts/tables; every N/M attribute including null/default/nondefault and opaque vendor attrs; deserialize/validate/serialize/edit/save/reopen without silent loss |
| F02 — full editing corpus | Each E01–E12 command via native UI and shared core: typing/split/join/copy/paste, marks/toggles, nested list indentation, table row/column/span/width/alignment, selected atoms/move/delete, code whitespace/copy/highlight, diagrams/formulas render success plus failure retaining source; Undo/Redo and restart preserve canonical tree |
| F03 — Unicode/positions | Cyrillic+emoji+combining/ZWJ/CJK, repeated quotes in one/two blocks, hardBreak, marked split text nodes, inlineMath, nested cells/list items; exact scalar↔UTF-16↔byte conversion and structural positions, no surrogate interior, original normalization retained; test every projection discrepancy explicitly |
| F04 — identity/provenance | Existing/missing/duplicate block IDs, transcript segment refs and `assistant` refs; split inherits refs but gets independent ID, join unions refs, copy regenerates relevant IDs; actual source clip is bounded and absence is truthful; assets/source media never silently rewritten or discarded |
| F05 — annotations | Note and dictionary link on the second repeated occurrence; save/reopen/focus preserves mark, note editing/resolve/reopen/delete, deleted-term snapshot, insert before/after, whole replacement, partial overlap/deletion→detached, explicit reattach, stale base rejection, duplicate/conflict preservation and complete transactional undo |
| F06 — canonical command equivalence | For create/insert/append/marks/replace, identical initial document + corresponding UI/headless/MCP-adapter command yields equivalent canonical tree, attrs, asset refs, revision and anchor/history outcome; staged proposals do not mutate canonical state; apply once, reject, stale apply, guard undo and protected-range refusal |
| F07 — assets and hostile input | Real small PNG/JPEG/GIF/WebP, valid alt/caption/dimensions; empty/oversized/corrupt/spoofed bytes, invalid asset ID/foreign project, source mismatch, failed upload, edit-during-upload, missing media; safe rich clipboard and malicious HTML/URLs; invalid spans, unknown node/mark and future schema leave no partial writes |
| F08 — durability and history | Crash/restart with dirty rich draft, autosave acknowledgement during continued typing, disk failure, stale canonical base, navigation/close, restore existing rich revisions, proposal preview invalidation; original and new data retained with actionable recovery, exact source/attrs/annotations intact |
| F09 — actual native UX | All E13–E27 behaviors, every toolbar mode/command at compact and large sizes, all typography values, dark/light, all eight UI languages, keyboard-only and actual ru/zh/ja IME/clipboard selection, spellcheck, stats/focus/find/source/annotation/assistant integrations; independent compiled captures + interaction report |

Minimum exact-range fixture: paragraph `p-repeat`, sourceRefs `["segment-a"]`, text `А😀 é повтор / повтор`; choose the **second** `повтор` using the real command's returned scalar anchor. Insert before it, format it, create a note, stage replacement, edit concurrently, reject stale Apply, then apply against a fresh base and Undo. Record the exact range rather than hand-counting. Repeat with a hardBreak and inline math before the quote, then within a nested list/table. Assert block IDs, refs, unrelated marks and note identity, not only plain text equality.

Full acceptance requires all N/M schema rows, E behavior rows and applicable C command rows to pass with F01–F09 evidence. Keep a per-row implemented/tested/visually accepted disposition; no checklist item becomes passed because its source file or Rust type exists. The isolated 12-test ProseMirror model spike can seed fixtures, but does not pass rendering/input/MCP/tool/persistence gates. The current native String editor is not evidence of rich parity. A reusable model/renderer must prove unknown-attribute, ID/provenance, position and history preservation at its adapter boundary before replacing opaque imported originals.

## Audited file identity

Captured: 2026-11-07 19:42 UTC. Original Git HEAD: `24ac9ef222700a6e3d618139b701e53abc61d6a5`. The working tree is dirty; **this is not a pristine HEAD audit**. Below, `M` is a tracked working-tree modification, `clean` means no Git change for that audited tracked file, and `dependency` identifies installed files whose package version is fixed by the audited lockfile. Hashes identify exact current bytes; no source content, author project content, credentials or local machine paths are embedded.

| Audited path | Working-tree disposition | SHA-256 |
| --- | --- | --- |
| `package.json` | M | `1d44bf40b8aebdf83c8f76360428bb24b0af4c8cad10f94a2a0fd29160d2a4ef` |
| `bun.lock` | clean | `89b8027df73faea4f58ef5ae8b31e4e44fe3af4c624825e54d6437297f36b831` |
| `internal/editor/document.go` | clean | `60b2b761cb6af26c06056b3a0494d3113d19de3a680914e56b304df14b3a83a9` |
| `internal/editor/document_test.go` | clean | `cdbe3068e6937384c78c71a8618520665c24f414bbb94cfa6350b6a2d860dcd7` |
| `internal/projectmcp/catalog.go` | clean | `b0bc584a47704fcb5dee515fa9bc7c9394fb7d29416680babedc879484858471` |
| `internal/projectmcp/server.go` | clean | `19d4bd715134f3aba03b0c84e4317f0b3b063032036cab62cb5f4b0e92f89596` |
| `internal/assistant/model.go` | clean | `578dcdbfa9c5aa542c0b54b4a79dc37c99472bd7d46f72c8324a8c2cdc1712ca` |
| `internal/assistant/policy.go` | clean | `29ab4014d763891c025111c5d6a38308c57268d66a387d467126ad4115eb672e` |
| `internal/assistant/resolver.go` | clean | `b3110772d13989c0b0bbf000715cc9216e0d53083a82752e88bf4c624d7b990b` |
| `internal/assistantapp/backend.go` | clean | `6e08a850a3ec9a79d7876a77afe2845d581a159135832764682e0e634de8aa1d` |
| `internal/assistantapp/backend_test.go` | clean | `b59a3a275fd6c3dd98d3b28301027dba1bd056cac88394b391d12d6dcbcb3f2b` |
| `internal/assistantapp/native_code_roundtrip_test.go` | clean | `f1a87a3cbb5e174f431852e56fd95bd8a75110ad5d1112b6ecbc8513a477eed5` |
| `internal/assistantapp/native_rich_blocks_roundtrip_test.go` | clean | `41fbe2572f5e4a6476f613d6000f37760409fece80f633253046f64eecc2df0a` |
| `internal/documentannotation/annotation.go` | clean | `4621c942f6e9b88bcf57b5b15b590524d89f148627eac1e24c88df431615c116` |
| `internal/store/annotations.go` | clean | `89a2fc104380e9abdbcc6e43842ebe0f47b311eebf66becee44e1188cede52ba` |
| `internal/store/assistant_annotations.go` | clean | `fed93051ceee091c6a0213979bec04a0a4d3ec76fed15a8cfca91e41526ac05b` |
| `internal/projectmedia/image.go` | clean | `79b76d6dea645aea81c7d3b26df946d2a2861a18fe62f87fea000f31eecce5a3` |
| `src/features/editor/model/extensions.ts` | clean | `cab5b92255c6e7e60a7831a9b428346f632bafbab297c515b5cef9d874eee135` |
| `src/features/editor/model/rich-blocks.tsx` | clean | `f1c6c16ae35fca4c876222c5c0f0d32ad7336ca49ec6b00c73ae61a3acf603f0` |
| `src/features/editor/model/math.tsx` | clean | `daebcffc10f0b7709f6451d1fc21c79223b2351526a4ac1e3a2a5e9fb86d206b` |
| `src/features/editor/model/code-block.tsx` | clean | `b17029f832830222a177854a443a5f82305ac01273c77efae41d7a59190a94ba` |
| `src/features/editor/model/code-languages.ts` | clean | `120a3705405d86fcf70da88917aa869bdd193cdc83dc478f7a2b478ae74b9163` |
| `src/features/editor/model/image-drop.ts` | clean | `191dce4ffcc601c77b8041e9acf902b15cdaed49ccc565b223be84010e4d070a` |
| `src/features/editor/model/sanitize-pasted-html.ts` | clean | `c155bda82758d1498475ed6066279d222727a0d363f0ee29981fc4e2dd0ff299` |
| `src/features/editor/model/annotations.ts` | clean | `e846b56f3f0b7555e84751400c79e32e650a210b9734ce9358e05a64c9806349` |
| `src/features/editor/ui/formatting-toolbar.tsx` | clean | `11732c9424293add63819da115ca49cb733e8204d2e9cb1377d94b6cbb19f181` |
| `src/widgets/manuscript-editor/ui/manuscript-editor.tsx` | M | `a6fdda486c7d9e283c635b38e43d033d6d3d875c45464296d705527c370369d9` |
| `src/widgets/manuscript-editor/model/selection-context-menu.ts` | clean | `ebbfd796bb60b9ac52e063d5601cccce439b93e6a80d0169730b09f164679e30` |
| `src/widgets/manuscript-annotations/ui/manuscript-annotations-panel.tsx` | clean | `36e39ae58e9cd4e5230e05e889ba0c4ef25e084f5f988a7777bced92717adb09` |
| `src/pages/studio/ui/studio-page.tsx` | M | `196fa6ca7c24a5fdaf7b6bcf4282f4d04a8a05cdac47675fbe8dd677cc957c24` |
| `src/pages/annotations/ui/project-annotations-page.tsx` | clean | `4a3e3590f14b89cd52dd713c62041818f6ebaa0d54e84dc12dc99e769da6f915` |
| `src/features/autosave/model/use-editor-autosave.ts` | clean | `78d1a6487d08548ca2fd75384ce34a6e37c6d4298cc8337a3e5445d7950e9d2f` |
| `src/shared/lib/preferences.ts` | clean | `31ff8ba5364c2a4e757150db79b8fae97b116ccd34d60cb5b2cbe9546bed379d` |
| `src/shared/styles/globals.css` | M | `c92070c6274271839fa7c9d2eb04af78deeaa444a0cacc9c826a5366553a4ef3` |
| `src/pages/settings/ui/settings-page.tsx` | M | `6dc06590458e9405473378ef40298674493765ae9ce53afa4642d8b004459507` |
| `docs/specification/RICH_DOCUMENT_EDITOR.md` | M | `d878b7cd48820677e0acca401ed58e0c9ac47862440a037d75d2b3934b14a6b6` |
| `docs/specification/MANUSCRIPT_ANNOTATIONS.md` | M | `7af6031ddc0632c4548c00860da99d9ea7941908ae76286a3af3a591736e0c53` |
| `docs/specification/ASSISTANT_MODE.md` | M | `a700eafa1dd164f9ca466fefd4fea7c1bac2b8f6d49c5b81ce86f9e59e2503e0` |
| `docs/specification/WORKFLOWS.md` | M | `74eed0839a4fc1dd8fa7b733e346244bdd88e90dc25385019dead02b7c0f35ea` |
| `node_modules/@tiptap/extension-table/src/cell/table-cell.ts` | dependency | `0230129df761b0be1ceae95cbc54169016024814ca53c50dca1180483c8deb30` |
| `node_modules/@tiptap/extension-table/src/header/table-header.ts` | dependency | `ade044bb65115aff88117aaf3aa6591463021f36366ba9ef780e89c943f29594` |
| `node_modules/@tiptap/extension-table/src/utils/parseAlign.ts` | dependency | `8bb023eb8762edd7e83b1263791f078ce20cd1a5b6d82f743204716b27bc026c` |
| `node_modules/@tiptap/extension-list/src/ordered-list/ordered-list.ts` | dependency | `7b8b2ced8be56689fc3a68a0c695473349e3ff2e792ba65e06b7a9793f78ede6` |
| `node_modules/@tiptap/extension-link/src/link.ts` | dependency | `a86d57e74715057a26e0a0881b461e3468225d75fc66133bd8c151cec7949a89` |
| `node_modules/@tiptap/extension-code-block/src/code-block.ts` | dependency | `29eb2e1ae54b5817a524b9241ffb9af94397d35240f9f0c70fa2310aa62e03f1` |
| `node_modules/@tiptap/extension-heading/src/heading.ts` | dependency | `33cd19ab15dcd62132694e36293ed084cf383315da71a01e7dea9497f30128bc` |
| `node_modules/@tiptap/starter-kit/src/starter-kit.ts` | dependency | `4d9992cf4ec5c6e573b6eea615095fc23fbe7a84772b36f4babd3cef1ed06f2c` |

Validation of this document is read-only source inspection. No legacy or native application code was changed, no application was built/launched, and no runtime parity test is claimed. Hashes are a baseline capture, not a completed-editor acceptance fingerprint.
