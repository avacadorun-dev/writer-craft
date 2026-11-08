# Native editor UX parity contract

Status: design and independent acceptance plan for the authorized editor-first `alpha.2` milestone. No implementation or new UX acceptance is asserted. The owner requires **all current original editor functionality, without cuts**, subject to the single explicit native table-capacity exception approved in F-30 below. A first vertical slice is an internal engineering milestone, never a substitute for the final full-editor gate. Other product streams and installation packaging remain deferred until separately selected.

The intended acceptance scope is `full-editor-parity`, distinct from `full-product` parity. The existing `0.1.0-alpha.1` native-foundation review does not approve this scope. This document does not update a release version or stamp `UX_GATE.json` accepted.

## Source of truth and audit boundary

This contract uses the actual original editor source, not only its older narrative specification. Paths below refer to the original Writers Studio repository. Its HEAD at audit time is `24ac9ef`, with pre-existing uncommitted changes; the file hashes at the end identify the content actually inspected. Coordinate schema/attribute/API fixtures with the [complete technical editor matrix](../migration/EDITOR_PARITY_MATRIX.md). Its current-source and installed-Tiptap audit complements these author journeys; neither matrix alone is an implementation pass. Never substitute a polished screenshot or a model-library test for an operational editing journey.

Primary responsibilities inspected:

- `src/features/editor/ui/formatting-toolbar.tsx`: paragraph/mark/list/quote controls, typography, Insert, table operations, link editing, Undo/Redo, Find and Focus; expanded/compact modes.
- `src/features/editor/model/extensions.ts`: schema version, stable block IDs, source-reference split/join policy, all registered editor extensions.
- `src/features/editor/model/annotations.ts`: exact portable anchors, decorations, selected-quote reveal and known supported anchor granularity.
- `src/widgets/manuscript-editor/ui/manuscript-editor.tsx`: native-migration counterpart for typing, autosave/recovery, explicit selection actions, image drop, Find, title editing, source playback, stats and noncanonical assistant previews.
- `src/features/editor/model/{code-block,rich-blocks,math,image-drop,sanitize-pasted-html}`: block-specific authoring, paste transforms, validation and failure states.
- `src/widgets/manuscript-annotations/ui/manuscript-annotations-panel.tsx` and `src/pages/studio/ui/studio-page.tsx`: real note/dictionary composer and lifecycle, not a colored-selection demo.
- `internal/editor/document.go`, `internal/documentannotation/annotation.go`: supported schema, text projection, exact positions and safe mapping.

## Complete editor capability inventory

Every row is a required target, not a completed checkbox. Unsupported future/unknown data may remain opaque and read only; a known current block cannot remain an inert source placeholder and still pass full-editor parity, except for the explicitly approved read-only handling of canonical tables above the native 63-column editing capacity (F-30).

