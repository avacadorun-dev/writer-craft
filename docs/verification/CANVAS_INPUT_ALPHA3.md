# Canvas direct-input correction — alpha.3

Date: 2026-11-09. Scope: issue #15 only. Full Canvas parity remains #17; no full-product or OS-input acceptance is implied.

## Qualified generic dependency

CraftCanvas `4be05e9430639d1c666e416ad96d5f668686d48b` passed committed-lock verification, model/native adapter tests, formatting and strict Clippy on Linux/macOS/Windows in [run42560855344](https://github.com/avacadorun-dev/craft-canvas/actions/runs/42560855344). Its 23 native-event tests retain IME platform output and cover actual creation handoff, Unicode input, exact insertion acknowledgement, composition, owned Escape, arrows, Tab/accessibility, disabled/foreign input and local history.

The preceding run42560555889 compiled but failed the headless test harness's required texture-output disposal. The followup explicitly releases headless texture/command resources while preserving IME flags and assertions. No local Cargo build/test/fetch was used.

## Application implementation under qualification

The application mounts one provisional editor before durable insertion and retains it through the exact acknowledgement. Empty pending cards block navigation. Retry reuses the original raw insertion/base; valid recovery prepares original insertion plus latest text atomically. Cancel freezes input and waits for the original write; Done waits for the latest committed input after painting.

For unpreparable drafts, a versioned manual-recovery JSON preserves exact board/node/candidate JSON strings and latest Unicode text. It is not an importable project or valid `.canvas` replacement. Successful complete flush/sync enables explicit Reload; later input invalidates that permission. Separate media is not bundled.

Independent source review identified and required these corrections before acceptance:

1. Done could remove the editor during persistence polling before same-frame Text/Paste/IME reached it. Completion now occurs after native input.
2. Keyboard Cancel could still accept the same batch's text. The generic widget becomes read-only immediately on a valid noncomposing Cancel.
3. Capacity refusal could prevent both Retry and valid-board preservation. The separate lossless recovery record provides an escape without corrupting a graph or discarding text.
4. Reload could use the previous frame's preservation permission before new input invalidated it. It now queues after card input, checks current-frame text/IME and active composition, then revalidates preservation.

The final bounded source re-review found no remaining blocker in these changes. It did not execute Cargo or native UI.

## Required evidence still pending

- Exact integrated application tests, Clippy and complete OpenSpec checks.
- Compiled native first-character creation, in-card selection/paste/local Undo, OS IME, clipboard and focus transitions.
- Zoom/clipping/scrolling, compact/wide surfaces, Light/Dark and localization review.
- Delayed/failed/stale saves, cancellation, exact preservation, restart and Studio/export compatibility.
- Independent review of the same compiled candidate before issue closure.

Engineering tests and screenshots cannot substitute for the remaining complete native journey. The authoritative full alpha.3 UX gate remains pending.
