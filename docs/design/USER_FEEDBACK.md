# Product-owner UX decisions and feedback

Append-only factual log. Established 2026-11-07 from the current conversation. This is required preparation for every UI/UX review. Later changes supersede a decision explicitly; do not silently rewrite earlier feedback. Distinguish owner decisions from team interpretations and implementation choices.

| ID | Date | Actual request or decision | Design consequence / regression check |
|---|---|---|---|
| F-01 | 2026-11-07 | Move toward an independent native Rust desktop application and remove the ordinary user's need to operate Docker. | Local first launch, local projects and no browser/server chrome. Runtime architecture is a separate acceptance concern from appearance. |
| F-02 | 2026-11-07 | Current UI feels like a window inside another window; supplied library and manuscript screenshots show nested framing/chrome. | One command surface; no whole-app inner rounded frame or another floating editor frame. Check UX-01. |
| F-03 | 2026-11-07 | Work on design/UI/UX from the beginning; utility icons must be coherent, useful and deliberately designed. | No emoji fallbacks or arbitrary mismatched icons. Verify focus, labels, hit targets and visual consistency, not only a screenshot. |
| F-04 | 2026-11-07 | Preserve the existing product's full functionality as the transfer target; choose Rust dependencies carefully. | Maintain full workflow map/parity inventory. A small engineering probe must not be described as a completed native replacement. |
| F-05 | 2026-11-07 | SolidCraft is a liked design reference; it is a different product and a literal copy is not requested. | Reuse compact utility composition, controlled panes and preference clarity; do not import CAD vocabulary, tools or information hierarchy. |
| F-06 | 2026-11-07 | Name ends in Craft; owner chose WriterCraft from the suggested options. | Display WriterCraft consistently; preserve `.wstudio` compatibility independently of branding. |
| F-07 | 2026-11-07 | Light and Dark from the start; retain old palettes except green. | Separate mode from palette. Team interpretation excludes green-accent `quiet` and `olive-ink`, preserving six other old IDs plus native blue. This exclusion mapping is an implementation interpretation, not a separately quoted owner list. |
| F-08 | 2026-11-07 | Use subagents, research, PRD/specifications and architectural decisions; avoid unsystematic development. | Reviewer requires requirement/evidence links and records decisions. The team selected OpenSpec; the owner did not mandate a specific framework by name. |
| F-09 | 2026-11-07 | Show real screenshots of the built application and launch it for personal exploration. | No mockup presented as a running build. Do not restart/replace/interact with the owner's active preview during their exploration without a clear request. |
| F-10 | 2026-11-07 | Consider porting existing TS behavior/libraries, without insisting on this technique. Selection is needed for model input, notes and NER-like markup. | Compare semantic reuse costs; do not translate DOM mechanically. Exact selected occurrence and persistent anchors are essential. Check UX-06/07. |
| F-11 | 2026-11-07 | Large specialized reusable libraries should live in separate repositories, keeping WriterCraft focused on business behavior; report when such work is needed. | Validate a real generic boundary first. Do not silently vendor a whole editor or create speculative platform repositories. |
| F-12 | 2026-11-07 | Include at least six or seven major interface languages beyond Russian immediately. | Team selected ru/en/de/fr/es/pt/zh/ja. Test actual UI expansion, glyphs, persistence and language changes; leave author content unchanged. The exact seven added languages are the team's implementation selection. |
| F-13 | 2026-11-07 | A separate UI/UX designer must review all author workflows as desktop interactions; weak/awkward/minimal UI must be redone and design belongs in acceptance. | Independent gate with blocker severity, workflow map, real evidence and retest. Minimal panels are not full-product acceptance. |
| F-14 | 2026-11-07 | After later owner corrections, adapt reviewer instructions and acceptance so the same miss is not repeated. | Append feedback, classify old miss vs new requirement, supersede invalid approval and add a concrete regression. See UX_ACCEPTANCE feedback procedure. |
| F-15 | 2026-11-07 | Confirmed first full release: macOS; Windows/Linux later, portable core retained. | Prioritize macOS native behaviors and evidence. Avoid forcing current verification to cover unimplemented platforms, while retaining future contracts. |
| F-16 | 2026-11-07 | Confirmed primary manuscript mode: continuous text inside a separate bounded pane; stationary navigation/inspectors; pagination during publication. | No forced Word-like page breaks in daily writing. A continuous manuscript pane is not the rejected infinitely scrolling whole webpage. |

