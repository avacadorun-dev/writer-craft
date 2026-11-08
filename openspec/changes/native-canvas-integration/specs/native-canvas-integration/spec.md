## ADDED Requirements

### Requirement: Canonical native board acknowledgement

A native board operation SHALL use the project's exact current saved head and one exact in-process Canvas session. The application SHALL commit the validated prepared graph as a durable append before acknowledging it to the view. A failed or stale save SHALL preserve the previous session and canonical graph and retain the candidate for deliberate retry.

#### Scenario: A move encounters a newer saved board
- WHEN another writer saves the same project board before a pending move is acknowledged
- THEN the move is refused without overwriting the newer board or reporting success, and the unchanged local session retains the candidate for conflict handling

### Requirement: Compensating board history

Undo and Redo SHALL prepare exact session transitions and append durable compensating board revisions without deleting earlier revisions or resetting history on a failed save.

#### Scenario: An author edits and undoes a text card
- WHEN the author saves a Unicode text-card edit and chooses Undo and Redo
- THEN each transition appends one saved revision and preserves stable IDs, unrelated source fields and exact text after reopening

### Requirement: Explicit board resources and recovery

Board file/resource references SHALL resolve only inside the selected project. Existing native recovery drafts SHALL remain separate from the saved board until explicit author recovery. Unsupported media, unknown node kinds and stale drafts SHALL stay preserved and visibly identified.

#### Scenario: A recovered board is stale
- WHEN an author opens a board with an unfinished draft whose saved base is no longer current
- THEN the saved board stays authoritative and the draft remains available without silently replacing current content

### Requirement: Integrated native presentation

Canvas SHALL use the existing app command bar, bounded panes, theme tokens, official Lucide utilities and complete locale catalogs. Its source dialog and actions SHALL use the shared comfortable control/modal grid. Embedded media controls SHALL own their input rather than accidentally dragging a board node.

#### Scenario: A text edit is cancelled
- WHEN the author changes a text-card source then cancels its native dialog
- THEN no canonical content or saved revision is changed and the board returns with the same meaningful selection and camera
