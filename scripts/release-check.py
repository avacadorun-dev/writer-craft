#!/usr/bin/env python3
"""Validate authored release metadata and independent UX evidence without building."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import plistlib
import re
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEMVER = re.compile(
    r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-((?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
)
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"

_parity_spec = importlib.util.spec_from_file_location("writercraft_parity_gate", ROOT / "scripts/parity_gate.py")
PARITY = importlib.util.module_from_spec(_parity_spec)
_parity_spec.loader.exec_module(PARITY)


def version_parts(version: str):
    match = SEMVER.fullmatch(version)
    if not match:
        raise ValueError("Workspace version is not valid SemVer 2.0.0: " + version)
    return match.groups()


def quoted_field(section: str, key: str) -> str:
    match = re.search(r'^\s*' + re.escape(key) + r'\s*=\s*"([^"\\]*)"\s*(?:#.*)?$', section, re.MULTILINE)
    if not match:
        raise ValueError("Expected one plain quoted TOML field: " + key)
    return match.group(1)


def toml_section(text: str, name: str) -> str:
    match = re.search(r'^\[' + re.escape(name) + r'\]\s*\n(.*?)(?=^\[|\Z)', text, re.MULTILINE | re.DOTALL)
    if not match:
        raise ValueError("Missing TOML section [" + name + "]")
    return match.group(1)


def workspace_metadata(root: Path = ROOT):
    # These authored fields deliberately use plain quoted TOML, keeping this tool Python3.9-compatible.
    section = toml_section((root / "Cargo.toml").read_text(), "workspace.package")
    version = quoted_field(section, "version")
    parts = version_parts(version)
    manifest = (root / "Cargo.toml").read_text()
    required_scope = "native-foundation"
    if "[workspace.metadata.writercraft]" in manifest:
        required_scope = quoted_field(toml_section(manifest, "workspace.metadata.writercraft"), "required-ux-scope")
    if required_scope not in {"native-foundation", "native-rich-editor-alpha", "full-editor-parity", "full-parity"}:
        raise ValueError("Unknown required independent UX scope: " + required_scope)
    return {"version": version, "short_version": ".".join(parts[:3]), "license": quoted_field(section, "license"), "required_ux_scope": required_scope}


def source_fingerprint(root: Path = ROOT) -> str:
    # Hash actual build bytes; .gitattributes makes tracked text LF on every host.
    # Never hide a different compiled input by normalizing only its evidence hash.
    files = [root / "Cargo.toml", root / "Cargo.lock"]
    for name in ["DESKTOP_UX.md", "UX_ACCEPTANCE.md", "USER_FEEDBACK.md", "WORKFLOW_MAP.md"]:
        contract = root / "docs/design" / name
        if contract.is_file():
            files.append(contract)
    for directory in (root / "crates", root / "assets"):
        for parent, directories, names in os.walk(directory):
            directories[:] = sorted(name for name in directories if name not in {"target", ".git", "__pycache__"})
            files.extend(Path(parent) / name for name in names if name != ".DS_Store")
    scope_file = root / PARITY.CONTRACT
    if scope_file.is_file():
        scope = PARITY.contract(root)
        contracts = {PARITY.CONTRACT, "scripts/parity_gate.py", "scripts/release-check.py", "docs/governance/NEXT_RELEASE.md"}
        contracts.update(row["contract"] for rows in scope.values() for row in rows.values())
        contracts.update(json.loads(scope_file.read_text())["contract_inputs"])
        files.extend(PARITY.file_at(root, relative) for relative in sorted(contracts))
    files = list(set(files))
    digest = hashlib.sha256()
    for path in sorted(files, key=lambda item: item.relative_to(root).as_posix()):
        if path.is_symlink():
            raise ValueError("Runtime/source fingerprint refuses symlinks: " + str(path.relative_to(root)))
        data = path.read_bytes()
        digest.update(path.relative_to(root).as_posix().encode() + b"\0")
        digest.update(len(data).to_bytes(8, "big"))
        digest.update(data)
    return digest.hexdigest()


def changelog_section(root: Path, version: str) -> str:
    text = (root / "CHANGELOG.md").read_text()
    match = re.search(r'^## \[' + re.escape(version) + r'\][^\n]*\n(.*?)(?=^## |\Z)', text, re.MULTILINE | re.DOTALL)
    if not match or not match.group(1).strip():
        raise ValueError("CHANGELOG.md needs a nonempty section for " + version)
    return match.group(0).strip() + "\n"


def confined_file(root: Path, relative: str) -> Path:
    if not isinstance(relative, str) or not relative:
        raise ValueError("Evidence paths must be nonempty repository-relative strings")
    path = root / relative
    resolved = path.resolve()
    if Path(relative).is_absolute() or not resolved.is_relative_to(root.resolve()) or path.is_symlink():
        raise ValueError("Evidence must remain inside the repository: " + relative)
    if not path.is_file() or path.stat().st_size == 0:
        raise ValueError("Evidence file is missing or empty: " + relative)
    return path


def review_records(root: Path):
    path = root / "docs/design/UX_GATE.json"
    if not path.is_file():
        return []
    record = json.loads(path.read_text())
    if not isinstance(record, dict):
        raise ValueError("UX_GATE.json must contain one JSON object")
    # The current gate is authoritative; historical review approvals cannot override a rejection.
    return [(path.relative_to(root).as_posix(), record)]


def accepted_ux(root: Path, version: str, fingerprint: str, required_scope=None):
    reasons = []
    for review_path, record in reversed(review_records(root)):
        if record.get("status") != "accepted":
            continue
        try:
            if record.get("independent") is not True or not isinstance(record.get("reviewer"), str) or not record["reviewer"].strip():
                raise ValueError("acceptance must identify an independent reviewer")
            if record.get("version") != version or record.get("source_fingerprint") != fingerprint:
                raise ValueError("acceptance does not match the current version/source fingerprint")
            if not str(record.get("scope", "")).strip():
                raise ValueError("acceptance must name its implemented scope")
            if required_scope is not None and record.get("scope") != required_scope:
                raise ValueError("acceptance must cover the required scope: " + required_scope)
            if record.get("blockers") != []:
                raise ValueError("independent review still has release blockers")
            evidence = record.get("evidence")
            if not isinstance(evidence, list) or len(evidence) < 2:
                raise ValueError("acceptance needs at least two actual visual evidence files")
            paths = set()
            for item in evidence:
                if not isinstance(item, dict):
                    raise ValueError("visual evidence needs path and sha256 fields")
                relative = item.get("path")
                image = confined_file(root, relative)
                if not relative.startswith("docs/images/") or image.suffix.lower() != ".png":
                    raise ValueError("release image evidence must be checked-in PNGs under docs/images")
                data = image.read_bytes()
                if not data.startswith(PNG_SIGNATURE):
                    raise ValueError("image evidence is not PNG: " + relative)
                if item.get("sha256", "").lower() != hashlib.sha256(data).hexdigest():
                    raise ValueError("image changed after review: " + relative)
                paths.add(relative)
            if len(paths) != len(evidence):
                raise ValueError("visual evidence paths must be distinct")
            report = confined_file(root, record.get("interaction_evidence"))
            if report.suffix != ".md" or len(report.read_text().strip()) < 100:
                raise ValueError("acceptance needs a nonempty narrative interaction report")
            if record["scope"] in PARITY.SCOPES:
                if record.get("interaction_evidence_sha256") != hashlib.sha256(report.read_bytes()).hexdigest():
                    raise ValueError("interaction report is unbound or changed after review")
                coverage_errors = PARITY.issues(root, record, fingerprint)
                if coverage_errors:
                    raise ValueError("Completeness/task acceptance: " + "; ".join(coverage_errors))
            return {"review": review_path, "scope": record["scope"], "reviewer": record["reviewer"]}, None
        except (ValueError, TypeError, AttributeError) as error:
            reasons.append(review_path + ": " + str(error))
    return None, "; ".join(reasons) if reasons else "No independent accepted record for the current source; release remains blocked"


def source_issues(root: Path, metadata):
    issues = []
    try:
        changelog_section(root, metadata["version"])
    except (ValueError, OSError) as error:
        issues.append(str(error))
    if metadata["license"] != "MIT":
        issues.append("workspace.package.license must match the owner's selected MIT license")
    license_path = root / "LICENSE"
    if not license_path.is_file() or "MIT License" not in license_path.read_text():
        issues.append("The repository must include its selected MIT LICENSE")
    lock_text = (root / "Cargo.lock").read_text()
    lock_versions = {}
    for package in re.split(r'^\[\[package\]\]\s*$', lock_text, flags=re.MULTILINE)[1:]:
        lock_versions[quoted_field(package, "name")] = quoted_field(package, "version")
    for manifest in sorted((root / "crates").glob("*/Cargo.toml")):
        package = toml_section(manifest.read_text(), "package")
        name = quoted_field(package, "name")
        if not re.search(r'^version\.workspace\s*=\s*true\s*$', package, re.MULTILINE):
            issues.append(name + " must inherit workspace version")
        if lock_versions.get(name) != metadata["version"]:
            issues.append("Regenerate Cargo.lock for workspace crate " + name)
    for relative in ["docs/design/UX_ACCEPTANCE.md", "docs/design/USER_FEEDBACK.md", "docs/design/WORKFLOW_MAP.md", "docs/design/UX_GATE.json"]:
        try:
            confined_file(root, relative)
        except ValueError as error:
            issues.append(str(error))
    if not list((root / "docs/design").glob("UX_REVIEW_*.md")):
        issues.append("Independent UX review evidence is missing")
    for _, record in review_records(root):
        if record.get("schema_version") != 1 or record.get("status") not in {"pending", "changes-required", "accepted"}:
            issues.append("UX_GATE.json needs schema_version 1 and pending/changes-required/accepted status")
        if not str(record.get("scope", "")).strip() or not str(record.get("reviewer", "")).strip():
            issues.append("UX_GATE.json must name its scope and reviewer")
    for relative in ["README.md", "CHANGELOG.md", "docs/governance/RELEASING.md"]:
        path = root / relative
        text = path.read_text()
        if re.search(r'[\u0400-\u04ff]', text):
            issues.append(relative + " must use English prose")
        if relative == "README.md":
            for target in re.findall(r'(?:\]\(|\bsrc=")([^\s)"#]+)', text):
                if "://" not in target and not target.startswith(("#", "mailto:")) and not (root / target).exists():
                    issues.append("README link/image is missing: " + target)
    result = subprocess.run(["git", "ls-files", "-z"], cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if result.returncode != 0:
        issues.append("Cannot check tracked build artifacts")
    for name in result.stdout.decode().split("\0"):
        if name.startswith(("target/", "artifacts/")) or ".app/" in name or re.search(r'\.sqlite(?:3)?(?:-wal|-shm)?$', name):
            issues.append("Generated build/user data must not be tracked: " + name)
    if metadata["required_ux_scope"] in PARITY.SCOPES:
        try:
            PARITY.contract(root, metadata["required_ux_scope"])
        except (OSError, ValueError, TypeError, KeyError) as error:
            issues.append("Invalid parity contract: " + str(error))
    return issues


def bundle_issues(bundle: Path, metadata):
    issues = []
    info_path = bundle / "Contents/Info.plist"
    with info_path.open("rb") as stream:
        info = plistlib.load(stream)
    if info.get("WriterCraftVersion") != metadata["version"] or info.get("CFBundleShortVersionString") != metadata["short_version"]:
        issues.append("Bundle version must be derived from workspace Cargo.toml")
    if info.get("CFBundleIdentifier") != "app.writercraft.desktop":
        issues.append("Unexpected macOS bundle identifier")
    binary = bundle / "Contents/MacOS/WriterCraft"
    with binary.open("rb") as stream:
        header = stream.read(8)
    if len(header) != 8 or struct.unpack("<II", header) != (0xFEEDFACF, 0x0100000C):
        issues.append("Distribution bundle must contain a thin macOS ARM64 executable")
    if not (bundle / "Contents/Resources/WriterCraft.icns").is_file():
        issues.append("macOS application icon is missing")
    if not (bundle / "Contents/Resources/licenses/LICENSE").is_file():
        issues.append("Bundle must include the repository license")
    return issues


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=["source", "release"], default="source")
    parser.add_argument("--tag", default="")
    parser.add_argument("--require-prerelease", action="store_true")
    parser.add_argument("--fingerprint", action="store_true")
    parser.add_argument("--print-version", action="store_true")
    parser.add_argument("--github-output", action="store_true")
    parser.add_argument("--notes", type=Path)
    parser.add_argument("--bundle", type=Path)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(ReleaseChecks)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    metadata = workspace_metadata()
    if args.print_version:
        print(metadata["version"])
        return 0
    fingerprint = source_fingerprint()
    if args.fingerprint:
        print(fingerprint)
        return 0
    issues = source_issues(ROOT, metadata)
    if args.tag and args.tag != "v" + metadata["version"]:
        issues.append("Tag must exactly match v" + metadata["version"])
    if args.require_prerelease and not version_parts(metadata["version"])[3]:
        issues.append("This workflow creates prerelease candidates only; use a SemVer prerelease version")
    acceptance, ux_reason = accepted_ux(ROOT, metadata["version"], fingerprint, metadata["required_ux_scope"])
    if args.mode == "release" and acceptance is None:
        issues.append(ux_reason)
    if args.mode == "release" and int(version_parts(metadata["version"])[0]) >= 1:
        if (acceptance or {}).get("scope") != "full-parity":
            issues.append("Version 1.0+ needs current independent full-parity UX including both coverage axes")
    if args.bundle:
        issues.extend(bundle_issues(args.bundle, metadata))
    gates = review_records(ROOT)
    gate = gates[0][1] if gates else {}
    report = {"ok": not issues, "mode": args.mode, **metadata, "source_fingerprint": fingerprint,
              "release_ready": acceptance is not None and not issues, "ux_acceptance": acceptance,
              "ux_gate_status": gate.get("status", "pending"), "ux_open_blockers": gate.get("blockers", []),
              "ux_review_scope": gate.get("scope"),
              "coverage_blockers": PARITY.issues(ROOT, gate, fingerprint) if metadata["required_ux_scope"] in PARITY.SCOPES else [],
              "ux_status": "accepted for named scope" if acceptance else ux_reason, "errors": issues}
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        print("WriterCraft " + metadata["version"] + " — " + args.mode + " gate: " + ("PASS" if not issues else "FAIL"))
        print("UX: " + report["ux_status"])
        for issue in issues:
            print("- " + issue, file=sys.stderr)
    if issues:
        return 1
    if args.notes:
        args.notes.parent.mkdir(parents=True, exist_ok=True)
        args.notes.write_text(changelog_section(ROOT, metadata["version"]))
    if args.github_output:
        output = os.environ.get("GITHUB_OUTPUT")
        if not output:
            raise ValueError("--github-output requires GITHUB_OUTPUT")
        with open(output, "a") as stream:
            stream.write("version=" + metadata["version"] + "\nshort_version=" + metadata["short_version"] + "\n")
    return 0


class ReleaseChecks(unittest.TestCase):
    def test_semver_accepts_prerelease_and_metadata(self):
        self.assertEqual(version_parts("0.1.0-alpha.1")[3], "alpha.1")
        self.assertEqual(version_parts("1.2.3+build.7")[4], "build.7")

    def test_semver_rejects_ambiguous_or_shell_like_versions(self):
        for value in ["v0.1.0", "01.2.3", "1.02.3", "1.2.3-alpha.01", "1.2", "1.2.3;touch bad", "1.2.3\n"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                version_parts(value)

    def test_workspace_version_comes_from_single_authored_section(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "Cargo.toml").write_text('[workspace.package]\nversion = "0.1.0-alpha.1"\nlicense = "MIT"\n')
            self.assertEqual(workspace_metadata(root)["short_version"], "0.1.0")

    def test_changelog_requires_matching_version(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "CHANGELOG.md").write_text("## [0.1.0-alpha.1] - 2026-11-07\n\nKnown scope.\n\n## [Unreleased]\nNext.\n")
            self.assertIn("Known scope.", changelog_section(root, "0.1.0-alpha.1"))
            with self.assertRaises(ValueError):
                changelog_section(root, "0.1.0")

    def test_fingerprint_changes_with_runtime_source_but_not_review_prose(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "crates").mkdir()
            (root / "assets").mkdir()
            (root / "Cargo.toml").write_text("metadata")
            (root / "Cargo.lock").write_text("lock")
            source = root / "crates/core.rs"
            source.write_text("original")
            before = source_fingerprint(root)
            (root / "review.md").write_text("review")
            self.assertEqual(before, source_fingerprint(root))
            source.write_text("changed")
            self.assertNotEqual(before, source_fingerprint(root))

    def test_git_checkout_filters_preserve_exact_source_and_binary_bytes_on_windows(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True, capture_output=True)
            (root / ".gitattributes").write_bytes((ROOT / ".gitattributes").read_bytes())
            fixtures = {"Cargo.lock": b"version = 4\n\n[[package]]\nname = \"fixture\"\n", "crates/core.rs": b"// Native input\nfn main() {}\n", "assets/icon.png": b"binary\r\npreserve\0exact\n"}
            for name, expected in fixtures.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(expected)
                identity = subprocess.run(["git", "-C", str(root), "hash-object", "-w", "--no-filters", str(path)], check=True, capture_output=True, text=True).stdout.strip()
                actual = subprocess.run(["git", "-C", str(root), "-c", "core.autocrlf=true", "cat-file", "--filters", "--path=" + name, identity], check=True, capture_output=True).stdout
                self.assertEqual(actual, expected, name)

    def test_evidence_cannot_escape_repository(self):
        with tempfile.TemporaryDirectory() as folder:
            with self.assertRaises(ValueError):
                confined_file(Path(folder), "../outside")

    def make_review(self, root, status="accepted"):
        (root / "docs/design").mkdir(parents=True)
        (root / "docs/images").mkdir(parents=True)
        evidence = []
        for name in ["dark.png", "light.png"]:
            path = root / "docs/images" / name
            path.write_bytes(PNG_SIGNATURE + name.encode())
            evidence.append({"path": "docs/images/" + name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
        record = {"schema_version": 1, "status": status, "independent": True, "reviewer": "independent test reviewer", "scope": "fixture scope",
                  "version": "0.1.0-alpha.1", "source_fingerprint": "a" * 64, "blockers": [], "evidence": evidence,
                  "interaction_evidence": "docs/design/UX_REVIEW_002.md"}
        report = root / "docs/design/UX_REVIEW_002.md"
        report.write_text("# Synthetic test report\n" + "Interaction fixture only. " * 8)
        gate = root / "docs/design/UX_GATE.json"
        gate.write_text(json.dumps(record))
        return record, gate

    def test_current_independent_scope_can_pass(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.make_review(root)
            acceptance, reason = accepted_ux(root, "0.1.0-alpha.1", "a" * 64)
            self.assertIsNotNone(acceptance)
            self.assertIsNone(reason)

    def test_foundation_review_cannot_satisfy_selected_full_editor_scope(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, gate = self.make_review(root)
            acceptance, reason = accepted_ux(root, "0.1.0-alpha.1", "a" * 64, "full-editor-parity")
            self.assertIsNone(acceptance)
            self.assertIn("required scope: full-editor-parity", reason)
            record["scope"] = "full-editor-parity"
            gate.write_text(json.dumps(record))
            self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64, "full-editor-parity")[0])
            self.make_coverage(root, record)
            gate.write_text(json.dumps(record))
            self.assertIsNotNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64, "full-editor-parity")[0])

    def test_stale_blocked_and_unreviewed_scopes_do_not_pass(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, report = self.make_review(root)
            self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "b" * 64)[0])
            for field, value in [("blockers", ["P1-open"]), ("independent", False), ("status", "changes-required")]:
                changed = dict(record)
                changed[field] = value
                report.write_text(json.dumps(changed))
                self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64)[0])

    def test_image_replacement_invalidates_acceptance(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.make_review(root)
            (root / "docs/images/dark.png").write_bytes(PNG_SIGNATURE + b"changed")
            self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64)[0])


    def make_coverage(self, root, record):
        report = root / record["interaction_evidence"]
        record["scope"] = "full-editor-parity"
        receipt = {"source_fingerprint": "a" * 64, "build_ref": "https://example.invalid/synthetic-build", "platform": "synthetic fixture",
                   "observations": [{"kind": "inventory-audit", "id": "editor"}, {"kind": "semantic", "id": "E01"},
                                    {"kind": "native", "id": "E01"}, {"kind": "native-journey", "id": "PT-NOV-01", "paths": ["main", "cancel", "save-reopen"]}]}
        report.write_text("<!-- writercraft-evidence\n" + json.dumps(receipt) + "\n-->\n" + report.read_text())
        checksum = hashlib.sha256(report.read_bytes()).hexdigest()
        record["interaction_evidence_sha256"] = checksum
        contract_path = "docs/design/task-contract.md"
        (root / contract_path).write_text("Synthetic required task and source inventory.")
        scope = {"schema_version": 1, "review_scopes": ["full-editor-parity"], "contract_inputs": [contract_path],
                 "inventories": [{"id": "editor", "title": "Editor", "issues": [4], "contract": contract_path,
                                  "sources": [{"path": "legacy/editor.ts", "sha256": "b" * 64}]}],
                 "capabilities": [{"id": "E01", "title": "Write", "issues": [4], "contract": contract_path, "inventory": "editor"}],
                 "journeys": [{"id": "PT-NOV-01", "title": "Continue a draft", "issues": [4], "contract": contract_path,
                               "origin": "legacy", "capabilities": ["E01"], "required_paths": ["main", "cancel", "save-reopen"]}]}
        path = root / PARITY.CONTRACT
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(scope))
        def row(id_, kinds, **extra):
            return {"id": id_, "status": "accepted", "source_fingerprint": "a" * 64,
                    "independent": True, "reviewer": "independent fixture reviewer",
                    "evidence": [{"kind": kind, "path": record["interaction_evidence"], "sha256": checksum} for kind in kinds], **extra}
        record["coverage"] = {
            "inventories": [row("editor", ["inventory-audit"], complete=True)],
            "capabilities": [row("E01", ["semantic", "native"], native_entry="Document > type")],
            "journeys": [row("PT-NOV-01", ["native-journey"], covered_paths=["main", "cancel", "save-reopen"])]}
        return scope, path

    def test_parity_gate_rejects_missing_duplicate_unknown_pending_and_stale_coverage(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            self.make_coverage(root, record)
            self.assertEqual(PARITY.issues(root, record, "a" * 64), [])
            import copy
            for kind in PARITY.KINDS:
                for mutation in ["missing", "duplicate", "unknown", "pending", "stale", "self-review", "empty-evidence"]:
                    changed = copy.deepcopy(record)
                    rows = changed["coverage"][kind]
                    if mutation == "missing": rows.clear()
                    elif mutation == "duplicate": rows.append(copy.deepcopy(rows[0]))
                    elif mutation == "unknown": rows[0]["id"] = "unknown"
                    elif mutation == "pending": rows[0]["status"] = "pending"
                    elif mutation == "stale": rows[0]["source_fingerprint"] = "c" * 64
                    elif mutation == "self-review": rows[0]["independent"] = False
                    else: rows[0]["evidence"] = []
                    with self.subTest(kind=kind, mutation=mutation):
                        self.assertTrue(PARITY.issues(root, changed, "a" * 64))

    def test_inventory_omissions_and_partial_task_paths_cannot_pass(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            self.make_coverage(root, record)
            record["coverage"]["inventories"][0]["complete"] = False
            self.assertTrue(PARITY.issues(root, record, "a" * 64))
            record["coverage"]["inventories"][0]["complete"] = True
            record["coverage"]["journeys"][0]["covered_paths"].remove("cancel")
            self.assertTrue(PARITY.issues(root, record, "a" * 64))

    def test_changed_report_and_new_requirement_invalidate_parity_acceptance(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, gate = self.make_review(root)
            record["scope"] = "full-editor-parity"
            scope, path = self.make_coverage(root, record)
            gate.write_text(json.dumps(record))
            self.assertIsNotNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64)[0])
            report = root / record["interaction_evidence"]
            original = report.read_text()
            report.write_text(original + "Changed observation.")
            self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64)[0])
            report.write_text(original)
            scope["capabilities"].append({**scope["capabilities"][0], "id": "E02"})
            path.write_text(json.dumps(scope))
            self.assertTrue(PARITY.issues(root, record, "a" * 64))

    def test_required_contract_and_journey_text_are_bound_to_source_fingerprint(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            self.make_coverage(root, record)
            (root / "Cargo.toml").write_text("workspace")
            (root / "Cargo.lock").write_text("lock")
            (root / "scripts").mkdir()
            (root / "scripts/parity_gate.py").write_text("synthetic validator")
            (root / "scripts/release-check.py").write_text("synthetic release enforcement")
            (root / "docs/governance").mkdir()
            (root / "docs/governance/NEXT_RELEASE.md").write_text("selected scope")
            before = source_fingerprint(root)
            (root / "docs/design/task-contract.md").write_text("Changed required task.")
            self.assertNotEqual(before, source_fingerprint(root))
            before = source_fingerprint(root)
            (root / "scripts/release-check.py").write_text("changed enforcement")
            self.assertNotEqual(before, source_fingerprint(root))

    def test_invalid_or_hypothetical_contract_is_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            scope, path = self.make_coverage(root, record)
            scope["journeys"][0]["origin"] = "unselected-hypothesis"
            path.write_text(json.dumps(scope))
            with self.assertRaises(ValueError): PARITY.contract(root)
            scope["journeys"][0]["origin"] = "legacy"
            scope["journeys"][0]["contract"] = "../outside.md"
            path.write_text(json.dumps(scope))
            with self.assertRaises(ValueError): PARITY.contract(root)



    def test_old_or_unrelated_report_cannot_be_relabelled_as_current_evidence(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            self.make_coverage(root, record)
            report = root / record["interaction_evidence"]
            original = report.read_text()
            for replacement in [original.replace("a" * 64, "b" * 64), original.replace('"E01"', '"E99"'),
                                original.replace('"paths": ["main", "cancel", "save-reopen"]', '"paths": ["main"]')]:
                report.write_text(replacement)
                changed = copy.deepcopy(record)
                for rows in changed["coverage"].values():
                    for row in rows:
                        for item in row["evidence"]:
                            item["sha256"] = hashlib.sha256(report.read_bytes()).hexdigest()
                self.assertTrue(PARITY.issues(root, changed, "a" * 64))

    def test_selected_alpha_catalog_cannot_qualify_full_product_parity(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, gate = self.make_review(root)
            self.make_coverage(root, record)
            record["scope"] = "full-parity"
            gate.write_text(json.dumps(record))
            self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64, "full-parity")[0])
            self.assertIn("does not cover requested scope", ";".join(PARITY.issues(root, record, "a" * 64)))



    def test_missing_native_entry_and_reviewer_cannot_pass_as_stringified_null(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            self.make_coverage(root, record)
            row = record["coverage"]["capabilities"][0]
            for key in ["native_entry", "reviewer"]:
                original = row[key]
                for value in [None, {}, [], 12, " "]:
                    row[key] = value
                    self.assertTrue(PARITY.issues(root, record, "a" * 64))
                row[key] = original


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, json.JSONDecodeError, plistlib.InvalidFileException) as error:
        print("Release check failed: " + str(error), file=sys.stderr)
        sys.exit(1)
