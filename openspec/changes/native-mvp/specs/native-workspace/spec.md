# Native MVP capability delta

## Purpose

Let an author keep a local writing library, recover unfinished work, and inspect portable source projects without installing a server.

## ADDED Requirements

### Requirement: NW-01 Local project library

WriterCraft SHALL let an author create a named local project, create documents, and reopen their saved content without an account or network connection.

#### Scenario: Reopen a local document

- **WHEN** an author saves a document, closes WriterCraft and reopens the same workspace
- **THEN** the project, document identity, text and current revision remain available

### Requirement: NW-02 Durable working drafts

WriterCraft SHALL preserve unfinished working text separately from saved revisions and offer recovery without silently replacing a newer saved document.

#### Scenario: Recover an unfinished thought

- **WHEN** WriterCraft restarts after working text was captured but before a canonical save
- **THEN** the working text and its source revision remain recoverable while canonical history stays unchanged

#### Scenario: An older autosave finishes after a newer draft

- **WHEN** a canonical save succeeds for text different from the latest working draft
- **THEN** the newer differing draft remains available for recovery

#### Scenario: Preserve an older distinct recovery buffer

- **WHEN** a new working draft would replace differing recovery text captured from an older saved revision
- **THEN** WriterCraft reports a recovery conflict and leaves the older buffer unchanged

#### Scenario: Recover into an editable sibling

- **WHEN** the author opens a document with a stale recovery buffer
- **THEN** WriterCraft can atomically save that buffer as a separately named editable document, consume the buffer and preserve the original canonical document and history

### Requirement: NW-03 Compatible workspace opening

WriterCraft SHALL identify unsupported or foreign workspace files and preserve their existing contents when opening fails.

#### Scenario: Open a newer workspace

- **WHEN** an author opens a workspace whose format is newer than this application supports
- **THEN** WriterCraft explains that an update is required and leaves the workspace unchanged

### Requirement: NW-04 Honest source inspection

WriterCraft SHALL label inspected legacy projects as source previews, preserve their original structured content, and report unsupported migration capabilities before an author treats a preview as an editable import.

#### Scenario: Inspect a rich legacy project

- **WHEN** an author inspects a supported .wstudio source containing rich blocks or attachments
- **THEN** WriterCraft identifies the source format and preview boundary while keeping the source archive unchanged

### Requirement: NW-05 Data-preserving migration gate

WriterCraft SHALL claim full project migration only after the published parity inventory passes preservation and semantic round-trip acceptance for supported legacy versions.

#### Scenario: Migration is incomplete

- **WHEN** a source feature such as annotations, original audio or Canvas has not passed migration acceptance
- **THEN** the release identifies that limitation and does not claim full Writers Studio parity
