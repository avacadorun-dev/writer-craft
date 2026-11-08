# Tasks and evidence

- [x] Define the installer behavior delta and ADR-DIST-001 before repository implementation.
- [x] Implement read-only-permission hosted packaging and immutable same-source/version evidence.
- [x] Implement DMG filesystem/Applications-link/bundle-signature inspection.
- [x] Implement per-user NSIS owned-file installation/uninstall and Start menu application identity.
- [x] Implement Debian launcher/notices/dependency generation and no user-data maintainer scripts.
- [x] Implement Windows user-data/unowned-file preservation verification and Debian 12/Ubuntu 24.04 install/remove/purge verification.
- [x] Provide download, checksum, installation, removal and signing-limit documentation.
- [x] Run synthetic fixture tests: `python3 scripts/test-package-installers.py` (11 tests at implementation checkpoint; no native builds).
- [x] Run the hosted workflow and inspect all actual native build/install/remove results and combined artifacts. Final alpha.2 runtime 8af4233 passed every native platform and aggregate in hosted run 42493462622.
- [x] Verify candidate normal startup uses the integrated native editor under app.writercraft.desktop: final twenty normal-app capture cases passed in run42493462622; local synthetic new-document/paste/save also observed. Full editor parity remains alpha.3, not inferred.
- [ ] Perform owner final candidate checks and retain issue #3 open until accepted.
- [ ] Under a separate release instruction, qualify the exact final version/source against all release gates. The latest owner instruction explicitly authorizes the completed-checkpoint alpha.2 release; this checklist alone is not release acceptance.

Implementation checkpoint (2026-11-08): 11 synthetic fixture tests passed; Actionlint 1.7.12 passed for the new workflow; official OpenSpec 1.16.1 strict validation passed for `installable-distribution`; checked-in ICNS 256px PNG extraction passed without raster modification. No local Cargo/native build or install was run. Authenticated issue audit found issue #3 and eight other issues open; no issue was closed. Actual hosted installer/native startup checks and owner acceptance remain pending.

First hosted run: [42474787590](https://github.com/avacadorun-dev/writer-craft/actions/runs/42474787590), source `ffa9a989370dfbb006722b1e96df47e86b3e2589`. Source/fixture gate passed. Windows stopped before Cargo because the initial SourceForge response did not match the pinned NSIS ZIP hash. The follow-up uses native curl redirects/retries and official SourceForge endpoint fallbacks; every response must still match the unchanged SHA-256. Actual platform qualification is not inferred from this download fix.

The same first run's Debian dependency installation passed, but its compiler identity assertion correctly stopped the job: repository `rust-toolchain.toml` selected `stable` (1.99.1) despite the pinned image/default. The workflow now sets `RUSTUP_TOOLCHAIN=1.97.0` explicitly on all installer jobs so the declared compiler cannot be overridden by that repository channel. This preserves the intended MSRV qualification rather than silently accepting a newer compiler. macOS was still compiling at this handoff; Ubuntu install verification was skipped after the Debian failure.
