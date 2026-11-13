# Owner manual acceptance — structured authoring at 15bc85a

Date: 2026-11-10. Tested candidate: `15bc85a16149c7161f07d45f123bd16830693ee4`, macOS ARM64, WriterCraft Acceptance15bc85a. Project: **The Last Light / Последний огонь**,13 folders and18 authored documents. The project was originally created through native UI; its continued workspace was opened for the owner without modifying the writing.

The owner explicitly reported that all twelve supplied scenarios passed. This is owner-reported manual native acceptance, not an assertion that the agent observed every gesture or obtained screenshots for each subcase. The earlier automation drag attempts remain recorded as inconclusive; the owner's subsequent manual pass supplies the actual-user acceptance of drag/drop.

| Case | Manual scenario | Reported result |
| --- | --- | --- |
| 1 | Hierarchy and full folder/document/note CRUD, including3–4 levels, prose and rename preservation | Pass |
| 2 | Exact-row right-click/Shift+F10 and Add inside the intended folder | Pass |
| 3 | Direct document/folder drag, reorder, target/insertion feedback, hover expansion, edge scroll, invalid placement and cancellation | Pass |
| 4 | Independent document/parent Trash and ordered restore, preserving text/history | Pass |
| 5 | Consult and return to the second identical text occurrence, with selection/scroll and exact continuation | Pass |
| 6 | Meaningful notes, tag-only/multiple-label markup, matching manuscript/sidebar colors and clear quote/note presentation | Pass |
| 7 | Project label create/assign/recolor/rename/cancel/remove/restore with disclosed scope | Pass |
| 8 | One-action local assignment/mark removal, retained pending note text and nearby guarded Undo | Pass |
| 9 | Filtered cross-document project markup, exact activation/return and unavailable-document handling | Pass |
| 10 | Previous/Next with retained wording, explicit Apply, Resolve and next, final completion and reopen | Pass |
| 11 | Input focus, typing/Select all/Paste ownership, populated cancellation and retained unapplied wording | Pass |
| 12 | Save/quit/reopen preservation and readable Light/Dark/seven-palette presentation | Pass |

## Bound evidence

- Runtime/contract source fingerprint: `4955a8fc1c505ed62085d3b2f6d27879c18f1b1b6ff2108ab721e161b960e92b`.
- [Verify42751543437](https://github.com/avacadorun-dev/writer-craft/actions/runs/42751543437): successful macOS source/engineering plus Linux/Windows core checks.
- [Preview42751537846](https://github.com/avacadorun-dev/writer-craft/actions/runs/42751537846): successful297 desktop tests, strict Clippy and native engineering package.
- ZIP SHA-256: `97d9647d519c79d8f75b08832d172e3ee9837cd876285078621123872a54f60c`; verified dependency lock: `a1c2e7f40c4d7e185c6ee7ab8326a809b1bd6de16874ca21eb857db843611143`.
- Independent compiled/source review before the owner's pass remains separately attributed: current native batch1 report `0b3cd6904cb6f8193dcae97726e9413bf5c571e5287527a0375ccd7d58ad6116`, batch2 `3122730c74dbd4fc6c1fecd3f5b116dfa26880b3fb5059cdd4c4cd3236654575`.

## Decision and remaining scope

The selected #14 structured drafting/markup workflow may advance to the next application issue, #7. Broad #14 stays open for uncompleted research, image sizing and Canvas-related criteria. Full Canvas #17, full parity #4 and source-project compatibility #10 are not accepted or closed by this confirmation. #7 direct object editing, #11 readable/canonical handoff and #18 home-body work remain separate pending deliverables. The final `structured-writing-alpha` release gate stays pending until those implemented candidate criteria are verified.

The owner has taken over manual click-through acceptance. Stop repeated automated UI walkthroughs and do not implement the proposed drag-input trace merely to reproduce an already accepted gesture. Continue one issue at a time with existing hosted checks and a small reviewable candidate/checklist for changed behavior. Do not drive, replace or restart the owner-used Acceptance15bc85a or any earlier protected preview. Subsequent owner writing/settings are private and are not public fixtures.

The subsequent independent selected-scope audit recommends advancement to #7, with no new blocker. Report SHA-256: `dafb59a7755aafdd63710207ed988d9a7332f1b6690a71934b23bbf13673ea43`. Its fresh QA2 observations separately confirm revised parent-first restoration and keyboard-context rename/cancellation; it attributes the remaining manual cases to the owner.
