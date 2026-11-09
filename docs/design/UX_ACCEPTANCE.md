# Independent desktop UX acceptance gate

Owner role: independent UI/UX designer. Established 2026-11-07 from explicit product-owner instruction. Read [user feedback](USER_FEEDBACK.md), [desktop contract](DESKTOP_UX.md), [workflow map](WORKFLOW_MAP.md), the current [PRD](../PRD.md), and the previous failed review before every review. This gate complements OpenSpec, domain tests and release checks; it does not replace them.

## Acceptance levels

| Level | What can be accepted | What it must not imply |
|---|---|---|
| Engineering probe | A bounded architectural experiment whose limitations and data paths are explicit | Daily-use product quality, editor parity or completed migration |
| Author pilot | The declared implemented journeys are coherent, dependable and comfortable enough for sustained use | Unimplemented legacy workflows are complete |
| Migration/parity | Every legacy capability in the parity inventory has a native journey and preservation evidence | Public installers/updates are verified on untested platforms |
| Public release | Pilot/parity scope plus platform distribution, recovery and update acceptance | An installer on one OS proves support for every OS |

A compile, domain-test pass, large command catalog or attractive screenshot alone cannot pass Author pilot. A simple control panel must not stand in for a complete author workflow. Conversely, a future parity feature missing from an explicitly bounded engineering probe is recorded as scope, not misreported as a regression introduced by that probe.

The confirmed first platform is macOS. Keep a portable core and future Windows/Linux contracts; do not make current macOS evidence into a three-platform claim. The primary manuscript view is continuous text within a bounded editor pane. Navigation/inspectors stay stationary; pagination belongs to publication preparation.

## Severity and disposition

- **P0 — integrity or authority:** wrong selected occurrence, silent loss, false save/apply success, lost recovery or an unapproved canonical mutation. Stop the affected journey. No UX approval can waive data integrity.
- **P1 — blocked or misleading primary journey:** unreachable/clipped command, unusable editor at a supported window size, absent keyboard equivalent for a required operation, a primary state/action whose meaning is wrong, or a flow that cannot be completed/reversed normally. Blocks Author pilot acceptance for that journey.
- **P2 — substantial recurring friction:** excessive manual copying/scrolling, unclear scope, layout inconsistency, poor recovery explanation or loss of navigation/caret context. Blocks acceptance when it affects the principal daily workflow; otherwise it needs a named follow-up and explicit scope explanation.
- **P3 — local polish:** minor spacing/alignment or an uncommon cosmetic issue that does not obscure state or completion. Record and prioritize; it does not excuse P0–P2 problems.

The implementer fixes blockers and supplies new evidence; the independent designer retests the affected scenario and adjacent regression paths. Do not silently lower severity to get a green result. A source fix without execution evidence is **fixed in source, verification pending**, not closed.

Review outcomes: `not reviewed`, `changes required`, `engineering probe only`, `accepted for named scope`, or `superseded`. Approval always names the application revision/build, journey scope, evidence and remaining limitations.

## Mandatory checks

