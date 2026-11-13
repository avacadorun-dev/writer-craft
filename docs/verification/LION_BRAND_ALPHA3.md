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

## Exact-source compiled integration

Candidate: `c4e33025ec9f14b6736b46e20eb3523a033c430f` (asset commit `6944915`, native/banner commit `63b605b`). [Verify42550104096](https://github.com/avacadorun-dev/writer-craft/actions/runs/42550104096) passed macOS workspace/all-feature tests, Clippy, compatibility/spec/release tooling and Windows/Linux shared-core checks. [Preview42550098189](https://github.com/avacadorun-dev/writer-craft/actions/runs/42550098189) passed and produced the signed native package and capture artifacts.

Downloaded package SHA-256: `0f2aaaea4c3f76a518623064e937aaa39a374df00cdd5ef2733d30e00dfdad3d`. Its ICNS is byte-identical to the new repository ICNS. The binary contains the exact current runtime PNG and all three header renditions. The actual Windows packager produced an ICO whose PNG payload is byte-identical to the approved `ic08` rendition; this is icon-path verification, not Windows interaction qualification.

Four [native captures](../images/alpha3/README.md) from the original signed executable verify Light/Dark at 1000×680 and 1440×923 points, DPR 2. The independent designer accepted the actual 32-point header, 8-point wordmark gap, vertical centering, native-control clearance and shared navigator boundary, without clipping or overlap. The source/banner/asset review also passed.

A separately identified preview opened with an isolated synthetic workspace. macOS returned the approved lion from `NSRunningApplication.icon`; the [actual OS icon](../images/alpha3/running-os-icon.png) and [sanitized receipt](../images/alpha3/branding-evidence.json) are retained. Local re-signing changed embedded signature bytes, while all 13 native code/data sections remained identical. The four manuscript captures used the original, unmodified signed executable. Dock pixels themselves were not observed; no Dock screenshot or whole-alpha.3 acceptance is claimed.

The subsequent README/evidence commit is documentation-only. Historical alpha.2 images and published release artifacts remain unchanged. Existing live development/user workspaces were not changed or restarted.

## Final independent disposition

Accepted on 2026-11-09 by the separate `alpha3_independent_designer` reviewer for the exact issue #16 branding scope at `c4e33025ec9f14b6736b46e20eb3523a033c430f`. Assets, banner, all four compiled headers, packaged icon sources and the actual macOS-resolved running-application icon satisfy the stated criteria. No branding blocker remains. Direct Dock pixels were not observed and are not a criterion in issue #16. The full alpha.3 release gate remains pending.
