# Full native editor parity for alpha.2

## Why

The owner selected the editor as the prerequisite for WriterCraft's product hypothesis on 2026-11-08. A reduced rich-text MVP is explicitly insufficient: every capability in the current original editor must be covered before alpha.2 can be accepted. Future MCP must create and edit the same canonical blocks through the same validated engine as the UI.

## What changes

- Adopt the complete source-backed inventory in `docs/migration/EDITOR_PARITY_MATRIX.md` and the independent `docs/design/EDITOR_UX_PARITY.md` as required acceptance, not optional backlog.
- Build reusable document/transaction and native-view components in the separate [CraftEditor](https://github.com/avacadorun-dev/craft-editor) repository; keep project/storage/media authorization and assistant policy here.
- Preserve all 21 canonical node types, six marks, schema1–4 compatibility, metadata, source references and existing editing/integration semantics.
- Implement real continuous native input/selection, every block renderer/editor, rich clipboard, history, durable recovery, passage notes/dictionary links, editor find/preferences/statistics/source playback and assistant handoffs.
- Keep typed headless commands compatible with the original MCP create/append/insert/mark/exact-edit behavior, without requiring a new MCP network server for the engine tests.

## Capabilities

### Added capabilities

- `rich-manuscript-editor`: complete native structured-editor compatibility and verification gate.

### Modified capabilities

- `manuscript-editing`: richer canonical persistence, rich drafts/history, annotations and shared transaction integration; existing plain/opaque source preservation remains required.

## Scope and compatibility

Target `0.1.0-alpha.2` is a full-editor milestone, not full-product parity. Broad project tree/type management, named-version UI, project-wide search/replacement, installation packages, provider/recording workflows, Canvas, complete project transfer and publishing remain later work unless already required as a real editor integration point. Existing editor title/find/annotation/source/assistant actions cannot be hidden to avoid those integration dependencies.

Do not alter the original repository or owner manuscripts. Existing native databases need additive migration; imported opaque sources and historical revisions remain intact. An unsupported or ambiguous operation leaves canonical state unchanged and exposes a useful error. Original bugs are explicit migration decisions, not behavior to reproduce silently.

## Owner-approved table capacity

The owner subsequently accepted a native editing limit of 63 logical table columns for alpha.2 (F-30), avoiding a dependency fork solely to lift that capacity. Every table operation remains required within the limit. Wider canonical tables remain whole and read-only with an explicit capacity explanation; UI/MCP creation/widening/paste beyond the limit fails before mutation. Canonical schema span 1–100 and generic model compatibility remain unchanged. No other editor feature is removed.

## Acceptance

Every node/mark, E01–E27 behavior row and C01–C12 headless contract in the source inventory requires named implementation and actual verification. The independent reviewer must accept `full-editor-parity` for the exact alpha.2 source and compiled/native evidence. A model fixture, styled preview, one working paragraph or preserved unknown JSON cannot pass the release. No alpha.2 tag/publication until this complete gate passes.
