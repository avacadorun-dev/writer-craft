# Native MVP capability delta

## Purpose

Support dependable native writing with explicit revision conflicts and an upgrade path from plain prose to lossless structured manuscripts.

## ADDED Requirements

### Requirement: ME-01 Revisioned plain prose editing

WriterCraft SHALL save editable plain prose as successive revisions and reject a save whose expected revision is no longer current.

#### Scenario: A second editor saves an older revision

- **WHEN** another command has saved a newer revision before an older editor saves
- **THEN** the older save reports a conflict and preserves the newer text

#### Scenario: Text is unchanged

- **WHEN** the author saves text identical to the current revision
- **THEN** WriterCraft preserves the current revision number and avoids duplicate history

### Requirement: ME-02 Unicode-safe text identity

WriterCraft SHALL preserve Cyrillic, emoji and combining characters and address exact text ranges using Unicode scalar positions with verified expected text.

#### Scenario: Replace text after an emoji

- **WHEN** an exact phrase follows Cyrillic text and an emoji
- **THEN** the replacement changes that phrase alone and the displayed range identifies the same Unicode scalars

### Requirement: ME-03 Imported rich content preservation

WriterCraft SHALL retain imported structured source and present a read-only preview until editing can preserve its blocks, marks, identities, references and annotations.

#### Scenario: Plain text editor encounters a formula document

- **WHEN** an imported document includes a formula, table, image or other unsupported rich structure
- **THEN** plain text save and replacement operations cannot overwrite its preserved source
