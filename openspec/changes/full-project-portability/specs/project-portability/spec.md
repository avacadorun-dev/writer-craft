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

#### Scenario: Export targets active or future workspace files

- WHEN a user-selected export or backup path names a custom database, an existing alias, a future WAL/SHM/rollback journal or a sibling preference file
- THEN the operation refuses before creating or replacing output, including through a selected database symlink or Windows-equivalent filename, and all active data remains intact.

#### Scenario: Database path uses a settings filename

- WHEN the selected workspace filename or an existing alias collides with a reserved preference file
- THEN the workspace is rejected before creation or mutation with an actionable filename error, and settings cannot replace that database.

### Requirement: Complete migration acceptance

The application MUST qualify synthetic complete-domain fixtures on hosted CI and privately exercise the supplied reference project through native open/edit/media/restart/recovery/round trip before closing migration issues.

#### Scenario: Private acceptance data

- WHEN verification uses the owner's reference archive
- THEN the archive, manuscript, screenshots and media remain local and read-only, public evidence records sanitized outcomes only and repository/CI fixtures are synthetic.

### Requirement: Canonical native round trip

The application SHALL export and restore complete or selected native projects through a versioned
canonical representation with exact immutable artifact bytes and independently scoped semantic
validation. Native format 2 SHALL use a small root, bounded metadata shards and hash-referenced
payload objects instead of repeatedly embedding complete source/native/history JSON values.

#### Scenario: Native source, history and recovery

- WHEN saved native edits, current notes/tombstones/events, Canvas history, opaque sources or durable
  unfinished work exceed the source-compatible envelope's semantics
- THEN canonical restore recreates the saved project in a fresh namespace, retains unknown data and
  exact logical identities, and keeps external drafts/checkpoints/staged changes as readable
  untrusted evidence requiring explicit conservative recovery.

#### Scenario: Component and transport bounds

- WHEN a native bundle is prepared
- THEN its root is at most 1 MiB, each metadata shard at most 1 MiB, each metadata record at most
  256 KiB, and native ZIP compressed/expanded totals at most 1 GiB; original format 1–8 limits remain
  unchanged and oversize is refused before project writes or completed output publication.

#### Scenario: Large or repeated native payload

- WHEN a payload exceeds the inline record budget or appears in current/history/snapshot records
- THEN records use content-addressed references, readers validate only bounded relevant objects,
  raw evidence streams without a wholesale native-state Value, and logical identity remains separate
  from content deduplication.

#### Scenario: Unknown native domain and repeated restore

- WHEN native or extension data is not understood by the current semantic model
- THEN its exact raw object and scoped metadata remain flat hash-identified evidence, cannot become
  replay or Apply authority, and repeated restore/export does not recursively nest prior envelopes.

#### Scenario: Pre-release format-1 companion

- WHEN the author restores a supported bounded pre-release single-JSON companion
- THEN it converts through the same semantic validation and fresh-namespace transaction; an oversized
  old encoding produces an actionable re-export/backup error without silently widening allocation
  bounds or changing original archive behavior.

#### Scenario: Streaming integrity failure

- WHEN a root, shard, object, scope mapping, checksum, storage operation or cancellation check fails
- THEN no partial project is visible, no input or unrelated output is overwritten, and retained
  evidence does not populate trusted recovery/proposal tables.

#### Scenario: Large workspace outside portable bounds

- WHEN a native project exceeds explicit finite transport/resource guards
- THEN the application reports the specific unsupported size/count without truncation or a universal
  portability claim and still identifies the separate whole-workspace backup capability.

#### Scenario: Selected opaque extensions and physical transport aliases

- WHEN selected export encounters root/shard/source/evidence extension data whose document ownership
  cannot be proved
- THEN it omits or refuses that opaque data with an explicit non-content-bearing conversion report,
  preserves owned selected canonical writing and never expands unselected project bodies.
- WHEN equal hashes occur under different authorized file paths or reading views
- THEN transfer counts every physical member and expanded byte plus actual encoded container length
  independently of unique-object and materialized-row budgets.
