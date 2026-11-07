# Persistent annotation planning and future acceptance

Status: proposed and unscheduled. Checked planning artifacts do not authorize feature implementation or place this work in `0.1.0-alpha.1`.

## 1. Planning

- [x] 1.1 Record the owner's durable mark/note clarification and source-backed legacy behavior.
- [x] 1.2 Draft behavioral scenarios and preserve unresolved next-scope decisions explicitly.
- [x] 1.3 Link the actual duplicate-triaged owner-authored GitHub enhancement issue.
- [ ] 1.4 Choose the next release scope and version jointly after first-alpha findings.
- [x] 1.5 Validate with official OpenSpec 1.16.1: `validate --all --strict --json` passed all 7 items on 2026-11-08; see `docs/governance/NEXT_RELEASE.md`.

## 2. Before any implementation

- [ ] 2.1 Agree document identity, edit mapping, supported selection shape and native decoration behavior.
- [ ] 2.2 Define additive schema/API compatibility, conflict copies, undo/history and migration fixtures; add affected capability deltas.
- [ ] 2.3 Select a bounded accepted implementation scope and independent native UX acceptance plan.

## 3. Future implementation and evidence, only after 1.4 and 2.3

- [ ] 3.1 Implement persistent author-note metadata and exact revision-pinned attachment (PA-01, PA-02).
- [ ] 3.2 Implement deterministic mapping and unresolved-note preservation with Unicode/repeated-quote/deletion fixtures (PA-03, PA-04).
- [ ] 3.3 Implement explicit reattachment/lifecycle/history and transactional failure handling (PA-05, PA-06).
- [ ] 3.4 Verify focus changes and application restarts retain visible marks and note text; reader exports preserve manuscript semantics.
- [ ] 3.5 Verify compatible legacy transfer, conflict copies and undo; obtain independent review of the actual selected scope.

No annotation implementation tests have run or are claimed by this planning document.
