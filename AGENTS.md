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

## Issue completion audit

At the end of each implementation task and before preparing a release, inspect the linked GitHub issues and the repository's open issue list through authenticated `gh`/API. Match their acceptance checks against the actual committed implementation and verification evidence. Update the existing issue with the fix commit, tested build/version and concrete result; then close fully verified work as `completed` using the owner's authenticated account. This audit and closure are authorized without another routine confirmation. Do not close planned, partly implemented or unverified work, or infer completion from a specification, compiler success or release publication alone. State the specific remaining acceptance check for issues left open. Follow `docs/governance/ISSUE_WORKFLOW.md`; reopen the same issue when a verified regression invalidates its resolution.

## Next-release planning boundary

After the first alpha, collect the owner's findings and agree the next release scope together; see `docs/governance/NEXT_RELEASE.md`. Do not start an unscheduled feature or assume the next version. Persistent text marks with attached author notes are deferred legacy-parity work, captured in `openspec/changes/persistent-text-annotations/` and `docs/migration/PERSISTENT_ANNOTATIONS.md`; they are not part of `0.1.0-alpha.1`. Preparing specifications or an issue is planning, not implementation approval or evidence of completion.

Routine administrative edits (license changes, repository About/topics, release housekeeping and tool setup) do not require separate issues unless the owner explicitly requests one. Track actual owner-reported bugs and product enhancements; do not turn every instruction into an issue.
