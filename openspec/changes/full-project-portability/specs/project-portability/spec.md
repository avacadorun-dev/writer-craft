# Project portability capability delta

## ADDED Requirements

### Requirement: Complete project migration

The application SHALL import supported .wstudio envelope versions 1–8 as independent complete local projects, preserving original project-scoped logical IDs, all supported metadata, tree order/types, rich document history, named versions, source links, notes, Canvas/type-schema history, context/transcript records, unknown fields and original asset bytes.

#### Scenario: Whole project with repeated source identities

- WHEN the same valid archive is imported twice
- THEN both local projects retain the original logical identities and relationships in separate namespaces, reopen independently and cannot read or mutate each other's resources.

#### Scenario: Failed import

- WHEN validation, cancellation, storage, archive CRC or asset checksum fails
- THEN no partial project becomes visible, existing data and the source remain unchanged, and an actionable failure is shown.

#### Scenario: Unsupported future format

- WHEN an archive uses an unsupported envelope or editable content format
- THEN no destructive conversion occurs and the original data remains unchanged with its limitation identified.

### Requirement: Native Canvas and attached media

The application SHALL open and exercise all supported imported Canvas boards and attached image/PDF/audio/video resources with native interactions, resource navigation, canonical edits, history, guarded saves and recovery.

#### Scenario: Canvas content and media after restart

- WHEN an author opens a migrated board, edits known nodes/edges/drawings/metadata, plays or previews its attached media, saves and restarts
- THEN actual native rendering/playback and navigation work, the appended revisions reopen, and unrelated known/unknown fields and exact attachment bytes are retained.

#### Scenario: Stale Canvas save or decoder failure

- WHEN another revision supersedes a Canvas draft or a media decoder fails
- THEN newer canonical data and recoverable drafts remain intact and an explicit failure or unsupported-preview state replaces neither source JSON nor attachment bytes.

### Requirement: Revision consistent portable export

The application SHALL export selected documents and complete projects as Markdown plus real referenced assets and a versioned canonical companion retaining semantics, history, notes, recovery and relationships that Markdown cannot represent.

#### Scenario: Export during continued editing

- WHEN writing, notes, Canvas or project metadata change after an export snapshot is captured
- THEN all emitted records and asset references belong to the captured snapshot, and durable unfinished work remains separately identified in the companion.

#### Scenario: Markdown conversion difference

- WHEN a supported rich/project feature cannot be represented exactly in the Markdown view
- THEN the documented conversion report identifies that difference and the original canonical feature remains in the companion without a lossless-Markdown claim.

### Requirement: Safe paths and export completion

The application MUST use deterministic Unicode-compatible collision-safe portable paths, relative links, original artifact checksums and no-clobber output publication.

#### Scenario: Existing folder or interrupted output

- WHEN the requested output would collide with unrelated existing paths or an export is interrupted
- THEN unrelated files and the workspace remain unchanged, incomplete output is explicitly identified, cleanup touches only job-owned paths and success is not reported.

### Requirement: Transparent storage and consistent backup

The application SHALL show the actual selected workspace data location and provide native open-folder and accurate backup-scope information in ru/en/de/fr/es/pt/zh/ja.

#### Scenario: Explicit database override

- WHEN a database override or isolated preview profile is active
- THEN the displayed/opened folder matches that actual path and explains the database, immutable assets, histories and recovery scope without presenting a guessed default.

#### Scenario: Live WAL workspace

- WHEN the author requests a consistent backup
- THEN the backup uses an acknowledged SQLite snapshot or an orderly checkpoint/close procedure and does not advertise copying only a live database file as sufficient.

### Requirement: Complete migration acceptance

The application MUST qualify synthetic complete-domain fixtures on hosted CI and privately exercise the supplied reference project through native open/edit/media/restart/recovery/round trip before closing migration issues.

#### Scenario: Private acceptance data

- WHEN verification uses the owner's reference archive
- THEN the archive, manuscript, screenshots and media remain local and read-only, public evidence records sanitized outcomes only and repository/CI fixtures are synthetic.
