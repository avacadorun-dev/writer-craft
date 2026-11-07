# Persistent text annotations

Status: proposed future legacy-parity work. Not scheduled, not approved for implementation and not part of `0.1.0-alpha.1`. The owner will choose next-release scope after first-alpha findings.

## Why

An author needs a selected passage to remain visibly marked with an attached note after attention moves elsewhere and across sessions. Existing Writers Studio already has durable manuscript annotations; the native alpha's temporary selection does not provide that workflow.

## What Changes

- Introduce durable selected-passage marks with attached authored note/reminder text.
- Preserve exact Unicode-aware anchor identity through known edits, repeated quotes, reopening and undo.
- Preserve deleted or uncertain anchors as reachable unresolved notes rather than guessing another occurrence.
- Keep notes outside canonical reader text and preserve source-backed migration/history guarantees.

This is a planning candidate. It does not add a standalone NER feature or schedule notification behavior. Current-alpha bug fixes and release preparation continue independently.

## Capabilities

### New Capabilities

- `persistent-text-annotations`: durable author marks/notes with exact attachment, lifecycle and edit recovery.

### Modified Capabilities

None selected yet. Any required editor-model, migration or assistant-review contract changes must be identified in the eventual agreed implementation scope before coding.

## Impact

Potentially affects the native document model, annotation metadata/history, editor decorations, notes navigation and migration. Exact persistence/editor choices are deferred to joint scope review. No application source or schema changes are authorized by this proposal alone. Preserve original projects, reader exports and current working drafts.

Candidate tracking and release alternatives: `docs/governance/NEXT_RELEASE.md`; detailed source-backed behavior: `docs/migration/PERSISTENT_ANNOTATIONS.md`. The owner issue is being recorded separately; link its actual URL when available.

Tracking: [persistent text marks and author notes — issue #2](https://github.com/avacadorun-dev/writer-craft/issues/2).
