## ADDED Requirements

### Requirement: DCT-1 direct writable text cards

WriterCraft SHALL provide a real multiline native text input inside an activated unlocked text card. Creating a text card SHALL focus it after durable creation. Selection, clipboard, local Undo, scroll and composition SHALL remain owned by that input at the actual zoom/clip.

#### Scenario: A card is created or double-clicked
- **WHEN** a new card is acknowledged or an existing unlocked text card is activated
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
