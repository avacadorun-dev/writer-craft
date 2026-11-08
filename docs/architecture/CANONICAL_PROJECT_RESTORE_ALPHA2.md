# Canonical project restore for alpha.2

Issue #10 requires a usable round trip for the complete native project, including data that a
source-compatible archive cannot represent. Issue #11's Markdown files are reading views. Their
canonical companion and exact owned artifacts provide the restore representation.

This extends the approved project-portability architecture. Imported JSON never grants native
replay, checkpoint, proposal-Apply or filesystem authority. Public fixtures remain synthetic.

## Input and semantic authority

A private-field ValidatedPortableProjectImport has a bounded parse constructor and no Deserialize
implementation. Transfer supplies bytes and bounded asset readers; core validates every semantic
projection and owns the transaction.

Accept writercraft-portable-project version 1 with explicit complete-project or selected-documents
scope. Check duplicate keys before interpretation. Bound the envelope and native/recovery components
independently of the existing 64 MiB source-manifest and 512 MiB original-artifact limits.

When sourceManifestRaw exists, require its declared SHA-256 and semantic agreement with sourceManifest.
Preserve the complete UTF-8 bytes, including outer whitespace. An older companion containing only the
raw sourceManifest JSON value cannot claim preservation of unavailable outer framing.

Native empty projects and Canvas-only projects are legitimate canonical input. A private native-source
validation mode may permit empty nodes; original format 1–8 validation remains unchanged. Never invent
a manuscript document.

For selected scope, declared identities must equal supplied content-bearing nodes. A parent outside
that set stays in opaque selection context. The new project's editable tree has no invented parent.
Do not expand references into another manuscript, guide, memory or source session.

## One transaction and independent identity

The dedicated worker opens its own workspace connection. The desktop flushes pending writes and shows
the cancellable modal first. Large imports must not borrow the desktop's global save mutex.

Refactor the existing importer into a common private transaction body. Source documents, all rich and
Canvas revisions, artifacts, validated native projections and quarantine either commit together or
roll back together. Check cancellation during bounded work and asset streaming.

Captured documentAliases map old native handles to logical documents in this imported project. Never
look up old IDs globally. A second import creates a fresh project namespace and storage handles.
Preserve original logical IDs in their respective domains. Native-created notes need a separate alias
namespace so an arbitrary source note ID cannot collide with them.

## Notes and history

Native notes are ordinary author data, not replay proof. Validate scoped IDs, owning document, positive
metadata/base revisions, enums, bounds and the existence of each base revision. Rewrite only known
native handle fields into the fresh namespace; retain raw records and unknown fields separately.

Validate anchors against the exact rich base revision with AnnotationAnchor::selection and scalar
positions. Never search for another occurrence or infer UTF-16 coordinates. Only a proven anchor at
the captured current base may remain attached. Declared detached states stay detached. Unproven
anchors remain readable detached projections with raw evidence and no invented native coordinates.

Preserve text, resolved/attention state, tombstones and metadata revision. A native overlay replaces
the corresponding source-note projection, avoiding a stale source note beside its newer restored
state. Preserve supplied before/after events and document revisions after scoped validation. Do not
manufacture missing history; an explicit import audit event may describe the import itself.

Historical attachment snapshots require exact validation against their own rich revision before
normal history restoration may consume them. Unsupported or unproven snapshots stay in quarantine.

## Opaque source and recovery

Restore opaque originals into documents.source_json with read-only behavior. A named rich/plain
reading preview does not replace the original format.

External prose/rich/Canvas drafts, review proposals and checkpoint objects remain quarantined.
Never populate active rich_draft_annotations or active proposal rows from them. A scoped evidence API
exposes metadata and selected payloads for explicit conservative recovery. Recovery uses ordinary
validated commands at the current base, never imported sealed-journal or Apply authority.

Export retained evidence as flat hash-identified records. Reimport flattens and deduplicates existing
evidence instead of nesting earlier envelopes recursively. Unknown fields, raw checkpoints and history
remain readable without exponential growth. Retained native evidence forces canonical export; the
legacy-only writer keeps refusing loss.

## Transfer adapters

Both adapters use the same validated core DTO and transaction:

- A native canonical ZIP contains its distinct envelope and declared original artifact members.
  The pinned bounded ZIP reader checks count, path, size, CRC and SHA-256. Do not claim that an old
  source reader understands the native envelope.
