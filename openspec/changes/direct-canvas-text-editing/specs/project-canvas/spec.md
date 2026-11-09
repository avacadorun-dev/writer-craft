## ADDED Requirements

### Requirement: DCT-1 direct writable text cards

WriterCraft SHALL provide a real multiline native text input inside an activated unlocked text card. Creating a text card SHALL immediately expose and focus its provisional native input without waiting for durable insertion acknowledgement. This presentation draft SHALL NOT become a second canonical board. Selection, clipboard, local Undo, scroll and composition SHALL remain owned by that input at the actual zoom/clip.

#### Scenario: A card is created or double-clicked
- **WHEN** a new-card command is accepted for persistence or an existing unlocked text card is activated
- **THEN** the author SHALL type directly in that card without opening a separate Markdown dialog
- **AND** OS focus loss or IME events SHALL not discard/finish the live text

### Requirement: DCT-2 acknowledged recoverable text autosave

A debounced edit SHALL be checkpointed and saved against the exact acknowledged board/draft heads before local Apply. Later typing MUST survive an older save acknowledgement. Failed/stale writes MUST retain both the pending candidate and latest draft with explicit retry/preservation.

#### Scenario: Input continues while persistence is pending
- **WHEN** an older submitted source becomes durable after later text was entered
- **THEN** the native draft/selection/history SHALL remain, rebased against that exact acknowledged source
- **AND** the latest dirty text SHALL be queued for its own guarded save

#### Scenario: Save fails or another draft exists
- **WHEN** checkpoint/save refuses the captured head/token
- **THEN** canonical data SHALL remain unchanged and the latest draft SHALL remain accessible for retry or exact text preservation
- **AND** navigation/close SHALL not silently discard it

### Requirement: DCT-3 exact pending creation and input ownership

The same native editor SHALL retain text, selection, composition and local Undo across the exact insertion acknowledgement. The host SHALL acquire focus at creation and route owned lifecycle keys before generic focus navigation, using supported public APIs without masking platform IME output or requiring a hidden warmup frame.

#### Scenario: First input precedes insertion acknowledgement
- **WHEN** the first mounted frame receives Unicode text or IME preedit while insertion remains pending
- **THEN** the input SHALL retain that source immediately without a canonical edit of an absent node
- **AND** acknowledgement SHALL promote the same editor without replacement, an IME interruption or an extra Cancel

#### Scenario: First-frame Escape, arrows or another control
- **WHEN** an active exact-focus editor receives Escape during composition
- **THEN** it SHALL not turn the key into application Cancel or generic focus loss
- **AND** ordinary Escape SHALL request Cancel exactly once while arrows move its caret and Tab, other controls, blocked modals, other viewports and unfocused windows retain their appropriate ownership

#### Scenario: Insertion fails before a prepared candidate exists
- **WHEN** insertion fails with or without a prepared recovery candidate, including an empty provisional card
- **THEN** navigation/close SHALL not silently discard the pending record or draft
- **AND** Retry SHALL use the original insertion against its exact base, while preservation SHALL retain the original node plus latest text atomically

### Requirement: DCT-4 explicit cancellation during persistence

Cancel SHALL immediately stop accepting further edits into the cancelled draft and show an explicit waiting state while the in-flight outcome is unknown. Already durable insertion or autosaves SHALL remain; unacknowledged text SHALL not be presented as saved. Failure SHALL retain the exact draft/candidate and actionable retry/preservation.

#### Scenario: Cancel while a save is pending
- **WHEN** Cancel is requested after a request starts and before its acknowledgement
- **THEN** later text or IME SHALL not be accepted and then silently discarded
- **AND** success finishes the editor with the acknowledged content while failure preserves recovery choices

#### Scenario: Done during composition or an earlier save
- **WHEN** Done is requested while composition or an older request remains pending
- **THEN** finishing SHALL wait for resolved composition and durable acknowledgement of the latest intended source
- **AND** an older acknowledgement SHALL not claim the latest text saved
