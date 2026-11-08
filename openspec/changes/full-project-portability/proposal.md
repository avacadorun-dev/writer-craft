# Complete project import and portable export

## Why

The owner included all open issues in alpha.2 on 2026-11-08. Opening a .wstudio archive currently produces a read-only listing, while the required author workflow opens the entire project, uses its Canvas and media, restarts safely and exports writing with actual artifacts. Issues [10](https://github.com/avacadorun-dev/writer-craft/issues/10) and [11](https://github.com/avacadorun-dev/writer-craft/issues/11) stay open through owner acceptance.

## What changes

Implement complete transactional versions 1–8 project migration with preserved logical IDs, all metadata/domains/history/references/unknown data and exact attachment bytes. Add functional native Canvas and media workflows through separate reusable engines. Export revision-consistent Markdown folders and canonical companions with safe relative links and no unrelated-file overwrite. Show the actual active data location, native open-folder action and correct consistent-backup scope in all eight languages.

### Added capabilities

- `project-portability`: complete migration, native project/Canvas/media use, canonical round trip, Markdown/assets export and storage transparency.

## Scope and compatibility

The [source audit and architecture plan](../../../docs/architecture/PROJECT_PORTABILITY_ALPHA2.md) is the implementation basis. Schema 7 is additive over schema 6, preserves existing document/recovery APIs and introduces project-scoped logical aliases without globally rewriting imported JSON. Private reference data remains local and read-only; hosted fixtures are synthetic. Reusable Canvas/media repositories and dependencies need coordinated API/license/MSRV/footprint qualification before integration.

Earlier editor-only Canvas/import/export deferrals are superseded by the latest owner instruction. Real native workflows and independent acceptance remain required; archive inspection, model tests or successful compilation cannot close the issues. No public release or issue closure is authorized by this proposal alone.

## Acceptance

Pass P01–P22 in the architecture plan at an exact source/build, including hosted integrity tests, compiled interaction/independent review and private complete-project reopen/recovery/round trip. Preserve unrelated owner changes and never modify the frozen original or supplied project archive.
