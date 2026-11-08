# Alpha.2 release verification

Target: `0.1.0-alpha.2`, engineering prerelease, named scope `native-rich-editor-alpha`. The owner explicitly selected the completed coherent checkpoint for publication and moved unfinished large parity work to alpha.3. Publication does not close product issues or establish daily-use/full-parity acceptance. Published alpha.1 remains immutable.

## Frozen runtime and hosted checks

Candidate runtime source: `8af4233ba5e531b360ee4914e4a5d5996f7bb72e`. Runtime/asset/design fingerprint: `627547cc12f80c551c9ae84947d16d370dc9db263c80b1bcaee01864596da0c9`. App lock SHA-256: `002f30af62e81ef39e8949b4971987cde435de0dc3ca7dd32b41054e2c6ada6e`. CraftEditor stays pinned at `21788bdd9e9ea8c4d2d3f4c90900633098aced7c`; no unqualified table/Canvas consumer pin is included.

The preceding `bde1be1` runtime passed [main verification 42492511935](https://github.com/avacadorun-dev/writer-craft/actions/runs/42492511935) (270 default workspace tests, 12 isolated model tests, strict Clippy/fmt and ten strict OpenSpec contracts) and [all-feature verification 42492506690](https://github.com/avacadorun-dev/writer-craft/actions/runs/42492506690) (372 workspace tests). Windows/Linux shared-core checks passed. Final-source runs: [main 42493466703](https://github.com/avacadorun-dev/writer-craft/actions/runs/42493466703), [native 42493462208](https://github.com/avacadorun-dev/writer-craft/actions/runs/42493462208), [installers 42493462622](https://github.com/avacadorun-dev/writer-craft/actions/runs/42493462622). Final-source main verification completed successfully with 273 default workspace tests, 12 isolated model tests, strict Clippy/fmt and ten valid OpenSpec contracts. All-feature native verification completed successfully with **375 workspace tests**. Every native installer job and the exact-source aggregate also completed successfully in run 42493462622: combined artifact 13623035886, 166,833,983 bytes. The aggregate rechecked all three package records/checksums at one commit/version/fingerprint.

Release tooling separately passed 11 installer fixtures, six promotion/staging fixtures, four capture-runner fixtures and Actionlint. Optimized compilation and native package generation were performed only on hosted runners.

## Normal app and capture evidence

The package graph builds the native editor under `app.writercraft.desktop`, with `WriterCraftNativeEditor:true`; it never enables preview identity merely to pass the installer checks. Normal-app capture mode uses a fresh synthetic database per case, explicit demo data and actual WGPU screenshot readback. It records observed dimensions, current readiness, target visibility and unchanged canonical JSON. Requested wide sizes constrained by the hosted display are not wide acceptance.

At `bde1be1`, 19/20 normal-app cases passed; the enlarged lower-caption heading case failed honestly because semantic navigation consumed a pre-settled position and never revealed the target. No PNG/readback or success was invented. The capture-only `8af4233` correction permits one extra semantic navigation per renderer generation after three identical, settled native geometry observations, only for an unseen target. Visible tall objects never retry. Three regressions retain real visibility/readiness checks and prove bounded attempts. This changes capture instrumentation, not manuscript spacing or product rendering. The corrected final normal-app run passed **20/20** cases under the normal app identity. Artifact 13622593019 (2,237,128 bytes), evidence SHA-256 `e473a1bbf0de473846f7ea020c1f2087bc78e83e60934028e6b8edbc81f74d1d`, executable SHA-256 `72031566e901cf983f542ef6ed6b2c945d6f83f87fa4d029bc48fd4ceea44795`. Every target was visible, settled and canonical-preserving; actual-wide and complete native-interaction claims remain false.

README screenshots are real normal-app captures with synthetic public artwork/text; their source, case, actual dimensions and hashes are recorded in [CAPTURES.json](../images/alpha2/CAPTURES.json). The independent reviewer owns the final scoped decision and gate.

## Native local observation

A hosted `c277a3c` alpha.2 macOS ARM64 DMG was checksum-verified, mounted read-only, and its normal app signature verified. Package SHA-256: `e39642a3db4880079d5c78a8cce98b41557c3a73aab5dfab498502867848bbb3`. A private copy of that exact executable was launched through native computer use. Only its launch environment was redirected to a new temporary synthetic workspace/demo, and the copy was ad-hoc re-signed; normal app identity remained intact. The original DMG, owner projects and previous preview were untouched.

Observed: normal WriterCraft window, correct monk/book icon, version `0.1.0-alpha.2`, local/saved status and document navigation. The actual Add document modal/Return created a new rich document and exposed the real formatting/Insert/Undo/Redo toolbar. A native paste of a synthetic paragraph and Command-S produced the exact displayed paragraph, nine words, revision 2 and saved status. A read-only query independently confirmed canonical rich JSON at revision 2. A genuine wide local screenshot showed the bounded floating manuscript and reachable controls; this is not exhaustive adaptive-transition acceptance.

CUA `typeText` into the new-document title resulted in literal `2` rather than the complete requested ASCII title. This remains an unqualified automation/input observation; neither successful complete typing nor a proven application cause is claimed. The Mac then locked before remaining formatting, reopen and preference-error interaction checks. After the owner unlocked it, native CUA confirmed the full Insert menu and Mermaid source/caption dialog. The subsequent source-edit automation was interrupted by stale accessibility IDs and a CUA timeout; the next fresh observation showed the library with Files & recovery settings and the correct isolated database path. No successful final diagram insertion, formatting or reopen is inferred from that interrupted action. No automatic unlock, private manuscript use or fabricated success followed.

The subsequent source changes before this candidate are scoped preference-test semantics and explicit synthetic capture fixture/navigation logic; no ordinary authoring/persistence change was identified. The independent review must state any evidence carried from that earlier native executable. Full IME/accessibility/input, note/dictionary/proposal, preference recovery and other deferred journey matrices remain open in alpha.3.

## Installation and publication boundary

Required packages: macOS ARM64 DMG (ad-hoc signed, not notarized), unsigned per-user Windows x64 setup and Debian/Ubuntu amd64 package. Hosted checks cover normal identity, architecture/notices, mounted DMG/version/signature/Applications link, Windows owned-file install/removal/data preservation and Debian 12/Ubuntu 24.04 install/remove/purge/data preservation. Installation checks are not complete OS desktop UX acceptance.

The tag workflow must verify the current independent gate and exact existing prerelease tag, rerun source checks, rebuild/verify all three installers and stage their rehashed packages into a **new draft** with combined `SHA256SUMS` and source/release manifests. It refuses an existing release. Engineering evidence retains its original false platform-UX/release flags; separate scoped release evidence records the engineering-alpha decision. Publish only the fully checked draft under the owner's existing authorization.


## Final checkpoint disposition

All final-source hosted source/native/installer checks named above completed successfully. The independent review and authoritative UX gate record the accepted engineering scope separately from future full-parity acceptance. The tag run will rebuild the same runtime fingerprint from its final documentation/gate commit, verify installed-package identity and prepare new draft assets. README screenshot provenance binds to the final 8af4233 normal-app executable; subsequent repository documentation and gate commits do not change that runtime fingerprint. Product issues remain open for owner checks or alpha.3 completion.


## Published release receipt

Published prerelease: [WriterCraft 0.1.0-alpha.2](https://github.com/avacadorun-dev/writer-craft/releases/tag/v0.1.0-alpha.2), 2026-11-08 at 15:52:50 UTC. Immutable tag commit: `52770caaceb9c9e06b60130024fa230cf7e93ead`. [Tagged run 42494556276](https://github.com/avacadorun-dev/writer-craft/actions/runs/42494556276) completed successfully through source/default/native checks, all platform installers, Ubuntu verification, exact-source aggregation and draft staging. Independent scope remains `native-rich-editor-alpha`.

All 12 uploaded assets were inspected before publication. The three GitHub asset digests, byte sizes, combined SHA256SUMS and scoped distribution/release manifests agree. Anonymous public installer HEAD requests returned HTTP200 and matching lengths. Final tagged normal-app capture artifact 13622838164 passed 20/20 cases, executable SHA-256 `daccbbea2375c617de08426a7466c82d1e254fe141977de25bee94c55c966707`, evidence SHA-256 `e530ee26ececa39016af9d05c746f6e1499f4fa04c7659ccdd73ec497c669a22`.

| Published installer | Bytes | SHA-256 |
|---|---:|---|
| `WriterCraft-0.1.0-alpha.2-linux-amd64.deb` | 46544788 | `5fbf0908319ff0d618583946bec498aa972f9c99b806c7cfa8a2b95898aea9d2` |
| `WriterCraft-0.1.0-alpha.2-macos-arm64.dmg` | 61723094 | `d322bc5f0ae715a0d374e93b358072abb0667fabf94f452d5e3114d21a83c792` |
| `WriterCraft-0.1.0-alpha.2-windows-x64-setup.exe` | 58545743 | `312abeed8bff3e1b36d1fd8a4f695da47c5739b8e40246b4593f55fc3262274f` |

Issues #3 and #12 retain published evidence and verified checkboxes, remaining open for owner acceptance. The eight incomplete broader workstreams stay in the alpha.3 milestone. No published alpha.1/alpha.2 tag or asset was replaced. This documentation receipt does not change the accepted runtime fingerprint.
