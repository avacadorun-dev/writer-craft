# Native opaque-block flow regression

Status: bug #12 remains open; complete compiled-app/native interaction and independent acceptance are pending. Owner content and screenshots are private and are not copied here.

## Cause and correction

The owner observed a large image and independent Mermaid diagrams painting over code, tables, captions and prose in engineering preview `141fce5`. The application uses font-size-based line spacing. The shared layout assigned that exact prose strut to object-bearing paragraphs too. WordCraft's `Exactly` line-spacing policy intentionally ignores picture extent; its `AtLeast` policy includes natural text and picture metrics. This mismatch allowed complete object paint with insufficient flow advance.

CraftEditor `21788bdd9e9ea8c4d2d3f4c90900633098aced7c` retains exact font-size spacing for plain prose, while opaque block paragraphs and paragraphs containing inline math use the backend's existing minimum-height policy. No application offset, synthetic spacer or fixed large media height is involved. Canonical data and history are not rewritten to repair presentation.

[Engine qualification 42481747940](https://github.com/avacadorun-dev/craft-editor/actions/runs/42481747940) passed 291 semantic contracts, two replay-proof doctests, strict formatting/Clippy and native Mac/X11 probes. The mixed fixture checks actual native image/caption paint against allocated flow and following code/table/flowchart/sequence/formula/prose at 520/820-point widths. Tall pending and failed placeholders are covered separately. The production-identical evidence-palette correction `6682dbc54dbb76ec2f68019868a1c5e6395ad98e` passed [run 42482212131](https://github.com/avacadorun-dev/craft-editor/actions/runs/42482212131); all four full Light/Dark fixture PNGs were inspected. This is engine evidence, not compiled WriterCraft acceptance.

## Application integration and evidence protocol

WriterCraft `7b51adf` pins the qualified engine and replaces frame-8-only capture with explicit semantic target navigation, a bounded 38-second readiness deadline and at least three consecutive frames of unchanged visible-render generation and scroll offset. It records target visibility, nonzero visible opaque atoms for targeted captures, pending/failure counts, actual viewport/device scale, actual PNG size, active reading preferences and unchanged canonical data. A failed render or vacuous target cannot qualify as a successful valid-fixture capture.

Synthetic hosted views target image, flowchart, sequence, inline and block formulas in both modes, including large serif and small Inter profiles. Readback files and evidence sidecars refuse existing outputs and live workspace database/journal/preferences paths. Capture navigation uses the same semantic selection/scroll controller and does not allocate flow or mutate writing.

The earlier independent [182c9a3 capture review](../design/UX_CAPTURE_REVIEW_182C9A3.md) found that requested 1440×940 windows were constrained to actual1024×684 on the hosted WindowServer. New evidence records this limitation explicitly. A requested window size cannot establish observed wide-layout acceptance; real wide compiled-app evidence remains required. Capture state is not a substitute for pointer/keyboard, editing, selection, notes or Undo interaction acceptance.

## Qualification log and outstanding gate

- `7b51adf` compiled and passed the new capture/controller tests, but [app run42482420145](https://github.com/avacadorun-dev/writer-craft/actions/runs/42482420145) stopped at a newly wired export fixture whose `asset://` document-link scheme was invalid. The supported canonical media-link fixture was corrected without weakening validation.
- `59d6ae1` compiled and passed new scoped-image capability tests, but [run42482796096](https://github.com/avacadorun-dev/writer-craft/actions/runs/42482796096) stopped at an older test that incorrectly treated every non-UUID source ID as invalid. Source-project IDs are bounded opaque strings; the malformed fixture now uses an empty identity. Fixed-project ownership and exact bytes remain mandatory.
- Full application and settled opaque captures must pass at an exact committed source before the correction is described as verified in WriterCraft.
- Independent native review must inspect the actual settled block neighborhoods, typography/adaptive states and interaction outcomes. The owner is still exploring the separate `141fce5` window; do not replace it or use its clipboard/caret for automation.
- Bug#12 and all other alpha.2 product issues stay open until the owner checks the final complete candidate and approves closure. No alpha.2 release/tag is authorized by these engineering checkpoints.
