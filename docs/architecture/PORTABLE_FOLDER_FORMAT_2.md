# Canonical Markdown/artifact folder, format 2

Status: alpha.3 implementation design, 2026-11-09. Issue #11 remains open. This extends the approved canonical stream and complete project-portability contracts; it does not advertise an implemented folder reader or native export interaction.

## One captured source

Prepare the canonical stream and current reading projections in the same dedicated SQLite read transaction. Store only paged current document/Canvas metadata and bounded selected current content in the job-owned index. Do not clone all native history, note/recovery arrays or a whole-project `nativeState` Value to create a Markdown view. The canonical root/objects remain the preservation representation; Markdown/JSON Canvas are explicitly documented reader views.

The private prepared index exposes bounded metadata iteration and one current document/Canvas reading snapshot at a time. Node identities, titles, parent/order, current revision, source frontmatter and read-only-preview status come from the captured state. Reading conversion reuses the existing complete rich Markdown renderer and portable Unicode identity-based filename allocator. Original code language, Mermaid/formula source, marks, table conversion rules and captions remain covered. Source frontmatter remains verbatim. A plain/opaque preview is explicitly identified and never replaces its preserved original.

## Layout and object inventory

- `README.md` explains the captured scope and links writing, Canvas views, original artifacts and conversion details.
- `documents/` preserves the captured ordered hierarchy, using container `index.md` files where appropriate. Deterministic shortened paths and their conversions handle otherwise unrepresentable path depth.
- `canvases/` carries reading JSON Canvas files and companion Markdown indexes with verified relative project-resource links. Unresolved/unowned file cards and unsupported link/item kinds become clearly named non-active text placeholders. Reading nodes/edges use supported standard fields plus minimal known scoped resource IDs; private/unknown graph extensions and the complete original file references remain canonical. Source-valid external link URLs remain HTTP/HTTPS (original `internal/canvas/document.go::validLinkURL`), without pretending unavailable file paths are exported artifacts.
- `assets/` contains actual original encoded media/audio/opaque artifact bytes, with Unicode-safe deterministic names and exact checksums.
- `.writercraft/project.json` is the bounded native format-2 root.
- `.writercraft/objects/` stores other exact hash-addressed payload/shard objects. An explicit object map may bind a media digest to its verified `assets/` reading path, avoiding a second encoded copy solely for canonical transport.
- Bounded JSON Lines file-inventory shards describe every payload's relative path, byte size, digest and role. Object records identify the exact canonical digest and authorized member path; readers never guess an original filename or fetch a URL.
- Bounded conversion-report shards retain per-resource reader differences.
- `.writercraft/export.json` is the small final completion root, referencing the canonical root, object mapping, file-inventory and conversion shards by exact hashes/counts. It does not hash itself; inventory shards are covered by this root instead of recursively listing themselves.

Keep the existing root/shard/record/transport/member/resource guards. Count every regular output file and actual encoded byte even when two artifact reading names share a content digest. The stage budget includes the owned incomplete marker until removal; completed receipts exclude that removed file/bytes. Bound relative directory count independently at65,536, without charging directories as regular files. Writer-created directories and scanned actual directories must agree; no hidden marker or directory overhead is added only at final verification. Preserve complete or selected scope explicitly. A selected export cannot embed the original whole-project source or expand references into unselected writing, memory, guide or source bodies.

## Publication and restoration

Choose a new exclusive destination away from the active database/journals/preferences. Create `.incomplete` before payload writes. Stream and hash actual bytes, verify all required outputs, write and flush bounded inventory/report shards, then publish the completion root and remove `.incomplete` only after success. An existing incomplete marker always overrides a completion root. Failure preserves author data and does not merge, overwrite or recursively remove unrelated files. No universal filesystem power-loss guarantee is inferred.

The reader opens one user-selected directory capability (ADR-014), inventories regular members, rejects inner symlinks/noncanonical paths and verifies all listed files/control shards. Every control/object read uses established no-follow/nonblocking options and checks the actual opened handle is a regular file before reading; path metadata is never later-open authority. This prevents a replaced FIFO from waiting for a writer before cancellation is observed. Changed Markdown is not silently treated as an unchanged canonical backup. Unexpected/missing data or hash mismatches stop restoration before semantic publication. After transport validation, the existing verified private object index and one scoped semantic transaction restore an independent project; external checkpoints, drafts and review commands remain data-only evidence.

## Required qualification

Exercise every supported block/mark and exact artifacts, Unicode/case/reserved-name collisions, deep trees, empty/Canvas-only projects, source-original plus native deltas, repeated flat evidence restoration, selected privacy, read-view changes, symlink/root substitution, storage/cancellation errors and malformed controls. Measure source/native memory separately on hosted synthetic maximum workloads. Open the emitted reading files in an independent reader and complete the actual localized native export/restore interaction and independent UX review before closing #11.
