# Design

Use one EditorSession selection form (text, node or directional cells), one history and one engine transaction. A sealed SelectionBookmark also captures session identity, revision and stored typing marks; dialogs refuse foreign or stale bookmarks before changing content. The host configures a 63-logical-column edit limit without narrowing canonical parsing.

Generic table geometry uses TableGrid widths and canonical source cell paths. Native resize handles bind to a revision and originating logical column, derive widths from measured cell boxes and commit once on release. Cancellation, stale revision, composition and read-only state leave content/history unchanged. Widths map to upstream grid tracks; cell alignment is a presentation interpretation of stored attributes, not a manuscript rewrite.

Row/column actions derive complete selected ranges, stage atomic commands and retain a valid post-selection. Deleting every row or column is refused, matching the original separate explicit Delete table command. Explicit table deletion preserves a valid parent and nearby insertion target. Clipboard publication uses the existing deferred intent protocol, so Cut never deletes before the successful clipboard acknowledgement.

See the existing lossless table/clipboard decisions in CraftEditor and docs/migration/EDITOR_PARITY_MATRIX.md E06/E12/E16/EP-19. Tests complement, rather than replace, real native and independent UX qualification.

## Bounded wide-table components

A horizontal viewport is session-local presentation, never a canonical edit or selection authority. CraftEditor derives one path-aware projection over immutable measured table tracks. Rendering, inverse pointer hits, text/cell/node bounds, selection/decorations, caret/IME, visible-text scheduling and resize edges share that projection. Nested clips compose; a table does not widen ordinary prose or scale/omit cells. Wheel input chooses the innermost visible component and ordinary vertical scrolling stays in the manuscript. Caret movement reveals the exact semantic position, and toolbar/source dialogs retain their captured semantic target while the view scrolls.

The scrollbar has a proportional rectangular thumb, keyboard/accessibility range semantics and a complete host-localized label. Its gutter is native component extent through disposable standard table margins, never an authored spacer. The minimal maintained WordCraft margin repair and regression qualification are recorded in CraftEditor ADR 0008 at `specs/decisions/0008-bounded-table-viewports-and-backend-margins.md`. Keep all WordCraft packages/font globals on the same exact source identity; qualify the generic source before app repinning. Neither an engine regression nor native event harness substitutes for compiled app and independent UX acceptance.

## Distinct whole-table selection

Retain the existing revision/session-guarded NodeRef form for whole-table selection instead of introducing a second table authority or converting it to a rectangular CellSelection. Derive selected-table frame geometry separately from opaque-atom pointer hit geometry, so adding a table outline cannot make ordinary text clicks select the whole table. Native entry and arrow transitions must keep nested scope and one semantic focus target explicit. Clipboard and Delete reuse acknowledged publication and canonical atomic transactions; Undo restores the exact selected table and source attributes. Source-backed regression and native/independent acceptance are required after the currently qualifying overflow integration.
