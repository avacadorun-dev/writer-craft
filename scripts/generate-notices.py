#!/usr/bin/env python3
"""Inventory target-resolved normal Rust dependencies and preserve their notices."""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parent.parent
NOTICE_NAME = re.compile(r"(?:^|[._-])(?:licen[cs]e|copying|copyright|notice|ofl)(?:$|[._-])", re.IGNORECASE)


def normal_dependencies(metadata):
    """Traverse normal edges through workspace packages, attributing external ones."""
    packages = {item["id"]: item for item in metadata["packages"]}
    nodes = {item["id"]: item for item in metadata["resolve"]["nodes"]}
    workspace = set(metadata["workspace_members"])
    roots = [item["id"] for item in metadata["packages"]
             if item["name"] == "studio-desktop" and item["id"] in workspace]
    if len(roots) != 1:
        raise ValueError("Metadata must contain one studio-desktop workspace package")
    visited, pending = set(), roots[:]
    while pending:
        identifier = pending.pop()
        if identifier in visited:
            continue
        if identifier not in packages or identifier not in nodes:
            raise ValueError("Resolved dependency metadata is incomplete")
        visited.add(identifier)
        for dependency in nodes[identifier]["deps"]:
            if any(kind["kind"] in (None, "normal") for kind in dependency["dep_kinds"]):
                pending.append(dependency["pkg"])
    return sorted((packages[item] for item in visited - workspace),
                  key=lambda item: (item["name"], item["version"], item.get("source") or ""))


def package_notices(package):
    """Return package-relative names and verbatim decoded texts, plus availability notes."""
    directory = Path(package["manifest_path"]).parent
    if not directory.is_dir():
        return [], ["Package source is unavailable; license/notice text could not be collected."]
    paths, warnings = set(), []
    declared = package.get("license_file")
    if declared:
        candidate = Path(declared)
        if not candidate.is_absolute():
            candidate = directory / candidate
        try:
            candidate.resolve().relative_to(directory.resolve())
        except ValueError:
            warnings.append("Declared license file is outside the package source; not collected.")
        else:
            if candidate.is_file() and not candidate.is_symlink():
                paths.add(candidate)
            else:
                warnings.append("Declared license file is unavailable; text could not be collected.")
    for current, directories, files in os.walk(directory):
        directories[:] = sorted(name for name in directories if name not in (".git", "target")
                                and not (Path(current) / name).is_symlink())
        for name in sorted(files):
            candidate = Path(current) / name
            in_license_directory = any(part.lower() in ("license", "licenses", "licence", "licences")
                                       for part in candidate.relative_to(directory).parts[:-1])
            if (NOTICE_NAME.search(name) or in_license_directory) and candidate.is_file() and not candidate.is_symlink():
                paths.add(candidate)
    notices = []
    for path in sorted(paths, key=lambda item: item.relative_to(directory).as_posix()):
        relative = path.relative_to(directory).as_posix()
        try:
            raw = path.read_bytes()
        except OSError:
            warnings.append("Notice file unavailable: " + relative)
            continue
        try:
            contents = raw.decode("utf-8")
            encoding = "UTF-8"
        except UnicodeDecodeError:
            contents = raw.decode("latin-1")
            encoding = "ISO-8859-1 (lossless byte decoding)"
        if not contents:
            warnings.append("Notice file is empty: " + relative)
        notices.append((relative, encoding, contents))
    if not notices:
        warnings.append("No readable license/NOTICE/COPYING/COPYRIGHT/OFL files found in package source.")
    return notices, warnings


def public_repository(package):
    repository = package.get("repository") or ""
    parts = urlsplit(repository)
    if parts.scheme in ("https", "http") and parts.netloc and not parts.username and not parts.password:
        return repository
    return "Not supplied as a public HTTP(S) URL"


def render_notices(metadata, target):
    packages = normal_dependencies(metadata)
    sections = [
        "WriterCraft — Third-party Rust dependency notices\n"
        "================================================\n\n"
        "Target: " + target + "\n"
        "Resolved normal dependency packages: " + str(len(packages)) + "\n\n"
        "Inventory: Cargo's locked, target-filtered metadata, traversed from studio-desktop.\n"
        "Workspace packages are omitted; build/dev-only dependency edges are excluded.\n"
        "This records available package notice files and declared SPDX metadata, not a legal\n"
        "completeness assessment or an assertion that every listed file applies to the binary.\n"
        "Dependency terms remain their own; WriterCraft's own source license remains MIT.\n"
        "Bundled font and icon notices are supplied separately alongside this file.\n\n"
    ]
    warnings = []
    for package in packages:
        name = package["name"] + " " + package["version"]
        notices, missing = package_notices(package)
        sections.append("=" * 78 + "\nPackage: " + name + "\n"
                        "Declared license (SPDX): " + (package.get("license") or "Not declared") + "\n"
                        "Repository: " + public_repository(package) + "\n")
        for warning in missing:
            sections.append("Availability: " + warning + "\n")
            warnings.append(name + ": " + warning)
        for filename, encoding, contents in notices:
            sections.append("\n--- Package file: " + filename + " [" + encoding + "] ---\n")
            sections.append(contents)
            if not contents.endswith("\n"):
                sections.append("\n")
            sections.append("--- End package file ---\n")
        sections.append("\n")
    return "".join(sections), warnings


