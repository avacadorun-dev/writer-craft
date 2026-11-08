# Tasks and evidence

- [x] Define the installer behavior delta and ADR-DIST-001 before repository implementation.
- [x] Implement read-only-permission hosted packaging and immutable same-source/version evidence.
- [x] Implement DMG filesystem/Applications-link/bundle-signature inspection.
- [x] Implement per-user NSIS owned-file installation/uninstall and Start menu application identity.
- [x] Implement Debian launcher/notices/dependency generation and no user-data maintainer scripts.
- [x] Implement Windows user-data/unowned-file preservation verification and Debian 12/Ubuntu 24.04 install/remove/purge verification.
- [x] Provide download, checksum, installation, removal and signing-limit documentation.
- [x] Run synthetic fixture tests: `python3 scripts/test-package-installers.py` (10 tests at implementation checkpoint; no native builds).
- [ ] Run the hosted workflow and inspect all actual native build/install/remove results and combined artifacts.
- [ ] Verify candidate normal startup uses the complete native editor; application entry point is owned by the integration task.
- [ ] Perform owner final candidate checks and retain issue #3 open until accepted.
- [ ] Under a separate release instruction, qualify the exact final version/source against all release gates. No alpha.2 publication is authorized by this change.

Implementation checkpoint (2026-11-08): 10 synthetic fixture tests passed; Actionlint 1.7.12 passed for the new workflow; official OpenSpec 1.16.1 strict validation passed for `installable-distribution`; checked-in ICNS 256px PNG extraction passed without raster modification. No local Cargo/native build or install was run. Authenticated issue audit found issue #3 and eight other issues open; no issue was closed. Actual hosted installer/native startup checks and owner acceptance remain pending.
