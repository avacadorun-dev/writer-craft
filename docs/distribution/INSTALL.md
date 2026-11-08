# Installing WriterCraft alpha.2

Download the package matching your operating system from the [alpha.2 release](https://github.com/avacadorun-dev/writer-craft/releases/tag/v0.1.0-alpha.2). The release provides DMG, per-user Windows setup and Debian packages, combined `SHA256SUMS` and exact-source manifests. This is an engineering alpha of the implemented native rich editor; full original-product parity continues in alpha.3.


Installer work is included in the owner-approved alpha.2 scope. These instructions accompany **engineering artifacts**, not a published alpha.2 release or platform UX acceptance. The workspace version remains authoritative; an engineering artifact can still report `0.1.0-alpha.1` while alpha.2 is under development. Published alpha.1 tags and assets are unchanged. Issue [#3](https://github.com/avacadorun-dev/writer-craft/issues/3) stays open through hosted qualification and the owner's final candidate checks.

## Download and verify

Open the repository's **Verify alpha.2 engineering installers** Actions run for the intended source commit. Download its combined `WriterCraft-<version>-all-installers-engineering-<run-id>` artifact and extract it. It contains all three installers, `SHA256SUMS`, per-platform evidence and `distribution-manifest.json`. Check that the full version and commit in the evidence match the intended candidate. Failed/incomplete jobs may retain diagnostic artifacts; they are not qualified combined candidates.

On macOS, use `shasum -a 256 -c SHA256SUMS`; on Linux, use `sha256sum -c SHA256SUMS`. On Windows, compare each package using `Get-FileHash -Algorithm SHA256 .\WriterCraft-<version>-windows-x64-setup.exe` with its line in `SHA256SUMS`. Checksums detect changed downloads; these unsigned checksums do not authenticate a publisher.

No installer bundles a personal workspace, manuscript or `.wstudio` archive. Installed `package-metadata.json` records the full source version, commit, target and executable checksum. License and dependency notices accompany program files.

## macOS Apple silicon

Qualification target: macOS 14 or newer on Apple silicon (ARM64). Open `WriterCraft-<version>-macos-arm64.dmg` and drag `WriterCraft.app` onto the **Applications** link. Eject the image and open the app from Applications. Close an older app yourself before replacing its program bundle.

The packaged app has an **ad-hoc signature**, verified after mounting the DMG. It is **not Developer ID signed or notarized**; the DMG itself is unsigned. Gatekeeper may prevent the first launch. Only after checking the source and checksum, use Apple's normal **System Settings → Privacy & Security → Open Anyway** process if offered. No signing credentials or bypass commands are installed by this workflow. See [Apple's application-opening guidance](https://support.apple.com/en-us/102445).

To uninstall, quit WriterCraft and remove the application bundle. The normal workspace at `~/Library/Application Support/WriterCraft/` stays intact. An explicitly selected custom database stays at its selected location.

## Windows x64

Qualification target: Windows 10/11 x64; hosted installer verification uses Windows Server 2025. Run `WriterCraft-<version>-windows-x64-setup.exe` as your normal user. The installer does not request elevation. It installs into `%LOCALAPPDATA%\Programs\WriterCraft`, creates a **WriterCraft** Start menu entry with application ID `app.writercraft.desktop`, and registers a per-user Installed apps entry.

The installer and application are unsigned. Windows/SmartScreen may warn or block them; hosted installation does not establish publisher reputation or qualify every Windows desktop interaction. No certificates, accounts, paid tooling or signing credentials are required by the package.

Uninstall through **Settings → Apps → Installed apps → WriterCraft → Uninstall**, or run the installed `Uninstall.exe`. For unattended operation, use `/S` with the installer or uninstaller; use a process-waiting invocation when collecting its result. Uninstall removes the explicit shipped-file list and empty program/Start menu directories. Unknown files and workspace data under `%APPDATA%\WriterCraft\` remain intact.

## Linux amd64 Debian package

Installation qualification targets: **Debian 12 amd64** and **Ubuntu 24.04 amd64**. The binary is built on Debian 12 (glibc 2.36) in a pinned official Rust container on a GitHub runner. Users do not need Docker, Rust or a compiler. This is a GNU-libc Debian package, not a static universal Linux build; other distributions and older glibc versions are unqualified.

From the extracted artifact directory, install the matching file:

```sh
sudo apt install ./WriterCraft-<version>-linux-amd64.deb
```

`apt` resolves the package's declared runtime dependencies. Linked-library versions come from `dpkg-shlibdeps`; explicit X11/Wayland, GPU-loader and desktop-portal dependencies cover libraries opened dynamically. A functioning graphical desktop, compatible graphics drivers and an appropriate desktop-portal backend are still required for the GUI and native dialogs. Use the **WriterCraft** application-menu entry, or run `writercraft` in a desktop session.

Remove with `sudo apt remove writercraft`. `sudo apt purge writercraft` also leaves user workspaces intact: no package maintainer script deletes user files. The default data directory is `$XDG_DATA_HOME/WriterCraft/`, or `~/.local/share/WriterCraft/` when `XDG_DATA_HOME` is unset. An explicitly selected custom database stays at its selected location.

## What hosted qualification proves

The workflow builds all packages from one immutable commit and exact workspace version. It verifies ARM64 Mach-O, x64 PE and amd64 ELF payloads; mounted DMG filesystem/Applications link/app signature; Windows per-user registration/Start menu identity; Debian launcher and dependencies; installed binary versions/checksums; and Windows/Linux unattended install/uninstall with an unchanged synthetic sentinel in the real default user-data directory. Linux removal and purge run on Debian 12 and Ubuntu 24.04. Windows also preserves an unknown file in the program directory.

Only a successfully completed run and its evidence prove those checks passed for that source. They do not establish complete editor parity, native GUI/accessibility acceptance on every OS, signing/notarization, automatic updates or the owner's final acceptance. The packaging workflow itself creates no tags or publications. The separate gated release workflow stages these verified packages into a new draft under explicit owner authorization; a separate maintainer action publishes the prerelease.
