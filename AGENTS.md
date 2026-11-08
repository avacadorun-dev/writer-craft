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

## Current release decision — alpha.2 checkpoint, alpha.3 parity

The owner's latest explicit decision on 2026-11-08 supersedes the earlier all-open-issues alpha.2 boundary: finalize the completed coherent implementation as `0.1.0-alpha.2`, publish downloadable native installers (including macOS ARM64 DMG), update README with actual compiled screenshots, and carry all unfinished large workstreams into alpha.3. Publication is authorized; no second publication permission is required. Preserve published alpha.1 assets and tag unchanged.

The alpha.2 named independent scope is `native-rich-editor-alpha`, an engineering prerelease of the implemented native writing workspace. It is not full editor/product parity or a daily-use certification. Keep exact-source tests, package verification, independent scoped review, truthful limitations, data preservation and owner acceptance of product issues. Do not close issues merely because this checkpoint ships. Read `docs/governance/NEXT_RELEASE.md` for the current release boundary and alpha.3 backlog.

The complete source-backed editor contract, all 21 nodes/six marks, integration journeys, source-project/Canvas/media compatibility and portable restore remain required migration work; their unfinished parts move to alpha.3, rather than becoming implemented or being removed. The sole approved native table capacity exception remains 63 logical columns with lossless wider-data preservation and refusal before mutation.

The alpha.2 freeze was lifted by the subsequent alpha.3 authorization below. Do not repin unqualified editor/Canvas foundations or restart the owner's preview. Use only synthetic public CI fixtures; original manuscripts, archives and media remain private/read-only. Specialized generic mechanics remain in separate repositories.

Routine administrative edits (license changes, repository About/topics, release housekeeping and tool setup) do not require separate issues unless the owner explicitly requests one. Track actual owner-reported bugs and product enhancements; do not turn every instruction into an issue.


## Current owner authorization — alpha.3 execution

On 2026-11-08, after testing the published alpha.2 on macOS and confirming it functional, the owner explicitly authorized closing issues whose complete criteria were satisfied by that release, then implementing alpha.3 and closing the remaining issues as their complete acceptance is established. This supersedes the alpha.2 freeze and the requirement to request the same owner approval again for verified alpha.2 completions. It does not waive any unmet technical or independent UX criterion, establish untested Windows/Linux GUI behavior, or permit mass closure of partial issues.

Develop alpha.3 against the complete existing source-backed scope in `docs/governance/NEXT_RELEASE.md`. Preserve published alpha.1/alpha.2 tags and assets. Use `feature/alpha3-parity`, focused commits, exact-source hosted checks and independent actual native acceptance. Keep the alpha.2 accepted gate as historical evidence; alpha.3 requires its own current `full-editor-parity` gate. Continue issue audits after each completed slice and record the precise evidence when closing a finished issue. Do not create new product features to satisfy the plan.


## Issue prose and researched formatting

Write issues as direct product problems or feature requirements. Do not introduce them with `Owner report`, `The owner wants`, attribution to the requester, or similar filler. Use `Summary`, `Current behavior`, `Expected behavior`, research evidence and concrete acceptance checks. Preserve meaningful historical acceptance/build evidence without adding repetitive requester attribution.

The requested formatting discovery and object-sizing extension is tracked in issue #14. Research actual screenwriter/editor/writer needs, distinguish first-hand requests from competitor documentation and implementation inference, and record the selected feature scope before implementation. Existing `.wstudio` transfer versions1–8/editor schemas1–4 must continue to import and work. Schema extensions require explicit compatible defaults/migration, unknown-field and original-media preservation, and semantic round-trip checks; formatting additions cannot silently replace the Studio compatibility contract.


## Evidence-based project profiles and user scenarios

Issue #14 also covers project types specifically. Existing seven kinds are a backward-compatibility baseline, not a validated new-project taxonomy. Research STARC, Final Draft, WriterDuet/WriterSolo separately, Fade In, Celtx, Scrivener, Obsidian, FocusWriter and Google Docs as relevant workflow references. Separate project kinds, starting templates, document editing modes and output profiles through explicit hypotheses and evidence. Record stable-ID user scenarios and trace them to specifications, implementation, semantic tests and independent native acceptance. Do not implement a nominal selector as proof that professional project workflows are complete. Read `docs/research/PROJECT_TYPES_AND_FORMATTING.md` before changing creation profiles; preserve import of old kinds and custom schemas throughout.
