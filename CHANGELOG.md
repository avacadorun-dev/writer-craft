# Changelog

All notable product and compatibility changes are recorded here. Versions follow [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html); release status and known limits are explicit.

## [Unreleased]

- Alpha.3 continues complete table/clipboard parity, note/dictionary and assistant integrations, full project/Canvas/media migration, portable restore and the remaining native acceptance matrix.

## [0.1.0-alpha.2] - 2026-11-08

A native rich-editor engineering checkpoint. The owner approved shipping the completed implementation now and carrying unfinished large migration workstreams into alpha.3. This is not complete Writers Studio feature parity or a daily-use certification.

### Added

- Native rich manuscript editing with formatting, structured block insertion and property dialogs; integrated tables, image assets, Mermaid diagrams, inline/display formulas, code and callouts.
- Syntax decoration for code and structured text, local rich revisions and recoverable drafts, and the current persistent passage-note implementation.
- Adaptive bounded document surfaces, comfortable shared fields/modals, configurable manuscript typography and offline bundled fonts.
- Eight UI languages, Light/Dark/System, seven non-green palettes, Lucide controls and the monk/book app identity.
- Independent persistence acknowledgement, preserved unknown preference fields, explicit Retry/Reload and session-only error status for each preference file.
- Local data-location/backup controls and shared core foundations for complete original-project preservation and portable exports.
- Installable macOS ARM64 DMG, per-user Windows x64 setup executable and Debian/Ubuntu amd64 package, with checksums and same-source verification evidence.

### Fixed

- Images, diagrams, formulas and wrapped captions reserve their actual native flow height rather than painting over following blocks.
- Preserve exact checkout bytes across platforms so installer source fingerprints describe the actual compiled inputs.
- Preserve local workspace data when installing/removing program files and refuse protected-data export collisions.

### Scope and installation

- macOS Apple silicon on macOS 14 or newer is the primary desktop target. Windows x64 and Debian 12/Ubuntu 24.04 amd64 packages have hosted installation/removal checks; full desktop interaction on those systems remains unqualified.
- macOS uses ad-hoc signing and is not notarized. Windows is unsigned. Automatic updates are not implemented.
- Complete table/clipboard/IME/accessibility parity, full note/dictionary and rich assistant workflows, original-project import UI, Canvas, PDF/audio/video and canonical restore remain alpha.3 work. Current preservation APIs or a showcase do not imply completed end-user workflows.
- The normal application uses its normal WriterCraft data directory; an isolated engineering preview uses a separate workspace. App version, SQLite schema and transfer formats remain independent.
- Product issues remain open for their own verification and owner acceptance; shipping this checkpoint does not close them.

## [0.1.0-alpha.1] - 2026-11-08

Prepared first engineering prerelease. This entry does not assert that a tag, public release or UX approval already exists.

### Added

- MIT-licensed project source with retained third-party asset notices.
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
