# Versioning and releases

Build and package optimized artifacts on GitHub Actions. Never commit build trees, app bundles, installers, workspace databases or private manuscripts. Published tags and assets are immutable.

## Current decision

The owner explicitly authorized publishing the completed coherent checkpoint as **0.1.0-alpha.2**, with native installers and current README screenshots. The independent scope is `native-rich-editor-alpha`: an engineering prerelease of the implemented writing workspace. Complete original-editor/product parity and unfinished large workstreams continue in alpha.3. Publication does not close product issues or certify daily use. This supersedes earlier all-issues alpha.2 scheduling and editor-only deferrals.

## Versions and data

`workspace.package.version` in root Cargo.toml is the authored application version. Workspace crates inherit it; their Cargo.lock package versions must agree and hosted `cargo metadata --locked`, tests and builds must accept the unchanged dependency graph. Dependency revisions and checksums are never mass-updated to advance an app prerelease. Packaging derives exact SemVer, Apple numeric version and Windows/Debian equivalents. CI supplies the Apple build number.

Use SemVer 2.0.0. Advance alpha/beta/rc sequences for candidates of one intended release. Document breaking public contracts and data migrations; before 1.0, advance the minor version for incompatible public changes. Version 1.0 requires the complete declared legacy workflows and matching independent `full-parity` UX/migration gates. SQLite schema, rich-document schema, external `.wstudio` format and app version are independent. Preserve data and unknown source fields across their supported migrations.

## Engineering and release gates

- Source checks validate metadata, actual source fingerprints, tests, Clippy, OpenSpec and honest review disposition. They may produce clearly labeled engineering artifacts while a release gate is pending.
- The authoritative `docs/design/UX_GATE.json` must independently accept the exact version, current fingerprint and selected scope before a tagged release. Historical approval cannot override a current pending/rejected gate. The implementer does not approve their own review.
- Gate evidence includes at least two actual checked-in PNGs under `docs/images`, matching hashes, a named independent reviewer, no unresolved in-scope blockers and a nonempty interaction report. A source/assets/version or fingerprinted design-contract change invalidates acceptance. Gate JSON, review prose and screenshots are excluded from the fingerprint to avoid circular approval commits.
- `native-rich-editor-alpha` approval never means `full-editor-parity`, full migration, exhaustive IME/accessibility coverage or Windows/Linux desktop interaction. The release notes must name remaining work and platform limits.

```sh
python3 scripts/release-check.py --mode source
python3 scripts/release-check.py --fingerprint
python3 scripts/release-check.py --mode release --tag v0.1.0-alpha.2 --require-prerelease
```

## Hosted packaging

The reusable `.github/workflows/distribution-alpha2.yml` freezes one commit/version and builds the **native-editor-enabled normal application**, not the isolated preview or old default-editor portable archives. It verifies:

| Platform | Package | Hosted checks |
|---|---|---|
| macOS ARM64, 14+ | DMG | Mach-O, HFS+ image, Applications link, mounted version/signature, normal-app settled native captures |
| Windows x64 | Per-user NSIS setup | PE/static runtime, Start menu/app identity, install/remove, unchanged user data and unknown program files |
| Debian 12 / Ubuntu 24.04 amd64 | Debian package | ELF/dependencies/launcher, install/remove/purge and unchanged user data on both systems |

All three packages must share the exact commit, version and actual compiled-source fingerprint. `.gitattributes` preserves identical text checkout bytes across hosts; evidence hashing does not hide differences through normalization. The aggregate rehashes every installer and preserves per-platform evidence. Normal-app capture runs use isolated synthetic temporary databases and never access an owner's running preview or original project.

Engineering package metadata deliberately retains `release_ready:false` and `platform_ux_qualified:false`. Successful package checks do not manufacture independent GUI acceptance. `scripts/stage-installer-release.py` rechecks all three platform records/checksums against the tagged source and creates a separate scoped release manifest; original engineering records remain unchanged.

## Release sequence

1. Freeze completed scope, version/changelog and dependency pins. Preserve unfinished work in existing issues and the alpha.3 milestone.
2. Run hosted default/all-feature tests, strict Clippy/fmt, OpenSpec and native installer qualification. Check actual job outcomes and real captures; a workflow definition is not a successful run.
3. Inspect the normal packaged app with synthetic data, retain interaction/reopen evidence and actual README screenshots, then obtain independent exact-fingerprint scoped review. Keep owner issue acceptance separate.
4. Merge the reviewed source into main. Create and push its matching immutable prerelease tag only under an explicit release instruction (already granted for alpha.2).
5. `release.yml` verifies that existing tag and accepted gate, reruns default/native checks and calls the same installer graph at the frozen commit. The final job stages verified DMG/setup/deb, combined SHA256SUMS, installation notes and source/release manifests into a **new unpublished draft**. It refuses any existing release, including a draft; published assets are never replaced.
6. Inspect actual draft assets, checksums and source/version evidence, then publish the prerelease under the owner's release authorization. No automatic stable publication or unrequested signing/update infrastructure.

Build jobs have read-only repository permissions; only the draft job can write release contents. Actions, installer compiler and Linux build image are pinned. Optimized package scripts refuse accidental local builds. Source verification/fixture scripts may run locally without Cargo or creating installers.

## Installation and signing

See [INSTALL.md](../distribution/INSTALL.md). macOS is ad-hoc signed, not Developer ID signed/notarized; Windows is unsigned. Apple/Windows may require their normal user approval for downloaded apps. Linux requires its declared desktop/graphics/portal dependencies. Hosted installation checks do not qualify every desktop, file association or automatic update. Uninstall removes program files while preserving user workspaces. The normal app uses normal WriterCraft storage; isolated previews retain their own path. Never delete either while preparing a release.

Distribution retains MIT, bundled font/icon notices and target-resolved Rust dependency attribution. `generate-notices.py` inventories available normal-dependency texts and declared licenses, without private machine paths; it is not a legal completeness assertion. The published alpha.1 release keeps its original archives, metadata and evidence unchanged.

## Completeness and complete tasks

For alpha.3's `full-editor-parity` and later `full-parity`, the existing UX gate must also accept every inventory, capability and journey from `docs/migration/PARITY_SCOPE.json`. `scripts/parity_gate.py` is loaded by the existing release checker; no separate invocation or competing approval is required. Source mode reports pending coverage without claiming release readiness. Release mode refuses omitted/unknown/duplicate IDs, unreviewed inventory completeness, pending/stale evidence or incomplete task paths. Each independent observation carries the current source/lock/contract fingerprint and hashed reports; see `docs/design/ROLE_JOURNEYS.md`.

The initial101 capability rows, eight inventory groups and six journeys are a coverage scaffold. They are not101 delivered features, a percentage complete or a substitute for an exhaustive source audit. Grouped rows must be split/refined when the audit finds hidden behaviors. The validator proves consistent traceability only; independent native observation and real research judgment remain required.
