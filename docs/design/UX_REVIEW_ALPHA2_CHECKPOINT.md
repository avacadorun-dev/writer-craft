# Independent alpha.2 checkpoint acceptance

Review date: 2026-11-08. Reviewer: independent desktop UI/UX designer (`alpha2_scoped_ux`). **Disposition: accepted only for `native-rich-editor-alpha`, the owner-approved engineering prerelease scope.** This accepts publication of the implemented checkpoint; it does not accept Author pilot, complete original editor/product parity, full native input/accessibility, or completion of its open product issues.

## Decision and scope

The owner's latest explicit decision, recorded in F-38 and NEXT_RELEASE.md, publishes the completed coherent alpha.2 checkpoint and moves unfinished large workstreams to alpha.3. The reviewer assessed that boundary independently rather than turning the previous full-parity rejection into a full-parity approval. The former gate is preserved byte-for-byte in [UX_GATE_FULL_PARITY_PENDING.json](UX_GATE_FULL_PARITY_PENDING.json): it is historical, still pending, and not a current release authorization. Full-parity requirements remain in their original specifications and open issues.

The named scope includes the currently wired native rich writing workspace, supported formatting and block dialogs, actual image/Mermaid/formula output, bounded presentation, existing local persistence/preferences and native installer packages. Based on the reviewed source, exact compiled views, hosted results and limited native observations below, no observed P0/P1 defect remains that blocks this explicitly limited engineering checkpoint. Existing incomplete journeys are disclosed rather than certified. The reviewer grants no daily-use or data-migration certification.

## Independently checked source and hosted results

