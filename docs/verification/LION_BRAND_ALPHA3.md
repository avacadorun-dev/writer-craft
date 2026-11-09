# Lion brand integration — alpha.3

Issue: [#16](https://github.com/avacadorun-dev/writer-craft/issues/16). This is a branding-only acceptance record; it does not approve the alpha.3 release or unrelated editor workflows.

## Source and treatment

The supplied PNG is the unmodified 1254×1254 opaque engraving, SHA-256 `43cd82c1e450674a2dca93a1c68c8f89002c361d0112aea44f5a073b98fa3329`. The supplied rounded SVG remains byte-for-byte unchanged, SHA-256 `9475584f66032e04efb8a1a249a681102ea4aea5df91dfa3455fb75cddfcc95c`. It embeds this raster, not vector paths for the lion. Both live in `assets/brand/`.

Preserve the lion, eyes, quill, book and textured blue. The original blue was checked against Light/Dark panel fills; a palette tint would recolor the drawing or require a separate approved background mask. The shipped treatment therefore keeps the source pixels. Rounded corners use the supplied 250/1254 radius ratio; only the exterior is transparent.

## Active surfaces

- Header: 32 logical pixels, 8 px wordmark gap, existing native-control clearance; 32/64/128 px native renditions selected by display density. Decode at startup, no per-frame image conversion.
- Runtime/Linux icon: 512 px PNG, 448 px visible tile centered with 32 px inset.
- macOS: all ten standard ICNS renditions, including a 1024 px image with 896 px tile and 64 px inset.
- Windows: the existing packager preserves the ICNS `ic08` 256 px PNG inside its ICO; no second drawing or independent replacement icon.
- README: measured, centered 1280×300 logical banner, exported at 2×; 172 px tile, 32 px gap to text. Historical alpha.2 screenshots stay labelled historical.
- Utility controls remain Lucide. No product name, theme behavior, document data, dependencies or published releases change.

Regenerate assets with `./scripts/make-icon.sh`; an optional staging directory preserves size comparisons and alpha/bounds receipts without writing the live assets. Generate the banner using `swift -module-cache-path "$TMPDIR/writercraft-banner-cache" scripts/MakeBanner.swift assets/brand/writercraft-mark.png docs/images/writercraft-banner.png` on macOS.

## Checks recorded before hosted integration

- Native AppKit generator validates the source hash/dimensions, opaque interior and exact full-tile/OS bounds for every rendition.
- Two exports produced byte-identical icon PNG/ICNS files. Independent review decoded all ten ICNS slots, including small legacy renditions, and compared the PNG payloads.
- Independent asset review accepted preserved artwork, upright orientation, corners and 32/64/128 px legibility. The small-header resampling concern was addressed with density-specific native exports.
- Existing packaging contract tests: 11 passed, including preservation of the original 256 px Windows ICO payload.
- OpenSpec strict validation: 19/19 contracts passed; the unrelated pre-existing table capture heading remains an informational notice.
- Direct rustfmt and `git diff --check` passed. Rust compilation, native captures, actual packaged icon and final independent integration review remain pending until the exact hosted candidate is recorded below.
