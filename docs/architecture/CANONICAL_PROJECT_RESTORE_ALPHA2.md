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
