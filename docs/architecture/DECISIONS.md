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
