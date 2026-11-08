# Independent alpha.2 UX checkpoint — 2026-11-08

Reviewer: independent senior desktop UX reviewer (`ux_acceptance_alpha2`). Disposition: **changes required; full alpha.2 acceptance remains pending**. This review does not change `UX_GATE.json`, approve a release or authorize closing product issues.

## Scope and evidence

This checkpoint reviews the owner-approved Balanced/adaptive desktop direction, complete editor journeys, new preference and storage flows, and the expanded alpha.2 boundary containing all open issues. The only approved editor capacity exception remains 63 logical table columns. Original project/Canvas/media migration and portable exports are required alongside editor parity.

The reviewer read the working agreements, PRD, desktop/workflow/UX contracts, owner feedback through F-36, historical reviews001–003, passage-note source review and current OpenSpec presentation/portability requirements. Implementation source was inspected at `68f140ce52ca14493799b826660d23c107b16007`; concurrent implementation changes are outside that immutable source checkpoint. No original project, owner manuscript, running preview, clipboard or caret was manipulated. No local Cargo command or build was run.

The independently inspected images are mounted WGPU startup captures from `95313f7527ff1ecd41d8defdc01d13873991d07c`, produced on macOS15.8.2/ARM64 in [run42476048346](https://github.com/avacadorun-dev/writer-craft/actions/runs/42476048346). The implementation owner's qualification record reports264 workspace tests, strict formatting/Clippy, package identity checks and successful startup captures; [source CI](https://github.com/avacadorun-dev/writer-craft/actions/runs/42476053756) also passed. These checks were not independently rerun by this reviewer.

| Artifact | Observed state | SHA-256 |
|---|---|---|
| `compact-dark-en-manuscript.png` | 1000×680, English, Dark, navigator visible, inspector closed | `9c61ae8eb9a1febf388bda6da0f201caa95468f96b08d4e0abb12c6478846a33` |
| `compact-light-de-settings.png` | 1000×680, German, Light, Appearance settings | `4f64fc13fa1f9d9f62d7e56405d1e865c29bdfb9b50cec44d1fce6c523b6b595` |
| `startup-evidence.json` | Exact source/compiler/bundle identity and capture metadata | `8b47578a56c441faca2746118cf801b874b2396a2297120bbfa69f603713708f` |

The startup record explicitly states `media_settled:false`, `editor_acceptance:false` and `native_interaction_acceptance:false`. These images establish the displayed initial views. They do not establish settled diagrams/images/formulas, OS window controls/Dock behavior, native input, font changes, recovery operations or complete project workflows. Later typography/proofing/source changes need their own captures and interaction records.

## Presentation observed in the qualified captures

The compact manuscript has one clear workspace, shared navigator/command-bar boundary, readable Lucide utilities and monk identity, stable identity/format rows, and a bounded writing measure. The inspected image shows no title collision or the previously reported independent separator alignment. The manuscript retains breathing room without a redundant hero title area.

Settings use a clear left-aligned category hierarchy, compact icon/label groups, a vertically aligned header/close control and bounded footer. Light theme previews communicate mode independently of palette. The German Files label truncates but has a source tooltip; longer labels and keyboard/screen-reader discovery still require execution. These observations support the chosen direction for the two displayed states. They do not approve wide/Focus transitions or the new19px typography.

## Required corrections

### A2-01 — Protect active data from every output path (P0)

At the inspected source, `StudioApp::export_text` writes the selected file through `atomic_write` without protecting the active workspace or its sidecars (`app.rs569–584`). A valid synthetic override such as `--database /tmp/active.txt` makes the live SQLite file selectable by the text-export filter. Confirming that target replaces the active database with prose. Recovery JSON already contains an alias-aware protected-path check; the ordinary export route does not use it.

There is a second deterministic collision: preference paths are derived with `database.with_file_name(...)`. If the selected database itself is named `appearance.json` or `interface.json`, changing the corresponding preference can replace SQLite with preference JSON. The new guarded editor store prevents an analogous undecodable `editor.json` replacement, but the other preference writers remain writable at this checkpoint.

Centralize the active-file boundary. Protect the workspace, reserved WAL/SHM names even when absent, canonical/symlink/hardlink/case aliases as appropriate, and reject conflicting internal preference namespaces before any write. Keep final no-clobber publication checks. Verify synthetic custom-extension databases, reserved preference names, absent sidecar targets and aliases, with unchanged canonical data/history and an actionable localized refusal. This is a source-deterministic integrity finding; no owner data was used to reproduce it.

### A2-02 — Make blocked backup prerequisites actionable (P1)

`start_workspace_backup` checks note drafts across all documents and paused composers, then shows only “Save or discard pending note edits before backing up” (`storage.rs91–99`). A draft may be parked in another project/document. The Files view identifies neither the affected draft nor a route to save/review it. Save-all is available through application-close confirmation, which is not a reasonable backup prerequisite journey.

Offer an explicit continuation such as Save pending notes and continue, Review pending drafts, and Cancel. Name affected documents and preserve exact bodies when a save fails. Do not silently commit or discard notes on a backup click. Retest with multiple parked drafts, a conflict/invalid draft, cancellation, and a subsequent verified backup without leaving the author to discover hidden state.

### A2-03 — Expose preference persistence retry (P2)

Editor settings save only when values differ from the current frame's initial values (`settings.rs496–498`). The store correctly retains in-memory choices after an I/O failure, but dismissing the error or reopening Settings provides no explicit Retry Save action. The same desired values therefore do not trigger another attempt. The displayed preferences can be runtime-only until another setting is changed.

Retain a visible pending-persistence state and explicit retry. Explain that current choices have not been saved; Done must not imply an acknowledgement that did not occur. Verify a failed write, dismissal, restored destination access, retry without changing the desired values, and successful restart restoration. Preserve the existing invalid/future-file write barrier.

### A2-04 — Explain an unavailable passage-note action (P2)

The rich selection menu and Notes header disable Add note when a selection cannot be represented as a supported single-block anchor (`notes.rs786–842`, `863–873`), with no explanation at that action. An author selecting across paragraphs cannot distinguish a scope restriction from broken note support.

Provide an accessible disabled-state explanation and a concise hint at the relevant surface. Preserve the current exact-occurrence rule; do not silently expand, flatten or guess an anchor. Verify same-block forward/reverse/Unicode selections, supported inline atoms, cross-block selection and composition.

### A2-05 — Finish the shared40px dialog action rhythm (P2)

The inspected Settings code still uses a36px Done button and36px content interaction minimum (`settings.rs437`, `460`), while F-35 and the presentation spec require40px dialog actions/fields. The implementation owner subsequently reported a source correction to40px. Its compiled visual/interaction retest remains pending.

Verify the corrected footer's equal vertical insets and full target bounds at1000×680 and common window size, including long German/French labels and CJK. Keep32px utility targets distinct from the40px form-action requirement.

### A2-06 — Distinguish the latest backup attempt from previous success (P2)

`last_backup` remains displayed in green after a newer cancelled or failed attempt, with the label “Verified workspace backup” and no attempt/capture time (`storage.rs60–73`, `128–163`). Make this explicitly the last verified backup and show the current attempt's outcome so an old receipt cannot be mistaken for new success.

A unique date/time default filename would also avoid the ordinary second-backup flow offering native Replace followed by the correct backend no-clobber refusal. This is a presentation improvement within the existing backup journey; the no-overwrite guarantee must remain.

## Required journeys still awaiting completion

These are required alpha.2 scope gaps, not regressions introduced by the qualified startup captures:

- Rich assistant attachments and proposal review: the rich Review branch still returns an unavailable message (`review.rs87–97`); the rich selection menu exposes notes only. Exact multiblock attachments, source reveal/removal, supplied unapplied proposals, stale state and guarded Apply/Reject/Undo need real shared command and native journeys.
- Dictionary links and source playback: preserved metadata or imported projections do not yet provide the original selected-term/reveal/deleted-snapshot and bounded-source-audio interactions.
- Full project migration/export: archive selection still invokes the read-only inspector, navigator remains a flat document list, and the exposed export is plain text. The new core import/companion primitives need actual import/tree/Canvas/media/Markdown UI and restart/round-trip acceptance. The private reference archive must remain local and read-only.
- Native selection, accessibility, composition, rich clipboard, complex table/fitter operations and all block authoring require complete execution evidence. Existing controller/model tests and a showcase do not establish those daily author journeys.

## Next compiled evidence and interaction bundle

Use isolated synthetic workspaces and record one exact committed app/engine/lock identity per bundle. Do not reuse95313f7 captures as evidence for later source.

1. Capture1000×680 and1440×940 with navigator/inspector hidden/shown/resized, wide/floating and constrained/joined document states, and reversible Focus. Compare exact caret, selection, scroll and Undo before/after.
2. Capture Editor preferences and live prose for all three families,17/19/21 and retained custom size, all spacing/measure choices and toolbar modes. Test all commands through compact overflow. Verify restart acknowledgement and the A2-03 retry path.
3. Capture Files, pending-draft prerequisite resolution, progress/cancel, failed destination and last verified backup. Execute the active-path safeguards and verify the published receipt against the resulting database.
4. Execute the second identical passage with Cyrillic, emoji and combining marks: add/edit/resolve/reopen/delete/reattach a note, move focus, save/restart, and preserve a deleted quote. Include cross-block disabled explanation, parked drafts, metadata conflict and read-only recovery.
5. Open/close every source-editing dialog through pointer/keyboard, inspect real code/structured syntax highlighting, author each supported block from empty content, edit again, delete/Undo/Redo and reopen. Include stale/cancel/failure states and local block overflow.
6. Verify real system spelling with supported language detection and measured cold/warm latency, code/atom exclusion, stale results and IME preedit. Availability text must remain honest on unsupported systems. The standalone OS probe reported by the implementation owner is supporting evidence, not mounted-app acceptance.
7. Exercise ru/en/de/fr/es/pt/zh/ja, representative long German/French labels, CJK glyphs and actual ru/zh/ja input. Inspect Light/Dark and every retained palette, native focus traversal and VoiceOver/AX names, roles, ranges and atom/cell navigation.
8. Complete native full-project import/Canvas/media/export/restart and installer qualification before final owner handoff. Keep all product issues open until the owner's final acceptance.

No final UX acceptance is granted by this document. `UX_GATE.json` correctly remains pending because matching complete native evidence and unresolved-issue retests do not yet exist.

## F-37 addendum — owner-observed native block overlap

Reviewed on 2026-11-08 after the earlier checkpoint. **A2-07: P1, changes required**, tracked in [bug #12](https://github.com/avacadorun-dev/writer-craft/issues/12). The owner screenshot was inspected privately and is not copied into the repository. It shows the lighthouse image covering adjacent code/heading/table flow and two independent Mermaid diagrams overlapping captions/prose and each other. This is a visible native flow/paint allocation failure. No canonical corruption is inferred from pixels.

The implementation owner supplied app source `141fce5e5fc27ca9bfeed1661831baa5f2370cc6` and engine `b96b4d971c649bc024f37a72fd7ba626e7393100`; the reviewer independently read matching local build metadata for app source, ARM64 target and lock SHA-256 `3e2299fce39e5126fdcf2425c239ce498f63866a241186868055b0f6b4b28d61`. That metadata explicitly has editor/native-interaction acceptance and release set false. The cropped screenshot does not supply effective preferences or complete viewport metadata. The implementation owner's proposed line-height/intrinsic-object repair remains implementation work until qualified and retested.

The positive `95313f7` observations above apply to the two displayed compact startup states. They did not inspect the affected opaque viewport; the evidence record explicitly stated `media_settled:false`. The newly reported failure confirms that startup captures cannot establish rich-block composition or media flow.

Retest the exact showcase at the affected blocks, with current media settled and source identity recorded, then cover every font/size/spacing/measure value and Light/Dark adaptive state through the F-37 matrix in `UX_ACCEPTANCE.md`. Assert actual paint-plus-caption containment and separation from adjacent independent blocks, not only positive declared rectangles. Exercise progressive acknowledgements, cache/theme invalidation, selection/hit testing, Undo and save/reopen while comparing unchanged canonical data for presentation-only operations. No final acceptance or issue closure is granted by this addendum.


## Settled native capture follow-up — f9c5b75 and 3ea5f6f

The independent reviewer subsequently inspected all 17 available PNG/state pairs from each exact source, verified their recorded hashes and PNG CRC/decompression, and recorded the limited result in [UX_CAPTURE_REVIEW_F9_3EA.md](UX_CAPTURE_REVIEW_F9_3EA.md). The shown constrained image/table/diagram/caption/math boundaries are separated; no new visible P0/P1 overlap is identified in those views. The new 28px Source Serif image capture is useful but leaves the lower wrapped caption outside the viewport.

Each 18-case bundle still has one unavailable capture, and every requested-wide case is observed at 1024×684. These are limited exact-source visual observations, not complete F-37, interaction or alpha.2 acceptance. Issue #12, outstanding journey retests, owner acceptance and `UX_GATE.json` remain pending.