| F-17 | 2026-11-07 | Feature scope is frozen to the functionality already present in Writers Studio; improve desktop UX and fix existing bugs, without inventing additional product features. | NER-like markup was an interaction analogy, not authorization to add a standalone NER feature. Local inference and new platform features are not requirements. Match the actual source-backed inventory. |
| F-18 | 2026-11-07 | Repository prose and identifiers should use English; multilingual values and Unicode test fixtures remain appropriate. | Keep design/spec/release documentation and source msgids English. Eight-language UI catalogs and original author content remain multilingual. |

F-15/F-16 and F-17/F-18 were delivered to this reviewer by the parent agent as newly received explicit owner decisions. Other entries summarize inherited user messages; no new preference was inferred from the reviewer's taste.

## How to add feedback

Append an entry with an ID, date, factual request, scope and test consequence. Link the revision/review it changes. An acceptance miss updates the reviewer preparation/gate; a new product direction updates specs and the map without pretending the old review could have known it. Preserve relevant historical evidence.

Do not add invented “user approved” statements after an agent-only design decision. Optional suggestions remain options until chosen or implemented under an explicitly stated reasonable assumption.

## F-19 — shared chrome alignment and overlapping titles (2026-11-08)

The owner observed that titles overlap in places and that the toolbar's vertical separator does not line up with the navigator edge; the owner described the result as ugly and required a correction. This is corrective feedback about the existing desktop composition, not a new product feature. The reviewer received the observation through the implementation owner during the native foundation retest.

Consequence: derive the toolbar identity-zone boundary and navigator boundary from the same effective geometry. At minimum, common and wide window sizes, with the navigator shown/hidden/resized, no project/document title may overlap another label or utility command. Long titles truncate within an allocated region and retain a way to inspect the full value; controls must keep independent hit targets. When navigation collapses, the toolbar uses the matching collapsed composition rather than leaving an unrelated decorative divider.

Regression evidence: actual compiled captures with deliberately long project/document titles in German and CJK, normal and maximum text sizes, navigator resize limits and compact inspector precedence. Check the shared vertical boundary, title truncation, utility visibility and hit targets. Read F-19 before any future shell review. The prior native-foundation acceptance remains pending; attractive typography and individual pane tests do not excuse cross-component alignment defects.

## F-20 — structured desktop preferences and spacing (2026-11-08)

The owner rejected the current Settings spacing as poor and requested a properly structured settings window with categories for the coming existing-product functionality, at least as a scaffold now. This correction changes presentation and information architecture; it does not authorize additional product features. The reviewer received the correction through the implementation owner during the native foundation retest.

Consequence: replace the small all-in-one form with a desktop preferences surface containing a stable category sidebar, a bounded content area and a fixed footer. Put current working settings in their relevant categories. Existing parity categories may be visible as clearly labelled informational scaffolds; they must not imitate operational provider/MCP/voice configuration. Use the spacing and category contract in `DESKTOP_UX.md`.

Regression evidence: actual compiled General, Appearance, Editor and scaffold-category views at common and minimum supported sizes; Light/Dark; long German and CJK labels. Check category selection, aligned fields, consistent spacing, independent scrolling, always reachable Done, Escape/focus return and language/theme persistence without draft mutation. Future settings reviews must assess information architecture and vertical rhythm, not only palette swatches or presence of controls.

## F-21 — Fixed modal geometry and release preparation

The owner observed displaced header text, inconsistent vertical centering and an oversized empty footer. Header and footer must have explicit bounded heights; labels and actions share vertical centers and a single inset grid. Acceptance requires an actual compiled Settings capture at normal and minimum window sizes. Prepare the first clearly scoped alpha after the corrected UX gate, never disguise incomplete migration as a full replacement.

## F-22 — SolidCraft appearance presentation

The owner supplied SolidCraft's Appearance screen and requested comparable visual quality. Present appearance modes and the seven palettes with recognizable miniature-window previews, a clear selected border and readable labels. Keep mode independent from palette; preserve the existing seven palette choices and green exclusions. The reference supplies presentation, not additional product features.

## F-23 — Left-aligned category hierarchy and established utility icons

The owner rejected centered settings group headings and the hand-drawn utility icon set. Group headings and navigation rows follow one left-aligned grid with deliberate spacing. The owner named Lucide as an example of an established utility icon family; this was not a final family choice. Compare consistent, licensed families and retain exact upstream path provenance and notices for the selected subset. Use appropriate palette, editor, connection and keyboard symbols. The monk/book mark is independent product branding. The earlier team wording overstated the Lucide example as a decision; F-25 records the clarification explicitly.

