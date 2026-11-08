# Alpha.2 release verification

Target: `0.1.0-alpha.2`, engineering prerelease, named scope `native-rich-editor-alpha`. The owner explicitly selected the completed coherent checkpoint for publication and moved unfinished large parity work to alpha.3. Publication does not close product issues or establish daily-use/full-parity acceptance. Published alpha.1 remains immutable.

## Frozen runtime and hosted checks

Candidate runtime source: `8af4233ba5e531b360ee4914e4a5d5996f7bb72e`. Runtime/asset/design fingerprint: `627547cc12f80c551c9ae84947d16d370dc9db263c80b1bcaee01864596da0c9`. App lock SHA-256: `002f30af62e81ef39e8949b4971987cde435de0dc3ca7dd32b41054e2c6ada6e`. CraftEditor stays pinned at `21788bdd9e9ea8c4d2d3f4c90900633098aced7c`; no unqualified table/Canvas consumer pin is included.

The preceding `bde1be1` runtime passed [main verification 42492511935](https://github.com/avacadorun-dev/writer-craft/actions/runs/42492511935) (270 default workspace tests, 12 isolated model tests, strict Clippy/fmt and ten strict OpenSpec contracts) and [all-feature verification 42492506690](https://github.com/avacadorun-dev/writer-craft/actions/runs/42492506690) (372 workspace tests). Windows/Linux shared-core checks passed. Final-source runs: [main 42493466703](https://github.com/avacadorun-dev/writer-craft/actions/runs/42493466703), [native 42493462208](https://github.com/avacadorun-dev/writer-craft/actions/runs/42493462208), [installers 42493462622](https://github.com/avacadorun-dev/writer-craft/actions/runs/42493462622). Their completed results must be inspected before tagging.

Release tooling separately passed 11 installer fixtures, six promotion/staging fixtures, four capture-runner fixtures and Actionlint. Optimized compilation and native package generation were performed only on hosted runners.

## Normal app and capture evidence

The package graph builds the native editor under `app.writercraft.desktop`, with `WriterCraftNativeEditor:true`; it never enables preview identity merely to pass the installer checks. Normal-app capture mode uses a fresh synthetic database per case, explicit demo data and actual WGPU screenshot readback. It records observed dimensions, current readiness, target visibility and unchanged canonical JSON. Requested wide sizes constrained by the hosted display are not wide acceptance.

At `bde1be1`, 19/20 normal-app cases passed; the enlarged lower-caption heading case failed honestly because semantic navigation consumed a pre-settled position and never revealed the target. No PNG/readback or success was invented. The capture-only `8af4233` correction permits one extra semantic navigation per renderer generation after three identical, settled native geometry observations, only for an unseen target. Visible tall objects never retry. Three regressions retain real visibility/readiness checks and prove bounded attempts. This changes capture instrumentation, not manuscript spacing or product rendering. The required final 20-case run must pass before the scoped release gate.

README screenshots are real normal-app captures with synthetic public artwork/text; their source, case, actual dimensions and hashes are recorded in [CAPTURES.json](../images/alpha2/CAPTURES.json). The independent reviewer owns the final scoped decision and gate.

## Native local observation

A hosted `c277a3c` alpha.2 macOS ARM64 DMG was checksum-verified, mounted read-only, and its normal app signature verified. Package SHA-256: `e39642a3db4880079d5c78a8cce98b41557c3a73aab5dfab498502867848bbb3`. A private copy of that exact executable was launched through native computer use. Only its launch environment was redirected to a new temporary synthetic workspace/demo, and the copy was ad-hoc re-signed; normal app identity remained intact. The original DMG, owner projects and previous preview were untouched.

Observed: normal WriterCraft window, correct monk/book icon, version `0.1.0-alpha.2`, local/saved status and document navigation. The actual Add document modal/Return created a new rich document and exposed the real formatting/Insert/Undo/Redo toolbar. A native paste of a synthetic paragraph and Command-S produced the exact displayed paragraph, nine words, revision2 and saved status. A read-only query independently confirmed canonical rich JSON at revision2. A genuine wide local screenshot showed the bounded floating manuscript and reachable controls; this is not exhaustive adaptive-transition acceptance.

CUA `typeText` into the new-document title resulted in literal `2` rather than the complete requested ASCII title. This remains an unqualified automation/input observation; neither successful complete typing nor a proven application cause is claimed. The Mac then locked before remaining formatting, reopen and preference-error interaction checks. No automatic unlock, private manuscript use or fabricated success followed.

The subsequent source changes before this candidate are scoped preference-test semantics and explicit synthetic capture fixture/navigation logic; no ordinary authoring/persistence change was identified. The independent review must state any evidence carried from that earlier native executable. Full IME/accessibility/input, note/dictionary/proposal, preference recovery and other deferred journey matrices remain open in alpha.3.

## Installation and publication boundary

Required packages: macOS ARM64 DMG (ad-hoc signed, not notarized), unsigned per-user Windows x64 setup and Debian/Ubuntu amd64 package. Hosted checks cover normal identity, architecture/notices, mounted DMG/version/signature/Applications link, Windows owned-file install/removal/data preservation and Debian12/Ubuntu24.04 install/remove/purge/data preservation. Installation checks are not complete OS desktop UX acceptance.

The tag workflow must verify the current independent gate and exact existing prerelease tag, rerun source checks, rebuild/verify all three installers and stage their rehashed packages into a **new draft** with combined `SHA256SUMS` and source/release manifests. It refuses an existing release. Engineering evidence retains its original false platform-UX/release flags; separate scoped release evidence records the engineering-alpha decision. Publish only the fully checked draft under the owner's existing authorization.
