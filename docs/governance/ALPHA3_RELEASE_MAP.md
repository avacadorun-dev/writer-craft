# Alpha.3 release work map

Owner-approved 2026-11-09. This is the single authoritative sequence for closing milestone `0.1.0-alpha.3` (issues #2, #4, #6, #7, #8, #9, #10, #11, #12, #14, #15, #17). It refines the queue in `ALPHA3_DELIVERY.md` with an explicit, non-contradictory order so no issue starves and no issue is closed before its complete criteria.

## Standing rules

- **Owner decision 2026-11-09 — lean verification:** stop building exhaustive new test matrices. Verify each issue with the existing hosted checks, targeted real/native checks and quick compiled inspection; record honest evidence and remaining gaps in the issue. Defects found later are handled as separate issues (owner-filed), not as release blockers. Deep engine/app test suites are added only when a real defect needs a regression.
- **WIP = 1.** One active issue at a time: specification → integrated behavior → relevant checks on the exact committed source → actual native evidence → independent acceptance → issue update/closure. Only then does the next issue start.
- Investigation without source mutation (research, audits, fixture design) may run ahead of the active issue; product code may not.
- Engine changes must pass their own hosted checks before the application adopts the exact revision. The app never adopts an unqualified engine chain.
- #4 and #17 close only with their complete gates; nothing else in this map is optional or deferrable without an explicit owner decision recorded in the issue.
- Every closure comment names the exact source SHA, hosted runs, native evidence and independent reviewer. Never use commit count as completion.

## Sequence

### P0 — complete: #15 direct Canvas card writing

Closed 2026-11-09 under the owner-approved lean verification. Exact evidence: [Verify 42575620140](https://github.com/avacadorun-dev/writer-craft/actions/runs/42575620140) at `bcba6e8`; package from [Preview 42570107425](https://github.com/avacadorun-dev/writer-craft/actions/runs/42570107425) (hashes in [the input verification record](../verification/CANVAS_INPUT_ALPHA3.md)); qualified engine `69f2382…`; real-OS journey/failure/crash/recovery pass; independent review PASS. Accepted caveat: live OS-IME composition not exercised on this machine (engine-level IME covered by qualified hosted tests; live findings become a separate owner-filed issue).

### P1 — complete: #12 media/diagram block-flow

Closed 2026-11-09 under lean verification: shipped flow repair in pinned engine `fd96a30`, existing engine separation tests and app Verify green ([run 42575620140](https://github.com/avacadorun-dev/writer-craft/actions/runs/42575620140)), showcase captures settled, real compiled showcase scroll inspection clean. Gaps (transition matrix, caption/paint-hit assertions) are usage-monitored separate-issue material.

### P2 — complete: #6 editor workspace layout

Closed 2026-11-09 under lean verification: Balanced hierarchy and adaptive ≥984 pt surface implemented; real runs at exactly 1000×680 (full toolbar, Focus toggle hides/restores both panes) and 1440 wide; eight-locale capture matrix settled. Pixel-exact surface/scroll-retention regressions are usage-monitored separate-issue material.

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