## F-24 — Show and open the actual updated app

The owner asked to see the complete interface with the monk mark and to click through the updated build after the utility icons are replaced. Provide a real compiled screenshot, open a separate updated preview with synthetic demo data, and leave it running. Do not replace user manuscripts or imply the complete migration is finished.

## F-25 — compare icon families with the owner before selection (2026-11-08)

The owner clarified that Lucide was an example, requested the designer's recommendation among suitable icon families, and wants comparison screenshots to participate in the choice. A final Lucide decision had not been made. The team is comparing official Lucide, Tabler and Phosphor Regular glyphs with equivalent meanings at 16/20/24px in Light and Dark.

Consequence: the current design contract requires one coherent licensed family, not a predetermined Lucide selection. The designer evaluates actual render clarity and fit with the existing typography/monk mark, explains tradeoffs, and presents the comparison. Record the owner's choice before representing a family as approved. Do not mix attractive glyphs from different families or substitute new hand-drawn approximations. Preserve each upstream license and source provenance; the monk mark remains independent product branding.

Regression evidence: identical semantic set, sizes, foregrounds and surfaces for the comparison; verify representative selected-family glyphs in the actual compiled toolbar/sidebar, including 16px metadata and 20px utilities, before final visual acceptance. A contact sheet is a selection aid, not a substitute for the compiled application.

## F-26 — Owner selects Lucide

After receiving the same-size Dark/Light comparison of Lucide Outline, Phosphor Regular and Tabler Outline, the owner explicitly selected **Lucide**. The independent designer separately recommended Lucide for its 16–20 px clarity and balance with Source Sans. Ship a single licensed Lucide utility family; do not mix competing families. The reading monk remains the separate product mark. Verify the actual native rendering against the official SVG comparison before acceptance.

## F-27 — Compact identity group and native window controls

The owner observed excessive space between the monk and WriterCraft name and asked for alignment with the sidebar. The owner then acknowledged the macOS traffic-light controls require a reserved leading region and accepted that constraint. Keep that native exclusion region; left-align the product name directly beside the mark with the normal fixed gap. Do not center the name within the entire remaining identity region.

## F-28 — Application and Dock icon

The owner observed an unfamiliar application icon despite the new toolbar mark. A bundled ICNS alone is insufficient when the native framework supplies a default runtime icon. The first alpha must explicitly use the generated matching monk/book PNG as its viewport/application icon, alongside the ICNS bundle asset. Keep both outputs reproducible through the icon-generation script. Verify a fresh isolated launch; do not overwrite the owner's active preview merely to refresh its icon.

## F-29 — Editor-first alpha.2 with complete original editor scope (2026-11-08)

The owner selected editor work first for the next alpha and deferred the other candidate feature streams. The owner then explicitly clarified that this means **all functionality currently present in the original editor, with no cuts**, not a basic-formatting subset. The implementation owner relayed this clarification to the independent designer. Earlier tree/named-version/installer planning is not implementation authorization for this editor-first scope.

Consequence: inventory the actual toolbar/extensions, all current blocks and marks, native selection across blocks/tables, context actions, typography, persistent note/dictionary decorations, clipboard/composition/accessibility, source references, editor history/save/recovery and real editor integration handoffs. The contract is `EDITOR_UX_PARITY.md`, coordinated with the technical parity matrix. Source bugs should be corrected rather than copied; do not introduce NER, notification or other unrelated domain features.

A prioritized native slice may validate the model/input/anchor architecture internally, but cannot satisfy the final release gate. Independent eventual acceptance must name `full-editor-parity`, distinct from full-product parity, and requires complete compiled/native evidence. The existing alpha.1 native-foundation acceptance does not approve new editor code. No acceptance or implementation-completion claim is created by this feedback entry.

## F-30 — Approved native table capacity for alpha.2 (2026-11-08)

The owner considered the native engine's 63-column table limit acceptable and unlikely to constrain normal use. The implementation owner accepted this bounded capacity for alpha.2; do not fork WordCraft solely to raise it. This is an explicit exception to F-29's otherwise complete editor scope, not permission to remove other editor capabilities.

Native UI and MCP creation or structural widening beyond 63 logical columns must fail before any mutation, with a clear capacity explanation. Existing canonical tables wider than 63 columns must remain whole and preserved, with an explicit read-only capacity state; never truncate, reshape or silently convert them to fit the native view. Logical column count accounts for merged cells/spans and is distinct from pixel width.