| ID | Observable acceptance |
|---|---|
| UX-01 One workspace | One application chrome, no URL/reload/browser-history layer and no rounded whole-app frame. Navigator, content and inspector share clear boundaries. Native window controls remain functional. |
| UX-02 Bounded writing | Long text scrolls within the editor; navigation, command bar and status remain stable. No forced publication page breaks in everyday writing. Font measure/line spacing remain comfortable at supported sizes. Per F-34, wide and Focus views retain a centered bounded document and readable maximum measure; constrained centers join the pane structure without losing selection/scroll state. |
| UX-03 Responsive panes | Test minimum supported window, common laptop window and wide display; inspector closed/open; navigator/inspector at both resize limits. Preserve a useful editor minimum, collapse inspector first, then offer explicit panel recovery. No clipped primary controls or horizontal-scroll workaround for ordinary writing. The toolbar identity-zone separator and visible navigator edge share one effective boundary at normal and resized widths; long project/document titles cannot overlap adjacent labels or utility controls (owner feedback F-19). |
| UX-04 Native command behavior | File/New/Open/Save/Find/Undo/Redo have conventional macOS behavior, coherent menu/shortcut labels and accessible names. Context-specific commands operate on the focused surface. A raw shortcut handler cannot open unrelated dialogs above another active dialog, including two shortcut events received within one frame. Creating a project/document with Enter and with the button must keep the app running and show the created item. |
| UX-05 Focus and transients | Keyboard reaches and activates every primary control. Context menus preserve the selected occurrence. Escape dismisses the innermost transient first and restores the previous caret/focus. A focus-mode button has a visible exit and restores the previous pane layout. |
| UX-06 Selected occurrence | Freeze document, revision, block/range, direction and exact quote at an explicit action. Select the second identical phrase and act on that occurrence alone. Selection alone does not send text, create a note or change tool scope. Verify Cyrillic, emoji, combining marks, reverse and cross-block selection. |
| UX-07 Anchored notes and entities | Notes and dictionary spans retain provenance, exact ranges and explicit detached/ambiguous states. Clicking a note reveals its quote; deleting text never attaches a note to a different matching phrase. Decorations do not rewrite canonical document JSON. |
| UX-08 Review that can be understood | Clearly identify document, scope, origin and unapplied state. Show a readable changed excerpt with sufficient context and old/new emphasis; do not force the author to compare two full chapters in a narrow inspector. Apply/Reject remain discoverable near the relevant change. Multi-document review expands to a suitable central review surface. |
| UX-09 Stale/cancelled/applied states | Editing the base visibly marks a proposal stale before a predictable failed Apply. Preserve the suggestion for revision. Cancellation prevents late output from applying. Applied state offers guarded Undo and explains conflicts without overwriting later work. |
| UX-10 Save and recovery | Save state reflects actual durability. Failed saves keep text and a visible route to retry/export/recovery. Avoid repeating modal alerts on every autosave retry. A recovered/conflict copy is named, findable and comparable; success is not only a fleeting footer message. |
| UX-11 Voice workflows | Dictation and assistant voice are explicit different modes. Record/pause/resume/stop, input level, duration, capture vs recognition status and finalization are clear. Network/provider failure preserves audio. Audio review links time and text while keeping the manuscript available. |
| UX-12 Settings and appearance | Light/Dark/System and palette are independent. Six non-green legacy palettes plus blue remain selectable. Settings have meaningful categories, aligned labels, previews and clear persistence behavior. Use a stable category sidebar, bounded content and fixed reachable footer; distinguish operational controls from explicitly requested informational parity scaffolds (F-20). Connection tests report actual results. Test all combinations for legibility, focus and selection, not only token contrast. |
| UX-13 Localization | Test eight built-in languages, long German/French labels and CJK text. Switching language preserves draft, focus/scope and content. Native file dialogs use expected OS language behavior. No missing glyphs, clipped actions, raw msgids or untranslated app messages. Glyph coverage is not IME acceptance. |
| UX-14 Honest capability and navigation | Unsupported capabilities do not look operational. Preview/import/export language matches the actual operation. Every secondary view returns to its source with meaningful scroll/caret/selection context. Multiple assistant chats have independent conversation state and a shared manuscript. |
| UX-15 Useful structure | The project navigator supports the actual project schema, not only novels/flat chapters. Large projects, notes, research, boards, trash and project utilities have clear locations; progressive disclosure avoids making all tools permanent chrome. |
| UX-16 Accessible interaction | Controls have visible focus, accessible labels/roles and conventional keyboard activation. Screen-reader/caret/selection behavior is verified on the real native implementation; custom drawing cannot substitute for an accessibility tree. Contrast tests do not prove accessible operation. |

## Required evidence bundle

For a change touching the implemented UI, retain:

1. The declared user journey, OpenSpec requirement, related feedback IDs and component boundary.
2. Build/revision and actual native screenshots, with window size, language, appearance and palette. Renderer screenshots exclude OS chrome; obtain native-window evidence when evaluating traffic lights, window movement or system menus.
3. A short interaction record covering success, cancellation/escape and the relevant failure or stale state. Use isolated demo data; do not alter an author’s running project for a review.
4. Targeted domain/interaction tests and the affected adjacent cases. Screenshot likeness cannot prove exact range identity, persistence or authority.
5. Independent findings, implementation response and retest disposition. Keep open issues visible; never mark work complete merely because review documents exist.

