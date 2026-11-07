#!/usr/bin/env python3
"""Build unsigned Windows/Linux engineering previews on GitHub Actions only."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import platform
import re
import struct
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGETS = {
    "x86_64-pc-windows-msvc": ("Windows", "windows-x64", "writercraft.exe", "WriterCraft.exe", ".zip"),
    "x86_64-unknown-linux-gnu": ("Linux", "linux-x64", "writercraft", "WriterCraft", ".tar.gz"),
}


def require_hosted_target(target, environment, host):
    if environment.get("GITHUB_ACTIONS") != "true":
        raise ValueError("Optimized distribution builds run on GitHub Actions; use a debug source build locally")
    if target not in TARGETS:
        raise ValueError("Unsupported portable preview target: " + target)
    expected = TARGETS[target][0]
    if host != expected or environment.get("RUNNER_OS") != expected:
        raise ValueError("This target requires a native " + expected + " GitHub runner")


def validate_binary(data, target):
    if target == "x86_64-unknown-linux-gnu":
        if len(data) < 20 or data[:6] != b"\x7fELF\x02\x01" or struct.unpack_from("<H", data, 18)[0] != 62:
            raise ValueError("Preview must contain a Linux x86-64 ELF executable")
    else:
        if len(data) < 64 or data[:2] != b"MZ":
            raise ValueError("Preview must contain a Windows x86-64 PE executable")
        offset = struct.unpack_from("<I", data, 60)[0]
        if offset > len(data) - 26 or data[offset:offset + 4] != b"PE\0\0" or struct.unpack_from("<H", data, offset + 4)[0] != 0x8664 or struct.unpack_from("<H", data, offset + 24)[0] != 0x20B:
            raise ValueError("Preview must contain a Windows x86-64 PE32+ executable")


def package_preview(root, binary, version, target, output, commit):
    if not re.fullmatch(r"[0-9A-Za-z.+-]+", version):
        raise ValueError("Unsafe preview version")
    _, platform_name, _, executable, suffix = TARGETS[target]
    data = binary.read_bytes()
    validate_binary(data, target)
    stem = "WriterCraft-" + version + "-" + platform_name + "-preview"
    preview = (
        "WriterCraft " + version + " — " + platform_name + " engineering preview\n\n"
        "This archive contains a native build, not a platform support or UX acceptance claim.\n"
        "The executable is unsigned. macOS notarization, installers and auto-updates are not supplied by this archive.\n"
        "Preserve a backup before trying an alpha on valuable writing.\n"
    )
    if platform_name.startswith("linux"):
        preview += (
            "\nExtract the tar.gz and run ./WriterCraft from its folder in a desktop session.\n"
            "Built on Ubuntu 24.04 (GNU libc); this is not an AppImage or a static universal Linux build.\n"
            "The system must supply compatible X11/Wayland, GPU and desktop-portal runtime libraries.\n"
        )
    else:
        preview += "\nExtract the ZIP before starting WriterCraft.exe in a Windows desktop session.\n"
    evidence = {
        "schema_version": 1,
        "kind": "engineering-preview",
        "version": version,
        "target": target,
        "platform": platform_name,
        "commit": commit,
        "executable": executable,
        "executable_sha256": hashlib.sha256(data).hexdigest(),
        "signing": "unsigned",
        "platform_ux_qualified": False,
        "full_parity_qualified": False,
        "build_runner": TARGETS[target][0],
    }
    evidence_bytes = (json.dumps(evidence, indent=2, sort_keys=True) + "\n").encode()
    members = {
        executable: (data, 0o755),
        "LICENSE": ((root / "LICENSE").read_bytes(), 0o644),
        "PREVIEW.txt": (preview.encode(), 0o644),
        "build-evidence.json": (evidence_bytes, 0o644),
    }
    notices = sorted((root / "assets/fonts").glob("*LICENSE*"))
    if not notices:
        raise ValueError("Bundled font license notices are missing")
    for notice in notices:
        if notice.is_symlink() or not notice.is_file() or notice.stat().st_size == 0:
            raise ValueError("Font license notice must be a nonempty regular file: " + str(notice))
        members["licenses/fonts/" + notice.name] = (notice.read_bytes(), 0o644)
    for notice in sorted((root / "assets/icons").rglob("*")):
        if notice.is_file() and ("LICENSE" in notice.name or notice.name == "NOTICE.md"):
            members["licenses/icons/" + notice.relative_to(root / "assets/icons").as_posix()] = (notice.read_bytes(), 0o644)
    output.mkdir(parents=True, exist_ok=True)
    archive = output / (stem + suffix)
    if suffix == ".zip":
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as bundle:
            for name, (content, mode) in members.items():
                info = zipfile.ZipInfo(stem + "/" + name)
                info.create_system = 3
                info.external_attr = (0o100000 | mode) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                bundle.writestr(info, content)
    else:
        with tarfile.open(archive, "w:gz") as bundle:
            for name, (content, mode) in members.items():
                info = tarfile.TarInfo(stem + "/" + name)
                info.size = len(content)
                info.mode = mode
                info.mtime = 0
                bundle.addfile(info, io.BytesIO(content))
    (output / "PREVIEW.txt").write_text(preview, encoding="utf-8")
    (output / "build-evidence.json").write_bytes(evidence_bytes)
    checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
    (output / "SHA256SUMS").write_text(checksum + "  " + archive.name + "\n", encoding="utf-8")
    return archive


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=sorted(TARGETS))
    parser.add_argument("--output", type=Path, default=Path("artifacts"))
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(PortablePackageTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    if not args.target:
        parser.error("--target is required outside --self-test")
    require_hosted_target(args.target, os.environ, platform.system())
    version = subprocess.check_output(
        [sys.executable, str(ROOT / "scripts/release-check.py"), "--print-version"], cwd=ROOT, text=True
    ).strip()
    subprocess.run(["cargo", "build", "--locked", "--release", "-p", "studio-desktop", "--target", args.target], cwd=ROOT, check=True)
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", str(ROOT / "target")))
    if not target_dir.is_absolute():
        target_dir = ROOT / target_dir
    binary = target_dir / args.target / "release" / TARGETS[args.target][2]
    output = args.output if args.output.is_absolute() else ROOT / args.output
    archive = package_preview(ROOT, binary, version, args.target, output, os.environ.get("GITHUB_SHA", "unknown"))
    print("Packaged " + str(archive))
    return 0


class PortablePackageTests(unittest.TestCase):
    def fake_binary(self, target):
        data = bytearray(256)
        if target.endswith("linux-gnu"):
            data[:6] = b"\x7fELF\x02\x01"
            struct.pack_into("<H", data, 18, 62)
        else:
            data[:2] = b"MZ"
            struct.pack_into("<I", data, 60, 128)
            data[128:132] = b"PE\0\0"
            struct.pack_into("<H", data, 132, 0x8664)
            struct.pack_into("<H", data, 152, 0x20B)
        return bytes(data)

    def test_local_release_build_is_refused(self):
        with self.assertRaisesRegex(ValueError, "GitHub Actions"):
            require_hosted_target("x86_64-unknown-linux-gnu", {}, "Linux")

    def test_cross_host_target_is_refused(self):
        with self.assertRaisesRegex(ValueError, "native Windows"):
            require_hosted_target("x86_64-pc-windows-msvc", {"GITHUB_ACTIONS": "true", "RUNNER_OS": "Linux"}, "Linux")

    def test_wrong_binary_architecture_is_refused(self):
        wrong = bytearray(self.fake_binary("x86_64-unknown-linux-gnu"))
        struct.pack_into("<H", wrong, 18, 183)
        with self.assertRaisesRegex(ValueError, "x86-64"):
            validate_binary(bytes(wrong), "x86_64-unknown-linux-gnu")

    def test_archives_include_executable_licenses_evidence_and_matching_checksum(self):
        for target in TARGETS:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "assets/fonts").mkdir(parents=True)
                (root / "LICENSE").write_text("MIT License fixture")
                (root / "assets/fonts/Source-LICENSE.md").write_text("OFL fixture")
                (root / "assets/icons/lucide").mkdir(parents=True)
                (root / "assets/icons/lucide/LICENSE").write_text("ISC + MIT fixture")
                (root / "assets/icons/lucide/NOTICE.md").write_text("Pinned icon provenance fixture")
                binary = root / "binary"
                binary.write_bytes(self.fake_binary(target))
                archive = package_preview(root, binary, "0.1.0-alpha.1", target, root / "output", "fixture")
                if archive.suffix == ".zip":
                    with zipfile.ZipFile(archive) as bundle:
                        files = {name.split("/", 1)[1]: bundle.read(name) for name in bundle.namelist()}
                else:
                    with tarfile.open(archive) as bundle:
                        entries = bundle.getmembers()
                        executable = next(item for item in entries if item.name.endswith("/WriterCraft"))
                        self.assertEqual(executable.mode, 0o755)
                        files = {item.name.split("/", 1)[1]: bundle.extractfile(item).read() for item in entries}
                self.assertEqual(files[TARGETS[target][3]], binary.read_bytes())
                self.assertIn("LICENSE", files)
                self.assertIn("licenses/fonts/Source-LICENSE.md", files)
                self.assertEqual(files["licenses/icons/lucide/LICENSE"], b"ISC + MIT fixture")
                self.assertIn("licenses/icons/lucide/NOTICE.md", files)
                self.assertFalse(json.loads(files["build-evidence.json"])["platform_ux_qualified"])
                self.assertIn(hashlib.sha256(archive.read_bytes()).hexdigest(), (root / "output/SHA256SUMS").read_text())


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print("Portable packaging failed: " + str(error), file=sys.stderr)
        sys.exit(1)
