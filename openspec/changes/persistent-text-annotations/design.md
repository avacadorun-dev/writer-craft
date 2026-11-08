# Persistent annotation design brief

Status: selected within the owner-approved full-editor alpha.2 scope. The shared-core implementation is an engineering checkpoint; hosted tests, native decoration/input integration and independent UX acceptance remain pending.

## Context

See proposal.md and `docs/migration/PERSISTENT_ANNOTATIONS.md`. Legacy annotations are separate author metadata with stable block/range anchors, exact quote/context, lifecycle and history. Current native writing uses a plain text buffer and revisioned proposals; an ephemeral selection cannot stand in for a persistent note.

## Goals / Non-Goals

Goals for a later agreed increment: durable visual attachment and authored note text, exact Unicode identity, deterministic mapping, unresolved-note preservation, transactional text/anchor history and source-backed compatibility.

The published alpha.1 and unrelated domain features remain outside this change.

## Decisions

The behavioral coordinates are Unicode scalars and exact text at a document revision. A stable document/block identity disambiguates repeated quotes. Annotation metadata and visible decorations remain distinct from manuscript content. Note lifecycle and anchor validity are independent. An unresolved anchor preserves user information and requires explicit reattachment instead of a first-match guess.

These contracts are accepted requirements for parity, not a choice of editor, data-table layout or migration number. Actual storage/editor mechanisms stay undecided until the next scope is agreed. Prefer established reusable mechanisms through narrow adapters; retain WriterCraft's author-note domain rules internally.

## Risks / Trade-offs

- Plain-text offsets alone may be insufficient for rich structural edits → review stable identity and edit mapping against real legacy fixtures.
- Repeated quotes can make text search ambiguous → preserve exact identity/range and never infer attachment from the first match.
- Uncertain deletions can erase useful notes → retain original quote/context with unresolved status.
- Independent text and anchor writes can diverge after failure → one transactional mutation/history boundary.
- A visible highlight can resemble a retained selection → verify persistent decoration and notes navigation after focus moves and after restart.

## Migration Plan

Before coding, agree the release scope, supported source cases and model compatibility. Then prepare additive migration/backup fixtures, preserve original archives and compare annotation/text semantics through import, edit, undo and export. A failed or unsupported migration leaves original projects intact. No migration is executed by this change.

## Decisions required before implementation

The owner chooses the next-release path and included workflows. Engineering then resolves stable text/block identity, native decoration/caret mapping, selection granularity, versioned storage/API shape, conflict copies and the exact acceptance fixture set. If those choices change existing behavior, add the corresponding spec deltas before implementation.

## Shared-core checkpoint (2026-11-08)

- Additive SQLite schema 4 stores passage notes separately from canonical rich JSON, append-only before/after audit events and attachment snapshots keyed by exact document revision. Opening schema 1–3 preserves existing documents, drafts and histories.
- The shared command surface captures a nonempty range within one stable text block, verifies its exact literal and document revision, and supports create/read/list/update/explicit reattach/delete/history. Note writes also require the current annotation revision. Cross-block note creation is not inferred from an assistant selection.
- Coordinates use CraftEditor Unicode scalars and its shared hard-break/inline-atom projection. Inline math occupies U+FFFC rather than either inconsistent legacy display spelling; offsets never cross projection boundaries implicitly.
- Verified canonical transactions map ranges using CraftEditor position maps. Whole-range replacement can update the quote; partial overlap, deletion and unknown snapshot edits detach while retaining the prior quote/context and authored lifecycle. Formatting and unchanged stable blocks retain attachment.
- Rich revision restore compensates attachment state under the same SQLite transaction while preserving later note edits, lifecycle and explicit deletion. Metadata and compensating events are retained instead of rewriting manuscript history.
- Native autosave must replay verified typed commands, not accept arbitrary caller-supplied position maps. Snapshot-only changed blocks remain conservatively detached until the verified journal bridge is integrated. Native Undo/Redo must identify actual history entries rather than locate an earlier matching quote or JSON snapshot.

Fourteen focused core fixtures were added for hosted qualification; they have not been run locally. Visible persistent marks, note panels, dictionary-link integration, transfer/copy retention and native interaction acceptance remain open.
