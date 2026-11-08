## ADDED Requirements

### Requirement: Evidence-backed project distinctions

New creation profiles SHALL follow documented task hypotheses and professional/user evidence. The existing seven kinds SHALL be treated as import compatibility data, not proof that a nominal selector satisfies a professional workflow.

#### Scenario: A type differs only by its name
- **WHEN** research finds no useful structure, editing, reference, review or output distinction for a proposed type
- **THEN** the decision SHALL record whether a template, metadata or shared mode better serves the task instead of claiming a completed professional type

### Requirement: Durable user scenarios

Every selected profile or formatting capability SHALL have stable-ID scenarios recording role, context, trigger, main actions, observable outcome, alternatives, failure/recovery and compatibility. Evidence SHALL link scenarios to specifications, implementation, semantic tests and actual native acceptance.

#### Scenario: A scenario has not been exercised
- **WHEN** a scenario is documented but no exact compiled interaction has been observed
- **THEN** its native acceptance SHALL remain unqualified even if source/model checks pass

### Requirement: Backward-compatible profile evolution

A refined creation model SHALL keep existing Studio versions1–8 and editor schemas1–4 importable and usable, including original kinds, custom structure, relationships, unknown fields and original media. New semantics SHALL use versioned extensions and compatible defaults/migrations without silent reclassification or loss.

#### Scenario: A legacy custom project is imported
- **WHEN** its saved kind or custom hierarchy differs from the new creation defaults
- **THEN** the project retains its identities, valid structure and data and can complete the supported edit/save/reopen/export workflow through an explicit compatibility mapping

### Requirement: Predictable direct object sizing

Selected image, rich-block and table sizing SHALL use actual native geometry and shared guarded commands, distinguish size from zoom, retain source data and preserve neighboring flow. Completed gestures SHALL support exact Undo/Redo; cancellation and stale/failure states SHALL not publish partial edits.

#### Scenario: Resizing is cancelled or the target changes
- **WHEN** a size gesture is cancelled or its captured document/object becomes stale before commit
- **THEN** the original dimensions, content, history and attachments remain unchanged and the author receives a clear current state
