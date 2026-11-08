## ADDED Requirements

### Requirement: Complete original editor scope

WriterCraft alpha.2 SHALL implement and verify every current-editor capability in the frozen `docs/migration/EDITOR_PARITY_MATRIX.md` and companion independent UX matrix. All 21 canonical nodes, six marks, schema 1–4 behavior, metadata and integration points SHALL remain covered. A reduced subset SHALL NOT qualify as full-editor acceptance.

#### Scenario: A required editor case remains unsupported
- **WHEN** any required node, interaction or integration row lacks a working implementation and appropriate verification
- **THEN** alpha.2 release acceptance remains pending, with the exact gap recorded rather than removed from scope

### Requirement: Lossless canonical rich transactions

Native UI and headless commands SHALL use one revision-guarded structured document engine that preserves stable block IDs, unknown supported metadata, source references and rich history. A failed batch SHALL leave the entire canonical tree/history/annotation state unchanged. Unicode-scalar, UTF16 and byte positions SHALL remain explicit and validated.

#### Scenario: Unicode exact edit and history
- **WHEN** an exact command edits the second repeated quote after Cyrillic, emoji and combining text
- **THEN** only that range changes, unrelated marks/attrs/sourceRefs remain unchanged, and Undo/Redo restore the corresponding structured state without overwriting later work

### Requirement: One complete native editing surface

The editor SHALL support continuous caret/selection/input across rich blocks and tables with native keyboard, mouse, clipboard, IME and accessibility behavior. Every supported rich block SHALL have its real editing and rendering interactions, including safe source-preserving failure states.

#### Scenario: Rich selection crosses a block boundary
- **WHEN** the author extends a selection across paragraphs and a table boundary and performs an allowed editing action
- **THEN** one document-level selection and transaction govern the result, rather than independent fields losing the original range or history

### Requirement: Persistent passage metadata and real handoffs

Notes and dictionary links SHALL keep exact revisioned anchors and remain reachable across edits/restart. Source audio and assistant selection/proposal actions SHALL use real shared commands and data rather than decorative or disabled controls.

#### Scenario: A noted quote is deleted
- **WHEN** the author deletes or ambiguously overwrites a noted passage
- **THEN** the note and original quote/context survive as detached, remain discoverable and can be explicitly reattached without guessing another occurrence

### Requirement: Full native acceptance evidence

An independent reviewer SHALL accept `full-editor-parity` only against the exact source and real compiled/native evidence for the complete matrices. Model or layout tests alone SHALL NOT establish native editing parity or complete product parity.

#### Scenario: A model-only milestone passes
- **WHEN** structured round-trip and transform tests pass but required native interactions remain unobserved
- **THEN** the milestone is recorded as internal model evidence and the full-editor release gate stays pending

### Requirement: Approved table capacity without data loss

The native alpha.2 editor MAY limit editing to 63 logical table columns, as explicitly approved by the owner. Every table operation within that capacity SHALL remain available. The canonical parser SHALL preserve valid wider tables and span values 1–100 without truncation or clamping. UI and MCP adapters SHALL reject creation, widening or paste beyond the runtime capacity before mutation and SHALL identify preserved wider documents as read only.

#### Scenario: A preserved table exceeds runtime capacity
- **WHEN** an existing table exceeds 63 logical columns
- **THEN** its complete canonical data/history remain intact, the editor explains its read-only capacity state, and no cells or spans are silently removed or rewritten

#### Scenario: A command would exceed table capacity
- **WHEN** a UI or MCP operation would create or widen a table beyond 63 logical columns
- **THEN** it returns a capacity error without partially changing document, history or annotations
