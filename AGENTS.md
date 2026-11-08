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

## Owner findings and GitHub issues

Read `docs/governance/ISSUE_WORKFLOW.md` for every owner-reported finding. The owner has authorized creating or updating routine public issues for future findings without repeating a permission question. Search existing open and closed issues before posting; update the existing issue when it is the same problem. Write titles, reproduction history, expected/actual behavior and acceptance checks in English, with the appropriate `bug` or `enhancement` label and source/build/verification links.

Prefer authenticated GitHub CLI (`gh`) or connector/API calls for routine issue work, with structured JSON or an exact `--body-file` for multiline bodies. Use browser interaction only as a last resort; do not repeatedly drive UI for routine issues. Use the owner's existing authenticated GitHub account (`avacadorun-dev`); verify the active identity before submitting. Do not use a bot account, switch identities, invent authorship or add bot attribution to owner-authored issues. If the connected account cannot perform the action, preserve a concrete draft and state the authentication blocker instead of silently posting as someone else. Publish synthetic reproduction text and sanitized evidence; never expose manuscripts, credentials or private machine details. Issue recording does not itself authorize a release, a new feature or a version assignment.

## Issue completion audit and owner acceptance

Audit linked issues and the open issue list after every implementation task and before candidate handoff. Record exact committed source, tested build, verified acceptance checks and remaining gaps in English. Do not infer completion from source review, a compiler, a showcase or release publication.

The owner's latest instruction on 2026-11-08 supersedes automatic product-issue closure during alpha.2 preparation: keep product issues open until the owner checks the final candidate and approves closure. Mark fully verified work ready for owner review with evidence; do not use automatic closing keywords in PRs or commits. Issue #1 remains closed because the owner already confirmed its Dock icon before this instruction. Once final acceptance is explicitly granted, the existing owner-account closure workflow applies without another redundant confirmation.

## Approved alpha.2 scope — all open issues

The owner explicitly selected all open issues for alpha.2 on 2026-11-08, superseding earlier editor-only deferrals. Read `docs/governance/NEXT_RELEASE.md`, the current issue bodies, `openspec/changes/full-editor-parity/`, `docs/migration/EDITOR_PARITY_MATRIX.md` and `docs/design/EDITOR_UX_PARITY.md`.

Complete editor parity remains mandatory: every original node, mark, authoring/input/history/integration behavior, persistent passage notes, eight UI languages, themes/palettes, native UX and shared headless contracts. The only approved capacity exception is 63 logical table columns: preserve wider canonical tables read-only and refuse widening before mutation, without lowering the generic parser.

Alpha.2 also requires issue #3 (native macOS DMG, per-user Windows installer and Linux Debian package), #10 (lossless original project import, Canvas/media workflows, histories and supported round trip), and #11 (transparent local data locations and portable Markdown plus real artifacts with a canonical preservation manifest). None of these is deferred beyond alpha.2. Project/tree/version work needed to satisfy those contracts is included. Do not add unrelated product features.

Large reusable Canvas/media/editor mechanics belong in separate repositories; business import/export, resource ownership and persistence remain here. Use only synthetic public CI fixtures; the owner's original archive, manuscripts and media stay private and read-only. All required implementation, hosted checks, compiled native interaction and independent UX reviews must be complete before final owner handoff. Do not publish alpha.2 or close its product issues before that acceptance. Preserve the published alpha.1 tag and assets unchanged.

Routine administrative edits (license changes, repository About/topics, release housekeeping and tool setup) do not require separate issues unless the owner explicitly requests one. Track actual owner-reported bugs and product enhancements; do not turn every instruction into an issue.
