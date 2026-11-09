# Alpha.3 release work map

Owner-approved 2026-11-09. This is the single authoritative sequence for closing milestone `0.1.0-alpha.3` (issues #2, #4, #6, #7, #8, #9, #10, #11, #12, #14, #15, #17). It refines the queue in `ALPHA3_DELIVERY.md` with an explicit, non-contradictory order so no issue starves and no issue is closed before its complete criteria.

## Standing rules

- **WIP = 1.** One active issue at a time: specification → integrated behavior → relevant checks on the exact committed source → actual native evidence → independent acceptance → issue update/closure. Only then does the next issue start.
- Investigation without source mutation (research, audits, fixture design) may run ahead of the active issue; product code may not.
- Engine changes must pass their own hosted checks before the application adopts the exact revision. The app never adopts an unqualified engine chain.
- #4 and #17 close only with their complete gates; nothing else in this map is optional or deferrable without an explicit owner decision recorded in the issue.
- Every closure comment names the exact source SHA, hosted runs, native evidence and independent reviewer. Never use commit count as completion.

## Sequence

### P0 — active: #15 direct Canvas card writing

Progress in the required order; #15 remains open:
1. **Complete — engine correction.** Fixed the `done_requested` regression in craft-canvas `b59aefafe64a7e8bbc3d39cda44983223e0fc534`: finish pointer blur on the actual native focus-policy event, including a completed click delivered across frames. Retained the failed `6365c8b`→`527deac` checkpoint and root-cause history in the [issue comment](https://github.com/avacadorun-dev/writer-craft/issues/15#issuecomment-6732763614).
2. **Complete — engine qualification and adoption.** That exact revision passed locked Linux/macOS/Windows tests, formatting and strict Clippy in [run 42565855041](https://github.com/avacadorun-dev/craft-canvas/actions/runs/42565855041). App commit `c95ee940b7c53988fdf25b3b2109170f15a0956b` adopts it with the host mountability, CJK font and clipped-card fixture corrections. This does not establish native product acceptance.
3. **Pending — final integrated candidate.** The c9cb895 app checks passed, but its real clipped-card capture failed. The follow-up keyboard-entry and clipped-input engine `69f23820b4f0f6a463944afc590a11caf1b0f864` is now qualified by [run 42569601695](https://github.com/avacadorun-dev/craft-canvas/actions/runs/42569601695). Verify + Preview and fresh native outcomes are required after its app adoption. Current run and candidate evidence belong in [the input verification record](../verification/CANVAS_INPUT_ALPHA3.md).
4. **Pending — complete native acceptance.** c9cb895 has bounded real macOS paste/selection/local Undo, failed-save Retry and clean-restart observations. OS IME, full layout/localization, delayed/stale writes, crash recovery, transfer and the complete final-candidate journey remain required; prior observations do not qualify later changed input behavior.
5. **Pending — independent compiled review.** Then issue update/closure.

### P1 — #12 media/diagram block-flow

Pending-ready-error-retry flow under typography/viewport changes; matching paint/hit bounds, source/assets/anchors and Undo. Reassess against complete criteria after #15 acceptance.

### P2 — #6 editor workspace layout

Wide/minimum layouts; panels/Focus/resize retain selection and scroll; long labels and table controls usable. Shares review context with #9.

### P3 — #2 persistent marks and author notes

Exact repeated/Unicode anchors; edit/delete/Undo/detachment; reopen/recovery/transfer; independent native review.

### P4 — #8 real syntax highlighting

Full declared grammar set. A check accepting `MissingRequiredGrammar` does not establish parity; native edit/copy/Undo/reopen/theme evidence required.

### P5 — #9 dialogs and input sizing

Focus/keyboard/error/disabled/minimum-size/localization matrix, including transfer dialogs; independent review.

### P6 — #7 complete block authoring

Empty-document authoring for every block; whole-table consumer integration; clipboard/container/history/reopen; narrow toolbar.

### P7 — #17 complete original Canvas board workflows

Inventory B01–B26. Builds on the qualified #15 input engine but requires its own exact native evidence and independent omission review of the inventory. #15 or #10 completion does not accept #17.

### P8 — #11 portable exports with assets

Adopt `docs/verification/PORTABILITY_ALPHA3_HANDOFF.md`; selected format-2 privacy/reference closure, conservative recovery, paused restore confirmation, independent reader/native/private/large-workload evidence.

### P9 — #10 demonstration project compatibility

Full hierarchy/domain authoring and import; audio/video beyond images/PDF; private project edit/history/recovery/round-trip qualification. Qualifies baseline compatibility after the preceding implementation issues because it exercises nearly all subsystems; selected #14 work follows in P10.

### P10 — #14 researched project profiles and scenarios

Research first per `docs/research/PROJECT_TYPES_AND_FORMATTING.md` (STARC, Final Draft, WriterDuet/WriterSolo, Fade In, Celtx, Scrivener, Obsidian, FocusWriter, Google Docs); record selected scope before implementation; keep transfer versions 1–8 importing; a nominal kind selector is not proof of professional workflows. Code follows the selected scope. Before P11, repeat affected transfer-version 1–8/editor-schema 1–4 import checks and private edit/history/recovery/round-trip journeys against the final candidate. P9 evidence cannot qualify later changed behavior without re-verification.

### P11 — #4 umbrella parity — closes last

Two-axis gate: `docs/migration/PARITY_SCOPE.json` inventory independently reviewed for omissions; six role journeys from `docs/design/ROLE_JOURNEYS.md` accepted; current `full-editor-parity` UX gate; `scripts/release-check.py` readiness true; actual exact-candidate native evidence. Alpha.3 ships only when this gate and every issue above are complete.

## Non-negotiable preserved boundaries

- Single approved table-capacity exception: 63 logical columns with lossless wider-data preservation and refusal before mutation.
- Existing `.wstudio` transfer versions 1–8 and editor schemas 1–4 keep importing and working.
- Six non-green legacy palettes and the native blue default; appearance and palette independent.
- Published alpha.1/alpha.2 tags and assets unchanged.
- Synthetic public CI fixtures only; original manuscripts, archives and media stay private/read-only.
