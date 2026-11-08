## ADDED Requirements

### Requirement: Revision-bound native table editing

The native editor SHALL expose every source table command through the one rich session, including rectangular selection, formatting, row/column changes, table deletion, clipboard and width changes. Source dialogs SHALL retain the exact cell/text/node selection and reject stale or foreign session bookmarks before mutation. Every operation SHALL preserve retained rich content, attributes, provenance and one coherent Undo/Redo entry.

#### Scenario: A selected cell rectangle is formatted and undone
- **WHEN** an author selects cells in reverse direction and applies a supported mark or heading style
- **THEN** every selected eligible block changes in one transaction, unselected content remains exact, and Undo restores the original tree and directional selection

#### Scenario: A source dialog loses its original target
- **WHEN** the dialog belongs to a different native session or its revision is stale
- **THEN** confirmation returns an actionable error and preserves canonical data, history and current selection

#### Scenario: A table column is resized
- **WHEN** an author drags a measured column boundary and releases it
- **THEN** native tracks and persisted widths change coherently once, including cells spanning that column, and Undo/save/reopen preserve the corresponding exact tree

#### Scenario: The author deletes the table explicitly
- **WHEN** the Delete table command is invoked from a text, atom or cell selection inside that table
- **THEN** the table is removed atomically, the surrounding document remains valid and a nearby insertion target survives with reversible history

#### Scenario: Row or column deletion would remove the whole table
- **WHEN** the selected row or column range covers its complete table extent
- **THEN** that row or column command refuses mutation and the separate explicit Delete table action remains reachable

#### Scenario: Native table capacity would be exceeded
- **WHEN** any insertion, widening, paste or other guarded command would exceed 63 logical columns
- **THEN** it fails before mutation without truncating preserved wider canonical data
