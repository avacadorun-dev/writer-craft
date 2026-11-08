# Independent review of settled f9c5b75 and 3ea5f6f native captures

Review date: 2026-11-08. Reviewer: independent desktop UX designer (`ux_acceptance_alpha2`). **Disposition: the inspected constrained opaque-block views show the expected separation; issue #12 and complete alpha.2 acceptance remain pending.** This review changes no release gate and approves no issue closure.

The reviewer inspected all 17 available PNGs in each bundle, verified their SHA-256 values and paired state hashes against the respective evidence record, and independently checked PNG chunk CRCs and decompression. No application binary was downloaded. No owner window, clipboard, selection, manuscript or private archive was touched; no local build was run. The two bundles remain separate source checkpoints and must not be combined into a fictional all-green 18-case run.

## Exact provenance

| Record | f9c5b75 bundle | 3ea5f6f bundle |
|---|---|---|
| App source | `f9c5b75c6db08a6e2fad730dec5974accb74533c` | `3ea5f6f630816e18ddfff29ca9a0ce8b7074cbe1` |
| Hosted run | [42483551113](https://github.com/avacadorun-dev/writer-craft/actions/runs/42483551113) | [42484403168](https://github.com/avacadorun-dev/writer-craft/actions/runs/42484403168) |
| Screenshot/state artifact | `13618842205`, 1,969,549 bytes | `13619202144`, 1,963,983 bytes |
| Evidence JSON SHA-256 | `e82eef46e4c2cd1391d0f6c95805181341eb0dbb76d538ad11221b905b483d53` | `76c9eed5b9a561e32a6b0ea956d687e18f3d72fed7359ddd669feb923ddfeb73` |
| Reported executable SHA-256 | `e64d4ea093c8b0d2e58848b2a312be5ea05cf7df357adf04d38ce7447ecd376a` | `5a39a335555a95c7a27826c9cceb0b7a452aea194cc81d9ce93812e5f5d2cb93` |
| Available valid PNG/state pairs | 17 of 18 cases | 17 of 18 cases |

Both use macOS 15.8.2/ARM64, device scale 1.0, bundle version `0.1.0-alpha.1`, and capture kind `native WGPU viewport readback after visible render and scroll stability`. Their dependency-lock SHA-256 is `85699a9adb0f94929513d3e3ceddd7d751937ce5609d821ce064d1de3e5efb47`. The reviewer independently calculated that lock hash from both committed sources. Each locks the editor crates to `21788bdd9e9ea8c4d2d3f4c90900633098aced7c` and the WordCraft crates to `da914ce674f18ebc4e08cca425cc8f8acca3372f`. The executable hashes are supplied provenance; this review did not download and independently hash those executables.

Both aggregate records correctly retain `all_captures_settled:false`, `actual_wide_coverage:false`, `editor_acceptance:false`, `native_interaction_acceptance:false` and `status:startup-capture-unavailable`. A successful packaging workflow does not erase an individual failed capture.

## Per-case observed coverage

“Present” below means a PNG/state pair exists, its hashes match, and it was independently inspected. Every present opaque case reports a nonempty visible atom set, a visible target, no pending or failed visible atoms, stable readiness, and unchanged canonical rich JSON. For present non-opaque startup cases, zero visible opaque atoms is expected and establishes no opaque coverage.

| Case | f9 | 3ea | Effective logical viewport and scope |
|---|---|---|---|
| `compact-dark-en-manuscript` | Present | Present | 1000×680; upper prose and formatting |
| `compact-light-de-settings` | Present | Present | 1000×680; Appearance settings top |
| `compact-light-fr-editor` | Present | **No PNG** | 1000×680; French Editor top available only at f9 |
| `compact-dark-ru-files` | Present | Present | 1000×680; Files location and backup entry |
| `compact-dark-ja-editor` | Present | Present | 1000×680; Japanese Editor top |
| `compact-light-zh-max-text` | Present | Present | 1000×680; 28px Latin manuscript, Chinese interface |
| `wide-dark-es-focus` | Present | Present | **1024×684**, requested 1440×940; constrained Focus |
| `wide-light-pt-manuscript` | Present | Present | **1024×684**, requested 1440×940; joined manuscript |
| `compact-dark-de-notes` | Present | Present | 1000×680; empty Notes inspector |
| `wide-dark-en-editor` | Present | Present | **1024×684**, requested 1440×940; Editor top |
| `compact-light-en-image` | Present | Present | 1000×680; image region, different settled scroll offsets |
| `compact-dark-en-image` | Present | Present | 1000×680; image, caption and next heading |
| `compact-light-en-flowchart` | Present | Present | 1000×680; image tail, caption, flowchart and sequence start |
| `compact-dark-en-sequence` | Present | Present | 1000×680; independent sequence, caption and next heading |
| `compact-light-en-formula` | Present | Present | 1000×680; inline and display math |
| `compact-dark-en-inline-formula` | Present | Present | 1000×680; inline and display math with following prose |
| `compact-dark-en-large-serif-image` | **No PNG** | Present | 1000×680; 28px Source Serif, line-height 2.0, 60ch |
| `wide-light-en-small-inter-flowchart` | Present | Present | **1024×684**, requested 1440×940; 17px Inter, line-height 1.5, 80ch |

The usual opaque cases use Sofia Sans 19px, line-height 1.75, paragraph spacing 0.9, list spacing 0.25 and 70ch. The large-serif case uses paragraph spacing 1.1/list spacing 0.5; the small-Inter case uses 0.65/0.0. This is useful variation, not exhaustive coverage of all values or combinations. No 21px opaque case or actual wide/floating-paper state is supplied.

### Failed captures are missing evidence

At f9, the large-serif image reaches ready generation 3 with one visible atom and zero pending/failed atoms, but scrolling never remains stable until the deadline. The implementation owner traced repeated nearest-edge target navigation on an oversized object to capture instrumentation and changed it to navigate once per metric generation. This review did not reproduce that failure through ordinary owner interaction and does not classify it as canonical corruption or a demonstrated product paint defect.

At 3ea, the large-serif image is successfully captured at generation 7/stable frame count 6. A different case, French Editor settings, produces no PNG: its failure state reports stable frame count 255, settled readiness, no pending/failed atoms, unchanged canonical JSON and a missing completed readback before the deadline. That is an unavailable capture, not a demonstrated French layout defect. Do not substitute the f9 French image as an exact-source 3ea capture or claim 18/18.

## Visible issue #12 geometry

No new P0/P1 visible overlap is identified in the available images. This supports the repaired separation in the displayed viewports:

- At 3ea, the Light image view exposes the merged-table tail, following image heading and top of the lighthouse. Their painted content remains separate. The image continues below that viewport; this particular image alone does not show its caption.
- The Dark image views show the full lighthouse with its retained aspect ratio, caption and following diagram-section heading. The caption occupies its own space and the heading starts below it. The f9 Light image view supplies the corresponding complete Light image/caption boundary.
- Flowchart views show the preceding image tail/caption, section heading, flowchart and its caption, then the beginning of the independent sequence. Sequence views show its separate participant/message area, caption and following math heading. The flowchart and sequence are not painted into one shared area as in F-37.
- Light/Dark mathematics show the inline expression aligned with surrounding prose without replacing adjacent words or colliding with the next line. Two display equations and following prose remain separate. Formula strokes, diagram labels and captions are readable on the displayed theme surfaces; this is a visual observation rather than complete contrast/accessibility certification.
- The 28px Source Serif view shows an intact image and the first enlarged caption line. The wrapped caption continues below the viewport. Full lower-caption/next-sibling containment at that size still needs an overlapping bottom tile or equivalent native evidence; ordinary viewport clipping is not itself a defect.

Opaque 3ea PNG hashes, independently matched to the evidence record:

| Case suffix | PNG SHA-256 |
|---|---|
| `light-en-image` | `36b07f35011fe9978b44cff8248925c719e4e5f15e48e8ce77becd355246abeb` |
| `dark-en-image` | `110daf7ff6aa152b27d4031ef0b47f14ebea0b4249d0407d93940a1b91606eeb` |
| `light-en-flowchart` | `7caec8594386be32bc645e89f302b0d4d4cab7e972919f6d4e06379320156f8d` |
| `dark-en-sequence` | `314799c557dca747abc3d141def20b48a07ebd1b81d1786a8ec3649f4bcdc11c` |
| `light-en-formula` | `d9489da4b1d7d59baeffdf0d9ca87ae627c4f94261e8290cca9fa752de44c92b` |
| `dark-en-inline-formula` | `9f4703e94a585ada0b2ad5dfecb9c704b832545fa5c543e1151338bc620c6c99` |
| `dark-en-large-serif-image` | `fef8ba154c53b116aa97ea44f0cbb48ee7b58496254a8d020b45b5e5f7a3c363` |
| `small-inter-flowchart` | `5b431fd96168c2ae61c8671ec0ce9cfd2076c720179ff6e5c776c09a1b654a63` |

## What remains pending

The new sidecars improve on the earlier startup records by recording actual viewport, device scale, effective typography, target visibility, readiness generation and canonical JSON equality. They contain counts rather than per-atom IDs/types, actual paint/caption bounds and reserved-flow rectangles. They therefore support manual observations of these pixels, not a general proof that all geometry or adjacent independent siblings are non-overlapping. Canonical rich JSON equality alone also does not establish asset/history/selection/Undo identity.

For issue #12, finish the lower large-caption tile, a genuine wide/floating-paper state and reversible joined/Focus/pane transitions. Complete the explicit font/size/spacing/measure matrix, including 21px and justified boundary cases. Exercise pending-to-ready output, source edits, theme/cache changes, render failure/retry, caret/selection/hit testing, delete/Undo/Redo, save/reopen and presentation-only identity preservation. Use actual painted-bound checks or equivalent compiled evidence across independent siblings. Keep the owner’s 141 preview untouched while obtaining isolated synthetic evidence.

For the rest of alpha.2, these captures do not retest the earlier storage-path/backup/retry findings or establish persistent note marks, duplicate-quote exact anchors, rich assistant proposals/Apply/Undo, dictionary/source playback, all authoring dialogs, spelling/IME/clipboard/VoiceOver, full original-project/Canvas import, portable Markdown/artifacts or installers. The required journeys in `UX_REVIEW_ALPHA2_2026-11-08.md` remain a checklist for their own exact-source evidence; historical source findings must be rechecked against current implementation before being reported as still broken.

The narrow Settings/prose/empty-Notes presentation observations from `UX_CAPTURE_REVIEW_182C9A3.md` still apply to the matching displayed states. Local P3 polish remains: long built-in Settings navigation labels truncate in German/Russian Files and Japanese shortcuts, and the empty Notes list reserves substantial blank space before repeated selection guidance. No new feature is proposed by those observations.

**`UX_GATE.json` remains pending.** Settled native pixels are substantial progress over startup-only images, but neither bundle supplies full alpha.2 native interaction acceptance or the owner’s final approval.
