# Versioning and releases

WriterCraft ships source through Git and distribution builds through GitHub Actions. Do not commit `target/`, app bundles, ZIPs, databases or local build artifacts. Local packaging defaults to debug; optimized `--release` packaging is reserved for GitHub Actions.

## Version contract

`[workspace.package].version` in the root `Cargo.toml` is the only authored app version. All workspace crates inherit it. Update `Cargo.lock` through Cargo, add the matching changelog section and rerun checks. Packaging derives the full version and the numeric Apple marketing version from that source; the CI run number supplies the Apple build number.

Use [SemVer 2.0.0](https://semver.org/spec/v2.0.0.html). The first prepared candidate is `0.1.0-alpha.1`. Advance the prerelease sequence for another candidate of the same intended release. Before 1.0, document incompatible user/CLI/storage changes and advance the minor version rather than quietly changing contracts in a patch. Compatible fixes advance patch versions. Once 1.0 is reached, breaking public contracts require a major version, additive compatible behavior a minor version, and compatible fixes a patch.

The public contract includes documented CLI/JSON behavior and data-preservation/migration guarantees. App version, SQLite schema, rich-document schema and `.wstudio` container version are separate. Never infer one from another or bump the external transfer format merely for a UI release. A substantial independent library has its own repository and versioning when that boundary is justified.

A released version/tag is immutable: do not force-move a tag or replace published assets. Record known limitations. No feature outside the legacy parity scope should displace required migration work. Version 1.0 requires the declared complete legacy workflows plus macOS distribution and independent UX acceptance; it does not silently assert Windows/Linux support.

## Two different gates

| Gate | What passes | What it does not grant |
| --- | --- | --- |
| Source / engineering preview | Version/changelog consistency, source/evidence structure, tests, Clippy, OpenSpec and honest review disposition | Public-release or Author pilot approval |
| Tagged release candidate | All engineering checks plus independent acceptance of the current source fingerprint and named scope, no unresolved blockers, actual visual/interaction evidence | Stable publication or full parity outside that scope |

An open review may still accompany a clearly labeled engineering preview artifact. It cannot pass the tagged release gate. The independent designer owns acceptance; the implementer does not stamp their own review approved. Owner corrections update the feedback log, reviewer preparation and regression evidence.

## Hosted workflow

[Verify WriterCraft](../../.github/workflows/ci.yml) runs on pull requests, pushes to `main` and manual dispatch. macOS checks cover the complete workspace, OpenSpec and source/evidence gates. Linux/Windows verification jobs cover the shared core/transfer/CLI; they are not desktop interaction evidence. After those checks, push-main and manual runs use a native-runner preview matrix:

| Runner | Rust target | Engineering archive |
| --- | --- | --- |
| `macos-15` ARM64 | `aarch64-apple-darwin` | app ZIP, ad-hoc signed |
| `windows-latest` x64 | `x86_64-pc-windows-msvc` | executable ZIP, unsigned |
| `ubuntu-24.04` x64 | `x86_64-unknown-linux-gnu` | executable tar.gz, unsigned |

Each successful matrix entry uploads its versioned preview archive, SHA-256 checksums, source/build evidence and preview disposition for 14 days. The archive includes the repository license and bundled font notices. Native compilation is performed on its matching host; it is not presented as tested desktop interaction, platform support acceptance or complete workflow parity. Jobs fail independently, so one successful artifact does not imply all three builds passed. Inspect the actual hosted run before announcing availability.

`package-portable.py` permits optimized builds only with `GITHUB_ACTIONS=true` on the expected native runner. It reads the authored workspace version, builds with `--locked --release --target`, checks Windows PE32+ or Linux ELF x86-64 identity, and creates a ZIP/tar.gz containing the executable, license, font notices, preview note and build metadata. Its `--self-test` validates local-build refusal, host mismatch, architecture mismatch, archive contents, Linux execute permissions and checksum integrity without running Cargo. The Linux artifact is a GNU-libc binary from Ubuntu 24.04, not a universal static binary or AppImage; compatible graphics/window-system/desktop runtime libraries remain system dependencies. Windows/Linux archives are portable build previews, not installer packages.

[Release candidate](../../.github/workflows/release.yml) accepts a pushed `v<version>` tag or manual dispatch. It rechecks source/tests/OpenSpec and the stricter release gate, then builds the app on a hosted `macos-15` ARM64 runner. Manual dispatch without a tag produces a build-only candidate. A supplied tag must already exist and match the workspace version exactly. Neither workflow creates a tag.

Only an existing prerelease tag can create/update a **draft prerelease**. The draft job has `contents: write`; other jobs use read-only repository permissions. An existing published release is never overwritten. An existing unpublished draft may have its candidate assets refreshed; a published release is never changed. A maintainer reviews and publishes later under the owner's release instruction. No stable release is published by these workflows.

Actions are pinned to official repository commit SHAs verified on 2026-11-07. The [runner-image catalog](https://github.com/actions/runner-images#available-images) identifies `macos-15` as ARM64. The workflow also verifies architecture. Tooling uses the locked Cargo graph and pinned `@fission-ai/openspec@1.16.1` with telemetry/update checks disabled. Update pins deliberately and revalidate the workflow.

Native OS file-association registration is deferred until opening a file through the OS is implemented and tested; in-app archive preview remains available. The macOS preview ZIP contains `WriterCraft.app`, ad-hoc signed unless a configured signing identity is deliberately supplied. Windows/Linux engineering archives are unsigned; their code signing, installer integration, runtime qualification and auto-update channels remain separate acceptance work. A successful ad-hoc verification is not Developer ID notarization. Add signing/notarization as separately tested release work; never describe an unsigned/ad-hoc preview as notarized.

## Independent UX evidence record

Keep the narrative review and findings. For machine validation, the independent reviewer owns `docs/design/UX_GATE.json`. It is authoritative over historical review prose. Required fields:

```json
{
  "schema_version": 1,
  "status": "changes-required",
  "independent": true,
  "reviewer": "independent UI/UX designer",
  "scope": "native-foundation",
  "version": "0.1.0-alpha.1",
  "source_fingerprint": "SHA256_FROM_THE_COMMAND_BELOW",
  "blockers": ["finding IDs until independently retested"],
  "evidence": [
    {"path": "docs/images/workspace-dark.png", "sha256": "ACTUAL_FILE_SHA256"},
    {"path": "docs/images/workspace-light.png", "sha256": "ACTUAL_FILE_SHA256"}
  ],
  "interaction_evidence": "docs/design/UX_REVIEW_002.md"
}
```

This example is deliberately not an approval. A release record must use `status: accepted`, an exact current fingerprint/version, an empty blockers list, real checked-in image hashes and a nonempty interaction report. It accepts only its named scope. Use `pending`, `changes-required` or `accepted`; a pending/rejected gate cannot be bypassed by an older approved review. Machine checks verify references/freshness, not the truth of a human interaction claim. Version 1.0+ additionally requires `scope: full-parity` and a matching accepted `docs/migration/PARITY_GATE.json`; native-foundation approval cannot grant full parity.

```sh
python3 scripts/release-check.py --fingerprint
python3 scripts/release-check.py --mode source
python3 scripts/release-check.py --mode release --tag v0.1.0-alpha.1 --require-prerelease
```

The fingerprint includes app/core/transfer/CLI source, Cargo manifests/lock, runtime assets and the four desktop/UX/feedback/workflow design contracts. It excludes gate JSON, review prose and screenshots. A source/assets/version change invalidates prior acceptance. Recording a review in a later documentation commit does not create a circular commit-hash requirement. Evidence file hashes prevent silently substituting screenshots after approval.

## Maintainer sequence

1. Finish the named implementation scope, update specs/feedback and run required checks. Keep unsupported workflows explicit.
2. Set the workspace version, regenerate the lockfile and write its changelog entry. Preserve the existing repository license and third-party notices.
3. Capture the actual new build using isolated data; retain dark/light and affected language/size/failure evidence. Do not restart an owner's running preview merely to make a release screenshot.
4. Obtain independent review/retest, then record its exact version/fingerprint and evidence. `changes-required` remains blocked.
5. Push source to `main`, inspect each actual hosted matrix job and download/check its platform artifact. Do not report a hosted build passed until it completed; state which platform jobs passed or failed.
6. Under the owner's release instruction, create/push the matching prerelease tag or dispatch a candidate for an existing tag. Inspect the resulting draft, checksums, scope and known limitations.
7. Publish only after the corresponding explicit release decision. Never use an automatic stable-publish step.

GitHub [workflow triggers](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow) and [release API behavior](https://docs.github.com/en/rest/releases/releases#create-a-release) are the upstream references. Draft creation is distinct from publication, and artifacts are distinct from releases.
