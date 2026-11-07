# WriterCraft 0.1.0-alpha.1 release record

Status: **published prerelease**, not stable. Published at `2026-11-07T19:22:07Z` (2026-11-08 in the project owner's UTC+7 time zone).

- [Release and downloads](https://github.com/avacadorun-dev/writer-craft/releases/tag/v0.1.0-alpha.1)
- Immutable annotated tag: `v0.1.0-alpha.1`
- Source commit: `6b2756533487cf2e52f9aa3043be6c4f6cb610a6`
- Workspace version: `0.1.0-alpha.1`; project license: MIT, with retained third-party terms.
- [Source and three-platform preview verification](https://github.com/avacadorun-dev/writer-craft/actions/runs/42404787215): all jobs passed.
- [Tagged release workflow](https://github.com/avacadorun-dev/writer-craft/actions/runs/42405370213): accepted macOS candidate, both portable candidates and draft assembly passed.

## Qualified scope

The independent macOS `native-foundation` acceptance matches source fingerprint `6bb777e7ced2e4559bc14312ab54beea2f28160b63ccaaf7bb770d2bffd14fd0`. See [UX_GATE.json](../design/UX_GATE.json), [review](../design/UX_REVIEW_003.md) and [native interaction evidence](NATIVE_INTERACTIONS.md). The final packaging-only commit did not change this fingerprint.

The macOS ARM64 app targets macOS 14+ and is ad-hoc signed, not Developer ID notarized. Windows x64 and Linux x64 GNU-libc archives are unsigned engineering previews; native UX, installer integration, automatic updates and complete legacy workflow parity are not qualified. The Linux build uses Ubuntu 24.04 and requires compatible desktop runtime libraries.

## Completed verification

- 71 workspace tests, strict all-target/all-feature Clippy and formatting passed.
- 12 isolated editor-model hypothesis tests passed; that model is not a production editor dependency.
- 24 release/packaging fixture checks and 7 strict OpenSpec validations passed.
- All three final release archives were downloaded before publication. Local inspection verified archive integrity, SHA-256, native binary architecture, exact source metadata, MIT and bundled font/icon/dependency notices. The macOS bundle's monk ICNS/metadata and strict ad-hoc code signature were verified without launching it or replacing the owner's running preview.
- The release manifest, accepted macOS evidence, portable scope evidence and the tagged source agree. No local optimized release build was made.

## Published archive checksums

| Platform | SHA-256 |
| --- | --- |
| macos-arm64 | `8680e750c86008f28b9976db523560e268e0cc521564ab50930a76e14c6a4927` |
| windows-x64 | `716e1e2fbfec0b8971adc0e309511c5fdcbb9ae6f82205a09235e81e4d11ba6e` |
| linux-x64 | `85b5fd4220c611f719ed0cbb90482169328e05413db6de73416743da1e1d3128` |

The release supplies `SHA256SUMS`, `release-manifest.json`, per-platform evidence and readable release notes. Published assets and the tag are immutable. README/download-navigation and this verification record are post-publication documentation; they do not change the released source.

## Follow-up boundary

[Issue #1](https://github.com/avacadorun-dev/writer-craft/issues/1) retains the owner's final Dock-appearance check; a native About-panel check is not presented as direct Dock capture. [Issue #2](https://github.com/avacadorun-dev/writer-craft/issues/2) records future persistent text marks and author notes. Neither an open planning issue nor this alpha implies full legacy parity. The owner and maintainers will select the next scope together under [NEXT_RELEASE.md](../governance/NEXT_RELEASE.md).
