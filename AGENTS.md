# WriterCraft working agreements

- Use Rust for application code. No WebView, Electron or browser UI.
- Never create branches prefixed `codex/`; use `feature/`, `fix/`, `chore/`.
- Read `docs/architecture/`, `docs/design/` and relevant migration contracts before changing behavior.
- Follow `.agents/skills/rust-best-practices/SKILL.md` and relevant reference chapters for Rust work.
- Keep domain and persistence independent of desktop UI. GUI and headless tools share core commands.
- Preserve user text, source provenance, revisions, annotations, audio and unknown imported rich data.
- A proposal never changes canonical content until explicit Apply. Guard Apply and Undo against stale revisions.
- Use Unicode scalar positions in exact text anchors; never mix byte and UTF-16 offsets.
- Errors must be actionable. No production unwrap/expect, silent save failures or invented success.
- Never present scripted suggestions as AI output. Do not claim feature parity before migration acceptance passes.
- One desktop window: shared command bar, bounded panes, local scrolling, no nested app frames or browser controls.
- Icons use one coherent vector system, with tooltips and labels; no emoji as UI icons.
- Run cargo fmt, tests and clippy. Verify native app visually after UI changes.
- Commit focused, working increments. Update handoff and verification evidence when work stops.
- Do not modify the original Writers Studio repository or user projects without a separate instruction.

## Specification-driven changes

Read `docs/PRD.md`, `docs/governance/SPEC_WORKFLOW.md` and `openspec/config.yaml`.
Use the OpenSpec proposal → specs → design → tasks → implementation → verification flow for behavior changes.
Record architecture and dependency decisions in ADRs; distinguish target parity from verified MVP.
All visible UI messages belong in complete localization catalogs. Support ru/en/de/fr/es/pt/zh/ja; preserve user content verbatim.
Appearance mode and palette are independent; preserve the six non-green legacy palettes and the native blue default.
Large reusable generic engines belong in separate repositories; business workflows and integration remain here.
Do not replace/restart a running preview while the user is exploring it without a clear request.

## Independent UX acceptance

Read `docs/design/UX_ACCEPTANCE.md` and the append-only `docs/design/USER_FEEDBACK.md` before UI changes.
A separate designer must review named behavior, actual compiled captures and interaction evidence.
Treat blocking findings as required rework. A later user correction updates the feedback log, reviewer guidance and regression scenarios before another acceptance.
The authoritative gate is `docs/design/UX_GATE.json`; previews may disclose pending acceptance, but tagged releases require matching accepted source/evidence.
Match existing Writers Studio features one for one; add no new product features without a direct user request. Improve desktop presentation and fix documented source bugs.