- A Markdown/artifact folder has .writercraft/project.json and .writercraft/export.json. Refuse
  .incomplete, unsafe paths, symlink members/parents, size/hash mismatches and changed reading files.
  Match original artifacts to authorized assets/ members by digest and length, never guessed titles,
  unrelated local paths or fetched URLs.

Inputs remain read-only and imports create independent projects. Output publication continues using
protected new paths without overwriting or merging unrelated user files.

## Synthetic acceptance

1. Complete source and native notes/drafts/Canvas/opaque data survive export, import, reopen and a
   second export with stable logical IDs, fresh storage IDs and exact original artifact bytes.
2. Import twice stays independent; forged aliases cannot read or attach another project's data.
3. Notes, tombstones, events and detached anchors survive. Repeated text and emoji never trigger
   automatic reattachment.
4. Forged checkpoints/proposals remain readable evidence and never become replay or Apply authority.
5. Whitespace, unknown fields, precise numeric lexemes and opaque original JSON survive.
6. Selected scope excludes unselected writing, history, notes, guide, memory and source bodies.
7. Hash/raw mismatches, duplicate keys, malformed ownership, corrupt assets, incomplete folders,
   unsafe paths and midstream cancellation leave no imported project.
8. Repeated restore/export keeps evidence flat and deduplicated under explicit bounds.
9. Native UI demonstrates restore/recovery and localized errors in all eight languages. Source
   inspection, compilation and this proposal alone do not establish acceptance.

## Approved format 2 encoding and finite budgets

Format 1 was a pre-release single-JSON preservation companion. New native bundles use format 2.
Retain a separately bounded format-1 compatibility converter for existing preview exports; reject an
oversized old companion with an actionable re-export/backup instruction before writing project data.
Do not increase its JSON allocation limit to make a fixture pass. Original .wstudio versions 1–8,
including their existing 512 MiB transport and 64 MiB manifest bounds, remain unchanged.

Format 2 stores data once and uses SHA-256 references:

| Item | Encoding and bound |
| --- | --- |
| Root project descriptor | At most 1 MiB JSON; scope, version and descriptors for component shards |
| Component shard | At most 1 MiB JSON Lines; at most 256 KiB per metadata record |
| Inline content | Fits the metadata record; larger content uses an object reference |
| Original source manifest | Exact original UTF-8 object, at most 64 MiB, stored once |
| Saved native rich content / rich recovery candidate | One object at a time, at most 16 MiB; model validation before activation |
| Canvas content / recovery candidate | One object at a time, at most 12 MiB; scoped Canvas validation |
| Large anchor quote | At most 16 MiB object, referenced by current/history/snapshot metadata |
| Opaque original / unknown evidence object | At most 64 MiB; streamed as bytes and kept read-only |
| Original encoded media | Existing type-specific limits and at most 512 MiB aggregate logical bytes |
| Native canonical ZIP | At most 1 GiB compressed and expanded, with at most 65,536 members |
| Root/shard graph | At most 4,096 shard descriptors and 1,000,000 records |
| Markdown folder completion | A small completion root referencing bounded file-inventory shards |

These are finite transport/resource guards, not authoring limits and not a guarantee that every
native database fits. Whole-workspace SQLite backup remains available for larger state. Validate
declared counts, object lengths, path classes, aggregate sizes and graph reachability before project
writes or completed publication; verify actual lengths/hashes while streaming. Fail explicitly on
excess rather than truncating records, dropping history or calling a reading view lossless.

Root fields describe an optional immutable source object, project/selection scope and hashed shards.
Shards describe project/node/alias metadata, saved native revision deltas, note states/events,
attachment evidence, Canvas metadata/revisions, media descriptors, recovery, staged changes and
unknown extension domains. Native edits are deltas over the exact original source; native-created
projects have explicit project metadata and nodes instead of an invented original archive. This
avoids embedding another complete current manifest with duplicated historical rich/Canvas bodies.

Small ordinary metadata remains inline. Large current/history/snapshot/opaque payloads reference the
same object digest when their exact bytes match. Unknown domains use raw objects and flat references;
a restored envelope never nests whole previous envelopes. Semantic IDs remain domain/scoped fields,
independent of content hashes and transport filenames.