| ID | Existing capability | Required native daily-writing interaction |
|---|---|---|
| E-01 | Document, paragraph, text, hard break; heading 1–6; blockquote | Continuous bounded manuscript. Caret/selection crosses styled runs and block boundaries correctly. Paragraph/heading chooser reflects the actual level, including 4–6. Enter/split, Backspace/join and hard breaks preserve content and identity. |
| E-02 | Six text marks: bold, italic, underline, strike, inline code, link | Apply/remove through selection or typing state, conventional keyboard actions and one formatting surface. Show active/mixed state truthfully; editing a link does not launch it. Deliberate link opening and validation remain separate. |
| E-03 | Bullet/ordered lists, list items; horizontal rule and scene break | Indent/outdent, continue/exit a list and navigate separators without trapping the caret. Preserve ordered-list start/type and source attributes; no ordinary whole-window horizontal scrolling. |
| E-04 | Table, row, header cell and ordinary cell | Insert the existing header table; cell/text/rectangular selections and keyboard traversal work. Existing row/column add/delete and table delete target the current table. Resizing and stored spans/width/alignment survive editing/paste/reopen. Native editing supports at most 63 logical columns (F-30): reject wider creation/widening atomically; preserve wider canonical tables whole with a clear read-only capacity state. The generic parser keeps span 1–100 validation and no 63-total-column cap. Preserve existing attributes even when a command has no legacy toolbar button. |
| E-05 | Code block and inline-code transforms | Native editable monospaced text, language selector, syntax decoration and exact Copy. Preserve inherited existing typing/input rules rather than just toolbar insertion. Preserve unknown stored language labels. Recognize existing fenced-code paste and inline-backtick normalization without double-transforming already formatted code. |
| E-06 | Image block | Native file picker and position-aware external drop for PNG/JPEG/GIF, existing 12MiB limit, stored dimensions/asset ID, alt text and editable caption. Persist media before committing its reference; provide pending/error/cancel states. Selected-block delete is undoable. Qualify current animated-GIF/oriented-JPEG behavior and the technical matrix's backend/MCP WebP difference rather than silently reducing decoding to a static PNG-only view. |
| E-07 | Mermaid block | Insert/edit source and caption, show actual diagram preview, surface syntax/renderer failure with readable preserved source. Selection/delete and Undo work. Rendering remains isolated from canonical text and cannot make external requests or execute embedded HTML. |
| E-08 | Four callouts: info, idea, warning, todo | Insert the actual kind with editable nested block content; distinguish kind with text/icon as well as color. Preserve nesting, kind, IDs and source metadata on copy, split and reopen. |
| E-09 | Inline and block LaTeX | Insert/edit source and accessible description; render inline baseline and block preview correctly. Whole-expression clipboard forms follow existing `$...$`/`$$...$$` recognition. Invalid/unavailable render retains source and a useful error; never truncate an author draft silently. Respect existing source-length and command restrictions. |
| E-10 | Persistent passage notes and dictionary links | Explicit selected-quote action opens real note/term selection. Save durable marks outside reader JSON; click/reveal exact quote, edit note, resolve/reopen and delete. Detached marks remain reachable with the original quote/snapshot; deletion of a term never erases the link's readable history. |
| E-11 | Selected scope and assistant preview | Freeze exact document/revision/ranges/quote only at explicit attachment. Preserve multi-block range order and show/reveal/remove the attachment. Render real supplied proposals as unapplied previews; subsequent author editing marks them stale. Do not invent model output or implement a provider merely to populate the view. |
| E-12 | Source references | Preserve sourceRefs through splits/joins/copies. Reveal the relevant source and play only its bounded interval; missing source/device errors leave text intact. Editor source playback is a real integration dependency even while recording/transcription production flows remain deferred. |
| E-13 | Autosave, immutable revisions, recovery, manual Undo/Redo | Rich content, assets and mapped annotation state remain coherent across save, conflict, restart and Undo. Saved status means actual durability. Manual editing Undo is distinct from note metadata history and guarded assistant Apply Undo. |
| E-14 | Toolbar modes and typography | Existing Auto/Expanded/Compact modes, manuscript family/size/measure, line/paragraph/list spacing, word goal and spelling preference remain functional. Preserve the existing source-autoplay preference handoff; the manuscript currently has an explicit Play source command, so do not invent automatic playback on caret movement. Font/appearance/locale changes preserve selection, draft and Undo state. |
| E-15 | Editor utility modes | Find in the current document, reversible Focus, inline document-title editing, overview/status, word/scalar-character counts and reading estimate remain reachable. Localization applies to presentation, not author content. |
| E-16 | Clipboard, native text input and accessible editing | Rich/plain cut/copy/paste, image drop, native composition, selection direction, keyboard navigation, screen-reader roles/ranges and focus ownership work on the actual native implementation. A JSON/model parser or formatted text painter alone is insufficient. |

Current node names are `doc`, `paragraph`, `heading`, `blockquote`, `bulletList`, `orderedList`, `listItem`, `hardBreak`, `horizontalRule`, `sceneBreak`, `text`, `codeBlock`, `table`, `tableRow`, `tableHeader`, `tableCell`, `imageBlock`, `mermaidBlock`, `callout`, `inlineMath`, `mathBlock`. Current marks are `italic`, `bold`, `underline`, `strike`, `link`, `code`. The technical matrix must also preserve all inherited supported attributes, schema gates and serialization/API semantics.

## Native workspace and command presentation

Keep the accepted shared desktop chrome and bounded manuscript pane. Navigator, document header, formatting controls and status do not scroll away with prose. There is no second whole-app/editor window frame and no forced publication pages.

A coherent formatting row groups Paragraph/Heading, common marks, lists/quote, Insert and overflow. Use the selected Lucide family and complete accessible command labels. At narrow widths move secondary controls into a labelled overflow while preserving every operation; do not crop commands, create a horizontally scrolling toolbar or open an unstructured wall of controls. Expanded/compact are presentation modes, not capability tiers.

