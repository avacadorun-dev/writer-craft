# WriterCraft architecture decisions

Accepted decisions as of 2026-11-07. Each records an actual chosen direction and consequence. Replacing one requires a new ADR that names the superseded decision; do not silently rewrite history. Implementation or release verification remains tracked in the active change.

## ADR-001 — Native Rust application with shared core

Status: accepted. Context: the product is moving from server deployment to a desktop authors install directly. Decision: use Rust application code, egui/eframe native rendering and a GUI-independent command core shared with CLI and future MCP. Alternatives considered: retain a browser shell; wrap existing UI in Tauri; rewrite every subsystem immediately. Consequence: installation can avoid the server stack, while the editor and native accessibility require real implementation and validation. A Rust text widget is not rich editing parity.

## ADR-002 — Local SQLite, immutable revisions and durable drafts

Status: accepted. Context: current persistence uses PostgreSQL/S3 and browser recovery. Decision: local SQLite with foreign keys, WAL, FULL synchronous mode, bounded busy timeout and immediate write transactions; append document revisions and capture unfinished drafts separately. Versioned additive migrations reject newer schemas and foreign databases. Alternatives: bundle database/storage servers; save only ad-hoc text files; overwrite a single current snapshot. Consequence: users need no database administration; long I/O must leave the render thread, and power-loss/backup guarantees need separate evidence.

## ADR-003 — Proposal staging is separate from commit authority

Status: accepted. Context: suggested edits must preserve authorship and concurrent manual writing. Decision: exact expected text and Unicode-scalar anchors; separate before/proposed snapshots with integrity hashes; explicit Apply/Reject; atomic base checks; compensating Undo guarded by the applied revision/body. Alternatives: direct provider writes; text-matching without revision checks; unconditional restore. Consequence: ambiguous/stale operations fail visibly, and newer work survives undo conflicts. Multi-document groups remain later work.

## ADR-004 — Preserve imported structure before editable migration

Status: accepted. Context: legacy transfer versions1–8 contain editor schemas1–4, histories, audio/assets, annotations, Canvas and context. Decision: bounded read-only ZIP inspection first; retain complete supplied JSON and original archive; imported rich preview stays read-only. Alternatives: flatten every document to plainText; claim a full import from partial DTO parsing; copy server storage into the installer. Consequence: initial import usefulness is limited but data is preserved. Full migration needs reference mapping, atomic asset/database staging and semantic round-trip tests.

## ADR-005 — Independent brightness and non-green palette identity

Status: accepted product direction; implementation acceptance pending. Context: sustained writing requires Light/Dark and requested legacy themes. Decision: one semantic token system with separate brightness and palette IDs; retain cedar-and-copper, dried-rose-and-wine, fog-graphite, linen-clay, dusty-lavender and porcelain-ink; native charcoal/blue remains available. Exclude green-accent quiet and olive-ink. Alternatives: a single hardcoded dark theme; couple each palette to one brightness; restore excluded palettes. Consequence: preferences and every panel/dialog must consistently resolve mode/palette; pairwise visual acceptance is required.

## ADR-006 — Incremental native delivery, complete parity target

Status: accepted. Context: a complete rewrite spans rich editing, audio, Canvas, assistants and publishing. Decision: deliver a bounded plain-prose MVP with an explicit parity inventory, then migrate complete workflows against acceptance fixtures. Alternatives: freeze delivery until a full rewrite; announce equivalence from a basic editor. Consequence: limitations are visible, original projects stay unchanged, and release claims depend on passing workflow evidence.

## ADR-007 — Repository-local OpenSpec change contracts

Status: accepted. Context: agents need durable product intent and bounded changes beyond chat history. Decision: adopt [OpenSpec's default artifact structure](https://github.com/Fission-AI/OpenSpec/blob/main/schemas/spec-driven/schema.yaml), [config conventions](https://github.com/Fission-AI/OpenSpec/blob/main/docs/customization.md) and WHEN/THEN capability scenarios, connected to PRD, ADRs and named test evidence. Alternatives: chat-only requirements; parallel unmaintained planning frameworks; a heavyweight custom process. Consequence: manual bootstrap is useful immediately; official CLI initialization/validation must be recorded honestly when executed. Existing user authorization remains valid throughout the workflow.

## ADR-008 — External reusable engines, internal writing domain

Status: accepted. Context: rich editing, text layout, document formats, transcription runtimes and other substantial generic engines can outgrow a single application's business code. Decision: prefer a maintained existing specialized library. When new generic engine work has a real reuse boundary and substantial independent functionality, develop it in a separately versioned repository with its own API, tests, documentation, provenance and releases; WriterCraft consumes a pinned package/revision through a narrow adapter. Keep WriterCraft project/manuscript invariants, review authority, workflow policy and application commands in this repository. Alternatives: copy a large generic engine into business modules; split every helper into a repository; create speculative packages with no current consumer. Consequence: current domain crates stay internal, no speculative new repository is required, and a future extraction must identify an actual reusable contract and maintenance owner before it changes dependencies.

## ADR-009 — Eight built-in UI languages with explicit source preservation

Status: accepted product requirement; integrated acceptance pending. Context: the native MVP must support Russian plus seven additional interface languages immediately. Decision: bundle ru/en/de/fr/es/pt/zh/ja catalogs, switch semantic messages live, persist the selected language, and provide offline CJK glyph coverage. Keep manuscript/user titles/quotes unchanged and identify original technical diagnostics separately. Missing/invalid preferences use Russian with an actionable visible recovery state; unknown translation keys expose a neutral diagnostic rather than silently showing Russian in another locale. Alternatives: remote runtime translation; partial labels; translating stored user content; postponing internationalization. Consequence: catalog/parameter tests, hardcoded-string review and native CJK/layout verification are release checks, not assumptions.

## ADR-010 — macOS first full desktop release

Status: accepted product rollout. Context: native integration, fonts, installers and interaction behavior need direct platform evidence. Decision: deliver and verify the first full desktop release on macOS; keep the core portable and its CI useful, then add Windows/Linux desktop delivery with their own UI, dialog, accessibility and installer acceptance. Alternative: imply simultaneous three-platform readiness from a portable Rust build. Consequence: current releases name actual platform support, while platform-specific adapters remain separate from writing-domain rules.

## ADR-011 — Continuous manuscript writing, publication-stage pagination

Status: accepted product interaction. Context: authors compose and revise long text within one bounded writing workspace. Decision: use a continuous text flow with editor-local scrolling; pagination and page-size/margin layout appear in publication preview/export. Alternative: split the primary writing surface into simulated paper pages. Consequence: editor structure follows writing operations and stable document identity; print layout is a later rendering concern and must not redefine canonical text or caret behavior.