The generic canonical model/legacy parser keeps its existing span validation 1–100 and no new 63-total-column cap. Canonical compatibility and native editing capacity are separate contracts. Regression cases cover 63-column editing, rejected 64th-column creation/widening with unchanged revisions/content, and exact preservation/read-only handling of wider canonical tables, including merged cells. No implementation, passed test or UX approval is asserted by this entry; the full-editor gate remains pending.

## F-31 — Airy manuscript workspace and integrated formatting (2026-11-08)

The owner rejected the actual editor workspace because the top contains too much empty space and the formatting strip looks wrong. The owner requested designer-led presentation that is attractive and useful, then clarified that the interface must retain air rather than cram controls together. The owner wants approximate design pictures for comparison and will approve an option before implementation. The implementation owner relayed this correction while the owner was testing the separate preview; do not disturb or replace that running app before handoff. Track the finding in [issue #6](https://github.com/avacadorun-dev/writer-craft/issues/6).

Consequence: prepare polished same-size Comfort and Balanced concepts using the existing monk, selected Lucide family, typography and theme roles. Comfort proposes 52/76/42 px global/identity/format bars; Balanced proposes 48/56/40 px. Both keep comfortable manuscript gutters, 42–44 px navigation rows, fixed bounded panes, left-aligned headings and an integrated flat formatting row. Remove redundant hero/eyebrow spacing and outer-toolbar framing. Use measured grouped overflow, not clipped controls, tiny targets or horizontal toolbar scrolling. [WORKSPACE_EDITOR_REDESIGN.md](WORKSPACE_EDITOR_REDESIGN.md) records the comparison and behavior contract. The concepts are nonfunctional design illustrations, not native app captures, and no option is owner-approved yet. Await that choice before host layout implementation.

Regression evidence after selection: actual compiled 1440 × 940 and 1000 × 680 views; navigator/inspector hidden/shown/resized; long German/French and CJK labels; Light/Dark and independent palettes; normal/maximum manuscript text sizes. Measure the chosen fixed header/toolbar heights and first content origin; check shared dividers, vertical centers, comfortable group spacing, independent 32 px targets and visible Focus exit. Pointer/keyboard toolbar and overflow interactions preserve exact selection, active/mixed formatting, caret/focus and Undo. The full interactive demonstration includes all 21 node types and six marks with real images, Mermaid and formulas; concept drawings or placeholders are not implementation evidence. No acceptance or issue closure is granted until independent native retest.

Owner decision, 2026-11-08: after the implementation owner displayed the polished concept images, the owner explicitly selected **Balanced**. Proceed with the rendered 48 px global bar, 56 px document identity, 40 px flat formatting row and 28 px manuscript gutter, 264 px default navigator and unchanged comfortable control/body spacing. Comfort is a comparison artifact. This is design approval for implementation; native acceptance, regression verification and issue #6 closure remain pending. The owner's running source-99 preview remains undisturbed.


## F-32 — Complete authoring journeys, not only rendered specimens (2026-11-08)

The owner found that the running source-82 engineering preview exposed only a small formatting subset and no discoverable way to author most existing blocks. Pasting unfenced Mermaid source into prose produced paragraphs; the owner expected the original Insert and context workflows. The owner explicitly reiterated that alpha.2 must cover **the entire original editor**, not only Mermaid or a prebuilt gallery. Track the missing authoring surface in [issue #7](https://github.com/avacadorun-dev/writer-craft/issues/7), under [full editor scope #4](https://github.com/avacadorun-dev/writer-craft/issues/4).

Consequence: start from an empty structured document and create, insert at the exact target, configure, edit again, delete, Undo/Redo, save and reopen every current node/mark through actual native commands. The full original toolbar/Insert/More/context/keyboard inventory remains required. A gallery, parser round trip or renderer demonstration cannot satisfy this journey. Preserve explicit block insertion and audited original rich/fenced paste semantics rather than guessing that ordinary prose beginning with a diagram keyword should be converted.

Regression evidence: reproduce each original authoring journey with synthetic content, through pointer and keyboard, including cancellation, stale state, failure and Undo/Redo. The owner approved the complete eight-goal alpha.2 criteria after clarification; do not present the next incomplete checkpoint as acceptance of this scope. Keep the owner's currently open preview undisturbed while they test it.

## F-33 — Syntax highlighting for code and structured text (2026-11-08)

The owner found no visible syntax highlighting and explicitly included it in alpha.2. This covers programming languages and non-programming structured text formats that have an appropriate grammar, not indiscriminate coloring of ordinary prose. Track [issue #8](https://github.com/avacadorun-dev/writer-craft/issues/8).

Consequence: expose the source-backed language/format choice, preserve stored unknown labels and exact code text, and render real syntax decorations during editing and after reopening. JSON, YAML and other required structured formats need their appropriate grammar. All languages/aliases supported by the original editor remain in scope; dependency convenience cannot silently remove a language. Theme-aware token colors must remain readable in Light/Dark and all independent palettes. A static colorful screenshot is not evidence of editable syntax support.

Regression evidence: edit representative programming and structured-text samples, switch language/format, Copy exactly, Undo/Redo and save/reopen. Check Unicode, long lines with block-local overflow, font size and Light/Dark without changing canonical source text merely to apply highlighting.

## F-34 — Adaptive document surface and bounded focus measure (2026-11-08)

The owner supplied the original application's wide and panel-constrained states. With enough space, the manuscript appears as a distinct centered document surface with surrounding gutters; with panels open and less center width, it joins the pane structure. The owner rejected a focus state that visually stretches writing across the whole window. This updates the flat-only surface interpretation in F-31, while retaining its approved Balanced rhythm. Extend [issue #6](https://github.com/avacadorun-dev/writer-craft/issues/6); do not create a second workspace-layout issue.

Consequence: use the actual remaining center width to select floating document or joined-pane presentation. Focus hides panels while retaining the same readable maximum text measure, clear document boundaries and a visible exit. Continuous writing remains the chosen primary mode; this does not introduce forced print pagination, a second application titlebar or another whole-app frame. Existing panel/scroll/selection/history state survives presentation changes.

The designer prepared Light-mode wide/focus/docked images in [WORKSPACE_EDITOR_REDESIGN.md](WORKSPACE_EDITOR_REDESIGN.md), explicitly labelled nonfunctional concepts. The proposed 920 px document/760 px text measure and width threshold are reviewable design values, not an already accepted new geometry or native verification. Obtain the owner's image-based response before structural layout implementation, as previously requested. Model, source and mockup changes cannot close the issue.

## F-35 — Comfortable fields and consistent modal insets (2026-11-08)

The owner reported unusually shallow single-line fields in navigator search and New document, with inconsistent spacing among the modal title, body and actions. They requested comfortable field height/width and conventional uniform interior padding. Track [issue #9](https://github.com/avacadorun-dev/writer-craft/issues/9).

Consequence: use shared fields with 40 px minimum height, 12 px horizontal and 8 px vertical inner padding, full available form width and centered single-line text. Dialog actions use the same 40 px rhythm. A modal has one 24 px horizontal inset grid for its header, labels, fields, separators and trailing action row; title and close target share a vertical center. Source editors keep comfortable multiline insets and local scrolling. Avoid growing the manuscript font to compensate for short controls.

Regression evidence: actual compiled search and New document/New project/block-source dialogs in Light/Dark, common/minimum windows and long German/French/CJK text. Verify full field bounds, clipped long values with reachable caret, focus/Tab/Enter/Escape, error/disabled states, action placement and unchanged data after Cancel. Source fixes and the modal illustration remain unverified until the hosted build and independent native retest. The source fix also ensures an editor-preview document created from these forms starts as a canonical structured manuscript, rather than routing the author into the plain editor.

Owner decision, 2026-11-08: the owner approved the displayed adaptive wide/focus/docked concepts for F-34 and issue #6. This authorizes structural layout implementation while preserving Balanced rhythm and comfortable prose measure. The independent full-editor gate remains pending until exact compiled/native evidence passes.


## F-36 — All open issues in alpha.2 and final owner acceptance (2026-11-08)

The owner instructed the implementation team to continue autonomously until alpha.2 and its issues are ready for final review. After an explicit scope clarification, the owner selected all open issues, including installation #3, complete original-project/Canvas/media compatibility #10 and portable Markdown/artifacts #11. This supersedes earlier editor-only deferrals while preserving all eight approved editor goals and the single 63-column exception.

Keep product issues open until the owner checks the final candidate and approves closure. Independent actual native review covers the new project/import/export/recovery surfaces as well as the complete editor. Do not present partial engineering checkpoints as final acceptance, and do not drive or replace the owner's running preview while they test it. Source, compiler, fixture or mockup evidence alone does not complete these gates.