Baseline matrix for an Author pilot: macOS minimum and common window size; navigator/inspector combinations; Light/Dark; all palette swatches; ru/en/de/fr/es/pt/zh/ja; keyboard-only primary flow; fresh/empty/populated project; long/repeated/Unicode text; no-provider; save failure; stale Apply/Undo; recovery. Expand with each implemented workflow from the map. The feature scope is the existing source-backed Writers Studio inventory; NER-like markup is an interaction analogy, not a new standalone capability. Do not add local inference or other new product functions to satisfy this UX gate.

## Feedback-driven adaptation

When the product owner requests a change after a review:

- Append the actual request to `USER_FEEDBACK.md` with date, scope, affected journeys and whether it corrects an existing miss or introduces a new preference/requirement. Do not invent reasons or treat an optional idea as a mandate.
- Link the affected finding/spec. If the prior approval no longer holds, mark it superseded; do not rewrite its historical evidence.
- Add a concrete regression scenario to this gate or a linked capability spec. Update the reviewer’s preparation instructions when the miss was systematic (for example, repeated-phrase selection or a forgotten palette).
- Change the implementation and repeat independent acceptance for that scenario. An acknowledged comment is not a verified fix.
- Future reviewers read the log and unresolved findings before proposing a layout. Repository history is the durable memory; do not promise invisible model learning.

The designer’s rejection returns the change to implementation within existing user authorization. It does not create another user-permission gate for ordinary reversible fixes.

## Machine-readable release decision

The independent reviewer owns `docs/design/UX_GATE.json`. It records a named scope, version, current source/design fingerprint, concrete blockers and hashed native evidence. The JSON is a serialized result of this review process, not an automatic approval. A changed implementation or design contract invalidates accepted evidence until retested. `pending` or `changes-required` permits local source work/previews but blocks a tagged release; only a current `accepted` decision with no blockers satisfies the UX part of the release gate.

A `native-foundation` engineering-alpha acceptance does not approve full author-pilot or complete legacy parity. Versioned review prose must state which actual interaction paths were observed, which were exercised through automated domain/controller tests, and which native behaviors remain unqualified. Keep repository prose and msgids English; preserve localization values and author content.

## Shell regression added after owner feedback F-19

Before accepting a shell change, compare the toolbar boundary and navigator edge in the same actual compiled frame. Exercise navigator hidden/shown and both resize limits; narrow/common/wide windows; inspector requested at compact width; long German and CJK project/document titles; default and maximum manuscript type size. Verify allocated title rectangles and independent utility hit targets, not only absence of horizontal scrolling. A divider placed by a separate fixed padding value is insufficient evidence of shared pane geometry.

The reviewer must look for cross-component alignment and overlapping identities explicitly after F-19. Re-run these cases when the toolbar, navigator, type scale or responsive policy changes. A source correction requires new visual evidence; the owner's rejection cannot be closed solely by a layout formula or screenshot from an earlier build.

## Settings regression added after owner feedback F-20

Read the structured preferences contract in `DESKTOP_UX.md` before reviewing settings. Inspect General, Appearance, Editor and one unimplemented parity category; assess spacing, grouping and category placement as well as functional controls. Test minimum/common viewport, Light/Dark, German/CJK labels, maximum manuscript type size, category switching, keyboard focus, Done/Escape and persistence with an unfinished draft. Reject fake editable controls or implied operational providers/MCP/voice in a scaffold. A category list by itself is not acceptance of its future workflow.

### Owner correction regression matrix: F-21 through F-24

- Header/title/close control share a vertical center; the footer is bounded and Done has equal vertical insets. Verify normal and minimum-size Settings captures.
- Settings group headings align left with the navigation grid. No centered group labels, overlapping text or mixed utility icon families.
- All utility glyphs use the licensed Lucide subset; product branding uses the approved lion-writer mark. Check 32-pixel toolbar readability and light/dark contrast.
- Appearance preview cards display actual theme colors and keep mode and palette as separate controls. Selected state is legible without relying solely on fill color.
- Show the real complete updated interface and open the owner's isolated demonstration preview after the icon replacement.


## Owner correction regression matrix: F-32 through F-35

