# Dependency quality and advisory snapshot

Checked 2026-11-07 against the exact crates.io package names and versions in Cargo.lock. This review used official registry/release metadata and the OSV API; no project source, user files or credentials were uploaded and no audit tool was installed. It covers published dependency metadata and selected source paths, not a complete security assessment.

## Decisions and resulting lockfile

| Dependency | Initial → selected | Evidence and reason |
|---|---|---|
| zip | 3.0.0 → 9.4.1 | Current stable in [crates.io](https://crates.io/crates/zip/9.4.1) and the [official release](https://github.com/zip-rs/zip2/releases/tag/v9.4.1). Initial version had exact advisory matches; upgrading removes those matches. Deflate-only features and bounded read-only inspection remain. |
| rusqlite | 0.37.0 → 0.41.1 | Current stable in [crates.io](https://crates.io/crates/rusqlite/0.41.1) and the [official release](https://github.com/rusqlite/rusqlite/releases/tag/v0.41.1). Newer maintained binding and bundled engine replace the initial older baseline; all core persistence/migration/recovery tests pass. |
| libsqlite3-sys / SQLite | 0.35.0 / 3.50.2 → 0.39.1 / 3.54.1 | Versions verified from Cargo.lock and the downloaded crates' bundled sqlite3.h constants. The bundled engine is application-local and does not require a user's SQLite installation. |
| rfd | 0.17.2 → 0.18.0 | [Official registry metadata](https://crates.io/api/v1/crates/rfd/0.18.0) identifies stable 0.18.0, published 2026-11-07, MSRV 1.92. The GitHub releases page was behind this publication; registry data was checked before changing the constraint. Native file-dialog behavior still needs the integrated UI smoke test. |
| eframe / egui | 0.36.2 → retained exact0.36.2 pair | [Official releases](https://github.com/emilk/egui/releases) and registry metadata show0.37.0 is newer. The matching published0.36.2 pair is retained to stabilize the already integrated native UI and avoid mixing framework/API changes into localization. This is a compatibility choice, not a claim it is latest or permanently free of advisories. Review the0.37 migration as a separate tested UI change. |

Registry-reported minimum Rust versions for the selected updates: zip 1.88, rusqlite 1.89, rfd 1.92; all fit the repository's 1.97 minimum. The retained egui/eframe pair requires 1.95. No compatibility source change was needed for the core or transfer modules.

## Advisory checks before and after

The exact-version [OSV batch API](https://google.github.io/osv.dev/post-v1-querybatch/) was queried for all395 registry package/version entries before the upgrade and again for all395 entries after resolution. Counts include platform-conditional dependencies, not only those compiled on macOS. Advisory aliases can produce several records for one issue; they are not counted as independent vulnerabilities.

Initial zip3.0.0 matches included:

- [GHSA-8hxf-qr8p-62x4](https://github.com/zip-rs/zip2/security/advisories/GHSA-8hxf-qr8p-62x4): AES authentication on premature EOF; fixed9.0.0.
- [GHSA-m665-v7pm-7grj](https://github.com/zip-rs/zip2/security/advisories/GHSA-m665-v7pm-7grj): inconsistent ZIP headers and parser interpretation; fixed9.0.5.
- [RUSTSEC-2026-0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html), [0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html), [0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html) and [0429](https://rustsec.org/advisories/RUSTSEC-2026-0429.html): unsafe extraction permissions/symlinks or unbounded symlink allocation; patched ranges begin9.0.0.

The current inspector does not call extraction, does not enable AES, rejects nonregular entries/ZIP64 and bounds declared reads. These facts reduce exposure to the specific old paths but were not used to justify keeping an outdated archive parser. zip9.4.1 has no match in the repeated exact-version query.

After the upgrades, the remaining matches are:

| Package | Locked version | Published classification |
|---|---|---|
| zbus_names | 4.3.5 | [RUSTSEC-2026-0371](https://rustsec.org/advisories/RUSTSEC-2026-0371.html), unmaintained standalone crate |
| zvariant | 5.15.1 | [RUSTSEC-2026-0370](https://rustsec.org/advisories/RUSTSEC-2026-0370.html), unmaintained standalone crate |
| zvariant_derive | 5.15.1 | [RUSTSEC-2026-0372](https://rustsec.org/advisories/RUSTSEC-2026-0372.html), unmaintained standalone crate |

These are maintenance notices, not vulnerability proofs. Their functionality moved into zbus6. The reverse tree goes through egui-winit → accesskit_winit → accesskit_unix / AT-SPI / zbus5. They remain after the rfd update, so attributing them to rfd would be incorrect. `cargo tree -i zvariant --target aarch64-apple-darwin --edges normal --locked` reports nothing; `--target all` identifies the conditional Unix accessibility chain. Keep this visible for later Linux delivery, follow upstream migration and preserve accessibility rather than deleting it to hide notices.

No exact OSV match was returned for the selected rusqlite/libsqlite3-sys or egui/eframe versions in this snapshot. That observation does not replace bundled-C-engine, operating-system, license, provenance or future-advisory review. The report is deliberately not labelled “audit clean”.

## Verification after update

- `cargo update -p zip -p rusqlite -p rfd` updated the workspace manifest resolution and Cargo.lock.
- `cargo test -p studio-core -p studio-transfer --locked` at the dependency-upgrade checkpoint:35 passed (core24, transfer11), including additive database migration, stale-draft recovery and hostile ZIP fixtures.
- `cargo clippy -p studio-core -p studio-transfer --all-targets --all-features --locked -- -D warnings`: passed after the upgrades. The final integrated native build and file-dialog smoke test remain separate release evidence.
- Portable core/transfer CI remains useful, while the first full desktop release is macOS. Windows/Linux UI and installer readiness require later platform-specific verification.

Re-run exact-version advisory queries after dependency changes and before a release. Preserve finding classifications and target conditions; do not silently suppress unresolved notices or describe an untested dependency update as complete UI verification.
