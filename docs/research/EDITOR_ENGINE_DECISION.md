# Native editor engine decision

Date: 2026-11-08. Scope: the owner's complete editor-first alpha.2; no reduced subset qualifies. This is source research and implementation evidence, not a completed native editor claim.

## Result

A reusable native view/model component is necessary. The inspected libraries do not provide a verified drop-in replacement for the complete current Tiptap editor with continuous native selection, custom atoms/tables and lossless source metadata. This is an engineering conclusion from the inspected APIs, not a proof that no such library can exist.

The separate [CraftEditor](https://github.com/avacadorun-dev/craft-editor) workspace owns generic model/view mechanics. WriterCraft owns its schema/media/annotation adapters, persistence and command authorization. The first model source pin is `7084f99ce45199d81ee8edce6ee9b9388da07b7b`; later tested changes require a deliberate pin update.

## Reuse assessment

| Primary source | Useful existing component | Boundary and decision |
| --- | --- | --- |
| [prosemirror-rs](https://github.com/fiduswriter/prosemirror-rs) and the pinned existing compatibility spike | Typed schema validation, transform/position-map algorithms | Reuse behind a lossless authoritative JSON adapter. It does not supply a native rich view; its serializer cannot silently drop unknown top-level metadata. |
| [WordCraft canvas](https://github.com/storytold/wordcraft/blob/main/crates/ui-egui/src/canvas.rs), [input](https://github.com/storytold/wordcraft/blob/main/crates/ui-egui/src/keys/canvas.rs), [paragraph model](https://github.com/storytold/wordcraft/blob/main/crates/doc/src/para.rs) | Existing document hit testing, input, table/object layout and accessibility reference | Canvas consumes WordApp/Session, not a neutral PM-tree view. Model lacks a general original blockId/attrs channel, UI imports application modules, and the current egui version differs. Do not embed the whole word processor or treat a one-way model conversion as lossless. Any extracted code needs pinned provenance/notices. No WordCraft source is copied in this checkpoint. |
| [Quiet.md workspace](https://github.com/s1dashu/quiet-md/blob/main/Cargo.toml) | Real rich document core and GPUI implementation | Different UI/model boundary and vendored framework; not a smaller egui adapter. Its asset/optional-platform terms must be evaluated independently from source license. |
| [Cditor workspace](https://github.com/JYChen-8866/Cditor/blob/main/Cargo.toml) | Separate editor core/session/viewport architecture | GPUI, another canonical model and GPL terms; not adopted as a transparent MIT component. |
| [Parley](https://github.com/linebender/parley), [COSMIC Text](https://github.com/pop-os/cosmic-text) | Text shaping/layout/editing primitives | Useful future native layout ingredients; not a complete structured editor with the source's block/asset/annotation workflows. |

Viewer/Markdown/DOM-backed alternatives do not establish the required independent native editing experience. The owner's continuous composition surface remains; publication pagination is not made the canonical document model.

## Verified implementation checkpoint

The model has 19 local semantic tests plus strict Clippy/formatting, and [all three hosted source jobs passed](https://github.com/avacadorun-dev/craft-editor/actions/runs/42409101506). That pin covers structured JSON preservation, explicit nulls/unknown fields, typed positions, exact text/marks, top-level insertion, guarded split/join and revisioned Undo/Redo. It is not every structural command or a native view.

WriterCraft's new structured persistence has 13 semantic tests in addition to its 28 existing core regressions. It stores full rich snapshots/drafts, blocks legacy flattening writes, preserves formatting-only revisions and validates exact command transactions. A content-free draft compare-and-swap token prevents stale same-base drafts overwriting newer work; migration identity/version is checked under the migration write lock. Tests do not establish full assets/annotations or native UX.

## Still required

Complete structural/list/table/atom operations, rendering, a genuine nested/table native layout, input/IME/rich clipboard/accessibility, media/provenance/note integration and all source/UX matrix cases. A linear cell traversal is only a projection checkpoint, never an accepted WYSIWYG table editor. No alpha.2 release may pass from model/storage tests alone. See the [source matrix](../migration/EDITOR_PARITY_MATRIX.md), [UX contract](../design/EDITOR_UX_PARITY.md) and [full change](../../openspec/changes/full-editor-parity/tasks.md).
