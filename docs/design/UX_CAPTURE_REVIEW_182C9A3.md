# Independent review of 182c9a3 native startup captures

Review date: 2026-11-08. Reviewer: independent desktop UX designer (`ux_acceptance_alpha2`). **Disposition: limited narrow-startup presentation observations; complete editor and bug #12 acceptance remain pending.** No owner window, clipboard, manuscript or application binary was used by this review. No implementation code or release gate was changed.

## Exact evidence inspected

The reviewer downloaded only the 1,240,730-byte screenshot/state artifact `editor-preview-startup-smoke-182c9a35bfc5d191f87d9839409f5bc0e0e3ed81` (artifact 13617516063) from successful [run 42480079479](https://github.com/avacadorun-dev/writer-craft/actions/runs/42480079479). All ten PNG hashes matched `startup-evidence.json`; all ten images were independently inspected. No 57MB application artifact was downloaded.

- Application source: `182c9a35bfc5d191f87d9839409f5bc0e0e3ed81`.
- Reported executable SHA-256: `7022b0e458874ac922a93a6a84dc4f9ac78e154f81aebbc123fdecedb344c30d`; this review did not independently download/hash that executable.
- Reported dependency-lock SHA-256: `de45418e890ca176ee60f70df02a1aa6b40ec73a66793029fe9aa84a58dcea2a`.
- Host: macOS 15.8.2/ARM64. Capture kind: native WGPU viewport startup readback after frame 8.
- Every case is explicitly `captured-startup-only`; `media_settled:false`, `editor_acceptance:false` and `native_interaction_acceptance:false` remain correct.

| Case | Requested inner size, points | Observed PNG, pixels | Visible evidence |
|---|---|---|---|
| `compact-dark-en-manuscript` | 1000×680 | 1000×680 | English joined manuscript, upper prose only |
| `compact-light-de-settings` | 1000×680 | 1000×680 | German Light Appearance settings, top only |
| `compact-light-fr-editor` | 1000×680 | 1000×680 | French Light Editor settings, top only |
| `compact-dark-ru-files` | 1000×680 | 1000×680 | Russian Files location/backup entry, no backup action |
| `compact-dark-ja-editor` | 1000×680 | 1000×680 | Japanese Dark Editor settings, top only |
| `compact-light-zh-max-text` | 1000×680 | 1000×680 | Chinese interface with28px Latin manuscript prose |
| `wide-dark-es-focus` | 1440×940 | 1024×684 | Spanish Focus, constrained observed image |
| `wide-light-pt-manuscript` | 1440×940 | 1024×684 | Portuguese manuscript with navigator, constrained observed image |
| `compact-dark-de-notes` | 1000×680 | 1000×680 | German empty Notes inspector, navigator collapsed |
| `wide-dark-en-editor` | 1440×940 | 1024×684 | English Dark Editor settings, constrained observed image |

### Requested wide coverage was not observed

The three cases named `wide-*` request 1440×940, but their PNGs are 1024×684. The artifact does not record effective logical viewport size or device scale, so the requested dimensions cannot be substituted for observed layout coverage. The implementation owner identified hosted monitor clamping. These cases do not establish the intended 1440×940 native wide state.

The Portuguese manuscript remains joined with its navigator present; the Spanish Focus image also shows a constrained presentation. This is a capture-coverage limitation, not a demonstrated defect in the allowed constrained layout. Obtain an actual wide independent compiled capture later; do not fake a resize or relabel this evidence as wide acceptance.

## What the narrow images establish

The compact prose/header/toolbar is coherent, bounded and readable. New Sofia Sans manuscript presentation is visibly implemented; the 28px case wraps headings and prose without ordinary horizontal overflow in the displayed region. UI labels render in their selected languages while synthetic author/project text remains unchanged. This does not prove native CJK manuscript fallback or IME because the displayed manuscript text is Latin.

Editor settings show full-width font selection and comfortable size/line-height choices with aligned labels and selected state. French/Japanese top sections are legible; lower paragraph/list/measure/goal/proofing controls are outside these captures and remain unreviewed here. The footer now visibly follows the 40px action rhythm in the shown Settings states, supporting the visual portion of A2-05; pointer/keyboard target execution is not supplied by a startup image.

Files clearly displays the actual synthetic workspace location, backup scope and entry actions. The screenshot does not exercise pending note prerequisites, publication, failure, cancellation or a resulting backup receipt. The German empty Notes state now explains why a passage within one text block is needed, supporting A2-04 for the shown no-selection case. Actual cross-block/composition states and persistent marks are not displayed.

No additional P0/P1 product defect is identified in these ten startup images. Existing full-scope blockers and the owner-observed opaque-block overlap remain unresolved by this evidence.

### Visible local polish

- Normal built-in Settings labels truncate in the Russian Files category and Japanese Keyboard category. A tooltip exists in source. Consider shorter localized navigation labels or a responsive navigation allocation, retaining a full heading/accessibility name.
- The empty German Notes inspector reserves roughly 180px of blank list space before its empty-state message and then repeats selection guidance below another divider. Collapse the empty list reservation and use one coherent empty-state explanation. This is local empty-state polish; Add note is visible and its disabled reason is now understandable.

These are P3 presentation observations for the shown states, not a reason to invent new product features or obscure the P1 renderer-flow work.

## Minimum opaque-block topology for the next native retest

Use the existing synthetic showcase and target stable IDs through actual semantic navigation. Preserve its original source, image dimensions and captions. A genuinely tall block can span multiple overlapping viewport tiles; scaling it to illegibility is not required.

| Target region | Existing stable IDs | Failure exposed |
|---|---|---|
| Code/merged-table to image boundary | `showcase-table-title`, `showcase-table-wide`, then `showcase-image` | Image paint encroaching on preceding text/table or an insufficient image reservation |
| Image/caption to diagram heading | `showcase-image`, `showcase-diagrams` | Lost intrinsic 720×400 aspect/height, detached caption or following heading occupying the image |
| Independent diagram adjacency | `showcase-flowchart`, `showcase-sequence`, then `showcase-math-title` | Flowchart/sequence/captions painted into the same reserved rows or the following section |
| Inline and display mathematics | `showcase-inline-equation`, `showcase-quadratic`, `showcase-sum`, `showcase-closing` | Formula baseline/height replacing prose line-height assumptions, or block equations colliding with later prose |

First reproduce the issue with the default/migrated settings in Light and Dark; then test 28px and the 1.5/1.75/2 line-height values with narrow and Focus transitions. Extend the explicit F-37 coverage table across remaining font/size/measure values. Headless measurement checks complement these actual pixels and interactions.

### Capture evidence requirements

1. Record requested and effective logical viewport, PNG dimensions, device scale, effective font/size/spacing/measure and pane state. A size or state mismatch degrades that specific coverage claim even when a valid PNG exists.
2. Record target ID/path and the expected nonzero opaque atom IDs/types for each opaque case. Confirm the target region is actually visible; an empty visible-atom set must not vacuously establish settled opaque coverage.
3. Require valid expected outputs to be ready with no pending/failed visible atoms, and stable resolver/layout identity over at least two rendered frames. Record current generation/identity rather than inferring readiness from frame count or sleep duration. An expected render error is a distinct explicit failure fixture, not successful rendering of a valid showcase sample.
4. Bind the screenshot and sidecar to the same rendered state. Include reserved flow, actual paint and caption bounds for the target and adjacent independent siblings; exclude intentional parent/child containment and internal diagram composition from sibling-overlap assertions.
5. Preserve the existing PNG CRC/content/hash validation and bounded child-process ownership. Treat startup availability, settled rendering, geometry checks and native interactions as separate evidence kinds. Preserve canonical JSON/assets/revision/selection/Undo through presentation-only operations and record insertion/edit/Undo/save/reopen separately.

The current `scripts/native-startup-smoke.py` honestly labels its startup-only scope and provides useful source/hash/process provenance. Its PNG validator checks valid bounded image data, not intended viewport/state or renderer readiness. The new settled-target capture mechanism must add the observations above; the existing ten startup views cannot close [bug #12](https://github.com/avacadorun-dev/writer-craft/issues/12).
