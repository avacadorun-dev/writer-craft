# TypeScript to Rust: preserve behavior, choose engines by evidence

Research date: 2026-11-07. Source is the current Writers Studio working tree and the primary repositories linked below. This is a feasibility assessment; candidate dependencies have not been adopted or verified by WriterCraft. Version ranges below are declared in the original `package.json`, not necessarily exact lockfile versions.

## Recommendation

The complete product can be implemented natively without translating every TypeScript dependency. Porting a library is an optional technique when it protects useful semantics or reduces verified implementation cost. It is not a requirement to translate the entire web stack.

The strongest strategy is to port our own behavior, schemas, algorithms and acceptance fixtures; investigate existing Rust semantic ports where compatible; and implement platform-specific presentation/input against native layout and graphics. A generic text box is a useful MVP surface, but it does not establish rich-editor, annotation or selection parity. Screenshots are visual evidence only. They do not prove that the selected occurrence, block IDs, undo, original media or annotation anchors survive editing.

## Actual dependencies and practical decisions

Relative costs describe reaching our existing behavior with tests and native integration. They are not schedule estimates. Existing ports reduce algorithm work; they do not remove input, accessibility, file-fidelity or maintenance work.

| Current dependency / behavior | Native options and what is reusable | Recommendation and relative cost |
| --- | --- | --- |
| Tiptap `^3.2.2` / ProseMirror model, transforms and plugins | [ProseMirror architecture](https://prosemirror.net/docs/guide/) separates model/state/view/transform. [fiduswriter/prosemirror-rs](https://github.com/fiduswriter/prosemirror-rs), crate `prosemirror` 0.5.2, MIT, implements model/schema and transforms/position mapping with JS-compatible UTF-16 positions. It is not a native editor view or a port of all Tiptap extensions. | Run a schema/transform compatibility spike before choosing it over our own rich model or a WordCraft adapter. Native rich input/rendering remains **high** cost; isolated model reuse is **medium**. |
| Our block identity, source refs and annotation mapping | `src/features/editor/model/extensions.ts` and `annotations.ts` contain our behavior above ProseMirror: identity on splits, provenance merging, mapped anchors and detached ranges. Existing tests cover Cyrillic, emoji and insert/delete mapping. | Port these contracts and fixtures deliberately regardless of rendering engine. **Medium**, with important integration work; never replace with substring highlighting. |
| React Flow `@xyflow/react ^12.11.2` | [React Flow source](https://github.com/xyflow/xyflow) is React/DOM-oriented. [egui-snarl](https://github.com/zakarumych/egui-snarl) is a native graph-widget substrate, not an API/behavior port. FigCraft is another reference, with a much larger application model. | Preserve our Canvas JSON/IDs and own operations. Implement or adapt native hit testing, camera, selection, drag/resize, connections, snapping, keyboard and history. Do not translate React/DOM. **High** for the complete board; small operations are **low–medium**. |
| `elkjs ^0.12.0` auto-layout | [openedges/elk-rs](https://github.com/openedges/elk-rs) ports ELK with native Rust modules, workspace 0.11.0, EPL-2.0. [depetrol/elkrs](https://github.com/depetrol/elkrs), 0.1.1, declares Apache-2.0 and ELK 0.11 parity goals; provenance and original obligations need review. Neither version statement proves compatibility with our 0.12 input/options. | Prefer a tested upstream port to rewriting graph layout. Compare our layered RIGHT/DOWN, radial and force graphs, locked nodes and rounding. **Medium** to adapt a compatible engine; rebuilding all ELK is **very high** and unnecessary. |
| `perfect-freehand ^1.2.3` | [sibaiper/perfect_freehand](https://github.com/sibaiper/perfect_freehand), Rust 0.1.1, MIT, is an actual pressure-points → polygon algorithm port. It provides no native pointer capture, paint, selection or undo. | Strong narrow reuse candidate; compare exact options and output geometry against original stroke fixtures. Port our sampling/simplification/eraser logic separately. **Low–medium**. |
| `mermaid ^11.12.0` | [Merman](https://github.com/Latias94/merman), main 0.10.0-alpha.2, MIT OR Apache-2.0, implements native parsing/layout/rendering to SVG; large alpha workspace. [mermaid-rs-renderer](https://github.com/1jehuang/mermaid-rs-renderer), MIT, is another native renderer with an explicit early-development caveat. | Preserve Mermaid source and captions; test only the diagram families/syntax actually used. Native SVG paint and source-error UX are integration work. **Medium–high**; full Mermaid compatibility is **high**. |
| `katex ^0.18.4` | [katex-rs](https://github.com/katex-rs/katex-rs), workspace 0.3.0, MIT, is a Rust implementation but emits HTML/MathML. [RaTeX](https://github.com/erweixin/ratex) supplies native layout/display lists and SVG. These outputs have different integration needs. | Preserve original LaTeX/description and safety/expansion limits. Test representative formulas and accessibility. **Medium–high**. A native parser emitting HTML does not by itself create a native math widget. |
| `idb ^8.0.3` / IndexedDB recovery | [idb](https://github.com/jakearchibald/idb) wraps the browser's IndexedDB API. Rust-to-Wasm IndexedDB wrappers would still need a browser environment. Our recovery records include editor/Canvas snapshots and original audio chunks. | Port durability, retry, acknowledgement and recovery semantics to SQLite/files, not IndexedDB's browser API. Plain draft recovery **medium**; complete recordings/assets/reconciliation **high**. |
| Our assistant selection / notes / dictionary actions | `src/widgets/manuscript-editor/ui/manuscript-editor.tsx` captures stable selected ranges after autosave flush and implements context-menu focus rules. The domain core validates revisions, expected text and protected ranges. | Port selection as a first-class domain object and preserve review/apply authority. Notes/dictionary links/NER spans share anchors; providers stay separate. **Medium–high** across the complete rich editor. |
| React, Radix, router/query and styling helpers | These organize a browser UI, DOM, network cache and CSS. They do not contain the canonical manuscript meaning. Native GUI components/commands implement equivalent workflows without reproducing their framework APIs. | Reimplement the user flows against native controls and our shared core. Do not create a Rust React/Radix clone for this product. **Medium** for simple dialogs; **high** for complete interaction parity. |

## Text selection is a subsystem

Current native TextEdit selection is not the full selection experience of Writers Studio. The original app has several linked contracts:

1. Native/browser editing selects the actual occurrence, with direction and a current caret.
2. Opening a context menu preserves that selection rather than moving the caret on right click.
3. `ContextMenu` / Shift+F10 opens the same actions; menu focus, Escape and return-to-editor work from the keyboard.
4. A dictionary link or note has an exact same-block anchor; a cross-block selection is refused for those actions.
5. Attaching text to the assistant can contain multiple supported block ranges. Autosave flush establishes the referenced canonical revision.
6. Later edits map or detach annotations; they do not silently associate the note with another identical phrase.
7. Selection and temporary decorations do not mutate canonical rich JSON. Assistant scope is an input constraint; attaching text does not authorize Apply.

Source pointers: `src/widgets/manuscript-editor/ui/manuscript-editor.tsx` (`attachSelection`, `startAnnotation`, context menu handlers and `stableSelectionRanges`); `src/widgets/manuscript-editor/model/selection-context-menu.ts`; `src/features/editor/model/annotations.ts`; `src/features/editor/model/annotations.test.ts`; `src/features/editor/model/extensions.test.ts`. These are source-workspace references, not files already ported into this repository.

Freeze a `SelectionContext` before a popup/inspector changes focus: document ID, base revision, block identity, anchor/head, selected exact text and context. It must remain attached to its original occurrence. Re-running a text search when the user clicks an action is insufficient: repeated words/sentences are valid selections. The current unique-substring proposal API is useful when no precise selection exists; selected-range proposals need explicit position/anchor validation.

A future rich adapter should return a list of stable per-block ranges, not a single global plain-text range. An initial prose-only editor can use global scalar ranges internally, with an explicit boundary so this limitation does not leak into the public rich-document contract.

### Three coordinate systems must remain distinct

| Coordinate space | Meaning | Conversion boundary |
| --- | --- | --- |
| ProseMirror positions | Structural positions including node boundaries; text uses UTF-16 units, including in prosemirror-rs | Decode through document structure and each text leaf, not a global numeric cast |
| Native text layout | UTF-8 byte offsets or engine-specific clusters, caret affinity, visual lines and bidi movement | Use the selected layout engine's boundary/hit-test API |
| Persisted Writers Studio anchors | Block-relative Unicode scalar offsets plus exact text/context | Use explicit checked conversions and validate against the referenced revision |

For `A🪶Б`, there are three Unicode scalars, four UTF-16 units and seven UTF-8 bytes. Combining marks and ZWJ emoji add another distinction: user-perceived grapheme clusters are not identical to scalars. Persisted offsets can stay scalar-based while native caret movement follows grapheme/word behavior. Never change an archive's offset semantics silently to match a library.

[Parley](https://github.com/linebender/parley) offers shaping/layout and editing/selection utilities. [COSMIC Text](https://github.com/pop-os/cosmic-text) has bidi selection and [click/double-click/triple-click/drag actions](https://docs.rs/cosmic-text/latest/cosmic_text/enum.Action.html). They are native text substrates, not ports of ProseMirror's schema, transforms or our annotations. IME composition, clipboard formats, accessible trees and host window integration still need acceptance tests.

Manual annotations, dictionary links and future NER detections should use the same validated anchor model. A recognition model may report byte/token offsets; adapt those explicitly and store source/model provenance. A confident entity label cannot compensate for an anchor pointing to the wrong occurrence. NER suggestions should be reviewable and should not overwrite the manuscript or dictionary without the corresponding explicit action.

## What can be ported directly

Our pure transformations are the cheapest and safest starting points: Canvas operations and geometry, graph analysis, stroke sampling/simplification, layout request/response adapters, selection menu bounds, anchor construction/mapping rules, rich-schema validation, source provenance, proposal fingerprints and stale/undo behavior. Translate their fixtures into shared JSON inputs/expected outputs and run both implementations as an oracle comparison. Test artifacts may use the original TypeScript runtime during development; the distributed native app need not contain it.

Browser-specific mechanics need a native implementation: DOM selection, `contenteditable`, DOM geometry, React node views, drag events, browser clipboard, MediaRecorder/AudioWorklet, IndexedDB and browser layout. Mechanically translating those APIs is effectively writing a browser/editor subsystem and is not justified merely by code reuse.

A temporary JavaScript interpreter or Node sidecar can aid an experiment. It is a different runtime architecture from the chosen pure Rust native product. In particular, the similarly named [`katex` crate](https://docs.rs/katex/latest/katex/) embeds QuickJS by default, unlike `katex-rs`. Do not describe a Rust wrapper around JS as an engine port.

## Boundaries for larger reusable work

Start with small internal crates and tested public boundaries: document model/positions; native editor adapter; Canvas model/operations; layout engine adapter; diagram/math rendering; persistence/assets; assistant proposal commands. Keep the app's own schema and user workflows under WriterCraft's control.

If an engine becomes a substantial independent project, maintain it in a separate repository with its own tests, license/provenance, versioning and release cadence; consume a pinned release here. Do not vendor an entire application or all of ELK into the main product just to obtain one algorithm. Splitting prematurely into many repositories creates coordination cost, so make that decision when the engine actually has independent scope.

## Licensing and evaluation gates

Original Tiptap core, React Flow, perfect-freehand, Mermaid and KaTeX currently identify MIT licenses in the checked source packages. Original `elkjs` identifies `EPL-2.0 OR GPL-3.0-or-later`; upstream ELK-derived ports need source/provenance review. `idb` identifies ISC. Verify the selected immutable revision's LICENSE/NOTICE and bundled asset licenses; neither a port's package name nor an AI-generated rewrite removes original notices or applicable obligations. These are source facts and engineering requirements, not a legal compatibility determination.

Before choosing a candidate, pin its revision and verify:

- Rich JSON round trips, unknown attributes, marks, formulas, media refs, block IDs and source refs.
- Selected second occurrence of repeated text; forward/backward and cross-block selection; Cyrillic, emoji, combining marks and ZWJ; menu focus and keyboard actions.
- Position conversions, insert/delete/split/merge mapping, detached annotations, protected ranges and stale revisions.
- Clipboard, IME and accessibility on actual desktop systems; long-manuscript performance and recovery.
- Existing Canvas modes, locked nodes, drawing options, graph/history operations and representative Mermaid/math outputs.

The resulting choice may combine an existing Rust semantic port with our own behavior and native presentation. That is a practical route to all product features without translating every TypeScript library, and without treating a simple native replacement as established parity.
