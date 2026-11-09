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


### Requirement: Source-compatible connection endpoints

Connection creation SHALL expose the four side anchors of each connectable card. Authors SHALL be able to reconnect either end of an existing connection through a guarded native editor. An operation SHALL capture the exact prior node and optional side; changing one end SHALL preserve the connection identity, other end, label, colors, relation metadata and unknown fields. Same-card connections between different explicit side handles SHALL remain authorable. Cancelled and stale edits SHALL NOT mutate canonical content.

#### Scenario: Connect and reconnect explicit side handles
- WHEN an author connects two chosen side handles, edits one endpoint, saves and uses Undo/Redo
- THEN the visible route reflects the acknowledged endpoint sides and the unchanged connection data survives every revision and reopen

#### Scenario: Same-card loop and stale endpoint
- WHEN an author connects different handles of one card or edits an endpoint after its captured base is replaced
- THEN the distinct-handle loop is valid, while the stale edit refuses without losing the copyable draft

### Requirement: Reachable connection interaction and visual semantics

Stored connections SHALL show source-compatible routed paths and source/target arrow markers without rewriting imported omitted fields. Painting and pointer selection SHALL share derived geometry, preserve node hit priority and respect viewport clipping/zoom. Authors SHALL select an edge directly and delete edge-only selection from the keyboard; double-clicking its visible label SHALL open the same guarded editor as its contextual menu entry.

#### Scenario: Select, edit and delete a routed connection
- WHEN an author selects the rendered connection, double-clicks its label or deletes an edge-only selection
- THEN the exact targeted connection responds without dragging/deleting unrelated cards, and its label remains positioned on the route
