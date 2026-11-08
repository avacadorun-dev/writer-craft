# Authoring source review — 2026-11-08

Disposition: **changes required; source review only**. Scope: the issue #7 authoring integration, #9 shared controls and #6 adaptive document surface in the `feature/full-editor-parity` working tree, based on application commit `3db231c` and engine API revision `414cbeaa9257b1b28f1e6f5dd56b440bf6195adf`. The implementer was changing the working tree during this pass; this is a finding/rework record, not acceptance of an immutable build.

Reviewed `app/authoring.rs`, `rich_editor.rs`, `rendering.rs`, `components.rs`, `dialogs.rs`, application modal orchestration and localization integration. Consulted local egui 0.36.2 source and the pinned engine API for selection/history behavior. No Cargo, dependency fetch, native app interaction, screenshot capture or user-data mutation was performed. No release gate was changed.

## Findings and rework

| Finding | Severity | Source disposition |
|---|---|---|
| Replacing selected inline math through plain `apply_commands` invalidates its exact node selection; editing shorter code can leave an invalid text offset. Subsequent input/history must stay at the edited node. | P1 | Implementer changed inline atoms to exact post-edit `NodeRef` selection and code to an explicit clamped selection in the same history entry. Source rechecked; semantic/native regression remains required. |
| Cancelling a second image chooser discards a previously chosen pending image. | P2 | Implementer now replaces the pending path only when the chooser returns `Some`. Source rechecked; native chooser cancellation remains unobserved. |
| Formatting buttons expose glyph-only accessible names although localized labels exist. | P1 for accessibility acceptance | Implementer added localized `WidgetInfo` labels. Source rechecked; actual accessibility navigation/selected/mixed announcements remain pending. |
| Opening the link dialog expanded the current link range before confirmation; Cancel did not restore the original directed selection. Dialog application also depended on a mutable transient selection. | P2, with exact-target implications | Implementer now captures selection/node and restores the captured target before Apply. Link extension occurs on Apply, not on opening. Error-then-Cancel restoration and boundary-prefill cases below still require verification. |
| A caret immediately after a marked link and before an unmarked run produces an empty initial href, while `extend_mark_range` later correctly chooses the left link. Confirming an apparently empty dialog can therefore remove that link. | P1 | Required rework: mirror the shared engine's right-marked-then-left-marked boundary lookup for initial href without mutating the actual selection. Add a boundary fixture. After a failed Apply that expanded the selection, Cancel should restore the captured selection when the revision still matches. |
| Preparing any changed atom/theme hides the whole manuscript behind a spinner until the complete media catalog is ready. Slow/failing diagrams prevent editing unrelated prose. | P1 for daily authoring | Required follow-up in #7/#4: keep native prose and available exact cached atom output usable, with per-atom pending/failure handling and generation guards. Do not accept the all-document blocking spinner as full authoring parity. |

Existing callout-kind changes are now reachable, but any new control must remain justified by original functionality or explicit owner instruction. Do not infer a new product feature solely from a generic engine method. Source-authoritative checks govern whether the original UI exposed a kind change.

The heading-selection endpoint-at-zero case was raised as an investigation question and then explicitly withdrawn as a confirmed bug: ProseMirror block token semantics require an actual source command probe. Do not change it merely to match the list/quote bounds helper. The implementer requested that source probe separately.

## Shared control and layout observations

- egui 0.36.2 computes the TextEdit minimum height as the maximum of line height plus frame margins and `min_size.y`; the proposed 40-point field minimum does not inadvertently become 56 points. The single-line sizing/vertical alignment APIs are present in the inspected library source.
- The shared modal uses one 24-point frame inset, a 44-point title row and a reachable trailing action row. Its actual theme, locale, keyboard and minimum-window rendering still need compiled evidence.
- The adaptive source uses the approved width-based floating/docked choice and capped prose measure, rather than an inspector-visible flag. Verify frame-stroke geometry, title/toolbar boundaries, viewport clipping and scroll/selection retention in a compiled 1440×940 and 1000×680 application.
- The current row retains secondary commands in labeled overflow. Full locale width budgeting, exact active/mixed state semantics and contextual discoverability must be verified before #6/#7 closure.
- Programming language names/identifiers are technical content, while ordinary field/action text must be localized. Syntax highlighting, code copying and the native accessibility text tree remain separate required acceptance work; their absence is not approved by this source pass.

## Acceptance boundary

The issue #1 icon closure is based on independent owner confirmation and is unrelated to this review. Issues #4, #6, #7, #8 and #9 remain open until their complete checks pass. No native/UX acceptance, alpha.2 readiness, package release or full-project migration claim follows from these source corrections.

Root follow-up, 2026-11-08: link-prefill now uses a temporary semantic session to obtain the original right/left boundary range without changing the author selection. Apply restores the captured directed target before extending/patching the link; Cancel restores it only against the unchanged revision. Added exact boundary/cancel/Undo fixtures plus inline-formula, shortened-code and captured-insertion fixtures. These source corrections and fixtures await hosted execution and native review. The whole-manuscript media-readiness finding remains open and is assigned separately. The original installed Tiptap probe confirmed inclusive heading/list end-at-zero block behavior; do not apply the speculative half-open correction.


## Integrated source follow-up — 2026-11-08

The subsequent qualified application checkpoint `4dfe35d` passed all 160 workspace tests, strict all-feature Clippy, notices and macOS packaging in [hosted preview run 42465520996](https://github.com/avacadorun-dev/writer-craft/actions/runs/42465520996). This includes the 11 progressive-media fixtures; the whole-document blocking spinner finding has a tested semantic/source correction. Native scrolling, reflow and interaction acceptance remain pending.

The next integration uses the official Lucide subset for formatting, Insert and properties, mounts the persistent note inspector and post-input marks, and keeps inspector category selection reachable through a measured compact selector when translated tabs would overflow. Native copy/paste uses the application-lifetime registered-format provider and bounded worker with explicit retained-content recovery. These new changes require their own hosted/native validation.

Code properties now use granular shared `ReplaceText`/`SetBlockAttrs` commands for existing code, retaining untouched imported leaf metadata, exact position maps and a single Undo transaction. Unchanged omitted language metadata stays omitted. The new imported multi-leaf Unicode/Undo fixture is pending hosted execution at this source checkpoint. A properties Copy command supplements, and does not establish completion of, the original code-block inline chrome journey.

The owner approved the adaptive wide/focus/docked design in F-34. This remains design approval, not the independent compiled editor gate.