The archive uses deterministic object/shard member paths. A Markdown folder may also expose original
media under safe Unicode reading filenames. Its object index maps hashes to those authorized paths;
the reader does not guess names. File-inventory shards cover payloads. The final completion root
hashes those shards and the canonical root; neither a file inventory nor a manifest hashes itself.
An existing .incomplete marker always takes precedence over a completion root.

## Streaming API and transaction lifecycle

Core owns a fixed-project export worker and a semantic import worker. Transfer implements bounded
source/sink capabilities for a pinned ZIP or completed folder. The dependency remains transfer to
core; core does not open arbitrary transport paths or fetch URLs.

Export obtains its own SQLite read transaction, inventories metadata and object lengths, and emits
paged records plus object streams. No whole-project nativeState Value is constructed for format 2.
The core computes object descriptors; transfer verifies actual written bytes before final publication.
Use only job-owned temporary/index files and preserve no-clobber publication.

Import preflights the small root and bounded shard graph into a private disk-backed index, validating
counts, references, lengths and transport budgets before project writes. It then opens one transaction:

1. Validate/import the bounded original source if present; release that large parsed DTO before the
   native phase. Preserve its exact bytes separately from the native deltas.
2. Register validated scoped logical identities and storage aliases. A private temporary SQLite index
   holds cross-record mappings instead of a large native-state object graph.
3. Process saved revisions, normal note states/events and Canvas records in bounded pages. Validate a
   selected rich/Canvas object before activation; load only the relevant base graph for an anchor.
4. Stream artifact originals and unknown evidence through checked SQLite zeroblob handles with a
   64 KiB buffer. Hash and length mismatch or cancellation rolls the entire transaction back.
5. Store drafts/checkpoints/staged changes as untrusted evidence references, never replay/proposal
   authority. Publish the imported project only after every required record and blob succeeds.

Format-1 compatibility conversion produces the same internal record/object interface. Its old wrapper
does not bypass format-2 semantic validation or populate trusted recovery tables.

Schema 8 adds project_evidence_objects and project_evidence_references only. Objects use a
project-scoped digest, byte size, media type and BLOB. References use a domain/logical identity,
optional scoped logical document/Canvas, object digest and bounded metadata. Composite foreign keys
prevent cross-project object/reference bindings. Existing trusted tables and schemas 1–7 remain intact;
root owns the shared migration hook after the qualified native baseline. Rich proposal persistence
may follow independently in schema 9.

## Memory and hosted qualification plan

The streaming phase keeps a metadata page/record, 64 KiB I/O buffer and only the known rich/Canvas
objects required for the current validation. It never deserializes a 256 MiB native JSON envelope.
Unknown/opaque bytes stream without building a Value. The existing source parser still has its
explicit 64 MiB source limit; do not describe that byte limit as a proven RSS bound.

Measure the actual source and native phases separately on hosted Linux/macOS/Windows fixtures.
Include many small records, maximum-sized known payloads, shared current/history objects, large raw
evidence, source-sized originals and a 512 MiB streamed synthetic artifact. Record peak RSS where
available and cancellation/read responsiveness; set an evidence-based regression threshold before
claiming memory qualification. No local Cargo builds or private fixtures are permitted.

Acceptance additionally requires preflight oversize refusal before writes/publication, exact object
deduplication with stable logical identities, format-1 compatibility, source-format bounds unchanged,
schema-7-to-8 migration preserving data, corrupt/missing/chunk/duplicate/cancel rollback, and repeated
restore/export with flat evidence.

## Alpha.3 compatibility implementation checkpoint

The pre-release format-1 reader has a separate 64 MiB encoded-companion guard, with native-state and recovery components bounded at 32 MiB and 16 MiB respectively. These guards are intentionally not widened to fit large projects. `ValidatedPortableProjectImport::parse_v1` checks source raw bytes/hash/value agreement, source-scoped aliases and every activated saved note/history/opaque projection before writes. The existing original format 1–8 parser retains its nonempty-node rule; only the private native validation mode accepts empty/Canvas-only native projects. Selected documents lose outside parents only in their editable projection, while the exact selected source and outside context remain preservation evidence.