The paragraph chooser must report mixed selections and all supported heading levels correctly. Toggle state follows the real selection or next-typing marks; clicking a toolbar button must preserve that selection rather than apply to a collapsed caret elsewhere. Shortcuts and native Edit/Format/Insert actions dispatch the same command path as pointer controls.

Insert exposes table, code language, image, Mermaid, inline/block formula, callout kinds and separators with explicit type names. Selected rich blocks reveal only relevant controls; the manuscript remains available. Source/code editors are bounded surfaces with Save/Apply or deliberate commit rules and Cancel/Escape. Do not let an unrelated global shortcut open another modal. Restore focus only after the destination accessibility node exists.

Large tables and long unwrapped code may scroll inside their own bounded block, with clear edges; ordinary prose and whole-window chrome do not acquire horizontal scrolling. Table context commands must name and operate on the selected table/cell range. A deleted table/row cannot leave an invisible active target.

## F-31 workspace air, density and toolbar correction

The owner rejected excessive top whitespace and the formatting strip, then required visual air and reviewable design pictures before choosing an implementation. Follow [WORKSPACE_EDITOR_REDESIGN.md](WORKSPACE_EDITOR_REDESIGN.md) and [issue #6](https://github.com/avacadorun-dev/writer-craft/issues/6): Comfort proposes 52/76/42 px and Balanced 48/56/40 px global/identity/format bars, with identical side panes and unchanged comfortable manuscript type. The owner subsequently selected **Balanced** after viewing the concepts. Its approved implementation geometry is 48 px global commands, 56 px document identity, 40 px flat formatting and a 28 px initial manuscript gutter, with a 264 px default navigator and unchanged comfortable body/control spacing. Comfort remains a comparison artifact. This design choice is distinct from native acceptance.

Remove the redundant MANUSCRIPT eyebrow, hero spacing and outer toolbar card; retain an integrated flat row, clear group separation, 32 px targets, measured grouped overflow and all capabilities. Preserve the shared shell, user manuscript typography and selected Lucide family. Do not solve space by squeezing controls or reducing body air.

After implementation, independently verify actual native heights/content origin against Balanced, live pane boundaries, vertical centers and reachable controls at 1000 × 680 and 1440 × 940, with pane resizing and expanded translations. Locale/theme/type changes and menu/overflow movement preserve exact selection, active/mixed state, caret/focus, Undo and draft. The all-21-node/six-mark interactive specimen includes real image, Mermaid and formula output/editing. SVG concepts, source strings and decorative placeholders do not pass this regression or the full-editor gate.

## F-30 native table-capacity exception

The owner explicitly accepts a maximum of 63 logical table columns for native editing in alpha.2. Do not fork WordCraft solely to increase this capacity. This is the only table-size exception selected here; all other table operations and full-editor requirements remain in scope.

- **Native interaction:** table creation, paste/insert and column-adding or span-changing commands must preflight the resulting logical grid. UI and MCP requests exceeding 63 columns return a clear capacity result before canonical content, revisions, anchors, assets or Undo history change. A disabled context control must explain the limit; an attempted shortcut/tool command must not report success.
- **Canonical preservation:** a valid existing/imported table above 63 columns remains complete, with every row/cell, span, width, alignment, text, mark and metadata attribute retained. Never truncate columns, clamp valid canonical spans, split the table or silently flatten it to fit the view. Preserve a partially received/failed-operation state atomically rather than committing a smaller table.
- **Read-only scope:** visibly state that the affected table exceeds native editing capacity and is preserved read only, including the actual column count when known. If the adapter cannot safely open the surrounding document while retaining that table intact, keep the document read only and state that scope. Do not present partial editing as safe without lossless-retention evidence.
- **Separate model contract:** the generic model/legacy parser continues to accept its existing valid span range 1–100 and has no new63-total-column validation cap. The native adapter's capacity result must not become an incompatible canonical-schema rejection.
- **Count the grid:** logical columns account for colspan and occupancy across rows; checking only the number of cell objects is insufficient. Column count is unrelated to pixel width, and the existing row/span rules are not reduced to 63.

Required future evidence: a 63-column table with normal editing and native controls; attempted creation/column insertion producing 64 columns with no mutation; a merged-cell case whose logical width exceeds 63 despite fewer cell nodes; and wider canonical data that parses/round-trips exactly and enters an explicit read-only capacity state. These are pending acceptance scenarios, not verified results.

## Selection, marks and persistent notes

1. Mouse, keyboard, reverse selection and cross-block/table selection are ordinary editing operations. Selection alone does not create a note, attach a model scope or display a permanent extra toolbar.
2. Secondary-click/control-click and the native keyboard context route preserve the original selected occurrence and expose valid Copy/Cut plus existing note/dictionary/attachment actions. Menu bounds follow the viewport and the focused range; Escape restores selection and caret.
3. Freeze semantic identity before opening a composer: document ID, revision, stable block/range identity, selection direction, literal quote and context. Never use pixels, a first-match substring or stale renderer coordinates as the anchor.
4. Distinguish selection support from annotation policy. The original note/dictionary creator accepts one supported top-level paragraph/heading/blockquote/code block; crossing blocks or selecting unsupported table/list structure gets a clear explanation and no partial annotation. Multi-block assistant attachment is a different range collection. An expanded nested/cell-note contract needs an explicit compatible anchor decision, not a silent reinterpretation.
5. The note composer displays a bounded frozen quote, focuses authored note text and saves only on the deliberate action. Cancel leaves canonical text/annotation state unchanged; a failed save keeps the draft and gives an actionable retry. A stale source cannot silently retarget the note.
6. After saving, the mark remains when the caret/focus moves, after switching panels/documents and after restart. Active text selection, note mark, dictionary link, find match, source highlight and proposed edit use distinguishable decorations in every palette. Decorations never enter reader JSON or ordinary text export.
7. The Notes inspector shows list/detail, exact quote, attached/detached state and open/resolved lifecycle separately. Clicking attached notes reveals their range; detached notes show the last safe quote/context and an explicit reattachment route. Editing one note cannot overwrite another note's draft because focus changed.
8. Dictionary linking uses real active-term results and explicit selection. Missing/deleted targets preserve a readable snapshot; a note without a dictionary target is not automatically an invalid note. Do not create NER, notifications or a new dictionary model to fill this flow.
9. Known text steps move anchors deterministically. Partial deletion/uncertain mapping detaches rather than guessing; whole-range exact replacements follow the existing deterministic policy. Note metadata history remains distinct from document editing Undo while text-step Undo restores associated anchor snapshots atomically.

## Clipboard, input and async operations

The rich model, renderer, native selection and clipboard adapters must share revision-bound typed positions. UTF-8 bytes, UTF-16 units, Unicode scalars, graphemes and tree boundaries are distinct; use explicit conversion against the same document projection. Hard breaks, inline formula atoms, table boundaries and nested blocks must have explicit projection rules. Where existing GUI/core/CLI/MCP representations differ, convert through typed semantic positions instead of reusing raw offsets or pretending the strings are identical. Qualify the conversion and error behavior against the technical matrix before changing a public adapter contract.

Copy/paste preserves supported marks/structure; copied blocks receive independent IDs and valid reference remapping. Internal rich data and sanitized external HTML/plain text are explicit formats. Remove executable/unsupported external markup while retaining author-visible semantic formatting. Preserve opaque imported content until full validation; do not flatten it merely to make editing convenient. Cutting commits deletion only after the clipboard operation succeeds. Native plain-paste deliberately strips formatting through a named conventional operation.

Native IME preedit, dead keys, candidate windows, emoji/ZWJ and combining sequences require real platform input tests. Composition is not a stream of committed canonical insertions; cancel leaves no duplicate text or stray revision. Undo groups committed author operations coherently and never restores only half a surrogate/grapheme or loses note/asset mapping. Native caret geometry follows actual layout at wrapping, zoom/type changes and bidi boundaries.

Image insertion captures its document/revision/semantic insertion point before the picker/upload starts. After media persistence, map through known edits or report a stale target; never insert at the user's later caret or into a newly opened document. Failure/cancel preserves the manuscript and avoids dangling asset references. Preview rendering and late asynchronous work are similarly generation/revision guarded.

## Existing editor handoffs versus deferred product streams

| Editor contract that must work | Broader product work that is not silently added |
|---|---|
| Real note records/lifecycle/anchors, dictionary term selection/link snapshots and the existing project-annotation list/filter/deep-link route reachable from the note panel | Rebuilding unrelated project dictionary/guide/memory management screens |
| Exact selection attachment, reveal/remove and real supplied preview/stale handling | Provider setup, model orchestration, voice assistant and fabricated model responses |
| Existing source-reference reveal and bounded audio playback; empty-editor Dictate initiation preserves the editor target/draft and emits the actual capability handoff | Recording/transcription production flows and recognition providers; their deferred status must be explicit rather than represented as working dictation |
| Rich manual Undo/Redo, autosave revisions and recovery | Separate named-version/project-history workspace unless explicitly included by the owner |
| Native insertion/media durability and known schema preservation | Complete `.wstudio` project migration, publication/export workspace or Canvas |
| Editor Find and title editing | Project-wide search/replace, navigator/tree implementation or installer packages |

The empty-editor Start writing action focuses the actual first insertion point. Dictate and project-annotation-list routes are audited source actions, not optional decorative buttons. Define the real editor-to-subsystem target, capability/result/error and return-focus contracts; never create a recording, term or model response merely to simulate the handoff.

These boundaries are not permission to hide, disable or fake an original editor action. Required lower-level data/command handoffs must be implemented and verified. Synthetic fixtures may exercise a deferred external subsystem boundary, but a scripted output is not a functioning product integration or full-product acceptance.

## Corrective behavior for audited original pitfalls

These are source-audit findings/risks, not claims of a reproduced native defect. Preserve the original capability while fixing the unsafe or misleading behavior; record exact regressions in the implementation change.

- The original toolbar dropdown reports only H1–H3 even though the schema/shortcuts support H4–H6. Native style state must not label a loaded deeper heading as a plain paragraph; expose the supported style consistently.
- Original `selectFirstMatch` searches individual text leaves, so a query spanning a formatting boundary can be missed. Native Find uses the declared document projection and returns the correct semantic range, without discarding formatting.
- Original annotation CSS classification tests for an absent dictionary target even on a note. Native notes do not become invalid merely because they are not dictionary links; lifecycle and anchor validity remain separate.
- The original asynchronous image drop retains a raw pre-upload position. Native persistence/navigation/edit races must map the captured semantic target or reject stale insertion, never insert into later text by accident.
- Original offset projections differ across adapter paths, including hard breaks and formula atoms; this is a compatibility risk, not permission to copy offsets blindly. Use explicit conversions and canonical fixtures.
- Reattachment is supported by the annotation API but lacks a complete docked UI route. The native detached-note state needs an explicit verified reattachment interaction rather than an explanation with no way forward.
- Preserve actual source-autoplay preference handoff semantics. There is no evidence for inventing automatic audio playback whenever the manuscript caret moves.
- Oversized/invalid source fields keep the local draft and a useful validation result; do not silently truncate author source or report a failed clipboard/media operation as successful.

## Prioritized internal implementation slices

1. **Semantic foundation:** validate the whole current schema/attributes and source corpus, preserve opaque originals, choose typed position/projection contracts, and make revision/anchor/asset transactions coherent. No native-view approval from model-only tests.
2. **First native vertical slice, internal only:** two paragraphs containing mixed marks, a link, Cyrillic/emoji/combining text, a repeated quote and sourceRefs, plus a table boundary. Type, reverse-select, select across runs/blocks, format, copy/paste, create/reveal a note/link, split/join, Undo/Redo, save and reopen. This exposes selection/model/input coupling before polishing every block. It cannot pass `full-editor-parity`.
3. **Complete rich authoring:** table, list, code, image, callout, Mermaid and inline/block math controls with clipboard, async error states and all current attributes. No known block is counted complete because its raw source is visible.
4. **Complete cross-cutting journeys:** native composition/accessibility, exact selection actions, source/audio handoff, note/dictionary lifecycle, preview/stale behavior, typography/modes/localization, autosave/conflict/recovery.
5. **Independent full-editor acceptance:** final compiled/native observations and the entire required matrix below. Keep all failed or unqualified editor capabilities explicit; no early slice may approve the release under a weaker scope.

## P0 preservation and authority scenarios

All cases use synthetic content. Tests prove semantic outcomes; relevant cases also require actual native interaction. No passed screenshot can waive these checks.

| Case | Trigger | Required result |
|---|---|---|
| EP-01 | Open/reopen known schema 1–4 documents containing every current block/mark/attribute and unknown metadata | No rewrite merely on open, flattening, missing assets or provenance loss. Unsupported future data remains preserved rather than silently made editable. |
| EP-02 | Select the second identical quote after Cyrillic, emoji/ZWJ and combining text, across marked text runs; select in reverse | Actions target that exact revision/range only. Byte/UTF-16/scalar conversions and selection direction are explicit and verified. |
| EP-03 | Cross paragraph/table-cell boundaries, then invoke formatting, copy and annotation actions | Rich selection remains intact; valid operations preserve structure. An invalid annotation scope is explained and stages nothing; never silently drop selected blocks/cells. |
| EP-04 | Apply/remove marks or change heading/list style over a mixed selection | Preserve unselected runs, links, IDs, sourceRefs and all stored attributes. Actual heading 4–6/mixed state is not misreported as plain paragraph. |
| EP-05 | Split/join, move/copy rich blocks and edit before/inside/after a noted quote | IDs follow the declared split/copy policy; sourceRefs survive; anchors map or detach deterministically in the same transaction. |
| EP-06 | Save a note/dictionary link, move focus, switch document and restart | Mark/text/lifecycle restore at the exact occurrence. Reader content and document schema do not change because a decoration exists. |
| EP-07 | Delete/partially overwrite a quote or delete its linked dictionary term | Note/snapshot/history survive; no first-match retargeting. Attached/detached and open/resolved remain independent; explicit reattachment validates fresh document/note revisions. |
| EP-08 | Undo/Redo rich typing, formatting, paste, block deletion and a guarded reviewed Apply | Text, marks, anchors, provenance and media references remain consistent. Document switching cannot expose another document's Undo stack or write its text into the active document. Later edits are not overwritten; proposal/note metadata history is not confused with the focused text field's Undo. |
| EP-09 | Copy/paste formatted HTML/plain text, tables, code fences and formula expressions; malformed/executable clipboard input | Preserve known semantics and mint correct copy IDs. Invalid content causes no partial mutation/external execution. All adapters use compatible hard-break/math projections; no hidden positional drift. |
| EP-10 | Commit/cancel Japanese/Chinese composition and dead-key/emoji edits, including inside marked text and table cells | No duplicate/preedit leakage, invalid Unicode boundary, premature canonical commit or lost selection; a committed operation has coherent Undo and anchor mapping. |
| EP-11 | Add/delete/resize a table row/column with spans and nonempty cells, then Undo/reopen | Within the approved native capacity, the correct target and all retained cell content/attributes survive atomically. A failed operation leaves the table unchanged. F-30 boundary cases are independently required in EP-19. |
| EP-12 | Change code language, paste fences/backticks, copy code and edit syntax-highlighted content | Canonical source remains exact, including indentation; decoration does not become text/marks, normalization is not repeated, and failed Copy never reports success. |
| EP-13 | Drop an image, type or navigate while persistence is pending, then succeed/fail/cancel | Insert at the verified original target or explicitly reject stale placement. No later-caret/wrong-document insertion, dangling reference or false success. |
| EP-14 | Type invalid/oversized math or Mermaid source, lose renderer availability, then Undo/reopen | Preserve source and existing document; show a readable fallback/error. Do not truncate the draft, execute forbidden commands or commit a partial block. |
| EP-15 | Persist rich edits/notes/assets under save failure, stale revision and restart recovery | Keep recoverable original work and immutable history. No partial rich/anchor/media commit or false Saved state; dismissal does not reopen a retry-modal loop. |
| EP-16 | Split/join provenance-bearing blocks and play a selected source interval | Preserve all source refs and play only the authorized bounded interval. Missing source/device failure changes no text; do not silently substitute another source. |
| EP-17 | Show a supplied rich assistant preview, edit while it is visible, then attempt Apply/Undo | Preview remains noncanonical; known staleness is visible before Apply. Fresh validation and atomic rich/annotation updates preserve newer writing. |
| EP-18 | Change locale/theme/type size or open/close a modal while an unfinished edit/note/composition exists | Preserve content, exact scope and relevant Undo state. Focus references an emitted native accessibility node; no new-editor focus crash or unrelated same-frame modal stack. |
| EP-19 | Edit a 63-column table, attempt a 64-column creation/widening through UI/MCP, and open wider canonical tables with merged cells | Native capacity errors occur before mutation. The generic model keeps legacy span 1–100/no-total-cap compatibility. Wider tables round-trip whole and visibly enter the declared read-only capacity state; no truncation, clamping or implicit conversion. |

## P1 native usability and evidence matrix

- macOS actual typing, selection, contextual actions, focus return and accessible navigation: empty/populated/long document; minimum/common/wide client; navigator/inspector combinations. Tables/code use local overflow; primary controls stay reachable.
- Toolbar Auto/Expanded/Compact, inherited StarterKit/Link input rules, all current Insert/context actions, active/mixed state, block-local source editors, Cancel/Escape/Enter and a reversible visible Focus exit. The same command must work through pointer and conventional keyboard paths.
- All eight interface catalogs and parameterized/status/accessible labels; actual long German/French layout, Cyrillic typing, Japanese/Chinese IME and native CJK glyphs. Language switching leaves author text unchanged.
- Light/Dark across all seven palettes with representative selection/note/link/source/find/proposal decorations; System follows OS changes. Token checks complement actual render/interaction evidence. Do not require a wasteful Cartesian retest when a smaller matrix covers independent dimensions; expand only for a new failure.
- Source/preview/input/validation/save errors have distinct actionable states. No nonfunctional buttons masquerade as the corresponding editor capability. Screen-reader roles/selection/caret and native composition require actual platform evidence.
- Real private notes are excluded from reader text; known rich content/metadata survives save/reopen and semantic round trip. Verify clipboard destinations and original-project preservation using synthetic fixtures.

For every capability, retain source requirement, implementation command/component, exact test names, compiled build/fingerprint, screenshot/interaction evidence, finding and independent disposition. Distinguish personally observed native behavior from supplied records and raw-event/model tests. The new decision must name `full-editor-parity`; the first alpha's `native-foundation` approval and a successful first slice do not grant it. Full-product, installer and other-platform acceptance remain separate.

## Audited original-source hashes

These identify the working files inspected, not a claim that the original repository was clean or modified by this task.

| Original repository path | SHA-256 |
|---|---|
| `docs/specification/RICH_DOCUMENT_EDITOR.md` | `d878b7cd48820677e0acca401ed58e0c9ac47862440a037d75d2b3934b14a6b6` |
| `docs/specification/MANUSCRIPT_ANNOTATIONS.md` | `7af6031ddc0632c4548c00860da99d9ea7941908ae76286a3af3a591736e0c53` |
| `src/features/editor/ui/formatting-toolbar.tsx` | `11732c9424293add63819da115ca49cb733e8204d2e9cb1377d94b6cbb19f181` |
| `src/features/editor/model/extensions.ts` | `cab5b92255c6e7e60a7831a9b428346f632bafbab297c515b5cef9d874eee135` |
| `src/features/editor/model/annotations.ts` | `e846b56f3f0b7555e84751400c79e32e650a210b9734ce9358e05a64c9806349` |
| `src/features/editor/model/code-block.tsx` | `b17029f832830222a177854a443a5f82305ac01273c77efae41d7a59190a94ba` |
| `src/features/editor/model/rich-blocks.tsx` | `f1c6c16ae35fca4c876222c5c0f0d32ad7336ca49ec6b00c73ae61a3acf603f0` |
| `src/features/editor/model/math.tsx` | `daebcffc10f0b7709f6451d1fc21c79223b2351526a4ac1e3a2a5e9fb86d206b` |
| `src/features/editor/model/image-drop.ts` | `191dce4ffcc601c77b8041e9acf902b15cdaed49ccc565b223be84010e4d070a` |
| `src/features/editor/model/sanitize-pasted-html.ts` | `c155bda82758d1498475ed6066279d222727a0d363f0ee29981fc4e2dd0ff299` |
| `src/widgets/manuscript-editor/ui/manuscript-editor.tsx` | `a6fdda486c7d9e283c635b38e43d033d6d3d875c45464296d705527c370369d9` |
| `src/widgets/manuscript-annotations/ui/manuscript-annotations-panel.tsx` | `36e39ae58e9cd4e5230e05e889ba0c4ef25e084f5f988a7777bced92717adb09` |
| `src/pages/studio/ui/studio-page.tsx` | `196fa6ca7c24a5fdaf7b6bcf4282f4d04a8a05cdac47675fbe8dd677cc957c24` |
| `src/pages/settings/ui/settings-page.tsx` | `6dc06590458e9405473378ef40298674493765ae9ce53afa4642d8b004459507` |
| `internal/editor/document.go` | `60b2b761cb6af26c06056b3a0494d3113d19de3a680914e56b304df14b3a83a9` |
| `internal/documentannotation/annotation.go` | `4621c942f6e9b88bcf57b5b15b590524d89f148627eac1e24c88df431615c116` |