def host_target():
    details = subprocess.check_output(["rustc", "-vV"], text=True)
    for line in details.splitlines():
        if line.startswith("host: "):
            return line[6:]
    raise ValueError("rustc did not report a host target")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", help="Rust target triple; defaults to rustc's host")
    parser.add_argument("--manifest-path", type=Path, default=ROOT / "Cargo.toml")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(NoticeTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    if not args.output:
        parser.error("--output is required outside --self-test")
    target = args.target or host_target()
    if not re.fullmatch(r"[A-Za-z0-9_.-]+", target):
        raise ValueError("Target must be a Rust target triple")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--filter-platform", target, "--format-version", "1",
         "--manifest-path", str(args.manifest_path)], text=True))
    report, warnings = render_notices(metadata, target)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(report.encode("utf-8"))
    for warning in warnings:
        print("Notice inventory: " + warning, file=sys.stderr)
    print("Generated notices for " + str(len(normal_dependencies(metadata))) + " normal dependency packages"
          + " (" + str(len(warnings)) + " availability notes)")
    return 0


class NoticeTests(unittest.TestCase):
    def fixture(self, root):
        names = ["studio-desktop", "studio-core", "first", "second", "dev-only", "build-only"]
        packages = []
        for name in names:
            directory = root / name
            directory.mkdir()
            (directory / "Cargo.toml").write_text("# fixture", encoding="utf-8")
            packages.append({"id": name, "name": name, "version": "1.2.3",
                             "manifest_path": str(directory / "Cargo.toml"), "license": "MIT OR Apache-2.0",
                             "repository": "https://example.org/" + name})
        def edge(name, *kinds):
            return {"pkg": name, "dep_kinds": [{"kind": kind} for kind in kinds]}
        nodes = [{"id": item["id"], "deps": []} for item in packages]
        nodes[0]["deps"] = [edge("studio-core", None), edge("dev-only", "dev"), edge("build-only", "build")]
        nodes[1]["deps"] = [edge("first", None)]
        nodes[2]["deps"] = [edge("second", "build", None), edge("dev-only", "dev")]
        return {"packages": packages, "resolve": {"nodes": nodes}, "workspace_members": names[:2]}

    def test_transitive_normal_closure_through_workspace_excludes_build_and_dev(self):
        with tempfile.TemporaryDirectory() as directory:
            metadata = self.fixture(Path(directory))
            self.assertEqual([item["name"] for item in normal_dependencies(metadata)], ["first", "second"])

    def test_preserves_notice_text_metadata_relative_names_and_no_machine_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            metadata = self.fixture(root)
            text = "Copyright © 2026 Example\r\n\r\nPermission granted verbatim.\r\n"
            (root / "first/LICENSE-MIT").write_bytes(text.encode("utf-8"))
            (root / "first/NOTICE").write_bytes(b"Additional attribution\n")
            (root / "first/font-mit-license.txt").write_bytes(b"Embedded font license\n")
            (root / "second/COPYING").write_bytes(b"Second license\n")
            report, warnings = render_notices(metadata, "aarch64-apple-darwin")
            self.assertEqual(warnings, [])
            self.assertIn(text, report)
            self.assertIn("Package: first 1.2.3", report)
            self.assertIn("Declared license (SPDX): MIT OR Apache-2.0", report)
            self.assertIn("Repository: https://example.org/first", report)
            self.assertIn("Package file: LICENSE-MIT", report)
            self.assertIn("Additional attribution\n", report)
            self.assertIn("Embedded font license\n", report)
            self.assertNotIn(str(root), report)

    def test_unavailable_sources_and_missing_notices_are_explicit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            metadata = self.fixture(root)
            (root / "first/Cargo.toml").unlink()
            (root / "first").rmdir()
            report, warnings = render_notices(metadata, "fixture-target")
            self.assertIn("Package: first 1.2.3", report)
            self.assertIn("Package source is unavailable", report)
            self.assertIn("No readable license/NOTICE/COPYING/COPYRIGHT/OFL files", report)
            self.assertEqual(len(warnings), 2)
            self.assertNotIn(str(root), report)

    def test_explicit_nonstandard_license_file_and_missing_declared_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            metadata = self.fixture(root)
            (root / "first/legal").mkdir()
            (root / "first/legal/terms.txt").write_bytes(b"Explicit terms\n")
            metadata["packages"][2]["license_file"] = "legal/terms.txt"
            metadata["packages"][3]["license_file"] = "missing.txt"
            report, warnings = render_notices(metadata, "fixture-target")
            self.assertIn("Package file: legal/terms.txt", report)
            self.assertIn("Explicit terms\n", report)
            self.assertTrue(any("Declared license file is unavailable" in item for item in warnings))

    def test_incomplete_dependency_graph_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            metadata = self.fixture(Path(directory))
            metadata["resolve"]["nodes"] = metadata["resolve"]["nodes"][:-3]
            with self.assertRaisesRegex(ValueError, "incomplete"):
                normal_dependencies(metadata)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        print("Notice generation failed: " + str(error), file=sys.stderr)
        sys.exit(1)
