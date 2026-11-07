# Rich document and persistent anchor integration spike

Date: 2026-11-07. This is an architectural assessment backed by a small executable model spike. It does **not** claim a native rich editor has been implemented. No production WriterCraft dependency was added, no original Writers Studio file was changed, and no WordCraft application was forked.

## Result and decision

The native rich-document path is feasible. The strongest model candidate for preserving existing Writers Studio semantics is [FidusWriter prosemirror-rs](https://github.com/fiduswriter/prosemirror-rs), because Writers Studio already stores a ProseMirror/Tiptap-shaped tree. It can save implementing the model and step-mapping algorithms again. It does not supply a finished native editing surface.

Use it first behind a narrow adapter and qualify it against the complete real schema and corpus. Keep the original rich JSON opaque/read-only until that adapter proves round-trip and editing fidelity. Keep WordCraft as a separate candidate for a paginated writing/printing engine, rather than adopting its incompatible document model merely to obtain a renderer.

Do not establish several speculative external repositories now. Establish a tested library boundary first; split a repository once that boundary has an independent consumer, its own versioning and CI, and a maintenance owner. If an external library is needed immediately, generic anchor/mapping behavior is the strongest small extraction candidate. A “large universal editor library” created before its renderer/model contract is validated would turn product uncertainty into an additional platform project.

## Tested dependency and executable evidence

Disposable harness: `tests/editor-compatibility`. Exact dependency `prosemirror = "=0.5.2"`, MIT. The published package's `.cargo_vcs_info.json` records commit `576a8cd8e76251f32737718b5540d7d5b5837dcc`. The harness adds only this model library and `serde_json`; `Cargo.lock` fixes the resolved graph. No JavaScript, browser, Node addon, WASM binding, native GPU or WordCraft dependency is needed for these tests.

Primary source boundaries:

- [Package manifest](https://github.com/fiduswriter/prosemirror-rs/blob/576a8cd8e76251f32737718b5540d7d5b5837dcc/Cargo.toml): focused model/transform package and Rust serialization dependencies.
- [Dynamic model/schema](https://github.com/fiduswriter/prosemirror-rs/blob/576a8cd8e76251f32737718b5540d7d5b5837dcc/src/dynamic/schema.rs): runtime JSON schema and node construction.
- [Position maps](https://github.com/fiduswriter/prosemirror-rs/blob/576a8cd8e76251f32737718b5540d7d5b5837dcc/src/transform/map.rs): change maps, endpoint bias and deletion metadata.
- [Stateful editor helper](https://github.com/fiduswriter/prosemirror-rs/blob/576a8cd8e76251f32737718b5540d7d5b5837dcc/src/editor.rs): schema caching, JSON step application, atomic batches, version and serialization. This is not the complete native input/state/plugin/view layer.
- [Dynamic node serialization](https://github.com/fiduswriter/prosemirror-rs/blob/576a8cd8e76251f32737718b5540d7d5b5837dcc/src/dynamic/types.rs): attributes, UTF-16 node sizes and parsing behavior.

Actual result: **12 tests passed**. `tests/editor-compatibility/src/lib.rs` contains the reproducible tests; CI executes this isolated test-only workspace. Covered behaviors:

| Test | What it establishes |
| --- | --- |
| JSON round trip | A custom schema preserves heading/paragraph IDs, `sourceRefs`, nested declared vendor attributes, marks, Cyrillic, emoji and combining text in the fixture |
| Node sizes | `А😀Б` occupies four ProseMirror UTF-16 units, three Unicode scalars and eight UTF-8 bytes |
| Scalar/UTF-16 conversion | Every scalar boundary round-trips through UTF-16 in Cyrillic/emoji/combining/ZWJ text; a surrogate-pair interior and out-of-range scalar are rejected |
| Actual insertion | A real Replace step retains provenance attributes and maps the original emoji quote after text is inserted before it |
| Actual deletion | Removing the whole selected emoji produces deletion metadata and a collapsed mapped range |
| Actual inverse | The inverse of an insertion restores the fixture's text, marks, IDs and source references |
| Atomic batch | A valid edit followed by an invalid range leaves the document and version unchanged |
| Mapping limitation | Editing inside a quote can leave both mapped endpoints valid while the exact quote changes; endpoint maps alone cannot prove semantic attachment |
| Parser limitation | `node_from_json` alone accepts a node with an unknown type in the fixture; parsing success is not schema validation |
| Preservation limitation | Unknown top-level node fields disappear on typed serialization; arbitrary external metadata must remain in an opaque original or an explicit extension channel |

This is a deliberately narrow fixture. It does not establish support for all four legacy document schema versions, tables, images, formulas, Mermaid, callouts, source-reference merge/split rules, large manuscripts, IME, clipboard, accessibility, rendering or the complete import/export contract. Run `Node::check` and application-level prevalidation; reject unsupported names/attributes before making a document editable. A serializer preserving a known fixture does not justify rewriting unknown imported content.

Reproduce with the pinned manifest and `CARGO_TARGET_DIR=target cargo test --manifest-path tests/editor-compatibility/Cargo.toml --locked`. The checked environment used Rust 1.99.1 on macOS arm64. Compile time is not a runtime performance benchmark.

## Position model: keep three types distinct

Existing Writers Studio anchors are block-scoped Unicode-scalar ranges with exact text and surrounding context. Tiptap/ProseMirror positions are UTF-16 offsets in a tree, including structural node boundaries. WordCraft instead uses a story, path and byte offset. None of these units is interchangeable, even if an ASCII test happens to pass.

Use explicit wrappers such as `ScalarOffset`, `Utf16Offset`, `Utf8Offset` and `DocumentPos`. Conversion must use the exact document revision and text projection. A conversion cannot turn a paragraph-local count directly into a global tree position without accounting for the tree path and structural tokens. A grapheme may contain several scalars; visual caret/Backspace handling is a separate native input concern. Never normalize Unicode in canonical text merely to make offsets convenient.

The original integration already contains valuable behavior to port or adapt:

- `src/features/editor/model/annotations.ts:125–198`: selection → block ID/scalar range/exact quote/prefix/suffix and conversion back to ProseMirror positions.
- `src/features/editor/model/extensions.ts:95–198`: document schema version, block IDs, source references, split policy, duplicate-ID repair and merge provenance.
- `internal/documentannotation/annotation.go:23–78`: deterministic exact-edit remapping, detaching partial overlaps rather than inventing a new meaning.
- `internal/store/assistant_annotations.go:31–105`: canonical edit, annotation remapping, status and audit history are coordinated transactionally.

## Selections, notes, dictionary mentions and optional NER

An initial user selection is a snapshot, not a permanent screen coordinate. Capture document/block identity, base revision, scalar range, exact quote and surrounding context. Persist the note body and anchor together. Derive decoration geometry from the current document/layout; do not persist pixel locations.

For every committed edit:

1. Map endpoints through the actual editing steps, using an explicit policy for insertion at each boundary.
2. Resolve stable block identity after joins, splits, moves and deletions. Preserve source-reference semantics independently of text formatting.
3. Verify the exact quote at the proposed new location. If the edit modifies the quote, make the corresponding state change explicit. A surviving endpoint is not proof that the note still refers to the same phrase.
4. When a quote disappears or there are multiple plausible context matches, mark the anchor detached/needs attention and retain the original quote. Never silently attach it to the first repeated name elsewhere.
5. Commit document revision, anchor state, annotation revision and operation history atomically. Undo uses recorded before/after data and revision guards, preserving later manual edits.

NER is a producer of candidate spans/entities, not the anchor engine. A local model or remote provider may report bytes, UTF-16 units or scalars; its adapter declares and validates the unit. Normalize to typed offsets against the requested revision, verify the returned quote, and show a candidate before creating or linking dictionary entities. Entity merging, aliases, project dictionary meanings and review approval belong to WriterCraft business logic. The generic anchor layer should not know what a fictional character or world-building term is.

Selection-based AI gets an explicit scope and a revisioned quote. It produces a proposal against that scope. When the user edits while the request runs, the proposal becomes stale or is explicitly rebased and reviewed. The same core transaction applies text and anchors. AI may suggest a reattachment; it must not claim a guess is a verified persistent anchor.

## Comparison with WordCraft

The [WordCraft document model](https://github.com/storytold/wordcraft/blob/main/crates/doc/src/lib.rs) is comparatively focused as a crate: [its manifest](https://github.com/storytold/wordcraft/blob/main/crates/doc/Cargo.toml) depends on geometry, serde, base64, logging, errors and Unicode segmentation. It has paragraphs/runs/tables, stories, comments, tracked revisions, layout properties and media. It is more than a plain-text widget, and the model alone does not require importing the whole application.

Its JSON schema is a `Document { body, parts, styles, numbering, comments, revisions, ... }`, not a Tiptap node tree. It supports passthrough source-format parts, but this does not automatically preserve arbitrary Tiptap block attributes and semantics during editing. A mapping adapter plus opaque preservation remains necessary. Its positions are byte-based, as documented in the [control protocol](https://github.com/storytold/wordcraft/blob/main/docs/control-protocol.md).

The [WordCraft engine manifest](https://github.com/storytold/wordcraft/blob/main/crates/engine/Cargo.toml) brings document, fonts, layout, render, proofing, DOCX/docbin/PDF/formats and platform font-store components. Its complete native text experience includes custom canvas/input/clipboard/accessibility integration. Importing `wordcraft-doc` does not import that experience. Importing the entire engine is a substantially different commitment from reusing a data model.

Decision: evaluate a bounded WordCraft adapter if pagination/Word-compatible layout becomes a primary requirement. For preserving the existing project semantics first, a ProseMirror-shaped model is the lower-conversion-risk option. This is an engineering inference from the schemas and the executed spike, not a benchmark of either complete editor. WordCraft's model was source-inspected, not compiled or qualified in this spike.

## Cost and reuse boundaries

| Approach | What we avoid rewriting | Work that remains |
| --- | --- | --- |
| Mechanical TS→Rust port | Some algorithm decisions and test fixtures | DOM/React/Tiptap plugins cannot simply be compiled as a native view; selection, layout, input, clipboard and a11y must be redesigned |
| ProseMirror Rust model adapter | Tree/schema/steps/mapping foundation; close JSON shape | Full real schema and plugin semantics, native rendering/input, block ID policy, annotations, history boundaries, exports and compatibility tests |
| WordCraft model only | Mature document primitives and Office-oriented semantics | New canonical adapter, attributes/provenance mapping, positions, renderer integration; the model alone provides no editor UI |
| Broad WordCraft engine integration | More editing/layout/format functionality | Large dependency/platform boundary, data-model conversion, project workflows, upgrade management and whole-corpus acceptance |
| New document engine from scratch | No upstream model constraints | Highest maintenance cost; transforms, maps, serialization and semantic edge cases become our permanent responsibility |

A small model proof is measured in days; a robust native rich editor with full legacy parity is a multi-month product stream. Generating Rust syntax with an agent does not remove native IME, clipboard, rendering, source-provenance and migration qualification. Give calendar estimates only after a vertical slice that edits a representative imported document, preserves anchors through split/join/undo and survives save/reopen.

Candidate reusable boundaries, kept initially as isolated crates:

- **Document adapter/core**: validated canonical tree, typed positions, steps, versioned serialization, opaque unsupported content. Reuse a focused model rather than creating a generic framework without a consumer.
- **Anchored text ranges**: conversions, exact quote/context, change-map application, detached/ambiguous outcomes and deterministic tests. Keep persistence and note UI outside it. This is the best prospective external library once two concrete consumers exercise the API.
- **Native rich view**: renderer/input/selection bridge for the selected model. Keep the UI dependency out of the model crate; qualify native integration separately.
- **WriterCraft business core**: projects, dictionary/entity policy, assistant review/approval, project-scoped permissions, provenance meaning, publishing profiles and database transactions. These remain application-owned.

Do not start separate general-purpose NER, agent orchestration, rich publishing and widget frameworks simply because they could be reusable. First consume and measure a real upstream component. A separate repository must earn its release/compatibility burden with a real contract and consumer.

## Next decisive slice

Preserve the actual Writers Studio schema unchanged where possible; make one imported paragraph containing marks, Cyrillic/emoji, source references and a note editable. Perform typing, replace, split/join, undo/redo and save/reopen through one command path. Verify source refs, anchor outcomes and unknown content preservation. Only after this passes add tables/images/formulas and choose whether the native view is an independent reusable component. Keep unsupported imported documents read-only until their acceptance passes.
