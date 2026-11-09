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

## F-37 — Native rich blocks overlap after typography changes (2026-11-08)

The owner reported a serious visible overlap in the complete-block showcase and supplied a cropped screenshot. The independently inspected image shows the lighthouse illustration covering part of the code/heading/table flow, and two separate Mermaid diagrams, captions and prose occupying overlapping vertical regions. The implementation owner identified the tested preview as application `141fce5e5fc27ca9bfeed1661831baa5f2370cc6` with engine `b96b4d971c649bc024f37a72fd7ba626e7393100`; its local build evidence agrees with that application source. Track [bug #12](https://github.com/avacadorun-dev/writer-craft/issues/12), alongside workspace layout #6 and complete editor #4.

This is a **P1 native document-flow failure** and a correction to the expected usable rich-editor behavior. The screenshot proves visible overlap; it does not establish canonical document corruption. The screenshot does not record the active font, spacing or viewport metadata. Preserve the original JSON, assets, selection and history while investigating and correcting presentation. The earlier prose-only startup captures and successful source/model checks did not inspect this settled opaque-block viewport and cannot qualify its layout.

Consequence: every independently flowing image, diagram, block formula, table, code block, heading, caption and paragraph must receive sufficient reserved native layout space for its actual painted content. Prose line-height preferences must preserve opaque intrinsic dimensions, formula baselines and independent table/code geometry. Progressive pending/ready/failure transitions and theme/font/width changes must retain a coherent document flow.

Regression evidence: actual compiled captures must reach the affected blocks after visible media has settled, including the two Mermaid kinds and real image/formula output. Cover Sofia Sans/Inter/Source Serif 4; 17/19/21 and retained 28px; line-height 1.5/1.75/2; measures 60/70/80ch, Light/Dark and floating/joined/Focus transitions through a justified matrix. Compare actual painted atom/caption bounds with reserved flow and adjacent sibling bounds; positive abstract rectangles alone are insufficient. Verify the same source/selection/history before and after reflow and save/reopen. A source patch or top-of-document startup image cannot close #12 or pass the independent editor gate. Keep the owner's preview untouched until the authorized handoff.


## F-38 — publish the completed checkpoint and continue parity in alpha.3

Date: 2026-11-08. The owner requested finalizing the completed coherent application as alpha.2 now, publishing native installers including a macOS DMG, and moving unfinished large blocks to alpha.3. The owner also explicitly requested updating README with real current application screenshots. This supersedes the earlier all-open-issues alpha.2 scheduling decision; it does not waive data preservation, create full-parity evidence, close unfinished issues or authorize additional features. Regression/release checks: the installer must launch the current native rich editor under the normal application identity, screenshot claims must match a real compiled build, published alpha.1 stays immutable, and deferred work remains visible in issues and the alpha.3 plan.


## F-39 — owner-tested alpha.2 and complete alpha.3 execution

Date: 2026-11-08. The owner tested the published alpha.2 on macOS, confirmed it functional, and instructed the team to close issues whose complete criteria were met in that release. The continuing goal is to implement alpha.3 and close all remaining issues on complete verification. This lifts the checkpoint release freeze and supplies owner acceptance for already verified alpha.2 completions; it does not waive unfinished technical/native criteria or certify untested platforms. Reviewers must distinguish the closed installer request from remaining editor, workflow and compatibility gaps, preserve prior release evidence, and review alpha.3 independently against its exact compiled source.


## F-40 — researched manuscript formatting and direct issue prose

Date: 2026-11-08. Add research and resulting concrete formatting improvements for screenwriters, editors and writers in issue #14, starting with resizing images, rich visual blocks and tables directly on the native working surface. Distinguish object dimensions from workspace zoom, manuscript flow from free Canvas placement, and discovered needs from unselected competitor features. Issues should describe the requirement directly, without `Owner report` or repetitive attribution.

Existing Studio format compatibility is mandatory: current `.wstudio` archives must import and work, with additive schema extensions/defaults/migration and exact preservation of existing content, unknown fields and original media. New formatting requires old-project import plus native persistence/export/restore regressions. This clarifies the compatibility contract; it does not authorize flattening old content or abandoning the supplied private reference acceptance.


## F-41 — validate meaningful project types and record user scenarios

Date: 2026-11-08. The project kinds themselves require the research in issue #14: their prior definitions were added without hypotheses or comparison with professional writing products, and some distinctions are only nominal. Evaluate real tasks before fixing the new creation model. The comparison set includes STARC, Final Draft, WriterDuet/WriterSolo, Fade In, Celtx, Scrivener, Obsidian, FocusWriter and Google Docs. Record stable-ID persona/task/main/alternative/failure/migration scenarios, concrete outcomes, hypotheses/evidence and traceability into specs/tests/native review. Preserve all existing Studio kinds/custom schemas through migration regardless of the eventual creation-profile model. Initial evidence and proposed scenarios live in `docs/research/PROJECT_TYPES_AND_FORMATTING.md`; no completed research or native acceptance is implied.


## F-42 — write directly inside Canvas text cards

Date: 2026-11-09. During the isolated e501837 alpha.3 development preview, the supplied screenshots show a selected text card and a separate Markdown edit modal. Primary card writing must happen inside the card, with immediate focus for a newly created card and discoverable pointer/keyboard entry. Track [issue #15](https://github.com/avacadorun-dev/writer-craft/issues/15), alongside #10 and #9. The original `CanvasTextCard` has in-card textarea editing; the modal-only native path is a migration gap, not an accepted final interaction.

Regression requirements: real native typing, selection, clipboard, Unicode/IME, text-owned Undo while editing and board-owned Undo after leaving; correct zoom/pan/clip and caret geometry; no unintended drag/delete while typing; exact acknowledgement of saves without replacing a later draft. Cancel, stale updates, failed saves, navigation and restart must preserve acknowledged content and recoverable unsaved work. The separate source dialog must not remain the only writing path. User exploration does not authorize driving/restarting that running preview; source/event tests do not replace independent native acceptance.

## F-43 — coherent boards, manuscripts and narrative knowledge

Date: 2026-11-09. Extend existing [research issue #14](https://github.com/avacadorun-dev/writer-craft/issues/14) to establish a well-designed relationship between boards and documents for scripts/scenes/beats, character profiles/behavior, lore/world rules and related dictionary/memory records. These views must support one writing task, with predictable editing and return navigation. Investigate workflow hypotheses; do not claim the original application had no relationships or already offered complete synchronization.

Compare canonical multi-view records, stable live references, revision-pinned excerpts/snapshots and independent notes. Define editing authority, refresh/conflict handling, rename/move/delete/restore and source-compatible persistence before choosing synchronization. Board layout changes must not silently reorder or edit the manuscript. Maintain PT-LINK-01, PT-CHAR-01, PT-LORE-01 and PT-LINK-02, including precise starting/return context, failure/cancel/stale and export/restore outcomes. Research/proposed architecture is not implementation acceptance. Use this correction in the designer's next review and preserve all original Studio data while evaluating additions.


## F-44 — approved lion identity and completion-focused execution

On2026-11-09 the user supplied the lion-writer handoff and requested replacement of the old logo throughout current branding, native application and platform icons, with deliberate size/background treatment. Preserve the supplied character; maintain Light/Dark coherence and exact source provenance. Independent review must inspect actual compiled header and packaged icon, not only an asset board. Historical released screenshots remain historical evidence. Tracked in #16.

The same instruction corrects fragmented delivery: limit implementation to one product issue at a time, assign bounded file ownership and observable acceptance, and integrate/review/verify before starting another feature. Root owns the queue, CI and closure evidence. Research or a green library alone is not issue completion.

## F-45 — full Canvas and coherent task acceptance

2026-11-09: the latest preview exposed a reduced board, prompting a request for complete original Canvas coverage and an operational gate proving that migrated functionality is actually usable. Track the explicit original board inventory in #17, separately from direct card input #15 and migration #10. Balance exhaustive capability checks with complete writer/screenwriter/editor journeys and researched human needs in #14. Do not substitute a feature gallery for real work, or a polished journey for omitted baseline functions. No new reduction of the source contract was authorized.

Require both axes in the existing release/UX gate, including alpha.3. Missing source coverage, unreviewed inventory completeness, stale evidence or an unfinished role journey blocks parity acceptance. Existing product-scope boundaries remain explicit; role hypotheses and competitor references are not claims of user validation.


## F-46 — edit existing rendered blocks and reduce image size

2026-11-09, running macOS ARM64 preview68db76e: double-clicking an existing formula or Mermaid diagram does not open source/properties editing. The expected interaction is to edit already-created blocks directly. Source inspection confirms a hidden More formatting → Edit block fallback; a rendered specimen or reachable insertion menu does not establish discoverable re-editing. Reopened #7 supersedes its earlier complete-editing acceptance. Preserve historical closure evidence, but retest double-click, visible selected-block/keyboard entry, edit/Apply/render, Cancel/Escape, Undo/Redo, save/reopen, stable IDs/source/unknown fields and stale/failure behavior in the actual compiled application before reacceptance.

The same feedback explicitly requests making existing images smaller. #14 already selects direct sizing and exact width/height with aspect lock; those controls remain unimplemented in this preview. Changing workspace zoom is not a substitute. Preserve original media bytes, compatible stored dimensions and adjacent text flow.

The user additionally reaffirmed that Canvas must be completed. #17 remains the active implementation issue under WIP=1; record this finding and update review criteria without abandoning or silently deferring B01–B26. Do not drive, replace or restart the preview while the user explores it.


### F-46 interaction clarification

2026-11-09: the user found the existing Edit block command and confirmed that the main problem is its distant, undiscoverable location. Keep the main toolbar compact. Use double-click on an existing rendered formula or Mermaid diagram as the primary editing gesture, retaining single-click selection; provide Edit in the object context menu with a keyboard-accessible equivalent. Do not add a permanent row of block commands. This refines the earlier selected-block action requirement; it does not establish native acceptance of the proposed interaction.


## F-47 — reject improvised connection UX; evaluate native React Flow behavior

2026-11-09: exploration of the running preview68db76e found the connection interaction unacceptable. React Flow is the explicit interaction reference. Further menu/two-click mechanics and improved line geometry alone do not resolve this correction. Evaluate existing native Rust graph libraries first; if none fits, consider a deliberate native implementation of React Flow behavior in the separately maintained CraftCanvas engine. The user subsequently identified `avinkrisv/egui-xyflow` as a useful visual/technical reference while questioning maintenance activity and missing required capabilities; it is not an approved drop-in dependency or proof of complete Canvas parity.

Keep #17 active. Compare actual candidate source against B01–B26 and the existing durable authority/embedded-input contracts, rather than asking for the requirements again or replacing them with a node-editor demonstration. Primary creation and reconnection must work directly by dragging handles/endpoints with clear target feedback and cancellation. Selection, labels, keyboard actions, camera behavior and embedded text/media controls must form a coherent native interaction. Secondary properties menus remain useful but cannot substitute for these primary gestures.

Preserve current source, prepared drafts and exact verification history, including data-safety fixes, without treating them as independent UX acceptance. Native Rust, canonical JSON/IDs, unknown fields, media and history remain required. The running user preview stays untouched. Fresh independent compiled interaction review against this correction is required before accepting the connection workflow.

### F-47 delivery clarification

2026-11-09: waiting weeks for an upstream maintainer is unacceptable for this project. Upstream research informs implementation; delivery and fixes must remain under CraftCanvas maintenance. Do not make the schedule conditional on egui-xyflow releases or imply that upstream activity establishes readiness.

## F-48 — production CraftCanvas and complete React Flow parity first

2026-11-09: explicitly set other product work aside and focus on CraftCanvas as a standalone production-quality native Rust library with functional React Flow parity. Use that qualified library in WriterCraft to deliver complete original Writers Studio Canvas parity. This supersedes the immediate app connection-slice priority, not the retained B01–B26 scope or data-preservation contracts. Preserve unfinished work, existing public releases and the running preview.

Prepare a source-backed full capability inventory, native equivalents, architecture and concrete acceptance before freezing a narrow interaction API. React component syntax and browser plumbing need native equivalents; their observable capabilities cannot disappear by categorization. Keep core functionality and official example/integration patterns visible with explicit dispositions. Generic library readiness and actual WriterCraft journeys are separate required outcomes; neither a node demo nor a polished connector establishes either outcome.

CraftCanvas owns its maintenance and delivery. Reuse qualified open-source work with attribution when suitable, without waiting on another project's roadmap. Retain guarded canonical authority, stable identities, unknown data, original media, exact input ownership and recoverable history. Independently review inventory completeness and exact compiled interactions. The next app integration adopts only a hosted-qualified engine revision; no current production/full-parity claim is made.

### F-48 roadmap and independent-library distribution

The same instruction subsequently requests a published, strictly ordered issue roadmap with a first checkpoint covering current WriterCraft needs and the remaining full React Flow goal afterward. Close each library issue only for its verified scope. Expand the standalone README and repository description; prepare meaningful Cargo packages and a release/publication process with truthful prerelease status. Package registration is not production acceptance.

## F-49 — independent non-Canvas alpha.3 delivery

2026-11-09: prepare a handoff for another agent to assemble alpha.3 from the existing implementation, defer full Canvas, and complete a concrete user scenario such as character/profile and screenwriting work plus another unblocked issue. The Canvas agent continues standalone CraftCanvas; the WriterCraft agent resumes Canvas integration only after a qualified engine handoff.

This explicitly supersedes the requirement that full Canvas or full-product parity block alpha.3. It does not close #17/#4, remove B01–B26, accept the rejected connection UX or authorize misleading parity claims. Define a named bounded engineering-release scope, complete its chosen end-to-end native workflow, and retain full parity inventories/gates for the later integration. Existing preview, alpha.1/alpha.2, private data and unfinished work remain protected. Parallel engine and app workstreams are now authorized, each with one active issue and separate file ownership.

## F-50 — complete author workflows, hierarchy and manual markup

2026-11-09: the resumed application task was corrected after it narrowed the character example to an isolated entity form. Begin from an author's real working session and the original tree structure, informed by competitor workflows and #14. Complete organization, drafting, reference consultation, exact return and export instead of implementing the nouns of the request literally. Project-kind taxonomy is explicitly undecided and must not become a nominal mandatory selector.

The user explicitly requests expanding existing exact passage notes into author-defined tags/markers with different meanings and useful cross-document scenarios. This selects manual markup as an enhancement; it does not request automatic extraction or infer a fixed taxonomy. Documents, folders and markup need complete CRUD: create/read/edit/rename as applicable, move/reorder for tree items, remove and recover without breaking content/references. Record this in #14 and preserve the broader #10/#11 obligations.

## F-51 — substantial native acceptance manuscript and goal-directed delivery

2026-11-09: prepare a large coherent working project with several chapters, character branches, motivations and descriptions. Acceptance must follow how an author creates and works with it in the actual native interface. A synthetic test matrix, API-created project or successful command call is not user acceptance. Use newly authored demonstration content without private user manuscripts; public CI fixtures remain synthetic and separate from the editorial interaction record. Do not describe this as observed human research or pretend the authoring was performed by a person.

The user requested an explicit plan and goal covering all corrections, followed by execution. The selected scenarios and sequence are recorded in ALPHA3_STRUCTURED_WRITING_PLAN.md and the structured-writing-alpha OpenSpec change. Independent reviewers must reject feature-gallery or isolated-form acceptance and require the whole author outcome on the exact candidate.
