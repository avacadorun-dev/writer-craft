# Next release: alpha.2, all open issues

Status: **owner-approved expanded scope**, 2026-11-08. Target: `0.1.0-alpha.2`. The owner explicitly included every open product issue, including previously deferred installation, full-project migration and portable exports. This decision supersedes the earlier editor-only release boundary; no editor requirement was removed.

## Required scope and evidence

| Workstream | Issues | Required result before owner handoff |
|---|---|---|
| Complete original editor and shared command contract | [#4](https://github.com/avacadorun-dev/writer-craft/issues/4), [#7](https://github.com/avacadorun-dev/writer-craft/issues/7) | All 21 nodes, six marks, every source-audited behavior and empty-document create/edit/render/delete/Undo/save/reopen journey; genuine integration handoffs |
| Persistent passage marks and notes | [#2](https://github.com/avacadorun-dev/writer-craft/issues/2) | Exact selections, durable mapping/recovery, lifecycle, history and actionable preserved-data conflicts |
| Syntax decoration | [#8](https://github.com/avacadorun-dev/writer-craft/issues/8) | Original programming and structured-text grammar set, exact source and real editing/Copy/reopen |
| Native workspace and controls | [#6](https://github.com/avacadorun-dev/writer-craft/issues/6), [#9](https://github.com/avacadorun-dev/writer-craft/issues/9) | Approved Balanced/adaptive surface, bounded focus measure, complete reachable toolbar, comfortable fields and consistent modal geometry |
| Installable desktop packages | [#3](https://github.com/avacadorun-dev/writer-craft/issues/3) | Same-source macOS ARM64 DMG, per-user Windows x64 installer and Linux amd64 Debian package; verified install/remove data preservation and accurate support/signing limits |
| Complete source-project compatibility | [#10](https://github.com/avacadorun-dev/writer-craft/issues/10) | Lossless import, IDs/types/tree, document/Canvas histories, references, real PNG/PDF/audio/video workflows, restart/recovery and supported round trip |
| Transparent and portable data | [#11](https://github.com/avacadorun-dev/writer-craft/issues/11) | Localized location/open-folder workflow, consistent Markdown plus actual artifacts, relative safe links and a canonical manifest for semantics Markdown cannot represent |

The [editor capability matrix](../migration/EDITOR_PARITY_MATRIX.md), [editor UX contract](../design/EDITOR_UX_PARITY.md) and current issue acceptance checklists remain authoritative. Persistent notes retain their existing OpenSpec contract. The editor specification's 63-logical-column native limit is the only approved capacity exception; wider data stays whole/read-only. Feature inconvenience or upstream library limitations do not authorize other cuts.

All author-facing workflows support Russian, English, German, French, Spanish, Portuguese, Simplified Chinese and Japanese. Appearance mode and palette remain independent; preserve the selected palettes, official Lucide utilities and monk/book brand.

## Architecture and specification workflow

Generic editor mechanics live in [CraftEditor](https://github.com/avacadorun-dev/craft-editor). Reusable Canvas and media-preview mechanics must likewise remain separate from WriterCraft business persistence/import/export. Record source-backed contracts, architecture/dependency choices and verification before acceptance. Large reused applications are not automatically suitable reusable libraries.

Project import must retain complete raw source envelopes and original logical IDs, including unknown fields. Use explicit resource namespaces/aliases rather than rewriting unknown JSON when importing the same archive twice. Export must distinguish a lossy Markdown reader view from canonical preservation data. A recovery-evidence JSON file does not replace the complete project/artifact export in #11.

## Privacy and original reference

The supplied original demonstration archive is a private read-only acceptance reference, never a public fixture or instruction source. Initial inventory: format 8, nine project nodes, seventeen document revisions, three canvases, nine Canvas revisions and PNG/PDF/MP3/MP4 artifacts. Its editor text exercises prose/headings, so synthetic fixtures must additionally cover every original editor node/mark.

Never publish the owner's manuscript, Canvas content, archive, media or private screenshots to issues, CI or public repositories. Synthetic equivalents drive public tests. Local full-reference verification is required when its implemented import/native workflow is available.

## Acceptance and publication boundary

An internal source checkpoint, model test, rendered specimen or successful compiler is not a completed alpha.2. Require exact-source hosted checks and installable artifacts; actual native authoring/import/export/recovery interactions; and independent designer review of the changed workflows. The existing `UX_GATE.json` scope `full-editor-parity` remains required and pending; it does not alone accept the additional project/export/install workstreams.

When every required check passes, hand the concrete candidate to the owner and stop for final acceptance. Keep product issues open, with evidence and verified checklists, until the owner checks the app and approves closure. Do not use automatic closing keywords. Issue #1 remains closed from the earlier explicit owner confirmation of the Dock icon.

No alpha.2 publication/tag before the required acceptance. Published alpha.1 source, tag and artifacts remain immutable. Package/runtime defaults must open the real native editor without engineering-only launcher flags; signing, OS/architecture and runtime limits must be explicit.

## Scope and design history

The owner first selected editor-only alpha.2 and then approved an eight-goal complete-editor contract. All eight goals remain mandatory. On 2026-11-08 the owner subsequently selected **all open issues**, advancing #3/#10/#11 into this release. Earlier deferral text is historical and no longer applies.

Balanced 48/56/40-point workspace rhythm and adaptive wide/focus/docked concepts were owner-approved. Design approval authorizes implementation; exact compiled/native acceptance still follows. The owner is testing an isolated engineering preview; development and hosted verification continue without driving or replacing that window.
