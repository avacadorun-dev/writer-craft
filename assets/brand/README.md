# WriterCraft lion identity

The approved lion, quill and open book retain their original cream/ink artwork and blue texture. The same mark is used in Light and Dark. Utility icons remain the separate Lucide family.

## Approved sources

| File | SHA-256 | Format |
| --- | --- | --- |
| `lion-logo-approved.png` | `43cd82c1e450674a2dca93a1c68c8f89002c361d0112aea44f5a073b98fa3329` | Original opaque 1254×1254 raster |
| `writercraft-mark.svg` | `9475584f66032e04efb8a1a249a681102ea4aea5df91dfa3455fb75cddfcc95c` | Original supplied SVG containing the embedded raster and rounded clip |

Both supplied files remain byte-for-byte unchanged. The SVG contains raster artwork; it is not a traced vector lion. Its original rounded-rectangle radius is 250/1254 of the tile width.

## Application assets

- `writercraft-mark-{32,64,128}.png`: direct-from-source full-tile exports for a 32-point header at matching display density. These have no outer safe-area padding.
- `writercraft-mark.png`: 512-pixel full-tile export for larger brand surfaces and the README banner.
- `../WriterCraft.png`: 512-pixel system/runtime icon, with a 448-pixel visible tile and 32-pixel transparent inset on each side.
- `../WriterCraft.icns`: 16/32/128/256/512-point renditions at 1×/2×. The 1024-pixel rendition has an 896-pixel tile and 64-pixel inset. The existing Windows packager reads the 256-pixel PNG `ic08` member.

Rounded tile interiors are opaque. Only the approved rounded corners and system-icon safe area are transparent. Exports add no backing, crest or shadow and do not recolor the artwork. Header rendering uses its matching small rendition to preserve dense linework during minification.

## Regeneration

On macOS, from the repository root:

```sh
scripts/make-icon.sh
```

The native AppKit/CoreGraphics exporter verifies the source hashes, resamples in sRGB, applies the approved corner geometry and validates visible bounds/interior opacity. `iconutil` builds the ICNS. Temporary iconsets and the Swift module cache stay outside the repository and are removed after generation.

To retain the iconset, per-size validation receipt and same-size Light/Dark comparison in a separate review folder:

```sh
scripts/make-icon.sh /tmp/writercraft-icon-review
```

The comparison is an asset rendering, not an application screenshot. Source artwork and output assets are intentional application inputs; review plates, receipts and build caches are not committed.
