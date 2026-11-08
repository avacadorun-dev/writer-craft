# Native rich-block renderer integration

Status: implementation and integration qualification, 2026-11-08. Full editor release acceptance is pending.

## Reused components and boundaries

| Role | Component | Pin / evidence |
| --- | --- | --- |
| Rich model, exact selection, native view and renderer adapter | CraftEditor | Application consumes one exact Git revision for all three packages. |
| Continuous text/table flow and raster paint | WordCraft focused crates | `da914ce674f18ebc4e08cca425cc8f8acca3372f`; no application UI is copied. |
| Mermaid parse/layout/native SVG | [Merman](https://github.com/Latias94/merman) | `0.9.3`, full default capability closure including ELK; its resvg-safe SVG pipeline is rasterized without a browser. |
| KaTeX-compatible formula parse/layout/PNG | [RaTeX](https://github.com/erweixin/RaTeX) | `0.1.15`, embedded KaTeX fonts. Upstream coverage claims are not our verified source-parity result. |
| SVG rasterization | [resvg/usvg](https://github.com/linebender/resvg) | `0.47.0`; explicit provided fonts/media and bounded raster output. |
| Raster decoding | [image](https://github.com/image-rs/image) | `0.25.10` in the renderer; PNG/JPEG/GIF/WebP. Animation acceptance remains pending. |

CraftEditor exposes measured `ResolvedAtom` media and a generation-aware `AtomResolver`. Mermaid/TeX/image source stays in the original canonical nodes; the disposable WordCraft projection receives real PNG bytes and logical bounds. The formula baseline is measured from the native layout, including its descent. Captions remain canonical source attributes and native text layout, rather than being flattened into the manuscript or discarded.

WriterCraft prepares the catalog off the paint path, caches by exact atom source and presentation identity, guards late results, and does not re-render diagrams after an unrelated prose keystroke. The host supplies bundled or acknowledged media bytes; arbitrary URL/filesystem access is not delegated to a node renderer. A failed block keeps its original source and an explicit localized source fallback. A valid demonstration block must render successfully; a fallback is not evidence that its renderer works.

## First verified engine checkpoint

[Hosted run 42454627759](https://github.com/avacadorun-dev/craft-editor/actions/runs/42454627759), source `5e5d33882156df2f752845bb10a07587bc7fb3da`, passed 91 model, 35 view and 11 renderer tests (137 total), strict Clippy and geometry/canvas/native-atom examples. Its retained PNGs include actual flowchart, sequence and state diagrams, display/inline formulas, a vector illustration and an explicitly labelled invalid-source fallback. The renderer agent inspected the PNGs; application/native interaction and independent UX acceptance remain separate.

The subsequent pin adds source-backed formula-command restrictions and the proof's non-deserialization compile-fail example. Its results must be recorded after execution; an earlier passing run must not be attributed to a later commit.

## Complete application specimen

`crates/studio-desktop/src/app/editor_showcase.json` covers all 21 canonical node types, six marks and 44 unique block identities. It includes nested lists, source-preserving quotes, all four callout kinds, distinct separators, literal code, merged cells, a bundled original image, two Mermaid diagrams and inline/display formulas. The application integration test requires six actual prepared media atoms with zero source fallbacks and a successful full native-flow measurement.

The specimen is created idempotently in the isolated preview project. Its marker, rather than its editable title, identifies an existing specimen; subsequent startup preserves owner edits. `assets/demo/lighthouse.svg` is original MIT-licensed demonstration artwork. It exercises the real image pipeline and does not imply that durable media import, drag/drop or transfer is accepted.

## Remaining release checks

All original editor requirements remain mandatory. In particular, actual native selection/input across atoms, list behavior, structured clipboard, IME, accessibility, contextual source/media editing, inherited formula typography, image animation, durable media/transfer, passage decoration/recovery, source audio and assistant/MCP handoffs need their full source-backed evidence. The 63-logical-column exception remains the only approved capacity change. The owner-selected Balanced concept requires actual compiled, responsive and localized UX review; a design picture or a renderer PNG cannot pass it.