- Require a complete empty-document authoring journey for every existing block/mark. A known-good showcase verifies appearance only; discoverable Insert/context/keyboard commands, edit again, Cancel, Undo/Redo and durable reopen need their own interaction evidence.
- Inspect real syntax decoration for programming and structured-text languages while editing and after reopen, in Light/Dark. Source, Copy and unknown stored labels remain exact; colorful mockups do not qualify syntax support.
- Compare wide/Focus/joined manuscript states around the approved adaptive threshold. Keep a bounded text measure, balanced exterior gutters when present, local scroll, reachable full toolbar/overflow and a visible Focus exit. New adaptive concept pictures need owner image review before structural layout implementation.
- Inspect actual compiled search/New document/New project/source fields and modal actions: 40 px minimum standard height, full available width, centered text, one 24 px inset grid, deliberate header/body/footer rhythm, localized long values and minimum-window fit. Exercise Tab/Enter/Escape/Cancel, disabled/error states and exact focus restoration. A larger painted border around an unchanged tiny interaction target fails this check.

Do not access the owner's open preview for these checks until handoff. F-32–F-35 remain acceptance blockers until exact-source compiled/native evidence exists; the designer's source fixes and design artifacts do not change `UX_GATE.json`.

## Owner correction regression matrix: F-37 native rich-block flow

