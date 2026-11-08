# Project media clipboard jobs

This is an implementation checkpoint for the full editor alpha.2 contract, tracked in [issue #7](https://github.com/avacadorun-dev/writer-craft/issues/7) and [epic #4](https://github.com/avacadorun-dev/writer-craft/issues/4).

CraftEditor owns canonical rich slices, exact edit guards, one-shot write acknowledgements and explicit native transport formats. WriterCraft owns project authorization, SQLite resources, acquisition policy and localized diagnostics. No project/SQLite policy is added to the reusable engine.

`Workspace::project_images` creates a fixed-project background handle sharing only the connection Arc and project identity. Exports verify only selected assets. Bulk imports validate all descriptors, original encoded headers, MIME, dimensions, byte lengths and SHA-256 values before starting an atomic transaction. Existing IDs are reused only when the destination project already owns identical verified bytes. Foreign IDs never trigger an unscoped lookup. A later stale document guard can leave unused durable assets; it cannot change writing.

`ProjectClipboard::prepare_copy`/`prepare_paste` consume an immutable captured bundle off the UI thread. The registered `application/vnd.writercraft.images+json` format uses the valid macOS UTI `app.writercraft.clipboard-images-json`. Its strict versioned JSON envelope contains the digest of the exact private rich bytes and original image bytes encoded with standard padded base64. The combined native clipboard budget includes every representation and attachment. Only image `assetId`/`src` bindings are patched after destination import; opaque metadata and source references stay canonical. A capsule is not retained after those bytes change.

The base64 dependency is pinned to the already resolved maintained `base64` 0.23.1 package. The implementation uses the documented `general_purpose::STANDARD` engine and fallible decode APIs, rather than a new binary container or custom codec: [official Engine documentation](https://docs.rs/base64/0.23.1/base64/engine/trait.Engine.html). SHA-256 reuses the workspace dependency. Native raw capture remains subject to the clipboard backend's lack of a cross-format generation lock; digest binding prevents unrelated private media association, and no equal-plain-text cache is treated as authority.

External HTML URLs return an explicit import requirement before document application or image DB mutation. Acquisition and visible progress/error UX are subsequent required app work. The renderer never fetches. The original file-drop handler accepts one PNG/JPEG/GIF file up to12MiB; no raw bitmap/file clipboard upload feature was found in the frozen source, so that separate gesture is not invented here.

Verification must name exact hosted source and tests. Local Cargo/fetch/build and author clipboard access are not used. Internal capsule tests do not establish complete native clipboard UX or full-editor acceptance.
