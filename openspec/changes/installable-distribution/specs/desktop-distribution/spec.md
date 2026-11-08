## ADDED Requirements

### Requirement: Same-source native installable packages
WriterCraft SHALL provide an Apple-silicon DMG, per-user Windows x64 installer and amd64 Debian package from the same exact committed source, Cargo version and native editor feature selection. Each package SHALL include source identity, application icon, repository license and applicable third-party notices. Engineering packaging SHALL NOT create tags or releases or alter published alpha.1 assets.

#### Scenario: Collect engineering candidates
- **WHEN** hosted packaging completes for all three targets
- **THEN** collection MUST reject missing/duplicate targets, changed package/checksum bytes, mismatched source/version/fingerprint or incomplete required platform checks
- **AND** the combined artifact MUST retain installer hashes, per-platform evidence and clear installation/signing instructions without claiming final release or native UX acceptance.

### Requirement: macOS drag installation
The ARM64 DMG SHALL mount as a verified filesystem containing `WriterCraft.app` and an exact `/Applications` symbolic link. The packaged bundle's version, source, architecture and signature SHALL be verified after mounting. Ad-hoc signatures SHALL NOT be described as Developer ID signing or notarization.

#### Scenario: Inspect the mounted image
- **WHEN** the hosted macOS job mounts the read-only image
- **THEN** it MUST verify the filesystem, drag target, unchanged ARM64 executable and app signature before marking package checks passed.

### Requirement: Per-user Windows installation and safe removal
The Windows installer SHALL install without an elevation request, register a per-user uninstall entry, and create a Start menu shortcut with WriterCraft's application identity. Uninstall SHALL remove only explicit owned files and empty program directories, preserving manuscripts, workspace data and unknown files.

#### Scenario: Unattended install and uninstall
- **WHEN** the hosted Windows job installs and uninstalls silently
- **THEN** it MUST verify the installed source/version/binary, notices, per-user registry and shortcut identity
- **AND** the exact synthetic sentinel bytes under the real default AppData workspace and an unknown program-directory file MUST remain unchanged.

### Requirement: Debian installation and safe removal
The amd64 Debian package SHALL supply a desktop launcher and icon, declare linked and dynamically loaded native runtime dependencies, and target the Debian 12/glibc 2.36 baseline. The same package SHALL undergo installation, removal and purge verification on Debian 12 and Ubuntu 24.04. It SHALL NOT install or delete user workspaces.

#### Scenario: Install and purge the same package on both distributions
- **WHEN** hosted Debian 12 and Ubuntu 24.04 jobs install, remove and purge the candidate
- **THEN** installed binary/source/version, launcher, notices and runtime loading MUST be verified
- **AND** a sentinel in each real default user-data directory MUST survive unchanged.
