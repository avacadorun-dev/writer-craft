## ADDED Requirements

### Requirement: Direct source editing of existing rendered blocks

The native manuscript SHALL let an author re-open the source or properties of an already-created Mermaid diagram, block formula, inline formula, code block or image through a double-click on that object. The double-click SHALL open the same revision-guarded source dialog that the existing **Edit block** command uses.

#### Scenario: An existing diagram is double-clicked

- **WHEN** an author double-clicks a rendered Mermaid diagram in a writable manuscript
- **THEN** the source dialog opens with that exact diagram's stored source and caption, the document revision, selection and history are unchanged, and rendering occurs only after Apply

#### Scenario: An existing block formula or inline formula is double-clicked

- **WHEN** an author double-clicks a rendered block formula or an inline formula
- **THEN** the dialog opens with that exact formula's stored LaTeX and description, including a formula inside surrounding prose

#### Scenario: The author edits the current source and applies

- **WHEN** the author changes the source in the dialog and confirms Apply
- **THEN** the exact object is updated once, its stable identity, surrounding text and unknown attributes are preserved, the edited source renders, and Undo restores the previous source in one history step

### Requirement: Single-click selection and compact chrome

A single click SHALL retain ordinary text or whole-object selection without opening a source dialog. The permanent toolbar SHALL NOT gain further block commands or a second command row.

#### Scenario: A single click selects without editing

- **WHEN** an author single-clicks a rendered block or ordinary prose
- **THEN** the usual text or whole-object selection is produced and no source dialog opens

### Requirement: Object context menu and keyboard entry to the same editor

The manuscript SHALL expose the same source editing command through its object context menu and a keyboard equivalent. The context menu SHALL offer the command only for the exact object under the pointer, or for the current whole-object selection when invoked from the keyboard, so a nearby or previously selected object is never edited by mistake.

#### Scenario: The object context menu is used

- **WHEN** an author right-clicks a rendered block that is the current object selection
- **THEN** the context menu offers block editing and choosing it opens that same dialog at the same revision

#### Scenario: The keyboard requests editing directly

- **WHEN** the manuscript has focus, an editable object is selected, and the author presses F2
- **THEN** the same source dialog opens for that object without requiring the toolbar or a menu

#### Scenario: The context menu is invoked over other content

- **WHEN** the author right-clicks prose, or press-opens the menu with Shift+F10 while no editable object is selected
- **THEN** no block editing command is offered for an object that is not the current target, and existing note provisions remain available

### Requirement: Guarded failure and history behavior for direct block editing

Direct block editing SHALL retain every existing dialog guard. Cancelling, Escape, a stale revision, a failed save or a failed render SHALL NOT lose the typed source or mutate canonical content; retrying SHALL remain possible. Saving and reopening the project SHALL preserve the same block identity and source.

#### Scenario: Cancel or Escape leaves the document exact

- **WHEN** the author opens an existing block, types a change and cancels or presses Escape
- **THEN** the document, revision, history and surrounding content remain identical to before the dialog opened

#### Scenario: The document changed while the dialog was open

- **WHEN** the dialog's captured revision is no longer current and the author confirms Apply
- **THEN** the command refuses mutation with an actionable error, the typed source stays in the dialog, and the newer document is not overwritten

#### Scenario: The edited block is saved and reopened

- **WHEN** the author applies a source change, saves and reopens the project
- **THEN** the same block identity, edited source, caption/description and retained unknown attributes are present and render from the saved canonical document
