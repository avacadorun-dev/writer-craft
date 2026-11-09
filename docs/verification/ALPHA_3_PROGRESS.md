# Alpha.3 implementation and qualification

Active development, not a release or complete native acceptance. First fully green source: `e5018379498e0dcf1fd99ec07a6298e80dbbbe21`. Work is in [draft PR #13](https://github.com/avacadorun-dev/writer-craft/pull/13). Published alpha.1/alpha.2 tags and assets remain immutable.

## Issue disposition

Alpha.2 was tested on macOS and confirmed functional. Installer issue #3 is closed after complete package checks and acceptance; its milestone is closed. Issues #2, #4, #6–#12 retain their unfinished alpha.3 criteria. New #14 covers researched formatting, meaningful project types and stable user scenarios; its candidates are not implicitly selected features.

## Exact-source evidence

[Verify run42501608167](https://github.com/avacadorun-dev/writer-craft/actions/runs/42501608167) passed every required job: macOS313 default-workspace tests,428 all-feature-workspace tests and12 isolated-model tests; formatting; strict all-target/all-feature Clippy; strict OpenSpec and source/tool checks; shared core/transfer/CLI tests and strict Clippy on Windows and Linux. Default/all-feature counts overlap and are not749 unique tests. No local Cargo build/fetch/test was used.

Qualified pins are CraftEditor `26e71d1c2e0b611e0c46fc21980eccef4e01ab92` and CraftCanvas `ca8b74aed8df0ab5db38308a3cf0ef235f3443a1`. Generic/controller checks do not replace actual consumer/native acceptance.

| Workstream | Actual implementation/controller result | Remaining work |
|---|---|---|
| Tables | Native cell/node/bookmarks, selected structural ranges, header inheritance, explicit deletion, widths/alignment, column resizing and exact cell selection; frozen original-output comparisons. | Table-local wide overflow, complete input/clipboard and native interaction/history acceptance. |
| Structured review | Exact retained Unicode attachments; scoped detached proposals; atomic replayed Apply/Reject/compensating Undo; note metadata preservation; native structured before/after preview; core and desktop-controller navigation/stale/rollback checks. | Actual native review journeys, real provider/context integration and complete command parity. |
| Canvas/import | Metadata-confirmed pinned `.wstudio` import, cancellable atomic creation; native board navigation/edit/move/resize/history/ack/recovery; five focused controller regressions pass. | Full Canvas/type/media workflows, native/private-reference acceptance. |
| Portability | Bounded framing, format1 compatibility restore, content-addressed paged format2 export, verified no-clobber archive and immutable stream preflight. | Semantic format2 activation/roundtrip, selected-scope streaming, native export/restore and full-workload acceptance. |
| Project domains | Existing kinds/custom structure, hierarchy, dictionary/guide/memory and media are preserved. | Preserved JSON/flat navigation do not complete native hierarchy/domain workflows. Research must justify new creation profiles. |

## Requested isolated dev package

[Preview run42501773427](https://github.com/avacadorun-dev/writer-craft/actions/runs/42501773427) is pinned to this source. The requested local dev copy uses a separate name/workspace. Packaging or exploration does not grant full alpha.3 acceptance. Do not drive or replace the running dev window after handoff while the user explores it.

## Continuing acceptance

The independent `full-editor-parity` gate remains pending. Generic media mechanics are separately developed in [CraftMediaPreview](https://github.com/avacadorun-dev/craft-media-preview), with no unqualified consumer pin. PDF/audio/video/source playback, editable hierarchy, dictionary/guide/memory, remaining native controls/syntax/flow/input/IME/accessibility/performance, full compatibility and portable restore retain their requirements.

Read the [independent audit](../design/UX_ALPHA3_AUDIT.md), [release plan](../governance/NEXT_RELEASE.md), [type research/scenarios](../research/PROJECT_TYPES_AND_FORMATTING.md) and source matrices. Partial issues remain open.

## Connected-writing and semantic restore checkpoint — 2026-11-09

Source49bf627 / [Verify42506140068](https://github.com/avacadorun-dev/writer-craft/actions/runs/42506140068) passed shared core/transfer/CLI tests and strict Clippy on Windows and Linux. macOS passed the default workspace suite and ran the native integration suite:198 desktop cases passed, including the direct-card slice and imported hierarchy projection, while two native-media cases failed because their synthetic source project had no required document node. This is a failed overall candidate, not a fully green checkpoint. The fixture is corrected in179f9da without weakening Studio validation; later source needs a new full run.

Semantic format2 restoration and canonical ZIP reading are now implemented and passed the above shared-core cycle, including repeated restoration into independent namespaces, flat preservation evidence, note capacity, original artifacts, source-tree baselines and malformed/cancelled rollback. The reader pins one opened archive and verifies bounded inventory/object integrity. Native export/restore UI, streamed reading folders/Markdown, selected scope and full-workload/private-project acceptance remain unfinished.

Direct card editing is tracked in [#15](https://github.com/avacadorun-dev/writer-craft/issues/15). The native host now mounts the qualified Canvas58ce123 input widget in the card, focuses after durable creation, checkpoints/acknowledges saves without replacing later typing and routes explicit Save to the active board. Local text Undo and composition/navigation guards have focused regressions. Additional tests and independent save-status/close/new-card-position corrections after49bf627 remain pending the next hosted cycle and actual native review. No issue was closed from source/widget-event checks.

The native project navigator now projects imported sections/parent/order and exact document identities, with bounded rows, current titles, ancestry-preserving search and trashed-node-safe fallback. Source-compatible hierarchy authoring, empty imported node initialization and custom-schema controls remain required; this is not complete project workflow parity. Research #14/F43 records the existing Studio link/promotion baseline and proposed PT-LINK-01, PT-CHAR-01, PT-LORE-01 and PT-LINK-02 with canonical/link/snapshot/independent-note hypotheses. No new synchronization model or professional project taxonomy has been selected.

A separate independent designer is reviewing this slice and extending explicit synthetic hosted capture coverage. Existing rich-editor startup captures do not qualify Canvas input or imported nesting. Its `full-editor-parity` disposition stays pending. The owner's running e501837 development window remains untouched; new work is not silently substituted into that session.


## Root takeover progress — 2026-11-09 (owner-directed, lean verification)

The owner directed root (Kimi Code) to continue alpha.3 delivery and then approved a lean-verification mode: existing hosted checks plus targeted real/native checks per issue, honest gap records, and separate owner-filed issues for usage findings instead of exhaustive new test matrices.

Closed under lean verification on the exact candidate line (`bcba6e8` runtime; package from Preview run 42570107425; executable SHA256 `6f9008f3…45d67`), each with exact evidence in its issue:

- **#15** direct Canvas card writing: qualified engine `69f2382…`, hosted capture matrix (4 Canvas scenarios + eight locales), real-OS journey/failure-under-lock/`kill -9` crash/recovery-draft cycle, independent compiled review PASS. Caveat recorded: live OS-IME composition (engine-level IME covered by hosted tests).
- **#12** media/diagram block-flow: shipped engine fix (`fd96a30`), existing separation tests + showcase captures + compiled showcase inspection clean. Transition-matrix/caption asserts usage-monitored.
- **#6** workspace layout: Balanced hierarchy + adaptive ≥984 pt surface verified at 1000×680 (Focus toggle hides/restores panes) and 1440 wide; eight-locale captures settled. Pixel-exact surface/scroll regressions usage-monitored.
- **#2** marks/notes: real workflow end-to-end — typed text, bold mark with exact ranges in a durable revision, note creation with exact-occurrence anchor (prefix disambiguation), quit/relaunch with mark and note restored. In-text mark pixels confirmed rendered on the compiled app after Screen Recording was granted.
- **#8** syntax highlighting: engine suite 14/14 on the app-pinned source (every original named format resolves to a real grammar; `syntax.rs` identical between pin and test), palette-aware cached app integration with explicit unavailable-fallback. Token-color pixels in-app usage-monitored.
- **#9** dialogs/input sizing: settings (7 palettes, Esc-clean), new-document (disabled empty submit → create), note form share metrics; eight-locale captures settled.
- **#7** block authoring: every block command reachable in the grouped panel at 1000×680; create→configure→render→Undo→save→reopen covered by the green desktop/engine suites. Per-block automated click-through limited by automation input delivery; usage-monitored.

Also this window:

- **#11** paused-resume defect fixed in source (`fd15f3f`): a prepared transfer now arms and waits for an explicit confirmation; reopening the transfer window never starts an export/restore by itself. New regression test `reopening_armed_transfer_waits_for_explicit_confirmation`; hosted Verify green at `5b2560b`. Compiled export/restore pass with native dialogs remains before closure.
- **#14** decision record (`5b2560b`): seven legacy kinds formalized as an import-compatibility inventory (native creation kindless, no export reclassification); baseline/requested/unselected classification; selected formatting scope = image exact-size with aspect lock + table column resize on existing `width`/`height`/`colwidth` attributes, no schema extension. Implementation parked at P10 per the map.
- **#17** first implementation slice (card connections via the qualified engine's `InsertEdge`) in progress under the one-implementer rule.

Remaining before the alpha.3 gate: #17 slices, #11 compiled pass, #10 synthetic-profile import round-trip (+ owner-side private-archive checks), #14 selected sizing implementation, and the two-axis `full-editor-parity` UX gate with a fresh independent review on the final candidate.


## Root takeover reconciliation — 2026-11-09 (continued checkout)

Rechecked branch `feature/alpha3-parity` at `4bc85c28578ef97f7122ec39f5303ebb57d77ad4`, current GitHub issues/comments and hosted runs. Closed product issues remain #15, #12, #6, #2, #8, #9 and #7. The active sequence remains #17 → #11 → #10 → #14 → #4. Earlier active-#15 paragraphs in ALPHA3_DELIVERY are historical. No preceding worker is still implementing in this checkout.

Retained work: the uncommitted synthetic demonstration-profile tests in `crates/studio-core/tests/project_portability.rs` belong to parked #10 and are excluded from the #17 commit. Existing `canvas.rs` gesture corrections and `capture_connected_writing.rs` diagnostics are reconciled into the active #17 correction, not discarded. CraftEditor is clean at `7f2885c`; the previously reported Cargo.lock diff is absent. CraftCanvas is clean at qualified app pin `69f2382`; no engine repin is needed.

The 4bc85c2 Verify run42594554121 and preview run42595388932 succeeded, but downloaded startup artifact13662739907 explicitly reports `all_captures_settled:false`: canvas-inline, canvas-cancel and canvas-clipped-2x time out after toolbar arming unexpectedly creates a card. Packaging success is not Canvas acceptance. Existing local e21f8bb/c9cb895 packages and renderer/table handoffs are historical; none verifies these newer changes. Temporary drafts are retained.

Independent source reconciliation (`canvas17_review`) found global host pointer routing, wrong first/bottom-card hit order, stale press/drag handling and a double-click regression in the retained attempted fix. The active correction scopes clicks to the clipped board/layer, captures the tool before toolbar processing, clears abandoned presses, preserves Select double-click and excludes pan gestures. Targeted compiled checks and native acceptance are still pending. The prior issue comment's claim that locked endpoints are refused was inaccurate: the original source allows connecting locked cards while forbidding dragging them. Native behavior retains that baseline.

Next concrete delivery: qualify this repaired input boundary on hosted Rust checks and the exact native package, then finish B15 connection labels/selection/deletion/preview. #17 remains open: source audit finds substantial B01–B26 authoring gaps, including media/card creation, GFM, structured clipboard, drawing/shapes, groups/frames, schemas/templates/views/layouts, presentation/playthrough, board management and dedicated output. No full inventory or journey acceptance is inferred.


## Requested current preview — 2026-11-09, 68db76e

The user requested the current compiled preview while alpha.3 work was continuing. Exact source `68db76e6c7c17a7d65edcb56fbab96c0f9d92681` passed workspace/native tests (including the four new Canvas gesture regressions), strict Clippy, formatting and package verification in [native preview42610068366](https://github.com/avacadorun-dev/writer-craft/actions/runs/42610068366). [Verify42610065307](https://github.com/avacadorun-dev/writer-craft/actions/runs/42610065307) passed every job: macOS source checks, Linux/Windows shared-core checks and all three engineering preview packages. Earlier candidates c62d77e and6143918 failed only the new test harness's texture-drop and strict-expect checks; both were corrected before this package.

Native artifact13670187412 contains the ARM64 ZIP. Original executable SHA-256: `3350fd511cb0d1ec24e1b403d06c12c08d993af8dcfe9166ff95bcc185d8b47a`. The downloaded archive checksum and original/ad-hoc local bundle signatures verified. A uniquely named local wrapper `WriterCraft Preview 68db76e` uses a fresh synthetic workspace; its signature-independent Mach-O payload matches the original. CUA observed a running native workspace with the three demonstration documents. The isolated SQLite quick_check returned ok. The user is exploring this preview: do not drive, replace or restart it. This is an engineering preview, not full Canvas/parity acceptance.

Exact startup artifact13670298133 still reports `all_captures_settled:false`. Independent compiled review (`canvas17_review`) inspected the actual clipped2x PNG and matching sidecar: visible caret, terminal/content pass, revisions1→2→3 and durable workspace reread. PNG SHA-256 `cb380ef2497514a855dc860922d55058869cc9bbfad957c98f5dd253812b0657`; sidecar SHA-256 `3100acd83112575a2dcbf7239450aba6f345770b464dc70e27737f978ab8bf20`. This is scripted compiled-widget evidence, not OS-IME, restart or a full journey.

Inline/cancel scenarios remain at phase1 because all seven capture-selector candidates lie inside the auto-fitted seed card. They generate no placement event; canonical revision stays1, with no app failure or unintended card. Reviewer independently confirmed the observed geometry and empty inset corners. A narrow uncommitted correction to `capture_connected_writing.rs` adds observed corner candidates and one regression; it is not included in the opened preview and still needs hosted qualification.

### Retained next work and ownership

- Active issue stays #17. No issue was closed and the full-editor-parity gate remains pending.
- Prepared #10 changes in `crates/studio-core/tests/project_portability.rs` remain uncommitted and untouched.
- `crates/studio-desktop/src/app/canvas/connections.rs` is an untracked, unmounted implementation draft from `canvas17_input_tests`, reviewed read-only by `canvas17_review`. It is NOT in the running preview. It contains connection menu/selection, labels/delete, label painting and temporary preview. Parent lifecycle integration, localization, hosted checks and native acceptance remain.
- Integrate only after the gesture/capture slice is qualified. Add an optional ConnectionEditor; modal_open uses is_open, navigation/close/transfer guards retain any draft; call connection_tools outside the disabled group so Resume remains reachable; disable ordinary tools while the draft exists. Capture prepared.base before session.apply and call connection_acknowledged(base) after successful apply/snapshot update. Preserve the draft across reload_saved/keep_saved/recover without rebasing; the reviewer identified state replacement as a draft-loss blocker if omitted. Call connection_dialog/paint_connections in the corresponding passes.
- New catalog keys: Resume connection label; No connections yet; Edit connection label…; Delete connection; Connection label; Leave the label empty to remove it.; Connection labels can contain up to 500 characters.; The board changed. Copy your label or cancel and reopen the connection.; Return to board; Untitled card. All eight locales are required.
- Original connection reconnection, side anchors and arrow ends still need reconciliation; the prepared label/delete slice is not full B15 acceptance. All remaining B01–B26 workflows and subsequent #11/#10/#14/#4 stay required.
