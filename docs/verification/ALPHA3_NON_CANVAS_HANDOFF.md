# Alpha.3 non-Canvas execution handoff

Prepared 2026-11-09 under explicit F-49 authorization. This is an execution handoff, not a completed release. Continue the current checkout and reconcile newer commits/agents before editing. Repository: `avacadorun-dev/writer-craft`; branch: `feature/alpha3-parity`. Never create `codex/` branches.

## Current authority and division of work

The user explicitly authorized delivering alpha.3 from the existing implementation plus a coherent completed character/screenwriting scenario and other unblocked fixes, without waiting for full Canvas. This supersedes the historical all-parity alpha.3 release prerequisite and the old prohibition on advancing beyond #17. Read the latest overrides in AGENTS.md, NEXT_RELEASE.md and ALPHA3_RELEASE_MAP.md; older paragraphs remain historical evidence.

You own WriterCraft application implementation, integration, exact hosted builds, independent native acceptance and the bounded alpha.3 release. Keep one active application issue. The separate Canvas agent owns `craft-canvas`: its strict sequential roadmap aims first at all generic primitives needed by WriterCraft, then full native React Flow parity. Do not edit that engine or adopt its work in progress. Resume Canvas integration only after its explicit exact-qualified handoff.

Full Canvas #17, full parity #4 and all unmet issue criteria remain open. A bounded alpha.3 release must not masquerade as `full-editor-parity`. Define and implement a named scoped engineering-release gate that verifies the selected outcome while retaining the broader full-parity inventory and pending status. Do not weaken data-preservation or exact-source acceptance to make the release tool green.

## Reconcile before choosing the first issue

Read:

- AGENTS.md, docs/governance/ALPHA3_RELEASE_MAP.md, NEXT_RELEASE.md, ISSUE_WORKFLOW.md and the latest ALPHA_3_PROGRESS.md entries.
- docs/research/PROJECT_TYPES_AND_FORMATTING.md, docs/design/ROLE_JOURNEYS.md, USER_FEEDBACK.md F-40–F-49, UX_ACCEPTANCE.md and the current gate.
- Relevant OpenSpec proposal/spec/design/tasks, migration/portability contracts and current GitHub issue bodies/recent comments.
- Actual Git status/recent commits in the app and relevant editor/media engines; active agents and hosted runs. Do not infer completion from old checklists.

Application behavior baseline: `fb873727027c7e24e91d22a9448dbfe2c791908a`. Later reconciliation/handoff commits may contain documentation only; inspect HEAD. Canvas pin: qualified `6b68eb4e84aa41b713a148e6a456048f3c1168f9`. Editor pin remains `fd96a30…`; editor checkout `7f2885c…` must not be adopted automatically. Existing engine publication permissions are in AGENTS.md.

Open app issues at handoff: #7, #17, #11, #10, #14, #4. Closed #15 accepts its direct-input slice only, not complete Canvas. Recheck current GitHub state before acting.

## Preserve current work

- `crates/studio-core/tests/project_portability.rs`: retained uncommitted synthetic #10 tests, approximately312 added lines. Inspect intent/ownership; do not discard or include in an unrelated commit. This is the only known retained code diff at preparation start.
- Prior generic Canvas and app connection fixes are committed. They protect exact endpoints/labels, metadata, durable acknowledgements and embedded-control ownership; preserve them even though F-47 rejects the primary UX.
- The incomplete29-step connection capture draft is parked under `/private/tmp/wc-17-reactflow-reconciliation/fb87372-capture-draft/` with file copies, patch and hashed manifest. Its Python validator still expects15steps. Do not apply or dispatch blindly.
- Historical evidence/research remains under `/private/tmp/wc-alpha3-gui/`, `/private/tmp/wc-cap/`, `/private/tmp/wc-17-reconcile/`, `/private/tmp/issue17-progress.md` and the existing user temporary review/research directories named in the original handoff. Identify relevance by source SHA/content. No deletion is needed.
- The user is exploring `WriterCraft Preview 68db76e` in a separate synthetic local workspace. Do not drive, replace, restart or terminate it. Use isolated exact-candidate synthetic workspaces for your verification.

## Last completed builds and actual limits

