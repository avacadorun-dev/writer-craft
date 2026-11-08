# ADR: owned image assets and preserved canonical references

Status: implementation checkpoint within the approved full-editor alpha.2 scope. Core fixtures await hosted qualification; native chooser/drop/render/reopen interactions remain separate acceptance work. This does not implement full project archive import.

## Source contract

The frozen original is inspected read-only. `internal/projectmedia/image.go:18–76` accepts PNG, JPEG, GIF and WebP, with encoded bytes at most 12 MiB and each intrinsic dimension from 1 through 20000. It derives type/dimensions through `image.DecodeConfig`, without allocating a full decoded bitmap. `src/features/editor/model/image-drop.ts:1–25` exposes PNG/JPEG/GIF in the editor chooser; the backend/MCP also accepts WebP. The native core retains all four formats rather than introducing a smaller authoring capacity.

`internal/assistantapp/backend.go:160–240` stores an uploaded asset immediately, verifies a declared MIME type against its bytes and returns media identity; actual document insertion remains a separate reviewed mutation. `internal/editor/document.go:1080–1135` binds an asset ID to its exact project media source, with alt/caption limited to 1000 bytes and nullable display dimensions up to 20000. `internal/projectmedia/image.go:82–102` supplies the display filename normalization. Existing HTTPS/opaque imported sources must remain preserved; successful migration cannot be inferred from their strings.

## Storage decision

Additive SQLite schema 5 introduces `project_assets`. Asset identity, project ownership, filename, SHA-256, original encoded bytes, byte count and intrinsic dimensions commit together under the existing workspace transaction lock. The original file is never needed again after import. The app exposes no asset deletion or mutation command, so removing an image node, Undo/Redo, a failed insertion, draft recovery or revision restoration cannot collect still-referenced data.

The table is a generic media envelope: it has no image-only MIME or 12 MiB database constraint, its dimensions may be absent, and source_json can retain original opaque metadata. The current import adapter is image-only; future approved project import must retain PDF/audio/video/unknown resources and their original opaque metadata through their own validated adapters. No owner archive or private media fixture is copied into this repository.

Image inspection shares the existing `image` dependency with explicit PNG/JPEG/GIF/WebP features. Its header-only path retains the source's per-axis policy without adding a pixel-area clamp. Inspection is not a claim that every encoded frame or pixel was decoded successfully. The native render worker retains responsibility for actual decode/render errors.

## Authoring and preservation are distinct

New local authoring calls `import_image` or explicit `import_image_file`, then obtains an owned, validated node through `image_node`. The node uses a stable `asset://<UUID>` identity and has no source filesystem path. The helper verifies ownership/integrity and validates canonical alt/caption/block identity. Import alone never changes a manuscript.

Generic canonical create/save/import APIs remain preservation paths. They do not retroactively reject or rewrite unresolved historical image attributes, unknown metadata, missing resources or explicit host demo fixtures. Future UI/MCP authoring adapters must use the validated owned-image commands rather than treating arbitrary imported JSON as an upload. This is a deliberate separation, not a privileged synthetic asset namespace in core.

## Native worker boundary

`image_asset_snapshot(project, document)` captures only referenced project-owned metadata and diagnostics. It shares a private `Arc<Mutex<Connection>>` with the workspace; a worker can outlive the UI borrow without an ambient database path. Bytes are loaded lazily, one requested image at a time, instead of copying every encoded original in a document upfront.

Every `snapshot.load(node)` checks the exact src/assetId pair, captured identity, project, encoded length, checksum, MIME and dimensions. A second node with a wrong source cannot borrow bytes simply because another node uses the same asset ID. The adapter accepts owned native keys and preserved `/api/media/<same UUID>/content` references without fetching their old server route or rewriting the canonical source. HTTPS, file paths, data URLs, unknown keys and foreign resources remain unresolved. The worker receives already-authorized bytes through `AssetSource`; no network or arbitrary source-path loading is introduced.

## Verification and remaining acceptance

Fourteen new core fixtures cover four formats, source removal/reopen, original byte/dimension bounds, MIME spoofing, missing/foreign/mismatched references, opaque canonical preservation, lazy worker ownership, checksum corruption, failed writes, rich drafts/Undo/Redo/history, schema-4 upgrade and retention of a nonimage resource larger than the image limit. They have not been run locally; all Cargo checks run on GitHub.

The full native gate still needs author import/drop at the real location, cancellation/error feedback, image selection/sizing/alt/caption, actual rendering and restart evidence. The current renderer's 16M-pixel decode budget is not a core import limit and needs a large-image strategy before full E08 acceptance. Retaining original GIF/WebP bytes does not establish animation acceptance. Full `.wstudio` project compatibility, remaining media workflows, conflict copies and asset-aware export stay explicitly pending.

## Imported logical identities and frozen render capabilities

Source project IDs are opaque strings of at most 512 bytes, rather than necessarily UUIDs. Canonical image source/assetId bindings retain those original logical keys. The owning project resolves a media alias once into a native UUID; metadata and byte reads still require that same project. Native authoring asset APIs continue to use native UUIDs.

A render snapshot maps each requested logical key to verified native metadata and later reads that captured native identity, rather than resolving a mutable alias again. Clipboard exports retain the requested logical `ImageTransfer.source_id`; the destination imports/reuses verified owned bytes and returns an explicit logical-to-native binding. A foreign ID, capsule project hint, filename or source URL cannot authorize a lookup outside the fixed project. Syntax checks alone grant no resource authority.

Synthetic contracts cover two projects sharing an opaque image key, exact bytes, alias rebinding after snapshot capture, cross-project transfer, foreign alias targets, mismatched canonical src bindings and an actual original-project importer fixture. Their current hosted qualification is recorded separately from native interaction acceptance.
