# Independent UX review 001 — native foundation

Review date: 2026-11-07. Reviewer role: independent UI/UX designer. Scope: actual first native screenshot plus the current working source; no interaction with the owner's running application or modification of their settings/projects.

**Disposition: engineering probe only; changes required before Author pilot acceptance.** The flat three-pane composition is a credible foundation, but selection scope, review ergonomics, constrained layout and transient/focus behavior are not yet qualified for daily author use. This is not a rejection of Rust or a demand to implement every future feature in the first probe.

## Evidence and limits

- Observed actual render: `artifacts/screenshots/workspace-dark.png`, SHA-256 `03c9f94fd2ca37d6707ec7f076c10d677e31fc784308b61642afde691747d17b`, produced before later toolbar/i18n fixes. It shows real manuscript, navigator and review inspector in dark blue. It is a renderer capture, not evidence of OS window controls or menu integration.
- Source reviewed: `crates/studio-desktop/src/app.rs` functions `toolbar`, `navigation`, `editor`, `review`, `settings_dialog`, `shortcuts`, `nav_row`; `icons.rs`; `theme.rs`. This source was changing during localization integration. Findings therefore name behavior/functions rather than pretending a frozen released revision.
- Contracts: `DESKTOP_UX.md`, `PRD.md`, `FEATURE_PARITY.md`, legacy `WORKFLOWS.md` WF-01–37, annotations/rich-editor/Canvas/transfer specifications and current Canvas source. Some old narrative docs lag current implementation; source inventory must resolve that before parity acceptance.
- No keyboard, IME, screen-reader or live minimum-size pass was performed by this reviewer. No light/CJK screenshots were available at this review baseline. Tests or later source changes are not retroactive visual evidence.

## Current primary-flow findings

| ID | Severity / state | Evidence and impact | Required correction / retest |
|---|---|---|---|
| R001-01 | P1; open | `editor` copies a quote into `expected`; context action also retains only the string. `stage` calls unique-substring `stage_replace`. Selecting the second repeated phrase can fail or lose its occurrence identity, despite a selected-text action label. | Freeze document/revision/range/quote at explicit action; stage that exact range. Test repeated second occurrence, reverse selection, Cyrillic/emoji, edit-before-action and stale Apply. Quote-only manual search can remain a separate clearly scoped tool. |
| R001-02 | P1; open | `review` sends full `proposal.before` and `proposal.after` into two narrow inspector blocks. A long chapter produces duplicated long text and pushes Apply far below the actual change. | Render changed excerpt with sufficient surrounding context, old/new emphasis and document/scope label. Keep actions near the change; offer source reveal and expanded review for long/multi-document output. Test a one-word change inside a long chapter. |
| R001-03 | P1; source risk, execution pending | Navigator can reach340px and inspector460px. At the1000px minimum window, the remainder approaches200px, while `editor` clamps text to at least240px plus margins. No inspector-first adaptive policy is visible. | Enforce a useful central minimum and avoid overflow/clipped commands. Test all resize limits at minimum/common widths, long project titles and German/CJK labels; preserve/recover requested panel layout. |
| R001-04 | P2; open | The visible Focus button always sets both panels false; its label/state remains unchanged and clicking it again cannot restore the previous layout. Shortcut restores both, not necessarily the user's prior arrangement. | Provide a visible reversible Focus toggle and restore previous navigator/inspector widths/visibility. Test entry with either/both panels hidden and exit by button/shortcut/Escape policy. |
| R001-05 | P2; open | Any focused editor selection updates the review form before the author chooses an action. Original behavior intentionally uses explicit context actions, and the current quote field can look like attached model/edit scope without an explicit freeze. | Selection alone should remain selection. Capture a revisioned scope only through an explicit context action/shortcut; display a stable scope summary and let the author replace/clear it deliberately. |
| R001-06 | P2; open | Staged cards remain visually actionable after the base has changed; the core refuses stale Apply but the author first discovers this after clicking the primary action. | Show stale state as soon as the known base differs; preserve the proposed text and offer an understandable refresh/review path. Check subsequent manual edits and guarded Undo with clear conflict guidance. |
| R001-07 | P1 qualification gap | `dialogs` creates several ordinary floating egui windows, while `shortcuts` globally processes New/Open/panel toggles without dialog ownership checks. No explicit innermost Escape/focus return policy is visible. | Choose modal or deliberately modeless/owned-preferences behavior; enforce focus/command routing accordingly. Prevent stacked unrelated dialogs. Test Enter/Escape, settings→language switch, text-field editing, file-dialog cancellation and return to the original editor caret. |
| R001-08 | P2; open | Review form is permanently open in the default writing screenshot even without a proposal or provider; it consumes much of the window and presents a technical exact-replacement form as the main secondary activity. | Prefer a quiet inspector/closed default for unrequested tools; open contextual review when selected or when a result is ready. Keep the local exact-edit utility honest and accessible, without using it as a stand-in for the future assistant UX. |
| R001-09 | P2; verification pending | Header title/project identity, theme palette labels and status content lack fresh narrow/long-label screenshots. i18n now adds substantial expansion. | Capture actual minimum/common width, Light/Dark and long German/CJK settings. Verify font size controls do not make settings/actions unreachable. Visual quality is not established by catalog coverage or color tests. |
| R001-10 | P2; verification pending | Failed autosave is reattempted on a timer and reported through the error window. Repeated failures could reopen/distract while the author is preserving text. | Simulate persistent save failure; keep a durable unsaved indication and explicit retry/export/recovery route. Avoid repeated intrusive modal presentation. |

