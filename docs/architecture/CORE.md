# Local command core

`studio-core` owns projects, plain text documents, immutable revisions and reviewed exact replacements. It has no GUI dependency. The native interface and CLI call the same `Workspace` methods; no HTTP server or Docker runtime is required.

`Workspace::open(path)` takes a SQLite **file** path and creates its parent directory. `Workspace::memory()` creates an isolated test workspace. Every method is fallible. Invalid input, missing resources, stale saves, ambiguous literal matches, read-only imports and undo conflicts have typed errors.

## Persistence

SQLite uses foreign keys, WAL and FULL synchronous mode, with a bounded five-second busy timeout. Multi-write commands run in `BEGIN IMMEDIATE` transactions. Schema version and application identifier reject foreign databases and newer schemas. The initial schema is created transactionally; no tables or revisions are dropped. Future schema changes require additive numbered migrations and acceptance tests.

Schema2 adds durable working drafts without changing existing records. `save_draft` persists a revision-pinned working buffer separately from canonical history; `draft` reads it for crash recovery. Successful canonical saves clear only drafts with the same base revision and exact saved body, preserving newer differing drafts. Apply clears only drafts matching the old or newly applied canonical body; unrelated work remains recoverable. `save_draft` refuses to overwrite differing recovery text captured from another base revision. `recover_draft_as_document` atomically creates an editable same-project copy at revision1 with a recovery title suffix and consumes the old buffer; original text and history remain unchanged. A missing buffer creates nothing.

Canonical document revisions start at one. Changed text appends a revision, unchanged text saves are no-ops, and stale expected revisions fail. History is never rewritten. The workspace owns a mutex-protected connection; separate workspace instances still rely on SQLite serialization and revision checks. Synchronous large commands belong off the render thread.

## Proposal protocol

1. `stage_replace(document, revision, expected_text, replacement, reason)` verifies editable content, current revision and exactly one literal occurrence. It stores before/after snapshots with SHA-256 digests and a Unicode-scalar anchor. Overlapping matches are ambiguous. Canonical text and history do not change.
2. `apply_proposal(id)` requires staged state and the exact canonical revision/body used to prepare the proposal. It appends the new revision and changes proposal state atomically. A stale proposal remains staged and cannot apply without a new review.
3. `reject_proposal(id)` changes only staged proposal state.
4. `undo_proposal(id)` requires the exact revision/body resulting from apply. It appends a compensating revision and marks the proposal undone. A later manual or proposal edit causes `UndoConflict`, preserving newer work.

Revisions and proposals list newest first. GUI callers should compare a selected proposal with the currently loaded document before presenting Apply, while the core always performs the authoritative check.

This MVP supports one document replacement per proposal. Multi-document grouped changes, protected ranges, rich block transforms, annotations, provider orchestration and MCP remain migration work; they are not silently emulated by plain text operations.

## Imported data

`import_document(project,title,body,source_json)` stores the complete supplied JSON value and exposes `body` only as a preview. Imported documents are read-only in every canonical editing path. This prevents rich marks, stable block IDs, audio references, annotations or unknown fields being destroyed by a plain text editor. The transfer inspector separately validates `.wstudio` archives; inspection does not imply full migration or asset import.

The current core does not provide a destructive flatten command. An author can explicitly create a new plain document as an editable copy while retaining the imported original.
