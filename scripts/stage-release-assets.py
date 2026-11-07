#!/usr/bin/env python3
"""Verify and stage one macOS candidate plus Windows/Linux engineering archives."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

PLATFORMS = {
    "macos-arm64": (".zip", "aarch64-apple-darwin"),
    "windows-x64": ("-preview.zip", "x86_64-pc-windows-msvc"),
    "linux-x64": ("-preview.tar.gz", "x86_64-unknown-linux-gnu"),
}


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(chunk)
    return result.hexdigest()


def artifact_file(folder, name):
    path = folder / name
    if path.is_symlink() or not path.is_file() or path.stat().st_size == 0 or not path.resolve().is_relative_to(folder.resolve()):
        raise ValueError("Missing or unsafe candidate file: " + str(path))
    return path


def verified_archive(folder, name):
    archive = artifact_file(folder, name)
    checksums = artifact_file(folder, "SHA256SUMS").read_text().splitlines()
    if len(checksums) != 1:
        raise ValueError("Expected exactly one archive checksum in " + str(folder))
    match = re.fullmatch(r"([0-9a-fA-F]{64})\s+\*?([^/\\]+)", checksums[0].strip())
    checksum = digest(archive)
    if not match or match.group(2) != name or match.group(1).lower() != checksum:
        raise ValueError("Candidate archive checksum does not match: " + name)
    return archive, checksum


def evidence(folder, name, version):
    path = artifact_file(folder, name)
    value = json.loads(path.read_text())
    if not isinstance(value, dict) or value.get("version") != version:
        raise ValueError("Candidate evidence has a different version: " + str(path))
    return path, value


def stage_assets(downloads, output, version, commit):
    if not re.fullmatch(r"[0-9A-Za-z.+-]+", version) or "-" not in version:
        raise ValueError("An existing validated prerelease version is required")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("An exact source commit is required")
    if output.exists():
        raise ValueError("Release staging output already exists; use a fresh destination")
    copies = []
    builds = []
    notes = None
    macos_scope = None
    for platform_name, (suffix, target) in PLATFORMS.items():
        folder = downloads / ("WriterCraft-" + version + "-" + platform_name + "-candidate")
        if folder.is_symlink() or not folder.is_dir():
            raise ValueError("Missing platform candidate artifact: " + platform_name)
        name = "WriterCraft-" + version + "-" + platform_name + suffix
        archive, checksum = verified_archive(folder, name)
        copies.append((archive, name))
        if platform_name == "macos-arm64":
            report_path, report = evidence(folder, "release-evidence.json", version)
            acceptance = report.get("ux_acceptance")
            if report.get("ok") is not True or report.get("mode") != "release" or report.get("release_ready") is not True or not isinstance(acceptance, dict) or not acceptance.get("scope"):
                raise ValueError("macOS candidate lacks matching independent release acceptance")
            macos_scope = acceptance["scope"]
            evidence_name = platform_name + "-release-evidence.json"
            copies.append((report_path, evidence_name))
            notes = artifact_file(folder, "release-notes.md").read_text()
            qualification = "accepted macOS candidate for " + macos_scope
        else:
            report_path, report = evidence(folder, "build-evidence.json", version)
            if report.get("kind") != "engineering-preview" or report.get("platform") != platform_name or report.get("target") != target or report.get("commit") != commit or report.get("platform_ux_qualified") is not False or report.get("full_parity_qualified") is not False:
                raise ValueError("Portable candidate must identify the same source and engineering-only scope: " + platform_name)
            source_path, source = evidence(folder, "source-evidence.json", version)
            if source.get("ok") is not True or source.get("mode") != "source":
                raise ValueError("Portable candidate source checks failed: " + platform_name)
            evidence_name = platform_name + "-build-evidence.json"
            copies.extend([
                (report_path, evidence_name),
                (source_path, platform_name + "-source-evidence.json"),
                (artifact_file(folder, "PREVIEW.txt"), platform_name + "-PREVIEW.txt"),
            ])
            qualification = "unsigned engineering preview; platform UX not qualified"
        builds.append({"platform": platform_name, "target": target, "archive": name, "sha256": checksum, "evidence": evidence_name, "qualification": qualification})
    manifest = {"schema_version": 1, "kind": "draft-prerelease-assets", "version": version, "source_commit": commit, "macos_accepted_scope": macos_scope, "builds": builds}
    notes = notes.rstrip() + (
        "\n\n## Download scope\n\n"
        "The macOS ARM64 ZIP is the accepted candidate for the named scope in its release evidence; "
        "ad-hoc signing is not Developer ID notarization.\n\n"
        "The Windows x64 ZIP and Linux x64 tar.gz are additional unsigned engineering previews "
        "from the same source commit. Successful compilation does not qualify their native UX, "
        "installers, updates or full workflow parity. The Linux GNU-libc build uses Ubuntu 24.04 "
        "and requires compatible desktop/graphics runtime libraries; it is not a universal AppImage.\n\n"
        "Verify the archives with SHA256SUMS. release-manifest.json lists each platform's "
        "source identity, archive and separately named evidence.\n"
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".release-assets-", dir=output.parent) as temporary:
        staging = Path(temporary) / "assets"
        staging.mkdir()
        for source, name in copies:
            shutil.copyfile(source, staging / name)
        (staging / "SHA256SUMS").write_text("".join(build["sha256"] + "  " + build["archive"] + "\n" for build in builds), encoding="utf-8")
        (staging / "release-manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        (staging / "release-notes.md").write_text(notes, encoding="utf-8")
        staging.rename(output)
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--downloads", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--version")
    parser.add_argument("--commit")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(StagingTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    if not all([args.downloads, args.output, args.version, args.commit]):
        parser.error("--downloads, --output, --version and --commit are required")
    result = stage_assets(args.downloads, args.output, args.version, args.commit)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


class StagingTests(unittest.TestCase):
    version = "0.1.0-alpha.1"
    commit = "a" * 40

    def fixtures(self, root):
        downloads = root / "downloads"
        for platform_name, (suffix, target) in PLATFORMS.items():
            folder = downloads / ("WriterCraft-" + self.version + "-" + platform_name + "-candidate")
            folder.mkdir(parents=True)
            archive = folder / ("WriterCraft-" + self.version + "-" + platform_name + suffix)
            archive.write_bytes((platform_name + " validated archive fixture").encode())
            (folder / "SHA256SUMS").write_text(digest(archive) + "  " + archive.name + "\n")
            if platform_name == "macos-arm64":
                (folder / "release-evidence.json").write_text(json.dumps({"version": self.version, "ok": True, "mode": "release", "release_ready": True, "ux_acceptance": {"scope": "native-foundation"}}))
                (folder / "release-notes.md").write_text("# WriterCraft alpha\n\nAccepted macOS native-foundation candidate.\n")
            else:
                (folder / "build-evidence.json").write_text(json.dumps({"version": self.version, "kind": "engineering-preview", "platform": platform_name, "target": target, "commit": self.commit, "platform_ux_qualified": False, "full_parity_qualified": False}))
                (folder / "source-evidence.json").write_text(json.dumps({"version": self.version, "mode": "source", "ok": True}))
                (folder / "PREVIEW.txt").write_text(platform_name + " engineering preview\n")
        return downloads

    def test_all_platforms_keep_distinct_evidence_and_verified_combined_checksums(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = stage_assets(self.fixtures(root), root / "release", self.version, self.commit)
            self.assertEqual(len(result["builds"]), 3)
            self.assertEqual(len((root / "release/SHA256SUMS").read_text().splitlines()), 3)
            for build in result["builds"]:
                self.assertEqual(digest(root / "release" / build["archive"]), build["sha256"])
                self.assertTrue((root / "release" / build["evidence"]).is_file())
            self.assertTrue((root / "release/windows-x64-source-evidence.json").is_file())
            self.assertTrue((root / "release/linux-x64-source-evidence.json").is_file())
            self.assertIn("platform UX not qualified", result["builds"][1]["qualification"])
            self.assertIn("additional unsigned engineering previews", (root / "release/release-notes.md").read_text())

    def test_missing_platform_refuses_the_complete_release(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            downloads = self.fixtures(root)
            shutil.rmtree(downloads / ("WriterCraft-" + self.version + "-linux-x64-candidate"))
            with self.assertRaisesRegex(ValueError, "Missing platform"):
                stage_assets(downloads, root / "release", self.version, self.commit)
            self.assertFalse((root / "release").exists())

    def test_tampered_archive_is_rejected_before_any_staging(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            downloads = self.fixtures(root)
            next(downloads.rglob("*-windows-x64-preview.zip")).write_bytes(b"tampered")
            with self.assertRaisesRegex(ValueError, "checksum"):
                stage_assets(downloads, root / "release", self.version, self.commit)
            self.assertFalse((root / "release").exists())

    def test_different_portable_source_commit_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            downloads = self.fixtures(root)
            path = next(downloads.glob("*-windows-x64-candidate/build-evidence.json"))
            report = json.loads(path.read_text())
            report["commit"] = "b" * 40
            path.write_text(json.dumps(report))
            with self.assertRaisesRegex(ValueError, "same source"):
                stage_assets(downloads, root / "release", self.version, self.commit)

    def test_unaccepted_macos_gate_cannot_be_promoted_by_portable_builds(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            downloads = self.fixtures(root)
            path = next(downloads.glob("*-macos-arm64-candidate/release-evidence.json"))
            report = json.loads(path.read_text())
            report["release_ready"] = False
            path.write_text(json.dumps(report))
            with self.assertRaisesRegex(ValueError, "independent release acceptance"):
                stage_assets(downloads, root / "release", self.version, self.commit)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print("Release asset staging failed: " + str(error), file=sys.stderr)
        sys.exit(1)
