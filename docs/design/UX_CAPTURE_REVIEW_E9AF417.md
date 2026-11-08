# Static block-flow retest — e9af417

Review date: 2026-11-08. Reviewer: independent desktop UX designer (`ux_acceptance_alpha2`). **Disposition: the 19-case static native capture regression passes for the displayed configurations and is ready for final-candidate review. No visible P0/P1 block overlap remains in these images. Complete issue #12, native interaction and alpha.2 acceptance remain pending.** No product issue is closed and `UX_GATE.json` is unchanged.

## Exact evidence

The reviewer inspected all **19 of 19** PNGs and paired capture states in screenshot/state artifact `13620227185`, `editor-preview-startup-smoke-e9af41771babefca9c56658e382106faa6a2c944` (2,160,381 bytes), from [hosted run 42487095816](https://github.com/avacadorun-dev/writer-craft/actions/runs/42487095816). Each PNG and state SHA-256 matches `startup-evidence.json`; the reviewer independently validated every PNG's chunk CRCs and decompression. The implementation owner supplied the artifact locally; this reviewer downloaded no application binary, used no owner window/clipboard/caret/private manuscript, and ran no local build.

- App source: `e9af41771babefca9c56658e382106faa6a2c944`.
- Evidence JSON SHA-256: `7041d0f1a807a2fc865a4891e5a55da95f6d8fab9a71a644a8ddfbf89291c34d`.
- Reported executable SHA-256: `1af649c5eab84810d386120a12055c7feb3cfd757543545982545799b3e4a453`; the executable itself was not independently downloaded/hashed.
- Lock SHA-256: `85699a9adb0f94929513d3e3ceddd7d751937ce5609d821ce064d1de3e5efb47`, independently calculated from committed source. Editor revision remains `21788bdd9e9ea8c4d2d3f4c90900633098aced7c`; WordCraft revision remains `da914ce674f18ebc4e08cca425cc8f8acca3372f`.
- Host: macOS 15.8.2/ARM64, device scale 1.0. Engineering-preview bundle version: `0.1.0-alpha.1`.
- Capture kind: native WGPU viewport readback after visible-render/scroll stability. Aggregate status is now `startup-captures-complete`, `all_captures_settled:true`. `actual_wide_coverage:false`, `editor_acceptance:false` and `native_interaction_acceptance:false` remain correct.

Each successful state reports a visible target where specified, no failed/pending visible atoms, stable readiness, unchanged canonical rich JSON and one readback request. Opaque targets have nonzero visible atom counts. Non-opaque startup views have no visible opaque atoms and establish no opaque coverage. All readbacks completed on the first request; this run does not itself exercise the retry limit or every transport-failure branch.

## Inspected states and real coverage

| Case | Effective logical viewport / PNG pixels | Observed content |
|---|---|---|
| `compact-dark-en-manuscript` | 1000×680 | English upper prose, marks, quote and compact toolbar |
| `compact-light-de-settings` | 1000×680 | German Appearance top, mode/palette previews |
| `compact-light-fr-editor` | 1000×680 | French Editor top, complete readback |
| `compact-dark-ru-files` | 1000×680 | Russian location/backup entry, no storage operation |
| `compact-dark-ja-editor` | 1000×680 | Japanese Editor top |
| `compact-light-zh-max-text` | 1000×680 | Chinese interface, 28px Latin prose |
| `wide-dark-es-focus` | **1024×684**, requested 1440×940 | Spanish constrained Focus |
| `wide-light-pt-manuscript` | **1024×684**, requested 1440×940 | Portuguese joined manuscript |
| `compact-dark-de-notes` | 1000×680 | German empty Notes inspector |
| `wide-dark-en-editor` | **1024×684**, requested 1440×940 | English Editor top |
| `compact-light-en-image` | 1000×680 | Full image, caption and next heading |
| `compact-dark-en-image` | 1000×680 | Table tail, image heading, full image and caption |
| `compact-light-en-flowchart` | 1000×680 | Flowchart/caption and separate sequence upper region |
| `compact-dark-en-sequence` | 1000×680 | Previous caption, full sequence/caption and math heading |
| `compact-light-en-formula` | 1000×680 | Inline expression, two display equations and following prose |
| `compact-dark-en-inline-formula` | 1000×680 | Same mathematical boundaries in Dark |
| `compact-dark-en-large-serif-image` | 1000×680 | 28px Source Serif table/heading and image upper tile |
| `compact-dark-en-large-serif-image-bottom` | 1000×680 | Image lower tile, complete wrapped caption, next heading and flowchart |
| `wide-light-en-small-inter-flowchart` | **1024×684**, requested 1440×940 | 17px Inter image-caption/heading/flowchart/sequence boundary |

The usual opaque cases use Sofia Sans 19px, line-height 1.75, paragraph spacing 0.9, list spacing 0.25 and 70ch. Both large-serif tiles use 28px Source Serif 4, line-height 2.0, paragraph spacing 1.1, list spacing 0.5 and 60ch. The small-Inter case uses 17px, line-height 1.5, paragraph spacing 0.65, list spacing 0.0 and 80ch. There is no 21px opaque case or genuine wide/floating-paper case here. The four `wide-*` names remain requested configurations, not observed wide acceptance.

## Issue #12 visible result

The previously missing lower-caption tile now establishes the intended 28px image/caption/next-sibling separation. Its target `showcase-diagrams` is actually visible: generation 7, two visible atoms, zero pending/failed atoms, stable count 6, one readback request and unchanged canonical rich JSON. The pixels show the lighthouse tail above both wrapped caption lines, then a separate “See the connections” heading and flowchart below. The enlarged caption is neither clipped by its document reservation nor painted into the following heading.

The complementary large-serif upper tile shows table text wrapping within its cells, followed by the independent image heading and image top. Default Dark also shows a clean table→heading→image boundary; default Light shows full image→caption→following-heading separation. The Light flowchart and independent sequence are distinct; the Dark sequence's caption and next math heading are distinct. Light/Dark inline mathematics follows the surrounding prose baseline; the display equations and later prose have their own space. No visible recurrence of the owner's F-37 sibling overlap is identified in this tested set.

The reviewer inspected the exact-source visibility repair. `VisibleTargetFrame` observes actual visible native text selections from `DecorationSource::underlines_in_view` and delegates ordinary decorations unchanged. Only paragraph/heading/code targets replace the opaque resolver-path visibility predicate; opaque targets retain that predicate. This addresses the 68 heading-target instrumentation mismatch without granting unconditional visibility or changing the document. The visible heading in the new tile supports the source explanation.

| Key retest PNG | SHA-256 |
|---|---|
| Large-serif lower tile | `c0ad8296c9688731e88b11f4071fecbf33d7d643d248ee134c494884f283c817` |
| Large-serif upper tile | `1350be6f0d25e511e59bdc9bc0b91c5764d53e9afb6602b13049d0fa93683b55` |
| Dark image/table boundary | `cb7ccf971c363e0598413bd822395a4a5b417195d81a7f5393d8bad6ceeff66c` |
| Light image/caption boundary | `b5aa9f35ed3c0ae1194e5588e47114dbe1caf9e99a69dca6ed17d09b08a34fdd` |
| French Editor settings | `d93379dc12cb4933c4eb73a6362f29976e7244c3d0e2e1bcdd90d290e3eef388` |

The prior unavailable captures in f9/3ea/68 remain historical unavailable captures; they are not retrospectively relabelled. This is the first reviewed bundle with all 19 existing cases captured at one exact source. See [the earlier capture review](UX_CAPTURE_REVIEW_F9_3EA.md) for that diagnostic history.

## Remaining acceptance boundary

This passes the static regression at its observed configurations. It does not establish the full cross-configuration geometry contract, real authoring/input or release readiness. Before final alpha.2 acceptance:

- Obtain genuine wide/floating-paper and joined/Focus/pane-transition evidence, remaining justified typography boundary cases including 21px, and presentation-only selection/scroll/Undo identity.
- Exercise source editing, progressive output, theme/cache invalidation, failures/retry, selection/hit testing, delete/Undo/Redo and save/reopen across real opaque blocks. The sidecar reports readiness counts and canonical rich JSON equality; it does not contain all per-atom actual paint/caption/reserved rectangles or establish assets/history/selection identity.
- Finish persistent passage marks and duplicate-quote notes, native OS input/IME/clipboard/accessibility, guarded proposals/Apply/Undo, A2-08 preference acknowledgement and the remaining exact-source storage/import/Canvas/Markdown/install workflows. Installer CI success alone is not mounted-app UX acceptance.

No new blocking visual finding is added by these 19 images. Previously documented local P3 long-category truncation and empty-Notes spacing remain. Pending product journey findings must be rechecked against their subsequent implementation rather than inferred from these static views. Keep every product issue open until the owner's final acceptance; **`UX_GATE.json` remains pending**.
