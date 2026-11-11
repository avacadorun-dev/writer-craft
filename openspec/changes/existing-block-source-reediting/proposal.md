# Direct re-editing of existing rendered blocks (#7)

## Why

The implemented native editor can create Mermaid diagrams, block formulas, inline formulas, code blocks and images, and it can preserve their canonical source through the shared rich session. Re-opening that source is only reachable through **More formatting → Edit block**, a distant overflow entry. On the compiled preview the owner double-clicked an existing Mermaid diagram and formula and nothing opened (F-46), so insertion or model coverage does not provide a discoverable editing workflow. Issue #7 was reopened for that correction.

## What changes

- Make a double-click on an already-rendered Mermaid diagram, block formula, inline formula, code block or image open that exact object's existing source/properties dialog. A single click keeps the ordinary text or whole-object selection and opens nothing.
- Expose the same dialog from the manuscript object context menu (right-click and the existing Shift+F10 keyboard menu) and from a direct F2 shortcut while the manuscript owns focus and an editable object is selected.
- Reuse the existing revision-guarded `BlockDialog` and its Apply/Cancel/Escape/stale/Undo paths; do not add a second editing mechanism.
- Keep the permanent toolbar unchanged in height and command count. The overflow **Edit block** entry remains as a fallback.

## Impact

- Affected capability: `rich-manuscript-editor`; related canonical contract ME-04 and issue #7.
- No storage, transfer-version or editor-schema change. Existing documents, unknown block attributes, stable block IDs, provenance and source references are untouched.
- No dependency or engine-revision change: the pinned CraftEditor already selects an atom on primary click and exposes the measured selected-node rectangle; the gesture routing belongs to the desktop adapter.
- Native double-click/right-click/F2 evidence and independent review remain required before #7 can close; source and hosted tests are not interaction acceptance.
