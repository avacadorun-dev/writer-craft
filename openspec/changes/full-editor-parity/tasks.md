# Full editor parity tasks

Status: implementation started; all unchecked tasks are required. This checklist is not acceptance evidence.

- [x] Owner selects editor-first alpha.2 with all current-editor features and no scope cuts.
- [x] Audit actual source node/mark/attribute, MCP and integration contracts; record baseline hashes.
- [x] Establish separate reusable CraftEditor repository and explicit domain/view boundary.
- [ ] Finalize complete independent UX matrix and original-bug decisions.
- [ ] Implement lossless canonical model and typed, atomic, revisioned command API.
- [ ] Verify all 21 nodes/six marks/schema gates with real-source synthetic fixtures.
- [x] Implement additive SQLite rich snapshots/history/drafts and plain-path overwrite guards (13 new semantic tests +28 existing core regressions; annotation/media integration remains below).
- [ ] Implement exact persistent annotations with atomic edit maps/resolve/detach/reattach/Undo.
- [ ] Prove one continuous native selection/input session including table boundary, IME and clipboard.
- [ ] Complete prose/marks/links/headings/lists/separators and all keymap/typing rules.
- [ ] Complete table editing, merged spans, keyboard/multi-cell selection and column sizing.
- [ ] Complete code/language/syntax/copy and input/paste conversions.
- [ ] Complete durable image upload/drop/location/metadata and media error paths.
- [ ] Complete Mermaid, callouts, inline/block formulas and safe rendering failures.
- [ ] Complete toolbar modes/overflow, typography/spellcheck, title/find/stats/focus.
- [ ] Complete real note/dictionary/sourceaudio/assistant editor handoffs and shared headless contracts.
- [ ] Complete stale/concurrent save, restart recovery and history/proposal integration.
- [ ] Run all source/headless/native/UX matrices and record exact evidence per row.
- [ ] Obtain independent exact-source `full-editor-parity` acceptance without unresolved gaps.
- [ ] Only then prepare/test/publish alpha.2 under the owner's release instruction.

Checkpoint evidence: `docs/research/EDITOR_ENGINE_DECISION.md`. Model/storage tests do not complete any remaining native interaction or full editor integration task.
