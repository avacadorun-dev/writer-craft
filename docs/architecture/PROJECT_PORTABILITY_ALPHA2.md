# Complete project portability and native media plan

Status: source audit and proposed implementation contract, 2026-11-08. Issues [10](https://github.com/avacadorun-dev/writer-craft/issues/10) and [11](https://github.com/avacadorun-dev/writer-craft/issues/11) remain open. The owner has now included all open issues in alpha.2, including full project import, native Canvas and attached media, portable export and installation. Earlier editor-only deferments in AGENTS.md, NEXT_RELEASE.md and the editor proposal are superseded by this instruction and must be reconciled by the coordinating agent. This plan does not claim implementation or native acceptance.

## Source facts and current gaps

The original working checkout is read-only. Its HEAD is `24ac9ef222700a6e3d618139b701e53abc61d6a5`; the eight source files listed below are clean at that HEAD even though the overall owner checkout is dirty. No private manuscript, archive, media, screenshots or owner path belongs in this repository or hosted CI.

| Source | Relevant behavior |
| --- | --- |
| Original `internal/store/project_transfer.go:25–287,1230–1614` | Format `writers-studio-project`, envelopes 1–8; complete project/node metadata, all document revisions, named versions, dictionary/annotations, dictation/artifacts/drafts/audio, media, Canvas/type-schema history, guide/memory history and cross-reference validation |
| Original `internal/httpapi/project_transfer.go:25–29,170–312` | ZIP `project.json` plus declared bytes; 512 MiB archive/expanded total and 64 MiB manifest; size/hash checks; the Go typed decoder rejects unknown top-level fields, which native preservation must improve rather than imitate |
| Original `internal/canvas/document.go:14–259` and `schema.go` | JSON Canvas nodes/edges with schema 1/2 and retained `x-writers-studio` extensions; resource, card/type, drawing/frame and graph constraints |
| Original `src/entities/canvas/model/types.ts` | Unknown keys are explicitly allowed at document/node/edge/extension levels; card fields, promoted document, lock, preview and active-view metadata are real persisted data |
| Original `src/widgets/canvas-board/ui/canvas-board.tsx:464–664,3585` | Actual image view, PDF first-page render with open action, native browser audio/video controls, PNG/JPEG/GIF/PDF/MP3/M4A/Ogg/WAV/MP4/WebM chooser |
| Original `internal/export/markdown_tree.go` and `internal/markdowntree/path.go` | Ordered tree, Unicode NFC/case-fold collision keys, portable path allocation, relative document/media links and original bytes; the source Markdown fallback can silently use plain text, so native export must report conversion differences explicitly |
| Native `crates/studio-transfer/src/lib.rs:1–406` | Bounded ZIP directory/path/hash checks and full manifest Value, but only current document previews; no full tree/Canvas/annotation validation, transactional project import or asset persistence |
| Native `crates/studio-core/src/schema.sql`, `rich.sql`, `assets.sql` | Flat project/document API; immutable rich/text history; schema 5 generic immutable asset BLOB envelope; schema 6 proof-derived draft annotation checkpoints |
| Native `crates/studio-desktop/src/app/navigator.rs` and `dialogs.rs` | Flat document navigation and archive inspection modal; Open .wstudio currently does not migrate the project |
| Native `crates/studio-desktop/src/main.rs` | Active database comes from CLI/env/default selection; UI must display this actual path rather than recompute the default |

The private read-only reference has format 8, 9 project nodes, 17 document revisions, 3 canvases, 9 Canvas revisions and 4 assets. Local metadata-only inspection confirms MP3 audio and H.264 plus AAC inside MP4; no private media was extracted or uploaded. Canvas history includes all four JSON Canvas node kinds, frames/shapes/strokes and document/media/nested-Canvas references. It contains no named versions, dictionary terms, dictation sessions or project-memory records, so public synthetic fixtures must cover those absent domains. Current reader inspection and a prose-only rendering cannot qualify this project.

## Data model and stable identity

Use additive SQLite schema 7 after schema 6. Keep existing document, rich history, draft/recovery, proposal and image APIs backward compatible. Do not rebuild existing tables or rewrite original imported JSON merely to accommodate global native IDs.

Introduce project-scoped logical identities and explicit aliases:

- `project_metadata(project_id, source_project_id, metadata_json, structure_revision, structure_json, source_manifest, source_manifest_sha256)` retains complete source project/envelope data.
- `project_resource_aliases(project_id, resource_kind, logical_id, native_id)` has a unique project/kind/logical key. Original IDs remain the public logical IDs in canonical JSON and canonical export. Native UUID handles continue to identify existing document/asset/annotation rows internally. Importing the same archive twice creates independent project namespaces without changing either source identity graph.
- `project_nodes(project_id, logical_id, native_document_id NULL, parent_logical_id NULL, kind, status, position, current_revision, metadata_json)` models content and non-content nodes. Parent edges are same-project and acyclic. Preserve array order as a tie-breaker; do not derive it from title sorting.
- `project_source_records(project_id, domain, logical_id, ordinal, source_json)` retains complete raw record envelopes for every imported historical record and unsupported field. Known mutable records get typed tables/projections, not a second detached canonical authority.
- `project_canvases`, `project_canvas_revisions`, `project_canvas_drafts` and `project_canvas_schema_revisions` store complete JSON Canvas snapshots and type-schema history. Revisions are append-only; expected-base checks and compensating restores match document behavior.
- `project_named_versions`, dictionary/annotation metadata, dictation/session/artifact/draft/revision/audio, guide/revisions and memory/revisions preserve every source domain and relationship. Existing passage-note persistence is reused through scoped aliases and the source-compatible anchor adapter, not duplicated with a conflicting note authority.
- Asset aliases resolve an original media ID to an immutable project-owned `project_assets` row. Preserve original MIME/name/sourcePath/pageCount/duration and unknown media metadata in the raw record. Do not apply the 12 MiB image policy to PDF/audio/video/opaque attachments.

A complete import cannot call `create_rich_document` in a loop: it must insert every source ordinal, base revision, source, timestamp and raw envelope in one guarded transaction, including contentless nodes whose currentRevision is zero. Original revision/source metadata lives beside the existing projection history, whose source enum is deliberately narrow. Old source schema snapshots remain exact; a current editable working view may normalize through CraftEditor only if the original snapshot remains retained and the first edit appends a new revision.

Retain exact manifest bytes before parsing. Opaque JSON must not pass through a lossy floating-point representation. Use RawValue-backed source envelopes and an explicitly reviewed arbitrary-precision JSON representation for modified known records; unknown data is copied from its original envelope. Duplicate object keys are ambiguous and must fail validation rather than silently lose one value. Changing JSON whitespace is not a content change, but unknown values and IDs must survive semantic round trip.

Every public project command accepts a scoped resource reference. A string in imported JSON never authorizes global document/asset lookup. Existing native IDs remain available to legacy API callers; new project navigation and transfer adapters resolve logical IDs through the project capability. Renderer workers receive already-authorized bytes and logical-to-native lookup capabilities, never an arbitrary database path or file/network URL.

## Atomic import and shared commands

`studio-core` owns project-domain DTOs, semantic validation and all mutation/persistence. `studio-transfer` depends on core, never the reverse, and owns ZIP/path/checksum I/O, deterministic export conversion and path allocation. The core validator produces an immutable `ValidatedProjectImport` from original manifest bytes. Transfer holds that value plus a bounded reader capability tied to its opened archive. Core consumes the value and reader through one bulk import command; UI and CLI call that same command. Adding the existing rusqlite blob feature permits attachment streaming into transaction-owned zeroblob rows with checksum verification rather than accumulating all archive bytes in memory.

The reader must pin an opened source file, not reopen an unchecked path after inspection. Verify each entry length, ZIP CRC and SHA-256 while copying it to durable asset storage; never trust a prior preview when the source could have changed. Parse and validate all bounded tree/revision/source/annotation/Canvas/type-schema references before visibility. All rows and asset bytes commit together in one immediate transaction. Error, cancellation, disk exhaustion, malformed input or a bad final checksum leaves no visible partial project. Long I/O runs on bounded workers; cancellation ownership is explicit.

Preserve current 512 MiB archive/expanded and 64 MiB manifest limits. Add explicit limits for every domain count, depth, JSON nesting, total point count and aggregate memory. Match source limits where applicable: 20,000 nodes, 100,000 document/Canvas revisions, 2,000 canvases; Canvas 5,000 nodes, 10,000 edges, 12 MiB snapshot, 4,096 points/stroke and 200,000 total stroke points. Import integrity and authoring decoder capacity are separate: valid opaque resources are preserved even if a preview is unsupported. Future envelope versions fail unchanged with an actionable explanation.

Proposed core facade:

```rust
pub struct ProjectResourceRef { pub project_id: String, pub kind: ResourceKind, pub id: String }
pub fn import_project(&self, project: &ValidatedProjectImport, assets: &mut dyn ImportAssetSource, cancel: &Cancellation) -> Result<ImportedProject>;
pub fn project_tree(&self, project: &str) -> Result<ProjectTreeSnapshot>;
pub fn canvas(&self, resource: &ProjectResourceRef) -> Result<CanvasSnapshot>;
pub fn apply_canvas_command(&self, resource: &ProjectResourceRef, command: CanvasCommand) -> Result<CanvasRevision>;
pub fn save_canvas_draft(&self, resource: &ProjectResourceRef, base: Revision, expected_draft: Option<DraftVersion>, content: Value) -> Result<CanvasDraft>;
pub fn project_export_snapshot(&self, selection: ExportSelection) -> Result<ProjectExportSnapshot>;
pub fn project_assets(&self, project: &str) -> Result<ProjectAssetSource>;
```

These are proposed boundaries, not an instruction to expose public deserializable proofs or add a new cancellation dependency. Existing typed errors and version/proof ownership remain authoritative. Native and headless commands must produce the same revisions, aliases and asset references.

## Native Canvas and actual media

The approved reusable boundary is two separate repositories, pending final API/dependency review and creation by the coordinating agent:

| Component | Generic responsibility | WriterCraft responsibility |
| --- | --- | --- |
| `craft-canvas` | JSON Canvas model preserving opaque fields; viewport transform/culling/hit testing; node/edge selection; pan/zoom, move/resize, connectors, text/group/link/file cards, drawing shapes/strokes/frames, command history and egui view | Project/tree/type-schema semantics, source-style card fields/views/promotions, resource ownership, canonical SQLite revisions/drafts, localized actions and navigation |
| `craft-media-preview` | Authorized immutable byte source; bounded image/PDF/audio/video probing/decoding; PDF page textures; playback clock/seek/pause/volume/end/error; egui controls and cancellation | Asset identity and scope, source URLs/links, location/external-open policy, persistence, allowed format policy and user messages |

Canvas must open all imported boards, navigate referenced documents/nested boards, display/edit text and known drawing/card/edge metadata, use real attached PNG/PDF/audio/video, and save/Undo/Redo/reopen/recover revisions. Preserve unknown document/node/edge/extension/card fields through every known edit. Media control pointer/keyboard input must not drag its card. Read-only unsupported content remains whole and is identified; it never becomes an editable blank card or a fabricated image.

Proposed generic Canvas API uses `CanvasDocument`, `CanvasTransaction { base_revision, commands }`, `CanvasChange { before, after, mappings }` and `CanvasView::show(ui, session, host) -> events`. The host resolves opaque resource refs into authorized previews and handles navigation requests. The generic engine never dereferences file/URL strings. App revisions commit only after a validated engine change. Persistent draft/restore conflicts keep both versions recoverable.

Proposed media API uses `PreviewSource { bytes: Arc<[u8]>, sha256, media_type }`, `PreviewBudget`, `probe`, `render_pdf_page`, `open_playback` and a session accepting play/pause/seek/volume/close. Results carry exact source identity and request epoch. PDF/image results are disposable caches; AV uses a bounded latest-frame queue and bounded audio ring. Audio-device callbacks perform no database work, allocation-heavy decode or blocking lock. Cancellation closes resources and stale output cannot cross projects or replace newer source.

### Primary-source dependency review

Candidates are not yet dependencies. Source-size counts below are full crate subtrees including tests/examples/docs, obtained from GitHub tree metadata; they are not linked binary size. Hosted CI must measure the selected-feature dependency graph, compiled size delta and memory/cancellation behavior before adoption. Both proposed reusable repos need independent manifests/licenses/tests/docs and immutable pins; no application engine is copied into WriterCraft.

| Candidate and primary evidence | License and MSRV | Narrow role and cost |
| --- | --- | --- |
| [craft-libs at 0a9d1f48](https://github.com/storytold/craft-libs/tree/0a9d1f48da9e0e61a2c7b602ba96ef1c4672bcd9), root manifest | MIT OR Apache-2.0; Rust 1.95 | Compatible with host Rust 1.97. Reuse data/decoder crates only; do not adopt the egui 0.37 fork merely because it exists upstream. |
| [craft-pdf manifest](https://github.com/storytold/craft-libs/blob/0a9d1f48da9e0e61a2c7b602ba96ef1c4672bcd9/crates/pdf/Cargo.toml) and [reader contract](https://github.com/storytold/craft-libs/blob/0a9d1f48da9e0e61a2c7b602ba96ef1c4672bcd9/crates/pdf/README.md) | Inherited MIT OR Apache-2.0 / 1.95; retain vendor asset licenses/notices | Enable only `raster` for real page rendering, not PDF writing. Facade subtree 371,705 bytes/28 files; Hayro vendor subtrees 3,079,598 bytes/212 files before remaining transitives. Consumer root patches must point at the same reviewed craft-libs revision; dependency-local patches are ignored. Budgets/diagnostics do not prove PDF fidelity. |
| [craft-audio manifest](https://github.com/storytold/craft-libs/blob/0a9d1f48da9e0e61a2c7b602ba96ef1c4672bcd9/crates/audio/Cargo.toml) and [stream/seek contract](https://github.com/storytold/craft-libs/blob/0a9d1f48da9e0e61a2c7b602ba96ef1c4672bcd9/crates/audio/README.md) | MIT OR Apache-2.0 / 1.95; `decode` brings unmodified Symphonia 0.6.1 MPL-2.0 | Enable `decode`, not encoding. Real MP3 via Symphonia, AAC/WAV natively; streaming and seeking, no device output and no Opus. Subtree 649,438 bytes/55 files. Symphonia facade registry package is 17,306 compressed bytes, MPL-2.0, MSRV 1.85; codec/transitive sizes remain separate. Symphonia inputs are first read whole; AAC limitations and damage concealment require visible diagnostics and fixtures. |
| [craft-media manifest](https://github.com/storytold/craft-libs/blob/0a9d1f48da9e0e61a2c7b602ba96ef1c4672bcd9/crates/media/Cargo.toml) | MIT OR Apache-2.0 / 1.95 | Demux MP4/MOV, WebM/Matroska/Ogg; use the same revision as craft-audio. 620,005 bytes/34 files. Containers alone cannot render or play video. |
| [FilmCraft H.264](https://github.com/storytold/filmcraft/blob/4a56be9cff419a5a9b16a25259c97dc823ca8bae/crates/h264/Cargo.toml), codec-specific crates and root manifest | MIT OR Apache-2.0 / 1.95 | H.264 decoder has bitstream/thiserror and optional rayon; 474,287 bytes/31 files. Reuse it behind a feature for the actual reference MP4. Add independently reviewed HEVC/VP8/VP9/AV1/Opus features as required by the supported codec table and public synthetic corpus. VP8 subtree 141,889 bytes/21 files; Opus 284,922 bytes/22 files. Do not infer codec support from a MIME label. |
| [FilmCraft codec hub](https://github.com/storytold/filmcraft/blob/4a56be9cff419a5a9b16a25259c97dc823ca8bae/crates/codecs/Cargo.toml) | MIT OR Apache-2.0 / 1.95, Symphonia MPL-2.0 | Not selected as default: hub itself is 804,555 bytes/47 files and unconditionally pulls many video/audio/container crates. Prefer independently gated decoder adapters. |
| [CPAL 0.18.2 registry metadata](https://crates.io/api/v1/crates/cpal/0.18.2) and [primary source](https://github.com/RustAudio/cpal) | Apache-2.0; Rust 1.85; published package 233,947 compressed bytes | Candidate audio-device output only; pin published =0.18.2 rather than unpublished master 0.19.0. Keep ASIO/JACK/realtime options off. macOS CoreAudio/Windows/Linux system dependencies and selected-feature binary delta need qualification before adoption. |
| [craft-atomic API](https://github.com/storytold/craft-libs/blob/0a9d1f48da9e0e61a2c7b602ba96ef1c4672bcd9/crates/atomic/src/lib.rs) | MIT OR Apache-2.0 / 1.95 | 82,588 bytes/5 files; Apple-only rustix dependency. `write_new_with` offers no-clobber streaming file publication. Plain atomic_write replaces an existing file and must not be used for unapproved export overwrite. No generic directory publication API is implied. |
| [FigCraft engine manifest](https://github.com/storytold/figcraft/blob/main/crates/engine/Cargo.toml) and root manifest | MIT OR Apache-2.0 / 1.95 | Rejected as a direct Canvas engine: dependencies include design document/layout/render/collaboration/formats/plugins/font services and Figma-specific commands. Its model is not the source JSON Canvas contract. Study focused mechanics; do not transplant a whole design app. |

A file that is preserved but cannot decode remains exportable byte-for-byte with an explicit unsupported-preview state. An external-open action may complement real in-app PDF/AV workflows; it cannot replace those required workflows. The private sample establishes H.264/AAC/MP3 acceptance, while synthetic MIME/codec combinations define the broader supported table honestly. No new codec capability is considered complete from upstream README claims.

## Revision-consistent Markdown and canonical export

Capture the chosen document/project state once under the workspace transaction: project/tree metadata, exact current revisions, all requested history, notes/dictionary/source records, Canvas/type schema, guide/memory/transcript data, asset identities/hash/length and durable recovery checkpoints. The Markdown view uses the pinned saved revision; unfinished drafts and proof-derived note checkpoints are retained separately in the canonical companion. UI must explain saved-view versus recovery scope. Imported checkpoint data can be retained and conservatively validated for recovery; deserialization must never recreate a trusted model replay receipt or grant proof authority. Later edits cannot alter the captured export. Immutable asset bytes may be loaded lazily with the captured project capability and checksum checks; missing/corrupt bytes fail the export.

The canonical companion is versioned independently of .wstudio and contains a complete source-compatible manifest plus native extension/recovery data and checksums. Provide source-compatible `.wstudio` output for supported legacy semantics and a WriterCraft canonical companion for native-only data; do not put an unrecognized native extension into a legacy envelope and claim the old strict reader accepts it. No-edit legacy round trip preserves logical IDs, all original raw records and binary bytes. Known modifications patch the known current records and append revisions; all historical/unknown source records remain present.

Folder layout is deterministic: reading documents follow the ordered project tree, `assets/` contains original artifacts, `canvases/` contains canonical `.canvas` plus useful linked Markdown descriptions, and `.writercraft/` contains the canonical manifest/history/recovery and an explicit conversion report. Metadata, not a human filename, identifies each resource. Allocate all paths in one pass with Unicode-aware collision keys, per-directory file/folder collision checks, Windows reserved names, NFC/case-fold collisions, deterministic stable-ID suffixes, bounded UTF-8 byte lengths and relative percent-encoded links. Keep user titles verbatim in content/manifest even when a portable filename needs sanitizing. Asset paths retain a safe original basename plus stable identity; never flatten two assets named alike.

| Rich/project feature | Markdown view | Canonical companion |
| --- | --- | --- |
| Paragraph, H1–H6, hard break, blockquote | Standard Markdown with explicit line breaks | Original JSON/attrs/IDs retained |
| Bold, italic, strike, inline code, links | Supported delimiters with robust escaping; choose code delimiters longer than source backtick runs | Original marks, link attrs, source refs retained |
| Underline and unsupported mark attrs | Documented HTML or text projection recorded in conversion report; no silent deletion claim | Complete original marks retained |
| Ordered/bullet/nested lists, list items | Ordered/nested Markdown; preserve starting ordinal where representable | List type/start/opaque attrs retained |
| Code block | Fence longer than any source run; literal source and original language string | Exact code and language/attrs retained |
| Callout, scene break, horizontal rule | Documented quote/separator representation and conversion entry | Exact kind/attrs/identity retained |
| Table/row/header/cell | Pipe table only for representable simple tables; merged/complex tables use a documented HTML view or explicit linked rich snapshot | Full spans, widths, alignments, nested content retained; no plain-text fallback without report |
| Image | Relative link to exact original bytes, escaped alt/caption | Source URI/asset ID/dimensions/opaque attrs retained |
| Mermaid, inline/display math | Literal fenced Mermaid and formula source/delimiters; reader compatibility documented | Original attrs/descriptions/source retained |
| Notes/dictionary/provenance | Optional author-facing links/sidecars; reader view policy explicit | Exact anchors, lifecycle, metadata and history retained |
| Canvas/type schemas/tree/history/named versions | Linked Canvas files and readable index/description | Complete canonical records/relationships/history retained |
| PDF/audio/video/unknown media | Relative ordinary artifact links, never pretend to embed an image | Exact original bytes, metadata/hash and logical references retained |
| Unknown rich/project fields | Explicit conversion entry when not rendered in Markdown | Original raw JSON retained |

Export selected documents and whole projects separately. Selected-document exports include referenced bytes and a scoped companion retaining their semantics; they must not claim unrelated omitted project records form a complete project backup. Whole-project portable export includes all history-referenced and unattached source assets required for a full round trip.

Write only beneath a newly and exclusively created export root. Never merge into an existing arbitrary folder. The default folder mode reserves a unique root with create_dir and immediately writes an `.incomplete` marker. All files use create-new/no-clobber semantics. The UI names this output incomplete while the marker exists. Verify every path/relative link/checksum, flush data, then publish the completion manifest and remove the marker last. A crash between those final operations remains conservatively incomplete until verification resumes. This is completion-manifest publication, not an atomic directory rename. A later platform-specific no-replace directory publication is optional and must be qualified before use. A single ZIP bundle can use craft-atomic write_new_with. A pre-existing destination, cancellation, failed flush or missing asset never overwrites user files or reports success. Cleanup may remove only files created by this export job; crash recovery must not recursively delete an unowned path. Directory-fsync behavior is platform-specific and not a universal power-loss promise.

## Storage location and consistent backup

Pass the selected database PathBuf from the entrypoint into an immutable storage-location descriptor. Display the active database, containing folder and interface/appearance preference scope, including explicit database overrides and isolated preview profiles. Assets, histories, canonical imports and durable recovery live in the database; caches are disposable. Open-folder uses the actual resolved parent through a native platform service. Localized labels/errors/help must cover ru/en/de/fr/es/pt/zh/ja and leave owner paths/content unchanged.

A live WAL database cannot be backed up by copying only workspace.sqlite. Implement an SQLite acknowledged snapshot/backup operation or require orderly checkpoint/close for the documented manual-copy procedure. A backup receipt names the captured schema/snapshot and verifies database integrity and required assets/recovery data. Credentials, once implemented, remain outside project exports and backup content descriptions. A portable project bundle is distinct from a full workspace backup and from interface preferences.

## Acceptance matrix

All rows are required for alpha.2 and remain pending until their named evidence exists. Public tests use synthetic structures/text/binaries; private reference acceptance records only outcomes locally. No issue closes before owner review.

| ID | Required scenario and evidence |
| --- | --- |
| P01 | Envelopes 1–8, editor schemas 1–4 and Canvas schemas 1/2 import; future envelope/schema and ambiguous duplicate-key JSON fail unchanged |
| P02 | Synthetic complete-domain corpus retains original logical IDs, every metadata field, custom structure/kinds, tree/order/trash/frontmatter and all unknown JSON after save/reopen/export/import |
| P03 | Every immutable document/Canvas/schema/guide/memory/transcript revision keeps ordinal/base/source/timestamp; current heads and named versions resolve; restore appends compensating history |
| P04 | Same archive imported twice has independent namespaces and intact original references; no cross-project alias/asset/annotation authorization |
| P05 | Source links, notes/dictionary anchors, card/promoted/typed-field refs, nested Canvas, cover assets, dictation destination/artifact/audio refs validate against the correct project/revision |
| P06 | Valid image/PDF/audio/video/opaque attachments preserve exact hash/length/metadata, survive original-file removal and reopen, and include historical/unattached retained resources |
| P07 | ZIP traversal/duplicate/case collision/symlink/count/size/CRC/hash/undeclared/missing entry attacks, tree cycles/orphans, invalid edge/resource/anchor and deep JSON leave no partial project |
| P08 | Inject cancellation/disk/constraint/read failure throughout import; restart never exposes a partial visible project; source and existing workspace remain unchanged |
| P09 | Actual native project tree opens all content-bearing and non-content nodes in correct hierarchy/order; known document content edits/saves/reopens without flattening or changing unrelated source metadata |
| P10 | Actual native Canvas opens all boards, pans/zooms/selects/edits/moves/resizes/connects, retains drawings/frames/type/card/edge attrs and navigates document/nested-board links |
| P11 | Canvas Undo/Redo/history/draft/restart/recovery/stale-save cases preserve raw fields and exact revisions, including source-linked notes and media references |
| P12 | Real native image/PDF page preview/open and MP3/H.264+AAC playback/seek/pause/end/volume work after reopen; stale/cancel/error/no-device/unsupported-codec states are truthful |
| P13 | Synthetic complete source-supported MIME/codec table exercises MP4/WebM/Ogg/WAV and image variants; measured buffers/worker cancellation/input latency stay bounded without rewriting preserved bytes |
| P14 | Every node/mark conversion in the table has expected Markdown plus canonical data; independently open representative Markdown/assets in another reader |
| P15 | Unicode/decomposed/case-fold titles, reserved names, same-name files/folders/assets, long names, link punctuation and nested paths produce deterministic collision-safe relative links |
| P16 | Document-only versus whole-project export has explicit scope; every emitted referenced byte hash matches and every emitted local link resolves |
| P17 | Edit/save/annotation/Canvas changes during export do not mix snapshots; recovery-only content is retained separately with exact checkpoint/base identity |
| P18 | Missing/corrupt assets, existing unrelated destinations, symlink/race cases, interrupted writes/cancellation/full disk and crash recovery preserve workspace and unrelated files; no incomplete output reports success |
| P19 | Canonical no-edit and edited semantic round trip retains all known/unknown domains and exact artifact bytes; source-compatible .wstudio is accepted by a source decoder using synthetic data |
| P20 | Data-location/open-folder works with default, CLI/env override, relative/Unicode database path, preview profile and all eight locales; backup instructions match actual WAL/snapshot behavior |
| P21 | Hosted format/tests/Clippy, migration checks for existing schemas 1–6, feature/MSRV/license checks and selected dependency graph/binary-size evidence pass at the exact commit |
| P22 | Independent designer reviews compiled Canvas/media/import/export/location journeys at compact/wide/focus sizes and keyboard-only input; owner privately exercises complete reference, restart/recovery and round trip before issue closure |

## Sequencing and ownership

1. Coordinating agent acknowledges this proposal and owns reconciliation of release scope and shared core schema/module declarations. Add OpenSpec project-transfer/portable-export/native-canvas/media-preview requirements before behavior changes.
2. Portability implementer owns new project-domain SQL/modules, transfer parsing/export files and synthetic acceptance fixtures. Other agents retain app.rs/rich_editor.rs, annotation/recovery and clipboard ownership. Shared core lib/Cargo/i18n/UI wiring changes are coordinated explicitly.
3. First deliver complete validated domain snapshots, aliases, atomic bulk import and canonical round trip; this is an internal data checkpoint only, not #10 completion.
4. Create the reviewed reusable Canvas/media workspaces and implement narrow generic APIs/dependency feature sets. Hosted CI qualifies licenses/MSRV/source/binary footprint and actual decoding. Integrate real native Canvas/media and recovery through app-owned adapters.
5. Implement revision-consistent Markdown/assets/canonical export and storage-location/backup UI, then all native/independent/private acceptance rows. Keep issues open through owner review.

Do not run Cargo builds/fetches/releases locally. Hosted CI uses synthetic fixtures only. Do not touch the owner's application/clipboard or replace their running isolated preview. No push, issue closure or release publication is authorized by this plan.
