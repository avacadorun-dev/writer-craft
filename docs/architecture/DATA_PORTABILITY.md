# Local data and portable project contracts

Status: accepted preservation requirements, 2026-11-08; export/location UI implementation and acceptance pending [issue #11](https://github.com/avacadorun-dev/writer-craft/issues/11). This document is not a claim that full Markdown/project export already exists.

## Current storage

The authoritative workspace is SQLite with additive migrations, immutable document revisions and durable working drafts. The image-storage implementation stores exact immutable encoded media in project-owned database rows; renderers receive a scoped asset source and never resolve arbitrary paths or network URLs. Document insertion and asset ownership are distinct operations; deleting or undoing a block must not delete history-referenced bytes.

Default desktop database locations from the native entrypoint:

| Platform | Ordinary workspace |
|---|---|
| macOS | `~/Library/Application Support/WriterCraft/workspace.sqlite` |
| Windows | `%APPDATA%/WriterCraft/workspace.sqlite` |
| Linux | `$XDG_DATA_HOME/WriterCraft/workspace.sqlite`, otherwise `~/.local/share/WriterCraft/workspace.sqlite` |

The explicitly isolated editor preview uses an `Editor Preview` subdirectory. `WRITERCRAFT_DATABASE` and `--database` can select another location. Interface and appearance JSON preferences are siblings of the selected database. Legacy demo/engineering profiles may have their own explicit path; never present a guessed default as the current active location.

SQLite uses WAL. Copying only a live database file is not an advertised consistent backup. Implement an acknowledged snapshot/backup API or an orderly checkpoint/close workflow; preserve recovery data and verify the resulting backup before offering success. Secrets, once implemented, stay in platform credential storage and must not be exported as manuscript attachments or diagnostic logs.

## Portable Markdown and canonical exports

A Markdown export is a portable reading/editing representation, not automatically a lossless project backup. Preserve supported heading/list/table/code/link content and emit original code fence language, Mermaid and formula source. Copy referenced image and non-image artifact bytes into deterministic collision-safe local paths, use relative links, retain byte checksums, and prevent unsafe path traversal or unrelated-file overwrite.

Define conversion behavior for merged tables, persistent annotations, opaque rich attributes, source provenance, revisions, Canvas and project relationships. Carry semantics that Markdown cannot represent in a canonical project bundle/manifest alongside the Markdown view. The canonical export/import contract must retain the original supported `.wstudio` semantics and unknown imported resources; no silent flattening, missing media or fake-success conversion.

Export one revision-consistent database/asset snapshot. Write into a staging destination, validate links/checksums and commit atomically where the filesystem permits. Failures preserve the original workspace and make partial output explicit. Document-level export and whole-project export require separate acceptance cases.

## Mandatory migration reference

The owner supplied an original format-8 demonstration project as a private read-only reference: [issue #10](https://github.com/avacadorun-dev/writer-craft/issues/10). Initial structural inventory: 9 project nodes, 17 document revisions, 3 canvases, 9 canvas revisions and four attachments (PNG/PDF/MP3/MP4). The editor portion contains prose/headings; synthetic fixtures still cover all 21 nodes and six marks. The eventual complete application must open and exercise the whole project without lost references, histories or assets. Deferred Canvas/media workflows remain required and explicitly tracked.

Never commit the private archive, manuscript text, screenshots, media, absolute owner paths or credentials. Public CI uses synthetic equivalent structures and artifacts. Local reference validation records outcomes without publishing private content.
