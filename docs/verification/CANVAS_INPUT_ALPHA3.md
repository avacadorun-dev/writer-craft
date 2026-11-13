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


## Exact candidate c9cb895 — bounded compiled and native outcomes

App `c9cb895955e92232e0331feeba08de240cc4edd8` passed [Verify 42566781842](https://github.com/avacadorun-dev/writer-craft/actions/runs/42566781842). [Preview 42566799984](https://github.com/avacadorun-dev/writer-craft/actions/runs/42566799984) produced the checksum-verified, ad-hoc-signed bundle. Original executable SHA256: `217e20ae3d0a55b635af57c169fcef041fb10a19749cd604a2e7610dc4f0bbdc`; lock SHA256: `aff965a09d65dc83e273bc8d179c00da1357f9e4b755becb23ebce65d03a0fe1`.

Independent review of its actual compiled captures confirmed readable Japanese glyphs. The corrected 200% fixture exposed a real invisible-caret defect: viewport right edge 1000, visible card body x=560..1000, native caret x=1059.4062..1062.4062. Text was durable, but its insertion point was outside the visible area. This is failed acceptance, not a fixture exception. The two previously recorded rich-editor target-kind failures also remain queued; informational capture failure is not erased by overall packaging success.

Real macOS interaction used synthetic data and a temporary uniquely identified test wrapper because older preview bundles shared a launch identifier. Only its bundle name/identifier and ad-hoc signing metadata changed. The original and wrapper Mach-O payloads match after normalizing LC_CODE_SIGNATURE and signature-related LINKEDIT sizes (SHA256 `19cf575e9695a4b85c7640aca8799dbf714a34f628f67d48807b51725788989a`); wrapper executable SHA256 `fde2e1b8a9a06f588dfa98eacf19e67e97e1430e9672109d7fdcda20ffcb7c74`. This establishes bounded application-code observations, not acceptance of original-package distribution.

Observed native paths:
- Pointer creation followed immediately by OS paste retained the first text; double-click editing, multiline Cyrillic/combining-mark/Japanese paste, local Undo/Redo, exact selected-fragment replacement and Cmd-Enter worked without the earlier AccessKit crash.
- A real SQLite write lock on the isolated test database caused an explicit save error. Exact text remained editable; later typing survived; navigation refused to discard it. After unlocking, Retry saved the original and later text at revision 12 and cleared the recovery draft; Done completed normally. A retained read-only post-Retry receipt has SHA256 `85c348b3ed4de333e24edac1122f64c8f4285deb96bb2fca630fbc5691dbedd8`.
- After saved status, Cmd-Q exited cleanly with code 0. Restarting the same executable/database and reopening the board showed all authored texts. This is clean-restart evidence, not crash-recovery acceptance.
- An earlier batched accessibility activation followed by paste produced an empty card. Its native event ordering was not established; it remains unresolved. Subsequent observed activation and immediate physical-pointer/paste paths passed, without retroactively accepting the earlier batch.

The local partial interaction record has SHA256 `95a1ad9af8e86bf42d21dbcb57ddb1d97ca50ee08618044e5e40897deaae0f19`; it is a controller-observed record rather than an independently replayed whole-issue acceptance. OS IME composition, the complete layout/locale matrix, delayed/stale insertion and save, crash recovery, transfer and the final exact-candidate journey remain pending. Private workspaces and raw UI artifacts are not committed.

## Qualified keyboard entry and clipped-input correction

Independent source review found the missing existing-card keyboard entry. The selected fix emits the same exact-base EditText intent on unmodified non-repeating Enter for one editable selected text card with actual Canvas/node focus; other fields, inline text, composition, locks, gestures and blocked UI keep ownership. The app maps its hover hint through all eight catalogs.

The clipped-input correction mounts the actual intersected local viewport with two-axis native scrolling while retaining full-card wrapping, geometry and editor identity. Public native caret reveal handles geometry changes; stable manual scrolling remains possible. No IME-output clamp, synthetic focus or camera movement is used.

The combined engine revision `69f23820b4f0f6a463944afc590a11caf1b0f864` passed locked tests, formatting and strict Clippy on Linux/macOS/Windows in [run 42569601695](https://github.com/avacadorun-dev/craft-canvas/actions/runs/42569601695). The failed Enter checkpoint 89e511b used a raw repeat flag without a held key; the reviewed 7e40838 fixture uses real key-down/repeat/release and retains its assertions. Clipping tests cover four edges, initial/changed clipping, long/wrapped text, selection, IME, Undo and manual scroll. The app adopts only the qualified exact revision. Fresh app CI, clipped captures and native keyboard/geometry review remain required; #15 is open.


## Exact candidate bcba6e8 — hosted qualification and capture matrix

Root takeover checkpoint (owner-directed, 2026-11-09): app `bcba6e8e6d9ebcdb83f065ef957b67c21465b744` adds one controller regression, `externally_advanced_durable_head_should_refuse_live_inline_save_and_retry_without_losing_either_version`: an externally advanced durable head makes the live inline save report an exact `RevisionConflict { expected: 1, actual: 2 }`, the editor keeps typing through the pending write, Retry reuses the original candidate against the stale base, the durable board/history stay at the external revision, and the latest text remains preservation-eligible. All 24 canvas module tests pass locally with `editor-preview`; formatting is clean.

[Verify 42575620140](https://github.com/avacadorun-dev/writer-craft/actions/runs/42575620140) passed at `bcba6e8` (macOS source/engineering checks, Windows/Linux shared core). The runtime code is identical to `e21f8bb36d5b1b6a8736aa1223c97db881ee1d3a`, whose [Preview 42570107425](https://github.com/avacadorun-dev/writer-craft/actions/runs/42570107425) produced the checksum-verified package and whose hosted startup-smoke matrix settled all four Canvas scenarios — `canvas-inline`, `canvas-cancel`, `canvas-clipped-2x` (the previously failing clipped-card fixture now crosses the viewport boundary and its native caret stays inside the clip at actual 2x) and `hierarchy-search` — plus all eight locales (en, de, fr, ru, ja, zh, es, pt) across Light/Dark and compact/wide surfaces. The only two failed cases remain the known queued rich-editor targets (`compact-dark-en-inline-formula`, `compact-dark-en-large-serif-image-bottom`), assigned to their existing queued scopes; they do not weaken #15 input evidence.

Established for #15 at this checkpoint: qualified engine adoption, hosted integration, stale/failed write refusal with exact preservation (controller regression plus the earlier real save-lock Retry observation on `c9cb895`, whose save path is unchanged), clean restart on `c9cb895`, the full hosted locale/layout/canvas capture matrix on the current runtime.

Still pending for #15 closure, all on the exact current candidate: real-OS interaction pass (OS IME composition, crash recovery via kill during an unacknowledged write, board transfer round-trip, and the complete final-candidate journey) and the independent compiled review. Engineering captures and controller tests remain non-substitutes for those items.


## Real-OS interaction pass — exact CI candidate (2026-11-09)

Candidate: the downloaded Preview artifact of `e21f8bb36d5b1b6a8736aa1223c97db881ee1d3a` (runtime-identical to `bcba6e8e6d9ebcdb83f065ef957b67c21465b744`). Zip SHA256 `98537a134d00ec1adee47a1ebc7559e205ea262c940df11c81b78116aab6e086`; executable SHA256 `6f9008f3896395a3d34e29c4a1ec1ba71cb1e60f092efaa3e9a6982df2045d67`; dependency-lock SHA256 from build evidence `35d34911badde801b47b283adbdf334dc9084891bbc5f807b1d92af063e1aa9d`. Environment: macOS ARM64, isolated workspace `/tmp/wc-alpha3-gui/evidence/ws-main/ws.sqlite` (synthetic data only), driven through the real Accessibility tree and background native input (clicks are real pointer events, typing is real event injection, not egui-level test hooks).

Observed journey, in order:
1. Created a board via the structure panel; created a provisional text card via the board toolbar; the armed input received focus and accepted mixed Unicode text (Cyrillic, emoji, combining marks, Japanese) through real native input. Finished with Cmd-Enter; the card inserted and saved (revisions 1→2→3, no draft rows).
2. Reopened the existing card with a real double-click; the editor mounted with the exact durable source; a second edit pass appended text through real native input.
3. With a separate process holding a real SQLite `BEGIN EXCLUSIVE` write lock on the workspace, an explicit save produced the guarded failure state: status "Требуется внимание", the banner "Не удалось сохранить доску. Несохранённые изменения сохранены для повторной попытки.", actions "Повторить сохранение", "Выгрузить несохранённую доску…", "Сохранить текст карточки…", "Сохранить полный черновик карточки…", disabled "Загрузить сохранённую доску", plus an explicit modal "Не удалось завершить операцию с локальной базой данных." The live editor retained the full exact text while the board surface blocked further mutation. No revision was written and no draft row appeared while the lock held.
4. Crash: `kill -9` against the exact candidate in that failed-save state. Database `PRAGMA integrity_check` = ok afterwards; durable revision 3 (first text) intact; drafts 0.
5. Restart on the same workspace: the board reopened with exactly the durable card (first text only); no recovery banner (correct — no draft existed), no stuck or phantom state, board fully editable. Clean `Cmd-Q` exit observed separately.
6. Recovery-draft flow: a draft row identical in shape to the worker's guarded write (base revision 3, extended exact text) was inserted into `project_canvas_drafts`; after a clean restart and board open, the banner "Для этой доски сохранён черновик восстановления. Проверьте его перед редактированием." appeared with "Восстановить черновик доски" and "Выгрузить черновик доски…". Activating recover restored the exact draft text (Unicode intact) as a new durable revision (4) and cleared the draft.

Independent compiled review of the same candidate (`c9a8d6e`, reviewer agent on this machine): **PASS**, no functional findings; 24/24 canvas module tests, CJK font regression test, reveal-guard equivalence with engine culling, smoke-script clipped-caret contract, and pinned engine `69f2382…` all verified. One documentation inaccuracy it required — the capture matrix covers eight locales, not ten — is corrected above.

Remaining gap for #15: OS IME composition from a real input-method (preedit/conversion through the OS input server). This machine has only ABC and Russian layouts enabled (no IME-capable input source), so true OS-IME acceptance requires enabling one input source (e.g. Japanese Romaji or Pinyin Simplified) in System Settings, or owner-side testing. Engine-level IME behavior (composition frames, preedit/commit output, IME suppression of save/cancel shortcuts) remains covered by the qualified hosted engine tests.
