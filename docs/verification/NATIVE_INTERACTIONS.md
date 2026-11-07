# Native interaction verification — 2026-11-08

Platform: macOS Apple silicon. Build: WriterCraft 0.1.0-alpha.1 debug, native egui/eframe renderer with AccessKit enabled. Computer-use actions operated a separately identified QA app and a newly created synthetic workspace. No original Writers Studio projects were opened or changed. The owner's running preview was not replaced.

## Observed complete review workflow

1. Open the English sample at revision 1. It contains two occurrences of `the lighthouse`.
2. Select the second occurrence using its surrounding context: `and ` / ` itself stood behind the house`.
3. Secondary-click the manuscript. The native accessibility observation still reports the selected text, and the context menu exposes Copy selection and Prepare an edit from selection.
4. Choose Prepare. The scope is exactly Unicode scalars **486–500**. The Proposed text field receives focus; the manuscript remains unchanged.
5. Type `the old lighthouse` into that focused field and choose Prepare proposal. Status reports manuscript unchanged; revision remains 1. Review excerpts contain only the intended contextual change.
6. Apply. Revision becomes 2; the third paragraph contains `the old lighthouse`. The first occurrence in the preceding paragraph remains `the lighthouse`.
7. Undo apply. Revision becomes 3; the full original sample text is restored.

This is a complete native interaction run against repeated text, separately from the core/controller and raw-egui input regressions.

## Creation, focus and modal routing

- Command-N opens the project dialog with the title field focused. Typing `Native QA project` and pressing Enter creates the project and First chapter. The app remains running; the new manuscript is focused.
- Typing a synthetic sentence and Command-S persists it at revision 2.
- Command-comma opens categorized preferences. While the modal is already present, Command-N leaves the same modal visible and creates no project dialog.
- General opens the language selector with all eight supported languages. Selecting Japanese changes UI labels, including the categories and Done, while preserving the English manuscript and user-created names.
- Escape first dismisses an open language popup, then dismisses Settings. Focus returns to the existing manuscript; no accessibility crash occurs.
- Focus hides navigator/review and exposes a visible localized Exit focus action. Earlier native checks also observed Exit focus restoring the requested panes; current controller regression covers the reversible state.
- The isolated QA app is quit normally after the checks.

## Failure found and corrected

An earlier native Enter run terminated with `accesskit_consumer` reporting a focused node absent from its tree. The modal close path requested the next document's editor before that editor had rendered. Focus is now deferred until its TextEdit emits the accessibility node. The new regression `project_creation_enter_keeps_accessibility_focus_on_an_emitted_node` validates each tree across typing, submission and the next frame. The native successful creation run above verifies the correction.

A same-frame batch of two modal-opening shortcuts previously stacked dialogs. The command dispatcher now returns after opening a modal; `shortcuts_in_one_frame_never_stack_two_modals` covers that input batch. The native already-open-modal guard was checked separately.

## Verification boundaries

Native Shift-F10 injection did not expose the popup in this automation session. Its raw-egui input regression passes; the native keyboard route remains unqualified. The native secondary-click route was observed end to end. Do not describe synthetic input tests as a native keyboard/IME qualification.

Persistent save-conflict recovery was exercised by deterministic controller/storage tests, not an injected native disk failure. Windows/Linux native input, IME composition, screen-reader usability and full rich-document parity remain separate acceptance work. Actual compiled screenshots are complementary geometry/appearance evidence, not proof of those workflows.

## Final updated preview launch

After the final Lucide, monk/identity, title constraints and preferences integration, the packaged debug app was opened through native computer use. The observed window shows version 0.1.0-alpha.1, the reading-monk mark, Lucide controls, the Russian interface, a three-document synthetic project, a continuous manuscript and the saved-state indicator. A subsequent native screenshot/AX observation confirmed the window remained present. The app was left running for the owner to explore, without further automated clicking. This preview uses a separate persistent Demo workspace, not an original Writers Studio project.

The final local workspace check passed **71 tests** (28 core, 32 desktop, 11 transfer), formatting and strict all-target/all-feature Clippy. The packaged app passed ad-hoc code-signature verification and source-bundle metadata checks. The editor-model qualification harness separately passed 12 tests; release/portable-packager checks passed 9 and 4 tests respectively. None of these grant full-parity, IME or Windows/Linux desktop-interaction acceptance.

## Application-icon startup correction (F-28)

The runtime framework's default icon was previously selected because no viewport icon was supplied. The final source explicitly loads `assets/WriterCraft.png` into the native viewport, while the macOS bundle retains `WriterCraft.icns`. The PNG is byte-identical to the generator's 512px output (SHA-256 `9f14929016a6991929e337fbae0932302aebd2da7c963c409c95096360394b70`). Both outputs are reproduced by the icon-generation script.

After rebuilding and re-signing the proper debug bundle, native computer use opened a fresh WriterCraft process. Its standard macOS About panel visibly showed the monk-and-book application icon and WriterCraft/version metadata. Closing that panel returned to the saved manuscript; the updated app was left running for the owner. This is an observed OS application-icon/bundle check. The Dock was not independently captured by the automation surface; keep that distinction in issue #1 until confirmed.

The nine accepted client-surface captures predate only this startup/OS-icon delta. Their pane geometry, theme UI and utility rendering were unchanged; they are retained as client evidence rather than represented as recaptured OS-icon evidence. The current source again passed 71 tests, formatting and strict desktop/workspace Clippy after the icon change. The independent review separately assesses this bounded delta against the updated fingerprint.
