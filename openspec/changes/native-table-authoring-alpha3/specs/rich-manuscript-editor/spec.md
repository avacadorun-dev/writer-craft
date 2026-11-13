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

### Requirement: Bounded wide-table authoring

A table wider than its writing column SHALL remain fully reachable through local horizontal scrolling while surrounding prose keeps its original bounded measure. The same exact path-aware projection SHALL govern native pixels, pointer hits, selections/decorations, caret/IME and resize handles. Scrolling SHALL NOT modify canonical content, revisions, history, annotations or clipboard data.

#### Scenario: The final logical column is edited after scrolling
- **WHEN** a table has 63 logical columns and the author scrolls or moves the caret to its final cell
- **THEN** the visible text, exact hit, caret/IME and resize edge refer to that cell, and Copy/Cut/Paste/Undo retain the same canonical selection semantics

#### Scenario: A wide table is nested inside another table
- **WHEN** either table is scrolled within a narrow writing panel
- **THEN** the independent table offsets compose with ancestor clips, hidden content cannot intercept a visible sibling's input, and neither table's controls overlap following content

#### Scenario: Typography or width changes with a visible table caret
- **WHEN** the native component is remeasured after a width, typography, media or document revision change
- **THEN** presentation offsets clamp to real tracks, the previously visible semantic caret remains reachable, and ordinary prose and persisted table widths remain unchanged by scrolling

### Requirement: Whole-table node selection

The native rich session SHALL support a guarded whole-table node selection distinct from text selection and rectangular cell selection, matching the original explicit `allowTableNodeSelection: true` configuration. Its native outline, semantic target, clipboard publication, deletion, reversible history and arrow navigation SHALL remain coherent for ordinary, wide and nested tables. A whole-table node selection SHALL NOT be silently converted into a cell rectangle.

#### Scenario: A complete table is selected and copied
- **WHEN** the author selects the table node through a native table-selection entry point
- **THEN** one bounded table outline identifies that exact node and Copy publishes the complete structured table with spans, widths, retained metadata and nested content without modifying the document

#### Scenario: A complete table is cut or deleted
- **WHEN** Cut publication succeeds or Delete is authorized for the selected table node
- **THEN** the exact table is removed atomically with a valid surrounding insertion target, and Undo restores its original rich tree and whole-table selection

#### Scenario: A table selection is left through arrow navigation
- **WHEN** an arrow moves before or after the selected table according to the source-compatible block navigation rule
- **THEN** a valid nearby semantic text/node target receives focus and is revealed through the same bounded viewport projection

#### Scenario: A captured selected table becomes stale
- **WHEN** a command or clipboard acknowledgement references an older revision, a different session or a changed table snapshot
- **THEN** it refuses mutation and preserves the current document, selection and history

### Capture-only wide table qualification fixture

The hosted native acceptance harness may select `WRITERCRAFT_CAPTURE_DOCUMENT=wide-tables` only with literal `--demo` and `--capture` flags and an isolated startup workspace under `RUNNER_TEMP`. It must reject a simultaneous nonempty `WRITERCRAFT_CAPTURE_SCENARIO`. Unknown selectors fail rather than silently loading a different document. Ordinary launches and the owner's running preview remain unchanged.

The synthetic document includes a 63-column table and nested tables with canonical widths plus surrounding bounded prose. Final-cell capture targets are `wide-cell-62` and `nested-cell-2`. Actual visible text is counted independently from opaque media. Captures must establish current target identity, nonzero local table scroll offset, bounded viewport/clip and unmodified canonical source; loaded fixture/model tests alone cannot satisfy native UI acceptance.
