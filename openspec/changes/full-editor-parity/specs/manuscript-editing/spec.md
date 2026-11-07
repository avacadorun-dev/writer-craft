## ADDED Requirements

### Requirement: Durable canonical rich state

A structured document save SHALL persist the complete canonical tree and an immutable corresponding revision, including formatting-only changes. Its plain-text value SHALL be derived and SHALL NOT be accepted as a replacement for the canonical tree through legacy plain-save paths. Existing plain/opaque data and histories SHALL remain preserved by additive migration.

#### Scenario: Only formatting changes
- **WHEN** the author changes marks without changing visible words and saves
- **THEN** a new structured revision records the marks, the prior rich snapshot remains available and restart restores the exact canonical structure

### Requirement: Rich working draft and conflict safety

Rich drafts SHALL retain complete structured data and the exact canonical base revision. Acknowledging an older save SHALL NOT discard continued typing; a stale write or failed command batch SHALL leave the current tree, revisions and drafts intact.

#### Scenario: Continued rich work differs from an acknowledgement
- **WHEN** an older snapshot is acknowledged after the draft has advanced
- **THEN** the advanced structured draft remains recoverable and cannot be silently overwritten by a new base
