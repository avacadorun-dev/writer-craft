# Complete project portability design

The detailed source-backed design, dependency choices, API boundaries, Markdown conversions and P01–P22 acceptance matrix are in [PROJECT_PORTABILITY_ALPHA2.md](../../../docs/architecture/PROJECT_PORTABILITY_ALPHA2.md).

This change extends ADR-002/004/008/012 without weakening immutable revisions, explicit Apply, guarded Undo, original source preservation or non-deserializable model proofs. Project logical identities remain scoped to an import namespace; existing native document/asset handles remain internal and backward compatible. Core owns semantic validation and transaction authority. Transfer depends on core for validated domain values and owns archive/filesystem conversion I/O. Native Canvas and authorized media decoding live behind separate reusable engines; business metadata, aliases and persistence remain in WriterCraft.

Schema 7 uses additive tables and migration fixtures for existing schemas 1–6. Import streams assets into one transaction and makes no project visible before all validation and checksums pass. Export captures one transaction-consistent state and creates an exclusive output root with an incomplete marker, exact bytes, verified relative links and a completion manifest published last. Folder export does not claim atomic directory replacement. Single-file bundle publication uses no-clobber semantics.

No local Cargo build/fetch/release runs are permitted in this workstream. Hosted qualification uses synthetic data only. Existing app/rich-editor/notes/recovery/clipboard owners retain their files; shared lib/Cargo/schema declarations require explicit integration coordination.

## Canonical native restore and schema 8

The approved encoding, budgets, streaming API and memory qualification plan are in
[CANONICAL_PROJECT_RESTORE_ALPHA2.md](../../../docs/architecture/CANONICAL_PROJECT_RESTORE_ALPHA2.md).
Format 2 uses bounded shards and content-addressed object references. Exact original source is stored
once; native saved changes are domain deltas. Source format 1–8 remains unchanged. The single-JSON
format-1 companion is a bounded pre-release compatibility path, not a reason to allocate a larger
native Value. The 1 GiB native transport guard is finite and makes no universal fit guarantee.

Schema 8 adds scoped evidence object/reference tables and migration tests after the qualified native
baseline. No imported checkpoint or staged change populates trusted replay/proposal tables.

## Streamed native Markdown/artifact folder

The concrete format-2 layout/control contract is in [portable folder format2](../../../docs/architecture/PORTABLE_FOLDER_FORMAT_2.md). The current saved reading projection shares the canonical preparation transaction and persists in its private job index. Transfer materializes metadata/path maps only, then renders one saved document or current graph at a time through the established Markdown converter. Reading indexes, file inventories, digest mappings and conversion reports are paged. All canonical payloads use exact prepared object capabilities; original media files can serve both reading and canonical mapping roles without a second encoded copy solely for transport.

ADR-014's exact cap-std directory capability confines member reads/writes. A new directory is created exclusively, opened and checked empty before its incomplete marker; all later I/O is relative to the pinned root and uses exclusive file creation. No recursive cleanup or merge is permitted. The final controls must still equal the prepared publication intent, then the untrusted folder validator rehashes every file and inventories actual members before the incomplete marker is removed. Restoring edited reading files is explicitly refused rather than quietly importing older canonical content. Complete-project export is the first qualified slice; selected-document stream/privacy and localized native interaction remain separate required work.

## Native whole-project transfer orchestration

Use a separate `app/project_transfer.rs` state machine and the existing shared modal/components.
Export offers native archive or readable Markdown/artifact folder (both retain canonical restore
objects), shows the fixed current-project title, and explains the separate whole-workspace backup.
After explicit Continue, save current writing and wait for `flush_canvas_for_transfer` for that
project. Filter pending note identities by actual project-owned documents; Review returns to that
note without discarding paused transfer intent or parked notes in other projects. Native pickers
choose an absent archive path or a parent plus validated new folder name. Prepare/export run on a
separate worker with the actual database path.

Restore opens either a user-selected native archive or readable folder on its checking worker,
retains only a verified private canonical stream and lightweight preserved-content summary for
preview, then consumes that same stream after current-work prerequisites and explicit Restore.
Preview counts are current items/boards/artifacts; native delta revision counts are never called
complete historical totals. Explain preserved data separately from editable capability. Completed
restores expose the new project identity and an explicit Open action; a navigation failure cannot
become a false restore failure or cause duplicate automatic import. Success acknowledgement takes
precedence over cancellation flags. Failed or missing acknowledgements retain truthful diagnostic
and retry/review state without an automatic ambient source reopen. Actual native minimum-window,
localized keyboard/cancel/failure/collision/referenced-media interactions and independent UX
review remain required; source/controller checks alone do not complete issue11.
