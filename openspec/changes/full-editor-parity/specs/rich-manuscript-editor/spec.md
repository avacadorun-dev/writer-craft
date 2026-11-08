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

### Requirement: Owner-selected Balanced workspace

The editor SHALL follow the owner-approved Balanced concept in `docs/design/WORKSPACE_EDITOR_REDESIGN.md`, preserving comfortable prose measure, readable controls, distinct semantic groups, shared pane boundaries and reachable overflow across supported viewports and languages.

#### Scenario: Balanced vertical rhythm
- **WHEN** the author opens the manuscript workspace
- **THEN** its global bar, identity row and flat formatting row occupy 48, 56 and 40 logical points, followed by a 28-point manuscript gutter, with comfortable content spacing

#### Scenario: A constrained desktop pane displays the editor
- **WHEN** the author opens the editor at the minimum supported viewport or requests an inspector
- **THEN** document identity and actions remain independently reachable, secondary formatting commands use labelled overflow as needed, the toolbar does not wrap or require horizontal scrolling, and the manuscript retains its minimum useful width

#### Scenario: A design concept is reviewed
- **WHEN** an illustration shows notes, diagrams, formulas or other rich content
- **THEN** it is identified as a design concept and cannot substitute for the actual compiled 21-node/six-mark demonstration or native acceptance

### Requirement: Complete authoring from an empty document

Every original editor block and mark SHALL have reachable native creation, configuration, subsequent editing, deletion, Undo/Redo and durable save/reopen actions. UI and headless commands SHALL share guarded canonical semantics. A prepared gallery or a Mermaid-only insertion flow SHALL NOT satisfy this requirement.

#### Scenario: The author inserts and revisits a rich block
- **WHEN** the author starts with an empty manuscript and inserts any supported block at a selected caret or range
- **THEN** its source, language, caption and applicable properties remain editable, surrounding rich content survives, and Undo/Redo and restart preserve the exact canonical result

### Requirement: Source-preserving syntax highlighting

Code blocks SHALL highlight all original supported programming and structured-text formats using real grammars. Persisted source, whitespace and language identifiers SHALL remain unchanged by highlighting. Unknown imported languages SHALL stay readable and editable. Theme changes SHALL preserve token readability and selection/caret geometry.

#### Scenario: Structured text is edited and reopened
- **WHEN** the author edits a supported structured-text block, changes the interface theme, saves and reopens
- **THEN** grammar-derived highlighting and the selected language remain visible without injecting formatting markup into the canonical source

### Requirement: Comfortable shared fields and modal geometry

Search and dialog fields SHALL use shared comfortable height, usable width, inner padding and vertical alignment. Modal headers, bodies and action rows SHALL share consistent insets and spacing. Native keyboard/focus, disabled/error states and all eight interface languages SHALL remain usable at the supported minimum window size.

#### Scenario: The author creates a manuscript
- **WHEN** the author opens the New document dialog and types a long localized title
- **THEN** the input and buttons have consistent readable geometry, header/body/footer edges align, and keyboard submit/cancel preserve the correct manuscript focus and data

### Requirement: Approved adaptive document surface

The continuous manuscript SHALL appear as a centered bounded surface with exterior gutters when space permits, including focus mode. At constrained widths it SHALL join the docked workspace cleanly. Both states SHALL preserve a readable text measure, local scrolling, selection and reachable toolbar commands without requiring print pagination.

#### Scenario: The author opens or closes side panels
- **WHEN** available editor width crosses the approved floating/docked threshold
- **THEN** the document surface adapts while retaining the bounded text measure, exact selection and local scroll state; focus does not stretch prose across the whole window


### Requirement: Durable reviewed structured edits

The native review panel and command adapters SHALL stage project-scoped typed transactions without canonical changes. Apply SHALL revalidate and atomically commit content, history, note mappings and status against the exact base. Reject SHALL change status only. Undo Apply SHALL require the unchanged applied state. Drafts and imported evidence SHALL NOT become implicit mutation authority.

#### Scenario: Formatting changes before proposal Apply
- **WHEN** a staged structured proposal is reviewed after a formatting-only canonical change
- **THEN** Apply refuses the stale revision/tree and leaves canonical content, annotations, drafts, history and proposal state unchanged

#### Scenario: Note metadata changes after Apply
- **WHEN** an author edits a note after applying a proposal and then chooses Undo Apply without intervening document edits
- **THEN** the compensating revision restores the prior proven attachment and retains the author's current note text and lifecycle

#### Scenario: A proposal introduces an unavailable resource or over-capacity table
- **WHEN** a structured proposal references an image outside its project or creates a table wider than the supported native capacity
- **THEN** preparation or Apply refuses before mutation and preserves all existing content, notes and history

#### Scenario: Exact selected passage is attached to review
- **WHEN** the author captures a supported native selection for review
- **THEN** the saved selection chip retains exact project/revision/ranges and supports reveal, clear and explicit recapture without searching for another occurrence; actual document edits make it visibly stale


#### Scenario: Review meets a pending draft or imported evidence
- **WHEN** Apply or Undo encounters an unresolved working draft, or an imported archive contains proposal/replay data
- **THEN** the draft is preserved and blocks mutation, while imported proposal data remains quarantined preservation evidence rather than active proposals
