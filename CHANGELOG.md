# Changelog

All notable product and compatibility changes are recorded here. Versions follow [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html); release status and known limits are explicit.

## [Unreleased]

- Continue the verified native migration toward the complete legacy workflow inventory.
- Close and independently retest the desktop UX findings before accepting the named release scope.

## [0.1.0-alpha.1] - 2026-11-08

Prepared first engineering prerelease. This entry does not assert that a tag, public release or UX approval already exists.

### Added

- Native Rust macOS foundation with a local project library and continuous prose workspace.
- Shared SQLite core, immutable revisions, durable drafts and conflict-aware recovery.
- Staged exact-edit proposals with explicit Apply/Reject and guarded Undo.
- Bounded `.wstudio` v1–v8 inspection retaining source manifests and rich document JSON.
- Eight interface languages, persisted language/font settings and CJK font fallback.
- Independent Light/Dark/System modes and seven palettes with native workspace preview cards.
- Categorized desktop preferences, aligned shell boundaries and a native monk/book brand mark.
- Hosted macOS ARM64, Windows x64 and Linux x64 engineering preview packages.
- Headless project and archive CLI; OpenSpec contracts, architecture decisions and migration inventory.
- Independent desktop UX review, feedback-driven acceptance and GitHub-hosted preview/release plumbing.

### Scope and compatibility

- Engineering alpha for macOS on Apple silicon; Windows/Linux desktop distribution is not qualified.
- Native rich editing, anchored notes and dictionary links, live AI, voice/audio review, Canvas, complete project import and publishing remain unfinished.
- Current local prose editing and a `.wstudio` text preview do not imply legacy feature parity.
- App SemVer, SQLite schema, rich-document schema and `.wstudio` format versions are independent.
- Preview bundles are ad-hoc signed; notarization and automatic updating are not implemented.

### Verification

Release notes must link the actual GitHub run and independent named-scope UX acceptance when a candidate is produced. Source checks and hosted artifacts are separate from public-release approval.
