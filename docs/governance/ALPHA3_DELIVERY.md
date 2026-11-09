# Alpha.3 delivery queue

Updated 2026-11-09. This active plan supersedes the overnight pause in `docs/verification/PAUSED_2026_11_09.md`. It changes execution order, not the accepted alpha.3 scope.

## One integrated issue at a time

**Delivered: [#16 — lion identity](https://github.com/avacadorun-dev/writer-craft/issues/16).** Source `c4e33025ec9f14b6736b46e20eb3523a033c430f` passed hosted checks, actual compiled Light/Dark verification, package/runtime icon checks and independent named branding acceptance. [Final evidence](../verification/LION_BRAND_ALPHA3.md). Published releases remain unchanged.

**Active next: [#15 — direct Canvas card writing](https://github.com/avacadorun-dev/writer-craft/issues/15).** Root integrates and accepts; one implementation agent owns the bounded pending-creation/input lifecycle, with a separate independent reviewer. Finish creation, immediate typing, acknowledged persistence, retry/cancel, selection/Undo, clipping/zoom, reopen and transfer as one user journey. Review the existing generic-engine draft before publication or adoption. No new table or portability dependency pin belongs in this slice.

After #15 acceptance, reassess the bounded #12 block-flow and #6 layout issues against their complete criteria. All other issues remain queued. If the active issue is blocked, record the exact blocker and explicitly select the next issue rather than accumulating hidden streams.

## Delegation contract

Every assignment states the issue, observable deliverable, owned paths, permitted mutations, verification evidence and stop condition. At most one feature implementer works alongside root and a reviewer; investigation may run independently without source mutation. Root alone dispatches the integrated candidate and pushes the application branch. Engine changes must pass their own relevant checks before the application adopts the exact revision. Do not cancel a valid packaging run just to start another unrelated change.

An agent returns a commit or explicit uncommitted diff, check results, limitations and a handoff. Root reviews that result before integration. The reviewer checks the actual compiled candidate and specified behavior, not just an implementation report. A blocking finding is required rework within the same issue.

## Completion contract

1. Match the issue's complete acceptance checklist to stable specifications/scenarios.
2. Integrate the complete user journey, including error, retry, cancel and reopen behavior where applicable.
3. Run relevant checks on the exact committed source; retain hosted source, lock, test and package identity. Rust builds/tests run on GitHub, not this Mac.
4. Collect actual native evidence and independent UX acceptance for the named scope. Protect the running development app and private projects.
5. Update the issue with verified checks and remaining gaps. Close it only when its complete criteria are satisfied under the existing authorization. A library test, showcase, screenshot, or partial umbrella fix is insufficient.
6. Update this queue before starting the next issue. Never use commit count as a completion percentage.

## Audit snapshot

Before the branding work, no open product issue had complete closure evidence at this audit. Issue #16 now has the complete named evidence above; other rows retain their outstanding criteria. Existing passing tests and implementations remain useful, but are not silently promoted to end-to-end acceptance.

| Issue | Remaining acceptance boundary |
|---|---|
| #2 | Persistent text marks: exact repeated/Unicode anchors, edit/delete/Undo/detachment, reopen/recovery/transfer, independent native review. |
| #4 | Umbrella: complete table/input/clipboard, dictionary/source playback, real assistant/MCP/preferences, IME/accessibility/performance. |
| #6 | Wide/minimum layouts; panels/Focus/resize retain selection and scroll; long labels and table controls remain usable. |
| #7 | Empty-document authoring for every block; whole-table consumer integration, clipboard/container/history/reopen and narrow toolbar. |
| #8 | Real highlighting for the full declared grammar set. A check accepting MissingRequiredGrammar does not establish parity; native edit/copy/Undo/reopen/theme evidence remains. |
| #9 | Dialog focus/keyboard/error/disabled/minimum-size/localization matrix, including new transfer dialogs; independent review. |
| #10 | Full hierarchy/domain authoring and import; audio/video beyond images/PDF; private project edit/history/recovery/round-trip qualification. |
| #11 | Selected format-2 privacy/reference closure, conservative recovery, paused restore confirmation, independent reader/native/private/large-workload evidence. |
| #12 | Media/diagram pending-ready-error-retry flow under typography/viewport changes; matching paint/hit bounds, source/assets/anchors and Undo. |
| #14 | Researched project profiles and board/document/entity relationship authority; selected workflow and object-sizing implementation with Studio compatibility. |
| #15 | Immediate new-card input through pending/failed save, exact draft preservation and retry; native IME/clipboard/Undo/zoom/clipping/cancel/reopen/transfer. |
| #16 | Complete for the named branding scope; final evidence is linked above. |

## Parked work

- CraftEditor `7f2885c9fc24bd2ad5f9851a5144e39e327d5fa6`: table-engine work; application adoption and consumer acceptance remain separate.
- CraftCanvas `df6405f`: local pending-insertion draft; unpublished and unqualified, not active application behavior.
- Portability handoff: `docs/verification/PORTABILITY_ALPHA3_HANDOFF.md`; do not confuse accepted compilation with completed recovery/restore UX.
- Existing application source before branding: `a7deffdc319ae426d31d633390049490e41114e0`; its prior preview is not a lion-brand build.

Historical release evidence stays unchanged. Fresh alpha.3 evidence must name its actual source and scope; the full release gate stays pending until all required journeys pass.

## Coverage audit correction — 2026-11-09

The complete original Canvas obligation now has dedicated issue #17 and stable B01–B26 inventory. #15 remains active; #17 stays queued and cannot close from direct-text or import completion. The current inventory/capability/task ledger remains pending until exact native evidence and independent omission review exist. The pending-input engine candidate is now `73e4f9b7e3c3005d01db28fdac3fbf8d64162082` atop `df6405f`; it is not yet hosted-qualified or adopted. No application input behavior changed in this gate work.

The independent engine source review found first-frame IME interruption and Escape-focus risks in73e4f9b. That candidate is blocked from publication/adoption pending a supported focus-handoff fix and platform-output regressions; the application remains on58ce123. This is required rework within active #15, not a reason to open another feature stream.


## Active #15 input correction — hosted engine qualified

The supported input-handoff fix `4be05e9430639d1c666e416ad96d5f668686d48b` passed all-platform generic tests/fmt/strict Clippy in [run42560855344](https://github.com/avacadorun-dev/craft-canvas/actions/runs/42560855344). The app now adopts that exact revision with immediate provisional creation, retained input/acknowledgement, exact retry, frozen Cancel and lossless manual recovery for unpreparable drafts. Host source review additionally found a Done/poll ordering race; finishing now occurs after current-frame input. New host regressions and native qualification remain under review; #15 is open, and all other feature streams remain parked. This updates the older unpublished-candidate notes above without turning source progress into product acceptance.
