# Design

Use one EditorSession selection form (text, node or directional cells), one history and one engine transaction. A sealed SelectionBookmark also captures session identity, revision and stored typing marks; dialogs refuse foreign or stale bookmarks before changing content. The host configures a 63-logical-column edit limit without narrowing canonical parsing.

Generic table geometry uses TableGrid widths and canonical source cell paths. Native resize handles bind to a revision and originating logical column, derive widths from measured cell boxes and commit once on release. Cancellation, stale revision, composition and read-only state leave content/history unchanged. Widths map to upstream grid tracks; cell alignment is a presentation interpretation of stored attributes, not a manuscript rewrite.

Row/column actions derive complete selected ranges, stage atomic commands and retain a valid post-selection. Deleting every row or column is refused, matching the original separate explicit Delete table command. Explicit table deletion preserves a valid parent and nearby insertion target. Clipboard publication uses the existing deferred intent protocol, so Cut never deletes before the successful clipboard acknowledgement.

See the existing lossless table/clipboard decisions in CraftEditor and docs/migration/EDITOR_PARITY_MATRIX.md E06/E12/E16/EP-19. Tests complement, rather than replace, real native and independent UX qualification.