`ProjectImportWorker::restore_portable` uses the same source import transaction body, then restores current note metadata, supplied validated history and read-only originals, and streams bounded raw native/recovery/extension domains into schema-8 evidence. No external draft, checkpoint, attachment snapshot or review proposal populates trusted replay/proposal tables. Note matching uses only supplied scoped identities and exact base-revision scalar anchors; an invalid or stale current attachment detaches rather than searching another occurrence. Imports create new storage handles and preserve source/native-note logical namespaces.

This checkpoint is compatibility groundwork, not issue #11 completion. Paged format-2 semantic records/object export, final canonical ZIP/completed-folder adapters, stable repeated-restore evidence export, native UI and full synthetic/private/native acceptance remain required. The existing monolithic snapshot/export is not streaming format-2 evidence.

## Format-2 export stream record registry (alpha.3 implementation)

`ProjectPortableExportWorker::prepare` owns a separate workspace connection and one read transaction. It spools a job-owned content-addressed object directory and private SQLite descriptor index. Its public prepared capability exposes only bounded root bytes, deterministic descriptor visitation and checked copying of an exact indexed object; transport filenames and paths are never read authority. Dropping a failed/prepared job removes only its own temporary directory.

The source original is copied once as `sourceOriginal`. Shards carry `project`, `nodes`, saved native `documentRevisions` and `canvasRevisions` deltas, `nativeAnnotations`, `nativeAnnotationHistory`, `opaqueDocuments`, current `canvases`, `assets`, and flat `evidence` references. Rich/body/Canvas/state/metadata payloads are referenced by exact byte hashes. Source-imported immutable revisions stay in the original object rather than being repeated as native deltas. Existing source fields, including unknown/raw lexemes, remain in that immutable original. Current native tree/node identity and title/revision overrides are explicit records. Native-created projects have project/node/revision records without inventing a source archive.

`attachmentEvidence`, `proseDraftEvidence`, `richDraftEvidence`, `canvasDraftEvidence`, `reviewEvidence` and `richReviewEvidence` remain preservation-only domains. Drafts, raw snapshots and review commands do not authorize imported replay or Apply. Flat `evidence` retains its existing domain/logical identity, owning optional document/Canvas, object descriptor, metadata object and declared media type. A restored export therefore need not embed another whole prior companion.

The exporter iterates rows; it does not create a whole-project `nativeState` Value. Known text/JSON payloads are bounded individually; original artifacts, retained opaque evidence and the source original stream in 64 KiB chunks. The temporary object index deduplicates exact content hashes and keeps logical identities separate. A root/shard/object capability is not semantic import acceptance. Format-2 import preflight, final transport publication, selected-scope stream, semantic activation and native UI remain separate required work. Peak RSS and maximum-workload evidence remain pending hosted qualification.

The format-2 ZIP writer uses `project.json` plus deterministic `objects/<sha256>` members, storing every unique object once. `export_canonical_project_archive_file` stages an exclusive sibling temporary file, reopens that generated archive and checks the exact root/member count plus each object's actual length, ZIP CRC and SHA-256, verifies the actual encoded 1 GiB bound, and publishes with a no-clobber primitive. The caller supplies the actual selected database for reserved path/alias/journal checks. Cancellation/errors never replace an existing output or source data. This native `.writercraft` container is not advertised as consumable by the old Studio reader. The separate original `.wstudio` versions 1–8 path and limits stay intact. The writer's self-verifier is not a public untrusted native archive importer; a pinned, fully preflighted native reader remains required.

## Pinned format-2 preflight checkpoint

`PortableObjectSource` is an explicit compiled transport capability, never constructed from a manuscript URL/path. Its adapter verifies real container/path/member limits and enumerates every actual object, including unsupported domains. `VerifiedPortableProjectStream::preflight` independently checks the small root, all enumerated byte lengths/hashes, shard framing/counts, required known references, and distinct transport/materialization budgets into its own disk-backed temporary SQLite index. Known hash references cannot read an ambient workspace or unrelated local file. Every byte is pinned in that job-owned index before a verified value is returned, so the later semantic transaction never reopens external input. Unknown domains and surplus objects remain available as ordinary flat preservation data. A verified transport is explicitly not semantic activation or replay/Apply authority. Semantic source/native restoration and final native-reader UI acceptance remain pending. Temporary index connections are declared before their directory owner so Windows closes SQLite handles before directory cleanup. Ephemeral private indexes disable journaling/synchronization because failure discards the whole job; authoritative workspace transactions retain their existing durability settings.
