# Migration inventory and acceptance

WriterCraft is an independent native Rust MVP. Existing Writers Studio remains the source of migration contracts and is not modified. A read-only import preview is not full feature parity.

| Feature | Native MVP state | Remaining acceptance |
|---|---|---|
| Local projects | Implemented in SQLite | More metadata, custom project structures |
| Plain text documents | Implemented with optimistic saves | Rich document editor, tree/reorder, formatting |
| Working drafts | Durable revision-pinned recovery, exact-match cleanup on save | Recovery UX for drafts whose canonical base changed |
| Revisions | Immutable text snapshots | Named versions and complete original rich history |
| Exact reviewed replacement | Stage / Apply / Reject / guarded Undo | Grouped multi-document changes, protected ranges |
| Imported JSON | Opaque value retained; read-only preview | Canonical rich migration and local binary asset storage |
| `.wstudio` archive | Transfer inspection module | Full transactional import, reference remapping, complete round trip |
| Dictionary / guide / memory | Not implemented | Typed domain records and revision histories |
| Anchored annotations | Preserved only within supplied import JSON | Unicode anchors, reanchoring, detached states |
| Dictation / audio | Not implemented | Native capture, durable chunks, review, transcription providers |
| Assistant providers | Not implemented | Explicit provider setup, scoped typed tools, real model calls |
| Canvas | Not implemented | Native interactions, canonical JSON Canvas and extensions |
| Publishing exports | Plain text only where exposed by CLI | DOCX / EPUB / PDF, rich assets, diagrams, formulas |
| Automation / MCP | Shared core is callable headlessly | Versioned MCP/CLI protocol and capabilities |
| Native distribution | Build and packaging pipeline maintained elsewhere | Signing/notarization, platform releases and updates |

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
