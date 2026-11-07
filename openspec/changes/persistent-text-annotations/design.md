# Persistent annotation design brief

Status: future planning. The owner has not selected a release, version or implementation approach. This brief records invariants and the decisions needed before implementation; it is not a ready-to-execute technical plan.

## Context

See proposal.md and `docs/migration/PERSISTENT_ANNOTATIONS.md`. Legacy annotations are separate author metadata with stable block/range anchors, exact quote/context, lifecycle and history. Current native writing uses a plain text buffer and revisioned proposals; an ephemeral selection cannot stand in for a persistent note.

## Goals / Non-Goals

Goals for a later agreed increment: durable visual attachment and authored note text, exact Unicode identity, deterministic mapping, unresolved-note preservation, transactional text/anchor history and source-backed compatibility.

Current-alpha implementation, a selected next version and new unrelated domain features are outside this planning change.

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