`fb87372` [native preview42618620150](https://github.com/avacadorun-dev/writer-craft/actions/runs/42618620150) succeeded: formatting, all-feature workspace tests, strict checks, package and signature/identity verification. [Verify42618621132](https://github.com/avacadorun-dev/writer-craft/actions/runs/42618621132) passed Rust workspace/native tests, Clippy and Linux/Windows portable checks, but failed strict OpenSpec validation because one endpoint requirement exceeded500characters. The handoff documentation splits that requirement without reducing its scenarios; rerun validation on your exact candidate. It is not honest to call the entire old Verify run green.

Independent reviewer inspected all four Canvas PNGs from the exact preview. Inline/cancel/clipped cases pass their bounded existing contracts. Connection label/save/delete/Undo/Redo/Undo reaches phase15 with six independently reread durable revisions, preserved nodes/unknown edge data/three unrelated manuscripts and exact visible Unicode label. Connection PNG SHA-256 `805f5b6ce4d76055021932bdc5310a638dd6f60b6b3916552a6237b41b6a3355`; sidecar `85e3a0f9ccc12658b8e862875d440125091012ca6b62284e5db97d7f86ab85e2`.

These are scripted RawInput events through compiled widgets, not OS clipboard/IME, restart, endpoint reconnection or full journeys. F-47 UX rejection remains. Overall startup receipt remains false: inline-formula and large-serif-image-bottom target-kind mismatches and missing actual-wide coverage require honest disposition. Do not weaken assertions or recycle screenshots to accept newer source. Exact artifacts: `/private/tmp/wc-17-reconcile/fb87372-captures/`.

## Concrete application work

Start with a short reconciliation report and select one unblocked issue. The following requested outcomes guide scope; they are not permission to run all issues simultaneously:

1. **Complete one character/screenwriting journey.** Read the selected research/scenarios in #14 first. Use existing character/dictionary/profile and manuscript/document capabilities to create a character, write/edit a scene involving that character, navigate to its information and return to the exact writing context, save/reopen and recover/cancel without data loss. Record stable scenario IDs and actual outcome. If a selected relationship depends on the new board engine, keep that portion deferred and choose the source-backed non-Canvas path; do not fake linked views or implement a nominal project-kind selector as acceptance. No new unresearched taxonomy or implicit manuscript reordering.
2. **Fix discoverable re-editing in #7.** Existing formula/Mermaid blocks have a hidden More formatting → Edit block command. The requested primary gesture is double-click, preserving single-click selection, with context-menu and keyboard equivalents and no permanent toolbar wall. Qualify existing block → source/edit → Apply/render → Cancel → Undo/Redo → save/reopen, plus stale/failed draft preservation. Engine fixes belong in CraftEditor with independent exact qualification before repin. This task has no dependency on the new Canvas engine.
3. **Finish another genuinely independent issue when useful to the named scenario.** Audit #11's prepared portable-folder work and #14's selected image sizing. An image must become smaller with compatible stored dimensions/aspect lock and unchanged original bytes; workspace zoom does not satisfy it. Choose the bounded deliverable from existing source/criteria. Broad #10/#14/#4 cannot close for only a subset, even if that subset ships.

The first scenario should produce a useful completed task, not a gallery of controls. The current source inventory/role stories and researched scope remain authoritative. If actual source reveals a dependency, record it explicitly and select another user-authorized unblocked task; F-49 authorizes that sequencing without waiting for #17.

## Release delivery

Specify the actual alpha.3 scope and limitations in English before implementation acceptance. Complete the selected user journey, relevant checks and concise independent review of exact compiled interactions. Use GitHub for Rust builds/tests/packages; no local Cargo builds. Preserve the native architecture, eight complete localization catalogs, original rich/unknown data, media and history. Keep original Studio and private user projects read-only; public evidence is synthetic.

Prepare native installers including macOS ARM64 DMG, exact source/lock/build identity, checksum/signature/install verification, truthful platform support, release notes and actual compiled README screenshots. Preserve published alpha.1/alpha.2 tags/assets. Routine publication within this requested alpha.3 scope is authorized; no repeat permission question is necessary. Finish all concrete verification before publication. Update issues with precise evidence and close only entire verified criteria.

## Canvas integration handback boundary

Accept a handoff containing exact engine SHA or published versions, unchanged/qualified dependency lock, relevant three-platform hosted results, actual standalone native observations, independent review, API migration notes and explicit limitations. Adopt that exact revision and verify the app adapter before integration claims. Keep current JSON/IDs/unknown fields/media/history compatible and retest affected authoring/persistence/restore. The later full B01–B26 acceptance is still required independently of the bounded alpha.3 release.

Do not wait idle for that handoff. Complete the non-Canvas scenario and release work above. Coordinate through a durable progress record; no competing app or engine edits.
