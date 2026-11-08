# Installable desktop distribution

The owner expanded alpha.2 on 2026-11-08 to include all open issues, including [installation issue #3](https://github.com/avacadorun-dev/writer-craft/issues/3). Authors need normal macOS, Windows and Debian installation workflows with preserved user workspaces. Portable archives alone do not meet this acceptance.

Add build-only engineering installers from a single source/version: Apple-silicon DMG, per-user Windows x64 installer and Debian/Ubuntu amd64 package. Preserve notices, icons, application identity and user data. Verify actual hosted install/remove operations and package structure. Publish accompanying installation instructions and signing limitations in artifacts and repository documentation. Do not create tags/releases, modify published alpha.1 assets, close issues before owner final checks or claim full desktop/editor acceptance.

Capability: `desktop-distribution`. Architecture and dependency decision: [ADR-DIST-001](../../../docs/distribution/ARCHITECTURE.md). Rollback removes this separate workflow and scripts; application persistence formats and source data are unchanged.
