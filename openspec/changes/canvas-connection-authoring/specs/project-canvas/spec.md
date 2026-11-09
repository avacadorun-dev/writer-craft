## ADDED Requirements

### Requirement: Board-owned tool gestures

Text placement and connection tools SHALL act only on completed clicks owned by the board with the same tool active from press to release. Toolbar, outside-board, cancelled and drag gestures SHALL NOT mutate the board. Card hits SHALL match the topmost painted card. Existing empty-board double-click creation in Select mode SHALL remain available.

#### Scenario: Toggle the text tool or click another pane
- WHEN an author arms or disarms text placement or clicks outside the board
- THEN no card or revision is created until an actual empty-board placement click occurs

#### Scenario: Connect overlapping cards
- WHEN an author clicks overlapping cards in connection mode
- THEN the visible topmost card is the endpoint and the uncommitted preview changes no saved data

### Requirement: Complete connection lifecycle

Authors SHALL select existing board connections, edit Unicode labels, cancel unsubmitted edits and delete a connection through reachable native controls. Save, Undo and Redo SHALL retain stable identities, unrelated fields and original card content. Failed or stale changes SHALL preserve recoverable drafts and refuse silent overwrite.

#### Scenario: Rename and delete a connection
- WHEN an author edits a connection label, saves, deletes it, undoes and redoes the changes, and reopens the board
- THEN each acknowledged operation has a durable compensating revision and the restored edge retains its exact identity and unknown fields

#### Scenario: Cancel or stale label edit
- WHEN a label edit is cancelled or its captured board base is stale
- THEN canonical content remains unchanged and stale unsaved text stays available for deliberate recovery