The owner-observed overlap in [bug #12](https://github.com/avacadorun-dev/writer-craft/issues/12) is P1 and blocks the rich-writing journey. Read F-37 and the alpha.2 review addendum before accepting any layout, typography, renderer or progressive-readiness change. A cropped failure screenshot establishes a visible layout defect; canonical corruption requires separate storage evidence.

Required evidence and checks:

- Capture the actual compiled affected viewport after the expected visible images, diagrams and formulas have acknowledged their current source/presentation identity. Record app/engine/lock identity, effective preferences, viewport, scale, visible atom IDs and readiness. Fixed frame counts or elapsed sleeps do not establish readiness. An unexpected renderer failure is a failed valid-fixture case, even when its fallback is safe.
- Use the complete-block synthetic document with prose, all heading levels, multiline highlighted code, ordinary/merged-cell tables, captions, portrait/landscape images, flowchart and tall sequence Mermaid diagrams, inline/block formulas, callouts and separators. Reach the actual opaque blocks by deterministic targeting or scrolling; above-the-fold prose does not exercise them.
- Compare the **painted** image/diagram/formula/caption bounds with their reserved flow allocation and adjacent independent sibling blocks. Full content must remain readable at the correct aspect/baseline, with valid clipping at the viewport. Test overlap between separate blocks; intentional nesting inside a callout/table or internal diagram edge/node composition is not that failure.
- Cover all three manuscript families; 17/19/21 and retained 28px; line-height 1.5/1.75/2; measures 60/70/80ch; Light/Dark; and floated/joined/Focus states. Use an explicit coverage table for independent dimensions and expand the known failing font-size/line-height/opaque-content combinations. A wasteful full Cartesian screenshot set is not required; every value, relevant interaction and adaptive boundary must have named evidence. Headless bounds tests complement the native cases.
- Exercise pending→ready, source edit→rerender, failure→retry, theme invalidation, font/measure reflow and cache eviction/reentry. Verify sufficient space during transitions, stable meaningful scroll/caret/selection, correct hit targets, and no stale pixels or late result entering another block/document.
- Preserve and compare exact canonical JSON/assets, semantic selection, Undo history and revision state around presentation-only changes. Verify insertion/property edit/delete/Undo/Redo, save and reopen for each affected block family. A presentation repair must not rewrite the manuscript to hide overlap.
- Supply a fresh exact-source independent retest with complete affected block captures and interaction evidence. The owner review, implementation source fix, abstract geometry assertion or startup-only screenshot is not a substitute. `UX_GATE.json` remains pending until the required review actually passes; product issues remain open for final owner acceptance.


### Capture coverage audit after 182c9a3 review

Record effective logical viewport, device scale and PNG dimensions alongside the requested size. The three 182c9a3 startup cases labelled wide requested 1440×940 but produced 1024×684 PNGs without effective viewport/scale metadata; they cannot qualify wide geometry. See `UX_CAPTURE_REVIEW_182C9A3.md` for observed narrow states and the minimal target topology.

An opaque-block capture must name expected nonzero visible atom IDs/types, establish current ready outputs and stable resolver/layout state across rendered frames, and bind its sidecar to the captured frame. Empty or wrong-target visibility, failed valid-fixture rendering, mismatched viewport/state and startup-only readiness must explicitly degrade the affected coverage claim. Preserve PNG integrity checks and separate native interaction evidence.


## Owner-approved alpha.2 checkpoint boundary — 2026-11-08

The owner explicitly authorized releasing the current completed coherent native editor checkpoint and moving unfinished large blocks to alpha.3. For this prerelease, the independent named scope is `native-rich-editor-alpha`: the implemented writing workspace, nonoverlapping currently integrated block rendering, bounded manuscript/controls/preferences and installable packages. Review the actual final source and captures; block P0/P1 defects within that scope. Retain specific limitations, pending broad original-parity, IME/accessibility, full Canvas/media/import/export and rich-assistant checks. Historical full-editor alpha.2 acceptance matrices remain the alpha.3 migration target, not a false declaration that those checks passed. Public engineering-alpha publication does not close product issues or certify daily use. The independent reviewer must make the final scoped decision; the implementer must not rewrite a pending full-parity gate into an approval.


## Correction matrix: F-42 and F-43 — connected Canvas writing

- A new or existing text card is writable in place with a clearly discoverable editing state. Exercise actual native text selection, paste, CJK composition and text Undo at several board zooms, clipped card edges and minimum window size. Card movement, deletion and board shortcuts must not steal typing. Verify new-card focus and returning to board manipulation.
- Save acknowledgement never replaces a newer draft or cursor. Exercise typing during save, failed/stale save, retry, cancellation, navigation and reopen with exact content/revision checks. Modal-only editing cannot pass #15.
- Review the selected board↔document/character/lore relationship design against PT-LINK-01, PT-CHAR-01, PT-LORE-01 and PT-LINK-02. The author can understand whether they are editing a shared record, a live reference, a snapshot or an independent note, and can return to the exact manuscript/board context.
- Do not accept inferred automatic synchronization, navigation-only demonstrations or silently copied divergent text as a completed connected-writing workflow. Test rename/move/delete/restore, missing/stale references, separate layout/content Undo and compatible export/restore after scope selection. Written hypotheses and source preservation alone do not supply native acceptance.

## Two mandatory axes for alpha.3

The existing `UX_GATE.json.coverage` must accept every mandatory inventory, capability and role journey in `docs/migration/PARITY_SCOPE.json` before `full-editor-parity` or `full-parity` release acceptance. This is enforced before1.0 as well. Inventory sign-off explicitly confirms an independent omission review. Each capability needs a reachable native entry, semantic preservation evidence and native interaction evidence. Each journey covers the authored main/return-context/cancel/stale/failure-recovery/save-reopen/compatibility paths. See ROLE_JOURNEYS.md and F-45.

Every accepted record names the exact source/lock/contract fingerprint, independent reviewer and SHA-256-bound evidence reports. The full accepted gate also hashes its narrative interaction report. A changed contract, implementation or report invalidates the associated acceptance. The validator checks consistency and missing evidence; the reviewer remains responsible for actually observing the tasks and challenging omissions or awkward workflows. Two PNGs, a source audit, raw-data preservation, an engine test or a written persona cannot establish native completion or user-research validation.

Reviewed Markdown evidence begins with a `writercraft-evidence` HTML-comment JSON receipt declaring `source_fingerprint`, `build_ref`, `platform` and explicit `observations` with `kind`/`id` (and all `paths` for a journey). The validator checks the report's own candidate and observation claims, not only a hash copied onto a coverage row. The remainder records the actual steps/results, test/build URLs and limitations. A report can cover multiple IDs only by explicitly listing and substantiating them; this is not permission to fabricate observations.

The current contract declares only `full-editor-parity`. Requesting `full-parity` fails until an expanded full-product catalog and its independent coverage are authored; changing the label to1.0 cannot reuse the narrower alpha.3 inventory.


## Correction F-46 — existing-block editing and image reduction

The earlier #7 insertion/model evidence does not qualify discoverable re-editing of existing rendered blocks. Independently exercise an existing Mermaid block and block/inline formula in the exact compiled candidate: double-click opens the captured current source, a visible selected-block Edit action and keyboard path reach the same guarded editor, Apply rerenders, Cancel/Escape changes nothing, Undo/Redo and save/reopen preserve identity and surrounding prose. Verify stale/failed edits preserve the draft. Do not accept only a toolbar-source reading or insert-dialog screenshot.

For selected #14 sizing, make an already inserted image visibly smaller, retain aspect ratio when locked, Undo/Redo and reopen, and compare original asset bytes and stored dimensions. Workspace zoom alone fails this scenario. Canvas B01–B26 and its independent review remain mandatory and active under #17.