- Runtime source: `8af4233ba5e531b360ee4914e4a5d5996f7bb72e`; app version `0.1.0-alpha.2`.
- Runtime/assets/design fingerprint independently recomputed: `627547cc12f80c551c9ae84947d16d370dc9db263c80b1bcaee01864596da0c9`.
- Cargo.lock SHA-256 independently calculated: `002f30af62e81ef39e8949b4971987cde435de0dc3ca7dd32b41054e2c6ada6e`.
- CraftEditor remains pinned to `21788bdd9e9ea8c4d2d3f4c90900633098aced7c`. No unqualified rectangular-table/clipboard/Canvas consumer pin is included. Disconnected unfinished format-2 framing is excluded from this release and preserved as alpha.3 work.
- The reviewer independently queried GitHub and confirmed **completed success at that exact runtime source** for [main verification 42493466703](https://github.com/avacadorun-dev/writer-craft/actions/runs/42493466703), [all-feature native verification 42493462208](https://github.com/avacadorun-dev/writer-craft/actions/runs/42493462208), and [installer verification 42493462622](https://github.com/avacadorun-dev/writer-craft/actions/runs/42493462622). The installer graph includes successful macOS/Windows/Debian jobs, the same Debian package on Ubuntu 24.04, and aggregate exact-source/version/checksum verification.

The review read current source, working agreements, owner feedback, scoped release policy, previous failed/pending reviews and the final verification record. No local Cargo build/test, owner-window manipulation, original archive access or private manuscript/clipboard/caret use occurred during this independent review. Hosted results are supporting execution evidence; the reviewer does not present them as personally executed native interactions.

The shared preference correction addresses A2-08's source-level cause coherently: one bounded acknowledged raw-JSON publisher supports all three existing formats; preserves unknown values and malformed/future sources; tracks desired and acknowledged state independently; retains unchanged pending choices for explicit Retry; reloads only a named group from a fresh existing valid file; and keeps session-only settings visible separately from manuscript/notes saved status. Publication is documented as atomic rename, not universal compare-and-swap against external writers. Deterministic tests and source review support this implementation; the complete native failure/retry/restart/AX matrix remains unqualified.

The final capture-navigation correction waits for three identical settled geometry/scroll samples, then permits at most one additional semantic navigation per renderer generation while the target is unseen. It neither forces target visibility nor changes manuscript spacing. Visible tall objects do not retry, and failed/unstable rendering cannot qualify a capture. The source retains actual visibility/readback requirements and bounded attempts.

## Actual normal-application visual evidence

The reviewer inspected the **20-case normal-application capture bundle** from artifact `13622593019` (2,237,128 bytes) in installer run 42493462622. Every PNG/state hash matches startup-evidence.json; every PNG chunk CRC and compressed pixel stream was independently checked. All nine changed/new images were viewed directly, and the other eleven are byte-identical to already directly inspected bde1be1 images. Thus all twenty displayed states were visually assessed without inventing unseen screenshots.

- Bundle identifier: `app.writercraft.desktop`; capture identity: normal; version: `0.1.0-alpha.2`.
- Capture evidence SHA-256: `e473a1bbf0de473846f7ea020c1f2087bc78e83e60934028e6b8edbc81f74d1d`.
- Executable SHA-256 reported by that record: `72031566e901cf983f542ef6ed6b2c945d6f83f87fa4d029bc48fd4ceea44795`; the reviewer did not independently download/hash the executable.
- Every successful sidecar reports settled rendering, zero pending/failed atoms, unchanged canonical rich JSON, and an actually visible target where requested. All twenty complete successfully. Automated `editor_acceptance` and `native_interaction_acceptance` remain false; this later independent named-scope decision does not rewrite those automation flags.
- Actual logical/pixel viewports are 1000×680 or 1024×684 at scale1.0. Requested 1440×940 cases are constrained and do **not** establish genuine wide/floating acceptance.

| Reviewed configurations | Actual visible coverage |
|---|---|
| English Dark prose, Portuguese Light manuscript and Spanish Focus | Legible marks, quote, bounded continuous writing, reachable compact/expanded toolbar and visible Focus exit; no adaptive-transition execution claim |
| German Appearance; French, Japanese and English Editor settings; Russian Files; Chinese maximum text | Categorized settings, aligned fields/actions, native palette previews, long/CJK labels and actual glyphs; no storage operation or preference-error recovery inferred from an image |
| German empty Notes inspector | Explicit selection guidance, reachable inspector controls and preserved manuscript; no note lifecycle acceptance |
| Default Light/Dark image and 21px Sofia image | Table/heading/image separation; the 21px view includes the full wrapped caption and independent following heading |
| Light flowchart and Dark sequence | Distinct independent diagram allocations and visible preceding caption; source is rendered as a diagram rather than raw text |
| Light display mathematics and Dark inline mathematics | Inline expression follows prose, two display expressions and later text remain separated |
| 28px Source Serif upper/lower targets, 17px Inter flow target | Enlarged heading/image; **last** wrapped image-caption line followed by separate heading/flow/sequence; small-font image/caption/heading/flow separation |

No visible P0/P1 sibling-block overlap remains in these reviewed views. The large-serif lower view does not show the first wrapped caption line, and some selected objects extend beyond the scroll viewport; this is explicitly incomplete full-caption/cross-configuration coverage, not a claim that every pixel of every block was shown. The older nineteen-case failures remain historical failures. This final twenty-case success does not retroactively relabel them. Complete transitions, all object/source editing paths and the full #12 acceptance matrix remain future verification.

The five README images match the final artifact's exact bytes and [CAPTURES.json](../images/alpha2/CAPTURES.json), including the 21px image/caption view. They use synthetic public demo content. The extra lower-caption regression image also matches its final artifact. Existing P3 category truncation and generous empty-Notes spacing remain nonblocking for this engineering scope.

## Native interaction evidence and its limits

The implementation owner supplied the separate native record in [RELEASE_ALPHA_2.md](../verification/RELEASE_ALPHA_2.md). At source c277a3c, the hosted DMG checksum and mounted normal app signature were verified. An isolated copy of that executable retained normal app identity/version while its copy's launch environment was redirected to a fresh synthetic database and the copy was ad-hoc re-signed. The original DMG, prior owner preview and original data were untouched.

Native computer use observed the monk/book mark, normal window/version, Add document modal/Return, actual rich toolbar/canvas, exact native paragraph paste and Command-S. Saved revision2/wordcount9 was visible; a read-only database query separately confirmed the exact paragraph inside canonical rich JSON. A local genuine wide view reportedly showed a bounded floating manuscript. After unlocking, the owner also observed the full Insert menu and Mermaid source/caption dialog, plus Files settings with the isolated database path. These action observations were supplied by the implementation owner; the independent reviewer personally inspected the source, final compiled images and verification record.

A source diff from c277a3c to the final runtime contains only explicit synthetic capture fixture/navigation changes and a role-correct accessibility geometry test; ordinary authoring/persistence is unchanged. The earlier observation can therefore support the limited launch/create/paste/save decision, but is not falsely represented as a fresh final-source native run. CUA title injection produced literal `2` instead of the requested full ASCII title; its automation/product cause remains unresolved. The Mac lock and later stale accessibility IDs/timeout interrupted further actions. Complete typing, successful diagram insertion, formatting, reopen, preference-error recovery, IME or screen-reader usability are **not** inferred from those interrupted steps. The reviewer accepts the engineering checkpoint with those explicit limits, not the unexecuted journeys.

## Deferred acceptance and publication conditions

Complete table cell/clipboard/structural integration, dictionary/source-note lifecycle, rich assistant proposal review, complete original `.wstudio` project/tree/history import, Canvas, PDF/audio/video, Markdown/artifact export and canonical restore remain alpha.3 work. Complete native input/IME/AX/performance, comprehensive save/reopen/edit/Undo journeys, preference failure/retry/reload/restart combinations and the original parity matrices stay pending. Neither render specimens nor preservation APIs count as completed end-user workflows. Product issues remain open until their own verification and owner acceptance.

The macOS DMG is ad-hoc signed, not Developer ID signed/notarized; Windows is unsigned. Windows/Linux hosted install/remove checks do not certify all desktop interactions. Release staging must still verify the actual tagged packages, checksums and exact source/version, preserve original engineering evidence, and publish only the new authorized prerelease without changing alpha.1. A later source/assets/fingerprinted-contract change invalidates this acceptance and needs a scoped retest.

**Final disposition: accepted for `0.1.0-alpha.2` / `native-rich-editor-alpha` only. No full-parity or daily-use acceptance is granted, no product issue is closed, and all stated alpha.3/native verification gaps remain visible.**
