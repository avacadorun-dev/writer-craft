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

## Integrated checkpoint evidence

Application `a55f35939a07bf0b217b67bea7bea056aaab6b2f` passed the complete default/editor-enabled workspace tests, strict Clippy, isolated editor compatibility, OpenSpec and release-metadata checks on macOS, plus shared core/transfer/CLI tests and Clippy on Windows/Linux in [run42561067882](https://github.com/avacadorun-dev/writer-craft/actions/runs/42561067882). The independently reviewed Reload ordering and exact raw→poll→paint regressions in851ea42, and compiled probe sequencing in d2b4ed3, require the next exact integrated run; the earlier success is not silently reused for that delta.

## Required evidence still pending

- Exact integrated application tests, Clippy and complete OpenSpec checks.
- Compiled native first-character creation, in-card selection/paste/local Undo, OS IME, clipboard and focus transitions.
- Zoom/clipping/scrolling, compact/wide surfaces, Light/Dark and localization review.
- Delayed/failed/stale saves, cancellation, exact preservation, restart and Studio/export compatibility.
- Independent review of the same compiled candidate before issue closure.

Engineering tests and screenshots cannot substitute for the remaining complete native journey. The authoritative full alpha.3 UX gate remains pending.

## Exact integrated and native checkpoint — de161c43

[Verify42561873059](https://github.com/avacadorun-dev/writer-craft/actions/runs/42561873059) passed at `de161c43f22c224c08259af807c0d8dfbe8b16fb`, including the host input-order regressions. [Preview42561886139](https://github.com/avacadorun-dev/writer-craft/actions/runs/42561886139) completed packaging. Downloaded checksum and ad-hoc signature verification passed; executable SHA256 is `68031b3e18a74f36a0e79dd660aa47428b2e5f0e2802ecd7ce1c84664a870180`. The informational capture step failed three cases, so overall CI success does not certify native acceptance.

Independent compiled review found readable Latin/Cyrillic input and preserved exact Japanese text, but the actual Monospace editor rendered Japanese glyphs as squares. The existing bundled Noto CJK fallback must also serve Monospace. The font regression resolves actual glyphs and checks retained Latin monospace advances; source alone does not establish a corrected native image.

The old 2x clipping capture did not clip a card at all: moving the camera before Add caused creation to center the card at the new position. Its observed `card_is_clipped=false` was correct; the caret was visible. The revised fixture moves the existing mounted card across the viewport and waits for another observed frame before input. It retains actual geometry/platform output and distinguishes fixture failure from caret failure. Its fresh compiled outcome remains pending.

A local macOS OS-interaction attempt on an isolated workspace then exposed an AccessKit crash: `Focused ID #9005490981944993451 is not in the node list`; the exact candidate process exited101. The sequence involved board/card creation, OS paste and card activation; the triggering frame is still being isolated. The automation relaunched the application afterward, invalidating continuity of later observations. This is a blocking #15 focus/tree lifecycle finding, not a passed clipboard or first-input check. Do not disable accessibility or replace its output with synthetic success.

These findings are recorded in [issue comment6732488888](https://github.com/avacadorun-dev/writer-craft/issues/15#issuecomment-6732488888). Inline-save and Cancel captures establish only their narrow injected-event contracts. OS IME, local history/selection, clipping, failure/recovery, restart and the complete exact-candidate journey remain unaccepted. Other failed rich-editor captures (inline-formula target and large-serif image-bottom target) remain assigned to their existing queued scopes, without weakening their checks or opening another implementation stream.


## Reviewed focus correction under hosted qualification

The generic correction arms activation after paint and grants focus through the next supported pre-input hook. It preserves the captured prior-focus boundary and actual IME output. New regressions feed real `accesskit_consumer` trees through activation, first Unicode/Paste/IME, insertion acknowledgement, dismissal and an omitted mount. Independent source review also required deferred outside clicks to remain authoritative.

Hosted run42565013038 caught a test-only public trait re-export mismatch, fixed in527deac. The next run42565328945 compiled and passed29 of30 inline tests but exposed a press-versus-completed-click mismatch. The fixture had only sent a press, and the adapter also missed a real split-frame release that loses focus under egui's default click policy. The bounded b59aefa correction recognizes actual primary press or completed primary click while retaining `lost_focus`, native-window, interaction and IME guards. Tests exercise same-batch and split press/release plus the alternate press policy. No failing assertion was dropped and no native acceptance follows from this source correction.

The app's reviewed host integration additionally reveals an active existing or provisional card before painting only when it would be fully culled by the actual resized viewport. The world-rect intersection matches CanvasView; partially visible cards retain their camera. Zoom, selection and canonical content are unchanged. Host regressions exercise actual input mounting and emitted focused accessibility nodes. The app remains on its last qualified dependency until the corrected engine's exact hosted run passes; a new package and real macOS retest remain required.


The final generic revision `b59aefafe64a7e8bbc3d39cda44983223e0fc534` passed the locked model/native tests, formatting and strict Clippy on macOS, Linux and Windows in [run42565855041](https://github.com/avacadorun-dev/craft-canvas/actions/runs/42565855041). The application now adopts this qualified revision together with the reviewed host reveal guard, existing CJK fallback and corrected clipping fixture. The final integrated app tests, package, capture diagnostics and repeated real macOS crash scenario are still pending; this dependency qualification does not close #15.
