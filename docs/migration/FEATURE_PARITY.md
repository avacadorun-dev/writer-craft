# Migration inventory and acceptance

WriterCraft is the native Rust migration of Writers Studio. This table distinguishes current source implementation from completed native acceptance; it is not a full-parity or release claim. Exact committed builds, passing scopes and failures are recorded in [alpha.3 progress](../verification/ALPHA_3_PROGRESS.md), the linked issues and the independent pending gate. The original repository and private projects remain read-only.

| Workstream | Current implementation and bounded evidence | Remaining acceptance |
|---|---|---|
| Local projects and hierarchy | SQLite projects; imported scoped parent/order metadata; native nested navigator/search; shared source-typed node creation and first-content commands plus aggregate project creation added for hosted qualification | Structure-aware native authoring, move/reorder/trash/restore, dynamic schema workflows and researched creation profiles (#10/#14); new commands are not yet a verified UI workflow |
| Rich manuscripts | Canonical21-node/six-mark model, native WordCraft projection, real image/Mermaid/formula rendering and block authoring; qualified table-local overflow foundation | Whole-table selection integration, complete original input/clipboard/formatting/resize journeys and exact native interaction matrix (#4/#7/#14);63 logical columns is the sole approved native capacity exception |
| Working drafts | Revision-pinned rich drafts with replay-derived annotation checkpoints; Canvas durable candidate/recovery flow | Complete failure/Cancel/stale/IME/restart behavior, pre-ack card input and acknowledged transfer prerequisites (#2/#4/#10/#15) |
| Revisions | Immutable text/rich/Canvas revisions and guarded replay; imported source histories/named versions retained | Complete native named-version and cross-module history/recovery journeys (#4/#10) |
| Reviewed changes | Exact retained attachments, detached typed proposals, scoped stage/Apply/Reject/compensating Undo and native before/after views | Actual supplied-proposal handoffs and complete selection/protected-range/native lifecycle acceptance (#4); do not invent model output |
| Source data | Immutable original envelopes, scoped resource aliases, unknown-domain evidence and original owned bytes | Full private-reference native migration and lifecycle acceptance; preserving JSON alone does not implement its workflow (#10) |
| Studio archive import | Pinned versions1–8 archive validation and cancellable transactional whole-project import with native confirmation | Actual complete-project/media/Canvas interactions, cancellation/conflict/restart and original reference round trip (#10) |
| Canonical portability | Bounded paged format2 export/preflight/semantic restore and pinned ZIP reader; source-compatible format1 restore; Markdown/artifact folder transport added for hosted qualification | Safe selected scope, native export/restore UI, original artifacts/reader limitations, full workload/RSS and exact failure/native acceptance (#11) |
| Dictionary, guide and memory | Imported typed-domain records, histories/references and unknown fields preserved | Real term chooser/lifecycle and native guide/memory/context workflows; retained records do not replace editing/search/linking (#4/#10) |
| Passage annotations | Persistent exact Unicode notes, mapped marks, detached/reattached lifecycle and history, source-backed24000-scalar capacity | Complete native note/dictionary/source-link/review/backup journeys, stale/failure and independent input acceptance (#2/#4) |
| Canvas | Native scoped view, source-compatible graph/history/ack/recovery, move/resize and direct in-card TextEdit; real project-owned raster/PDF integration | Full GFM/card/schema/drawing/connector/media authoring, creation latency, clipped input, source cancellation semantics and native complete-project journeys (#10/#15) |
| Media and source playback | Original image/PDF/audio/video bytes and aliases preserved; qualified raster/PDF worker foundation; optional generic audio-source decoding qualified separately | Animation, actual device audio/video/segment playback, source navigation and original codec corpus; no playback claim from stored bytes/first-frame raster (#4/#10) |
| Assistant/MCP integration | Shared canonical core and typed attachment/review commands callable headlessly | Editor handoff and canonical outcomes must be exercised; provider connectivity, recording/transcription and released MCP transport are separate product milestones unless explicitly selected by an issue |
| Publishing | Plain reader text plus source publishing metadata preservation; portable reading Markdown work tracked in #11 | DOCX/EPUB/PDF publishing is a separate product workflow; source metadata retention is not output support, and alpha.3 requires every specifically selected editor/export criterion |
| Native distribution | Immutable published alpha.2 installers and package checks; installer request #3 completed | Exact final alpha.3 source/tests, compiled independent gate, installers/README and final acceptance; credentials/signing limits remain explicit |

## Legacy transfer facts

Implementation sources in the original repository:

- `internal/httpapi/project_transfer.go:25–29,170–312`: `.wstudio` ZIP, `project.json`, declared binary entries, 512 MiB total and 64 MiB manifest limits, checksums and MIME validation.
- `internal/store/project_transfer.go:25–55`: transfer format `writers-studio-project`, current version8, accepts1–8.
- `internal/editor/document.go:17–51,94–119`: editor JSON schemas1–4, independent of transfer version.
- `internal/editor/document.go:588–657`: exact anchors use Unicode code points; Rust wire offsets therefore use `.chars()` positions.
- `internal/store/project_transfer.go:1230–1614`: tree, revision, media, annotation and cross-reference invariants.

Transfer version gates: v2 annotations; v3 images; v4 publishing profile; v5 Markdown source frontmatter/media paths; v6 Canvas; v7 PDF media; v8 project audio/video media and Canvas type schemas. `.wstudio` includes guide/memory but does not export assistant conversation/change-set history or provider credentials.

Rich schema gates: v1 prose/lists/basic formatting; v2 code, tables, images, Mermaid, callouts; v3 headings4–6; v4 inline and block formulas. Plain text is a derived projection. Flattening imported JSON into fresh paragraphs destroys provenance and is not a valid lossless migration.

## Core checks

The core test suite covers restart persistence, canonical nonmutation during staging, exact Cyrillic/emoji/combining-character offsets, overlapping literal ambiguity, stale Apply, guarded Undo, proposal lifecycle persistence, cross-connection optimistic saves, additive schema migration and future-schema refusal, durable working draft recovery, exact-match draft cleanup, read-only imported JSON preservation, no-op saves and refusal to modify a foreign SQLite database.

Before claiming complete `.wstudio` migration: import all versions1–8; preserve every revision, named version, annotation, transcript artifact, original audio, image/PDF/audio/video asset, Canvas/type schema, guide/memory revision and unknown supported metadata; verify checksums and references; reject unsafe archives transactionally; export and compare a semantic roundtrip.
