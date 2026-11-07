# Native MVP

## Why

Authors need a desktop writing workspace they can install and use without operating Docker, PostgreSQL or object storage. The first release establishes durable local writing and safe review while preserving a path to the complete original workflow.

## What Changes

- Introduce a native WriterCraft window with local projects, plain prose documents and recoverable working drafts.
- Share revisioned save and exact proposal commands across desktop and CLI.
- Inspect legacy .wstudio archives with preserved structured source and an explicit preview boundary.
- Provide eight built-in interface languages with immediate switching, saved choice and CJK glyph coverage.
- Provide Light/Dark appearance and the six requested non-green legacy palettes.
- Record requirements, decisions, tasks and real verification before claiming completion.

## Capabilities

### New Capabilities

- `native-workspace`: local library, recovery, compatible opening and truthful source inspection.
- `manuscript-editing`: revisioned prose, Unicode identity and imported rich-data protection.
- `assistant-review`: exact staged proposals with guarded apply/reject/undo and truthful origin.
- `localization`: eight offline catalogs, live/persistent language choice, dynamic message and CJK coverage.
- `appearance`: brightness, retained palettes, one native workspace and accessible controls.

### Modified Capabilities

None; this change bootstraps the native repository's capability baseline. Canonical specs also contain explicitly deferred parity-milestone requirements; those are outside this MVP delta.

## Impact

Affects studio-core, studio-transfer, studio-cli, studio-desktop, UI assets and documentation. Existing Writers Studio and user source projects remain unchanged. Imported rich content is not editable migration. No AI-provider capability, Canvas, dictation, rich editor or publishing parity is implied.

Rollback keeps the original project/archive and old native application available. New workspace schema changes are additive; opening a newer schema must fail without rewriting it. Do not downgrade a newer database with an older binary.
