# Native MVP design

## Context

See proposal.md for motivation, docs/PRD.md for scope, and docs/architecture/DECISIONS.md for accepted choices. Existing Writers Studio combines browser editing with Go/PostgreSQL/MinIO; its manuscript schema and transfer histories exceed the initial native plain-text editor.

## Goals / Non-Goals

Goals: a useful native writing workspace; durable saved and unfinished text; shared safe mutation rules; honest archive inspection; theme parity requested for the first interface.

Non-goals for this increment: rich editing equivalence, complete binary import, on-device transcription, provider orchestration, Canvas and production-ready three-platform distribution. They remain product milestones with separate acceptance.

## Decisions

- First full desktop release targets macOS, with portable core/transfer CI and later Windows/Linux UI/installer acceptance.
- Manuscript composition uses continuous text in a bounded editor pane; publication preview/export owns pagination.
- Native egui/eframe with a separate SQLite command core; avoid duplicating save/apply logic in UI adapters.
- Immutable text revisions and independent revision-pinned working drafts. Clear recovery text only when it matches the exact successful save.
- Store exact proposal snapshots, hashes and Unicode-scalar anchors. Apply validates the base; Undo validates the applied result and appends history.
- Retain imported JSON and keep it read-only. Archive inspection reads bounded declared entries and verifies integrity without extraction or canonical mutation.
- Resolve interface strings from eight built-in catalogs; keep language and semantic message IDs in preferences/state so existing statuses rerender on switching. Preserve author text and separately labelled raw diagnostics. Verify CJK font coverage in the packaged build.
- Keep theme brightness separate from palette identity; use one semantic token set across editor, panes and dialogs.
- Adopt OpenSpec's default artifact structure manually first; official CLI integration and validation are recorded separately when executed.

## Risks / Trade-offs

- Basic native text editing does not provide the existing rich engine → preserve unsupported source and retain full parity inventory.
- A SQLite busy timeout can stall a render thread → move substantial storage/provider/import work to bounded workers and measure real native behavior.
- Rich imports may contain more than text → label previews and preserve archives until a complete atomic importer exists.
- Appearance preferences can drift across panels → centralize mode/palette tokens and verify every mode/palette pair visually.
- Restart tests do not establish all power-loss guarantees → keep crash/power-loss testing a separate reliability task.

## Migration Plan

Keep source projects untouched. Create a new local workspace, inspect source archives, and allow independent new native documents. Add schema upgrades transactionally with old-record preservation tests. Future full import must stage assets, verify references, commit atomically and retain the original archive. Rollback uses a compatible backup or supported old workspace; never edit schema version metadata to force a downgrade.

## Open Questions

Deferrable experiments: preferred manuscript font metrics; practical long-document latency; rich text engine fit for IME/accessibility; model setup usability. Changes to the chosen MVP contract require a new spec delta rather than silently treating an experiment as accepted behavior.