## Earlier defects already addressed in source

The first screenshot's toolbar stops after File. Later source removes the full-width MenuBar container, so treat this as **source-fixed, fresh-render verification pending**, not an unaddressed screenshot defect. The titlebar drag region is now registered before child controls, addressing the previously confirmed hit-test blocker; actual toolbar click/drag retest remains required.

The current source reloads a document from the core after saving instead of trusting an old navigator snapshot. Revision-pinned recovery now preserves a differing stale draft as a separate sibling. These are valuable integrity fixes; core tests support their domain behavior. User-facing discovery of that recovered copy still needs a journey check.

## Full-product acceptance remains separate

Rich editing, anchored notes/dictionary/NER, real model/provider setup, dictation/audio review, Canvas, named versions, complete migration and publishing are not implemented in this foundation. Their absence must remain visible in the parity inventory and workflow map. Do not solve the gap with permanently disabled toolbar buttons, empty generic panels or labels suggesting an available assistant. Each capability earns its own native workflow acceptance before a parity claim.

The confirmed first release is macOS and the primary manuscript is continuous within a bounded pane. Forcing page breaks into the everyday editor is not required to satisfy desktop quality; publication pagination is a later dedicated flow.

## Component and state boundaries

Split growing presentation by coherent user responsibility, with events returning to one command/controller path:

- `AppShell`: native/window chrome, shared command bar, responsive pane layout and status. It does not own document mutation semantics.
- `WorkspaceController`: selected project/document, save/recovery/proposal state, asynchronous jobs and command dispatch; reusable pure view state where possible.
- `Navigator`: hierarchy/search/keyboard navigation and explicit selection events.
- `ManuscriptSurface`: caret, selection, rendering/input and read-only states; emits a typed revisioned selection context, not an arbitrary global quote string.
- Peer `ReviewInspector`, `NotesInspector`, `AssistantInspector`, `VersionsInspector`: each has a clear state model; large comparisons use the center workspace instead of stretching a narrow sidebar.
- `Settings`: stable categories, language/theme controls, connection state and preference persistence; one explicit transient/owned-window policy.
- `FeedbackAndRecovery`: save errors, recovered copies, cancellation and source navigation without repeated context loss.

A file split alone is not UX improvement. Components need independent states/actions and must preserve the author's working context when composed. Do not create a general widget framework before the product flows require it.

## Retest handoff

Implementation owner should supply new isolated-data screenshots and interaction evidence for R001-01–10. Reviewer then records exact build, changed behavior and disposition. No Author pilot approval is granted by this document. Historical findings remain; later fixed/verified status is appended or linked in a follow-up review. User corrections update `USER_FEEDBACK.md` and the acceptance gate, rather than being acknowledged only in chat.
