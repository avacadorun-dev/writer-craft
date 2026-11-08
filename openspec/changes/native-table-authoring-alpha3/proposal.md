# Complete native table authoring for alpha.3

## Why

The source editor exposes table creation, row/column changes, deletion, pointer resizing, rectangular selection and clipboard editing. Alpha.2 retains table structure but does not mount the engine's complete cell controller or apply stored widths to native layout. This change advances issues #4 and #7; it does not independently complete full editor parity.

## What changes

- Mount revision-bound directional cell selection, the 63-column runtime guard and sealed source-dialog bookmarks.
- Preserve source table selection semantics for row/column changes, table deletion, native Copy/Cut/Paste and ordinary formatting.
- Apply canonical widths/alignment to measured native geometry and support one reversible pointer-resize operation.

## Compatibility and scope

No storage migration or new dependency is required. Generic selection/layout mechanics remain in CraftEditor. Canonical unknown metadata, IDs, spans and source references survive all retained content. Wider tables remain preserved/read only. Alpha.1 and alpha.2 are immutable. Actual native, clipboard and independent UX evidence remains required before issue closure.
