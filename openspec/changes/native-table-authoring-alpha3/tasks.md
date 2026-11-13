# Native table authoring tasks

- [x] Audit original toolbar/TableKit and qualified engine91607a3 plus held consumer integration patches.
- [x] Mount full cell-selection/controller and sealed source-dialog bookmarks with host tests.
- [x] Complete selected-range row/column changes and whole-table deletion with reversible targets.
- [x] Implement generic canonical-width/alignment layout and native pointer resizing in CraftEditor.
- [x] Qualify width/alignment/resize source26e71d1 on hosted semantic/layout/native clipboard checks before its consumer pin.
- [ ] Qualify bounded wide/nested table viewport source and minimal maintained backend margins before consumer repinning.
- [ ] Mount host-localized table controls and verify consumer default/all-feature checks against the exact qualified viewport pin.
- [ ] Implement source-backed whole-table NodeSelection distinct from cell rectangles, including native outline/entry, clipboard, deletion/Undo and block arrow navigation.
- [ ] Verify real compiled native pointer/keyboard/Copy/Cut/Paste/format/property/Undo/save/reopen workflows.
- [ ] Record exact source, independent UX disposition and remaining full-parity gaps in #4/#7.

## Evidence boundary

Source26e71d1 qualification: [CraftEditor42500876681](https://github.com/avacadorun-dev/craft-editor/actions/runs/42500876681). The app's fixed selected-range/history/source-oracle cases passed default and all-feature tests in [WriterCraft42500967937](https://github.com/avacadorun-dev/writer-craft/actions/runs/42500967937); that run's unrelated import Clippy failure does not establish an entirely successful source candidate. Bounded local overflow is a separate current qualification, not included in that pin or the running development preview. Product issues #4/#7 remain open until their complete compiled/independent acceptance.

## Whole-table selection audit

The original `src/features/editor/model/extensions.ts` configures TableKit with `allowTableNodeSelection: true`. Installed Tiptap `table/table.ts` passes it directly to ProseMirror tableEditing; `prosemirror-tables1.8.5` keeps a table NodeSelection when this option is true while normalizing selected cells/rows to distinct cell selections. Installed `prosemirror-view/src/input.ts` carries platform modifier-node selection and `capturekeys.ts` carries block selection navigation. The current native `EditorSession::select_node` permits only opaque atom kinds and rejects `table`; its native opaque-node geometry likewise does not provide table selection bounds. This is an explicit outstanding #4/#7 acceptance gap, not covered by passing rectangular-selection/overflow tests or the sole approved63-column capacity exception.
