# Next release: approved editor-first scope

Status: **owner-approved implementation scope**, 2026-11-08. Target: `0.1.0-alpha.2`. The owner selected the complete current editor and explicitly rejected a reduced rich-text subset. The previously discussed tree/version/find/installation bundle was not selected as the next workstream.

Tracking: [complete native editor parity — issue #4](https://github.com/avacadorun-dev/writer-craft/issues/4).

## Required scope

The complete [source-backed editor matrix](../migration/EDITOR_PARITY_MATRIX.md) and [independent UX matrix](../design/EDITOR_UX_PARITY.md) define alpha.2. All 21 nodes, six marks, every E01–E27 behavior and C01–C12 shared command contract must be implemented and verified. Existing editor title/find/history/recovery/annotation/source/assistant affordances remain required even when broader adjacent product modules are deferred.

See [full-editor proposal](../../openspec/changes/full-editor-parity/proposal.md), [design](../../openspec/changes/full-editor-parity/design.md) and [tasks](../../openspec/changes/full-editor-parity/tasks.md). Persistent passage marks and notes are part of this selected scope; [issue #2](https://github.com/avacadorun-dev/writer-craft/issues/2) and its existing specification remain relevant. The separate reusable engine is [CraftEditor](https://github.com/avacadorun-dev/craft-editor), not vendored application code.

## Deferred workstreams

Broad project-type/tree administration, named-version workflow, project-wide search/replacement and installation packages follow the editor hypothesis. [Installation issue #3](https://github.com/avacadorun-dev/writer-craft/issues/3) stays open and unscheduled for implementation. Provider/recording workflows, Canvas, full project import/export and publishing remain in the product parity inventory; their deferral must not turn required editor integrations into fake controls.

## Internal milestones and release boundary

Source audit, model fixtures, a first native paragraph/table slice and individual block implementations are internal milestones. They never pass the final release by themselves. Keep required gaps explicit and complete them; do not remove a requirement because an upstream library lacks it.

The independent gate must name `full-editor-parity`, match the exact source/version and include actual compiled/native evidence for the full matrices. It is separate from full-product parity. No alpha.2 tag/publication before that acceptance. The published alpha.1 source/tag/assets remain immutable.

## Issue completion audit

On 2026-11-08, issues #1–3 were inspected: issue #1 has the source/runtime icon fix and packaged-signature evidence but lacks its final direct Dock check; issue #2 remains unimplemented; issue #3 remains unimplemented and deferred. No issue was closed from a specification or a passing compiler. Reaudit after each implemented task and before a release under [ISSUE_WORKFLOW.md](ISSUE_WORKFLOW.md).
