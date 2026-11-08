# Design

Follow [ADR-DIST-001](../../../docs/distribution/ARCHITECTURE.md). An Actions source job, dispatched manually or triggered by installer-owned changes on `feature/full-editor-parity`, freezes the SHA/version; macOS and Windows run native packaging; Linux builds and installs in a pinned Debian 12 container, then installs the exact artifact on Ubuntu 24.04. Read-only repository permissions and no publishing code keep engineering artifacts separate from releases.

`scripts/package-installers.py` stages only explicit binaries and notices, generates NSIS owned-file commands and Debian metadata, verifies the DMG, records hashes/identity and rejects incomplete or mixed-source artifact collections. `scripts/verify-windows-installer.ps1` verifies real per-user registration, shortcut identity and install/uninstall preservation. Runtime data is never an install payload or uninstall target. `scripts/test-package-installers.py` uses synthetic fixtures and no Cargo/native builds.

Exact Cargo feature selection is recorded. Engineering installers compile the native editor feature while retaining the normal application identity; an independent macOS packaging hook separates this feature choice from preview branding. The root-owned application entry point controls its normal editor startup. No launcher wrapper or hidden runtime flag compensates for missing app behavior.
