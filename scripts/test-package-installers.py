#!/usr/bin/env python3
"""Synthetic installer contract tests; no Cargo, native compiler, install or network."""
import importlib.util
import json
import struct
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("installers", Path(__file__).with_name("package-installers.py"))
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)


class InstallerContracts(unittest.TestCase):
    def test_local_and_cross_host_operations_are_refused(self):
        with self.assertRaisesRegex(ValueError, "only on GitHub"):
            packaging.require_hosted("windows-x64", {}, "Windows")
        with self.assertRaisesRegex(ValueError, "native Windows"):
            packaging.require_hosted("windows-x64", {"GITHUB_ACTIONS": "true", "RUNNER_OS": "Linux"}, "Linux")

    def test_versions_order_prereleases_and_preserve_exact_semver_separately(self):
        versions = ["0.1.0-alpha.1", "0.1.0-alpha.2", "0.1.0-beta.1", "0.1.0-rc.1", "0.1.0", "0.1.1-alpha.1"]
        numeric = [tuple(map(int, packaging.windows_version(value).split("."))) for value in versions]
        self.assertEqual(numeric, sorted(set(numeric)))
        self.assertEqual(packaging.debian_version("0.1.0-alpha.2"), "0.1.0~alpha.2")
        for value in ["v0.1.0", "0.1.0-alpha.300", "0.1.0-dev.1", "65536.1.0", "1.0.0;bad"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                packaging.windows_version(value)

    def test_windows_icon_preserves_the_original_256px_png(self):
        png = b"\x89PNG\r\n\x1a\n" + b"\x00\x00\x00\rIHDR" + struct.pack(">II", 256, 256) + b"fixture"
        chunk = b"ic08" + struct.pack(">I", len(png) + 8) + png
        with tempfile.TemporaryDirectory() as folder:
            source, destination = Path(folder) / "icon.icns", Path(folder) / "icon.ico"
            source.write_bytes(b"icns" + struct.pack(">I", len(chunk) + 8) + chunk)
            packaging.icon_from_icns(source, destination)
            self.assertEqual(destination.read_bytes()[22:], png)
            source.write_bytes(b"icns\x00\x00\x00\x08")
            with self.assertRaisesRegex(ValueError, "256px"):
                packaging.icon_from_icns(source, destination)

    def payload(self, root):
        (root / "assets/fonts").mkdir(parents=True)
        (root / "assets/fonts/FONT-LICENSE.txt").write_text("Font attribution fixture")
        (root / "assets/WriterCraft.png").write_bytes(b"Synthetic image fixture")
        (root / "LICENSE").write_text("MIT License fixture")
        (root / "notice.txt").write_text("Locked dependency notices fixture")
        (root / "binary").write_bytes(b"Synthetic executable fixture")
        (root / "private.wstudio").write_text("Never enter package fixture")
        metadata = {"version": "0.1.0-alpha.2", "commit": "a" * 40, "platform": "windows-x64"}
        packaging.stage_payload(root, root / "binary", root / "payload", metadata, root / "notice.txt")
        return root / "payload", metadata

    def test_payload_retains_license_source_identity_and_excludes_workspace_data(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            payload, metadata = self.payload(root)
            self.assertEqual(json.loads((payload / "package-metadata.json").read_text()), metadata)
            self.assertEqual((payload / "licenses/fonts/FONT-LICENSE.txt").read_text(), "Font attribution fixture")
            self.assertFalse(any(path.suffix == ".wstudio" for path in payload.rglob("*")))
            (root / "notice.txt").unlink()
            (root / "notice.txt").symlink_to(root / "LICENSE")
            with self.assertRaisesRegex(ValueError, "regular nonempty notice"):
                packaging.stage_payload(root, root / "binary", root / "refused", metadata, root / "notice.txt")

    def test_nsis_uninstall_uses_only_explicit_owned_files_and_empty_directories(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            payload, metadata = self.payload(root)
            source = root / "installer.nsi"
            owned = packaging.nsis_source(payload, metadata, source, root / "setup.exe")
            text = source.read_text()
            uninstall = text.split('Section "Uninstall"', 1)[1]
            deleted = [line for line in uninstall.splitlines() if line.startswith('Delete "$INSTDIR')]
            self.assertEqual(set(deleted), {'Delete "$INSTDIR\\' + name + '"' for name in owned + ["Uninstall.exe"]})
            self.assertNotIn("/r", uninstall)
            self.assertNotIn("$APPDATA", uninstall)
            self.assertNotIn("*", uninstall)
            self.assertIn("RequestExecutionLevel user", text)
            self.assertIn("SetCompressor zlib", text)
            self.assertIn("${PKEY_AppUserModel_ID}", text)
            self.assertIn('"DisplayVersion" "0.1.0-alpha.2"', text)
            self.assertIn('"SourceCommit" "' + "a" * 40 + '"', text)
            self.assertIn('"ApplicationID" "app.writercraft.desktop"', text)

    def test_deb_contains_launcher_icon_notices_and_declared_runtime_without_scripts(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            payload, metadata = self.payload(root)
            (payload / "WriterCraft.exe").rename(payload / "writercraft")
            original = packaging.ROOT
            try:
                packaging.ROOT = root
                launcher = packaging.make_deb(payload, root / "package", metadata, ["libc6 (>= 2.36)"] + list(packaging.LINUX_RUNTIME))
            finally:
                packaging.ROOT = original
            control = (root / "package/DEBIAN/control").read_text()
            self.assertIn("Architecture: amd64", control)
            self.assertIn("Version: 0.1.0~alpha.2", control)
            self.assertIn("libc6 (>= 2.36)", control)
            self.assertIn("libvulkan1", control)
            self.assertIn("libwayland-client0", control)
            self.assertIn("Exec=writercraft", launcher.read_text())
            self.assertEqual({path.name for path in (root / "package/DEBIAN").iterdir()}, {"control"})
            self.assertTrue((root / "package/usr/share/doc/writercraft/THIRD_PARTY_NOTICES.txt").is_file())
            self.assertTrue((root / "package/usr/share/icons/hicolor/512x512/apps/app.writercraft.desktop.png").is_file())

    def candidates(self, root):
        for platform, (_, _, target, extension) in packaging.TARGETS.items():
            folder = root / platform
            folder.mkdir()
            package = folder / ("WriterCraft-0.1.0-alpha.2-" + platform + extension)
            package.write_bytes(b"Synthetic package " + platform.encode())
            checksum = packaging.sha256(package)
            evidence = {"schema_version": 1, "kind": "engineering-distribution-candidate", "version": "0.1.0-alpha.2", "commit": "a" * 40, "source_fingerprint": "b" * 64, "platform": platform, "target": target, "cargo_features": ["editor-preview"], "application_id": packaging.APP_ID, "release_ready": False, "platform_ux_qualified": False, "checks": dict.fromkeys(packaging.REQUIRED_CHECKS[platform], True), "package": {"filename": package.name, "sha256": checksum, "size": package.stat().st_size}}
            if platform == "linux-amd64":
                evidence["linux_install_tests"] = {name: {"passed": True} for name in ["debian-12", "ubuntu-24.04"]}
            packaging.write_json(folder / "distribution-evidence.json", evidence)
            (folder / "SHA256SUMS").write_text(checksum + "  " + package.name + "\n")

    def test_all_three_matching_and_verified_installer_artifacts_are_accepted(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.candidates(root)
            self.assertEqual(set(packaging.validate_candidates(root)), set(packaging.TARGETS))

    def test_tampered_installer_is_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.candidates(root)
            next(root.rglob("*.dmg")).write_bytes(b"Tampered fixture")
            with self.assertRaisesRegex(ValueError, "checksum/size"):
                packaging.validate_candidates(root)

    def test_mixed_source_missing_checks_and_missing_linux_baselines_are_refused(self):
        for mutation, message in [("commit", "same exact source"), ("checks", "acceptance check"), ("linux_install_tests", "Debian 12 and Ubuntu")]:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                self.candidates(root)
                path = root / "linux-amd64/distribution-evidence.json"
                evidence = json.loads(path.read_text())
                evidence[mutation] = "c" * 40 if mutation == "commit" else {}
                packaging.write_json(path, evidence)
                with self.assertRaisesRegex(ValueError, message):
                    packaging.validate_candidates(root)

    def test_missing_platform_and_unearned_release_claim_are_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.candidates(root)
            path = root / "windows-x64/distribution-evidence.json"
            evidence = json.loads(path.read_text())
            evidence["release_ready"] = True
            packaging.write_json(path, evidence)
            with self.assertRaisesRegex(ValueError, "cannot claim"):
                packaging.validate_candidates(root)
            path.unlink()
            with self.assertRaisesRegex(ValueError, "All three"):
                packaging.validate_candidates(root)


if __name__ == "__main__":
    unittest.main(verbosity=2)
