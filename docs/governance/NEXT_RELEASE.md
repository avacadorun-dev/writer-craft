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

On 2026-11-08, issues #1–3 were inspected: issue #1 was resolved after the source/runtime icon fix, packaged verification and owner confirmation of the correct Dock icon; issue #2 has verified core/anchor/replay work but native marks/notes acceptance is pending; issue #3 remains unimplemented and deferred. No issue was closed from a specification or a passing compiler. Reaudit after each implemented task and before a release under [ISSUE_WORKFLOW.md](ISSUE_WORKFLOW.md).

## Approved readiness and issue map (2026-11-08)

The owner explicitly accepted the eight-goal readiness contract and issue-driven execution. Start with an empty document in the compiled application, exercise every original editor capability, save, reopen and preserve the result. A showcase, model test, partial toolbar or successful compiler run is not full editor acceptance. Missing required functionality blocks alpha.2.

| Approved goal | Tracking | Current acceptance status |
|---|---|---|
| Complete source-audited editor parity and shared headless contract | [#4](https://github.com/avacadorun-dev/writer-craft/issues/4) | Open; all matrix cases remain required |
| Create/configure/edit/render/delete/history/save/reopen for every block and all formatting | [#7](https://github.com/avacadorun-dev/writer-craft/issues/7) | Open; rendered examples do not replace authoring |
| Full text/table interaction, rich clipboard, keyboard and native input | #4 and #7 | Open; only approved capacity exception is 63 logical table columns |
| Code and structured-format syntax highlighting | [#8](https://github.com/avacadorun-dev/writer-craft/issues/8) | Open; audit original grammar set and preserve source/language |
| Persistent marks/notes and genuine dictionary/source/assistant handoffs | [#2](https://github.com/avacadorun-dev/writer-craft/issues/2), #4 | Core partly qualified; visible/integrated workflows pending |
| Adaptive floating/docked bounded document surface, including focus | [#6](https://github.com/avacadorun-dev/writer-craft/issues/6) | Balanced approved; adaptive concept/native acceptance pending |
| Comfortable inputs and consistent modal header/body/footer spacing | [#9](https://github.com/avacadorun-dev/writer-craft/issues/9) | Open; shared components and native verification required |
| Eight languages, independent themes/palettes, Lucide/monk and independent UX review | #4, #6, #9; UX_GATE.json | Required exact-source full-editor acceptance pending |

Track implementation and evidence in these issues. Search open/closed issues before adding a new finding; close only when every issue-specific acceptance condition is verified. No issue was closed at this approval checkpoint.

## Original demonstration project as migration reference

[Issue #10](https://github.com/avacadorun-dev/writer-craft/issues/10) records the owner's mandatory eventual compatibility reference. The supplied private archive is read-only acceptance data, never a public fixture or instruction source. Its editor portion informs alpha.2 now; complete project import and all attached Canvas/media workflows remain required for eventual product parity. Preserve and track deferred features explicitly rather than flattening or dropping them.

An initial inventory found format version 8, 9 project nodes, 17 document revisions, 3 canvases, 9 canvas revisions and four media attachments (PNG/PDF/MP3/MP4). Its editor revisions exercise prose/headings; they do not replace synthetic coverage of all 21 nodes and six marks. No manuscript or binary media was copied into either public repository.

On 2026-11-08 the owner explicitly confirmed the reading-monk Dock icon in the latest build. [Issue #1](https://github.com/avacadorun-dev/writer-craft/issues/1) was closed with source/build/owner evidence. Issues #2–4 and #6–11 remain open; no editor, installer or full-project acceptance is inferred from that icon resolution.

[Issue #11](https://github.com/avacadorun-dev/writer-craft/issues/11) now tracks transparent data locations and portable Markdown export with real attached artifacts. Storage/preservation compatibility is an immediate architecture requirement. Broader project export must preserve unsupported Markdown semantics in a canonical bundle/manifest and document conversion behavior; exact UI implementation staging remains explicit.

The owner approved the adaptive wide/focus/docked concept images for issue #6 on 2026-11-08. Implement the distinct centered document surface when space allows, retain the bounded readable measure in focus, and join the pane at constrained widths. This design approval is not native acceptance.
