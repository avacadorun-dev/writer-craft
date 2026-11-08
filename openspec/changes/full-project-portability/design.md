# Complete project portability design

The detailed source-backed design, dependency choices, API boundaries, Markdown conversions and P01–P22 acceptance matrix are in [PROJECT_PORTABILITY_ALPHA2.md](../../../docs/architecture/PROJECT_PORTABILITY_ALPHA2.md).

This change extends ADR-002/004/008/012 without weakening immutable revisions, explicit Apply, guarded Undo, original source preservation or non-deserializable model proofs. Project logical identities remain scoped to an import namespace; existing native document/asset handles remain internal and backward compatible. Core owns semantic validation and transaction authority. Transfer depends on core for validated domain values and owns archive/filesystem conversion I/O. Native Canvas and authorized media decoding live behind separate reusable engines; business metadata, aliases and persistence remain in WriterCraft.

Schema 7 uses additive tables and migration fixtures for existing schemas 1–6. Import streams assets into one transaction and makes no project visible before all validation and checksums pass. Export captures one transaction-consistent state and creates an exclusive output root with an incomplete marker, exact bytes, verified relative links and a completion manifest published last. Folder export does not claim atomic directory replacement. Single-file bundle publication uses no-clobber semantics.

No local Cargo build/fetch/release runs are permitted in this workstream. Hosted qualification uses synthetic data only. Existing app/rich-editor/notes/recovery/clipboard owners retain their files; shared lib/Cargo/schema declarations require explicit integration coordination.
