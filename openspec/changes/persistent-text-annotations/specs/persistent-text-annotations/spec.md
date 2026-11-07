# Persistent text annotations delta

## Purpose

Preserve an author's selected-passage marks and attached notes across attention changes, editing and sessions without guessing new text targets.

## ADDED Requirements

### Requirement: PA-01 Durable selected-passage notes

WriterCraft SHALL let an author save an exact selected passage with attached authored note/reminder text as persistent metadata, preserving the manuscript text.

#### Scenario: Save a selected-passage note

- **WHEN** the author selects a supported passage and saves an attached note
- **THEN** the note and exact attachment are saved together and the manuscript content remains unchanged

### Requirement: PA-02 Persistent visible marks

WriterCraft SHALL keep an attached note's visual mark visible after the active selection or focus moves and restore the mark and note after reopening.

#### Scenario: Move attention elsewhere

- **WHEN** the author places the caret elsewhere, changes panes or selects another passage
- **THEN** the existing attached note remains visibly marked and its text remains accessible

#### Scenario: Reopen a marked document

- **WHEN** the author closes and reopens the document or application
- **THEN** its saved marks, notes and attachment states are restored

### Requirement: PA-03 Exact Unicode-aware edit mapping

WriterCraft SHALL preserve exact annotation identity using the source revision and Unicode-scalar text ranges, distinguish repeated quotes and update anchors only from deterministic edit mapping.

#### Scenario: Annotate a repeated second quote

- **WHEN** the author marks the second occurrence of an identical quote after Cyrillic text and emoji
- **THEN** only the selected occurrence is attached, including after known edits before that passage

#### Scenario: Edit after the marked passage

- **WHEN** a known edit changes text after the attached passage
- **THEN** the existing exact anchor remains attached to the same passage

### Requirement: PA-04 Preserve unresolved notes

WriterCraft SHALL preserve note text, original quote/context and lifecycle when deletion, partial overlap or uncertain mapping removes a valid anchor, exposing the note as unresolved instead of retargeting it by text search.

#### Scenario: Delete a marked passage

- **WHEN** an edit deletes or ambiguously changes the marked passage while an identical quote exists elsewhere
- **THEN** the note remains reachable as unresolved and is not moved to the other quote

### Requirement: PA-05 Explicit reattachment and independent lifecycle

WriterCraft SHALL require a current exact author selection for reattachment and keep note resolution independent from anchor validity, rejecting stale note or document updates without overwriting newer work.

#### Scenario: Reattach an unresolved note

- **WHEN** the author selects a current passage and explicitly reattaches an unresolved note
- **THEN** WriterCraft verifies the quote and revision and retains the attachment history

#### Scenario: A newer note revision exists

- **WHEN** an older note editor tries to save over a newer note or attachment revision
- **THEN** the operation reports a conflict and preserves the newer metadata

### Requirement: PA-06 Consistent metadata history and exports

WriterCraft SHALL preserve annotation consistency through text saves, reviewed changes, undo and compatible transfer, keeping note metadata and decorations outside ordinary reader exports.

#### Scenario: Undo a text change

- **WHEN** an accepted undo restores a text revision that affected a note
- **THEN** text and corresponding annotation state remain consistent and prior history remains available

#### Scenario: Export reader text

- **WHEN** the author exports the ordinary manuscript for reading
- **THEN** note text and editor-only decorations are excluded from the reader content
