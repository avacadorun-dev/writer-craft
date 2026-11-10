# Alpha.3 release work map

## Latest milestone boundary — authoring and final manuscript delivery (2026-11-10)

The [alpha.3 milestone](ALPHA3_MILESTONE.md) is authoritative: retain the manually accepted twelve-case author workflow at15bc85a; finish #7, #18, mandatory #20 single-manuscript Markdown/assets and #19 editable DOCX. #21 EPUB is a nonblocking stretch goal, subject to exact Rust-backend and real output qualification. Preserve TXT. WIP remains one product issue at a time.

New Canvas work/integration, full parity and complete project migration/recovery are outside this release gate. Broad #4/#10/#11/#14/#17 remain open without an alpha.3 milestone assignment; the selected delivered #14 slice remains in the release. #11's full canonical project restore is preserved as deferred work, not replaced by manuscript export. `structured-writing-alpha` remains pending until the final exact candidate passes its separate gate; historical full-parity plans below do not override this boundary.

The latest override above defines the bounded milestone. The historical P0–P11 sequence retains eventual product obligations without requiring their completion in this release. No issue closes before its complete criteria are verified.

## Current priority override — production CraftCanvas (F-48)

The explicit 2026-11-09 instruction sets other product implementation aside: first bring the standalone CraftCanvas library to production readiness with complete functional React Flow parity, then integrate it to complete original Studio Canvas B01–B26 in #17. The library plan and source inventory live in `craft-canvas/docs/PRODUCTION_PLAN.md` and `craft-canvas/docs/REACT_FLOW_PARITY.md`. CraftCanvas owns its maintenance; upstream fixes/releases are not a delivery dependency.

This is an authorized foundation-first dependency of P7, not closure or abandonment of #17. Exact engine qualification precedes app adoption; actual WriterCraft acceptance remains required after library qualification. Preserved #10 tests, research and capture drafts stay separate.

## Latest release override — independent non-Canvas alpha.3 (F-49)

The subsequent explicit instruction authorizes another agent to complete a coherent character/screenwriting user scenario and other unblocked work, then ship the current implementation as a bounded alpha.3 without waiting for full Canvas. See [the execution handoff](../verification/ALPHA3_NON_CANVAS_HANDOFF.md). This supersedes the old all-parity alpha.3 release boundary and the prohibition on advancing past #17; it does not waive unfinished issue criteria or close #17/#4.

Two separate workstreams are authorized: CraftCanvas follows its own sequential issues; the WriterCraft agent owns application integration and release with one active app issue at a time. The historical P0–P11 inventory below remains required eventual parity, not the new alpha.3 release checklist. Choose and document a named bounded alpha.3 scope/gate with exact native evidence; do not mark `full-editor-parity` accepted to release a subset. Resume Canvas integration only after the engine handoff. Preserve alpha.1/alpha.2 and the running preview.

## Latest acceptance execution override — manual author verification

On 2026-11-10 the owner stopped repeated automated native click-through, tested the supplied12-case checklist himself on `15bc85a`, and reported every scenario passing. See [the exact acceptance record](../verification/ALPHA3_OWNER_ACCEPTANCE_15BC85A.md). This accepts the selected #14 author workflow for advancement to #7. The later milestone decision above selects #18 and #20/#19 next; broad issue criteria and the final release gate remain open. Use existing hosted checks, focused defect regressions and concise owner-facing cases for each completed candidate. Do not rebuild an exhaustive UI matrix or add drag tracing to qualify an already manually accepted task.

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

### P3 — complete: #2 persistent marks and author notes

Closed 2026-11-09 under lean verification: real native workflow verified end-to-end on the exact CI candidate (typing → bold mark with exact ranges in durable revision → note creation with exact-occurrence anchor incl. prefix disambiguation → quit/relaunch → mark and note restored and readable). Core preservation suites green (65 core tests + mapping/replay/recovery/portability). Gap: in-text mark pixel rendering usage-monitored.

### P4 — complete: #8 real syntax highlighting

Closed 2026-11-09 under lean verification: engine syntax suite 14/14 on the app-pinned source (every original named format has a real grammar and colored sample; syntax.rs identical between pin and test), palette-aware cached app integration with explicit unavailable-fallback, source preservation via canonical JSON in green Verify. Gap: pixel-level token colors in the compiled app usage-monitored.

### P5 — complete: #9 dialogs and input sizing

Closed 2026-11-09 under lean verification: settings dialog (7 palettes, modes, Esc-clean close, no phantom edits), new-document dialog (disabled empty submit → validation → create), note form and transfer dialogs share metrics; eight-locale captures settled. Pixel metrics usage-monitored.

### P6 — reopened: #7 existing-block editing correction

Closed 2026-11-09 under lean verification: all block commands reachable in the grouped panel at 1000×680 with configure surfaces; create→configure→render→Undo→save→reopen cycles covered by the green desktop/engine suites (198 native cases, insertion/paste/table, rich-editor anchors/restart). Gap: per-block automated click-through limited by automation input delivery; usage-monitored.

2026-11-09 correction F-46: #7 reopened after actual68db76e use found that double-clicking existing formula/Mermaid blocks does not open editing, with Edit block hidden in formatting overflow. The historical closure above is retained as evidence, not current complete-editing acceptance. Direct native re-editing and independent correction review are required. #17 remains the active implementation issue; this audit does not authorize abandoning it or silently change the remaining sequence.

### P7 — #17 complete original Canvas board workflows

Inventory B01–B26. Builds on the qualified #15 input engine but requires its own exact native evidence and independent omission review of the inventory. #15 or #10 completion does not accept #17.

Current bounded progress (2026-11-09): input routing and compiled Canvas inline/cancel/clipped captures pass at68ddfd8; connection selection/label/delete integration6832125 is under hosted/native review. Endpoint/anchor/marker behavior and the remaining B01–B26 scope are still pending. See the latest alpha.3 progress record; this is not issue acceptance.

F-47 correction: the current connection UX is explicitly rejected. #17 stays active while native graph-library/reference suitability is reconciled against React Flow interaction and B01–B26. No current connection slice is accepted merely from source/tests/geometry. Preserve useful implementation and data-safety work; update primary drag/reconnect behavior before reacceptance.

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
