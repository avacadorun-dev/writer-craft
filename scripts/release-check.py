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
STRUCTURED_SCOPE = "structured-writing-alpha"
STRUCTURED_CONTRACT = "docs/design/STRUCTURED_WRITING_ACCEPTANCE.json"
STRUCTURED_IDS = {"A3-WRITE-01", "A3-MARK-01", "A3-REEDIT-01", "A3-HANDOFF-01", "F52-VISUAL-01"}
STRUCTURED_INPUTS = {
    "docs/governance/ALPHA3_STRUCTURED_WRITING_PLAN.md",
    "docs/design/UX_ACCEPTANCE.md", "docs/design/USER_FEEDBACK.md",
    "openspec/changes/structured-writing-alpha/proposal.md",
    "openspec/changes/structured-writing-alpha/design.md",
    "openspec/changes/structured-writing-alpha/tasks.md",
    "openspec/changes/structured-writing-alpha/specs/structured-writing/spec.md",
}

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
    if required_scope not in {"native-foundation", "native-rich-editor-alpha", "full-editor-parity", "full-parity", STRUCTURED_SCOPE}:
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
    if (root / STRUCTURED_CONTRACT).is_file():
        selected, _ = structured_contract(root)
        contracts = {STRUCTURED_CONTRACT, "scripts/release-check.py", "scripts/parity_gate.py", *selected["contract_inputs"]}
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


def structured_contract(root: Path):
    """Selected author outcomes only; the full parity inventory remains independent."""
    data = json.loads(PARITY.file_at(root, STRUCTURED_CONTRACT).read_text())
    if not isinstance(data, dict) or data.get("schema_version") != 1 or data.get("scope") != STRUCTURED_SCOPE:
        raise ValueError("Structured-writing contract needs schema_version 1 and its named scope")
    inputs = data.get("contract_inputs")
    if not isinstance(inputs, list) or not all(isinstance(path, str) for path in inputs) or len(inputs) != len(set(inputs)) or not STRUCTURED_INPUTS.issubset(inputs):
        raise ValueError("Structured-writing contract must bind the selected plan, OpenSpec and F-52 UX inputs")
    for relative in inputs:
        PARITY.file_at(root, relative)
    expected = PARITY.indexed(data.get("observations"), "structured observations")
    if set(expected) != STRUCTURED_IDS:
        raise ValueError("Structured-writing contract must retain all four A3 scenarios and F52-VISUAL-01")
    for id_, row in expected.items():
        paths = row.get("required_paths")
        if not isinstance(paths, list) or not paths or not all(isinstance(path, str) and path.strip() for path in paths) or len(paths) != len(set(paths)):
            raise ValueError(id_ + " needs distinct named required paths")
        operations = PARITY.indexed(row.get("required_semantic_operations", []), id_ + ".required_semantic_operations")
        if id_ != "F52-VISUAL-01" and not operations:
            raise ValueError(id_ + " needs targeted hosted preservation corroboration")
        for operation in operations.values():
            if not isinstance(operation.get("title"), str) or not operation["title"].strip() or operation.get("native_os_required") is not False:
                raise ValueError(id_ + " semantic operations need named automated-test requirements, not native-input claims")
    surfaces = expected["F52-VISUAL-01"].get("required_surfaces")
    if not isinstance(surfaces, list) or not surfaces or not all(isinstance(id_, str) and id_.strip() for id_ in surfaces) or len(surfaces) != len(set(surfaces)):
        raise ValueError("F52-VISUAL-01 needs distinct required native surfaces")
    return data, expected


def structured_receipt(root: Path, item, version, reviewer):
    path = PARITY.file_at(root, item.get("path"))
    match = re.match(r"\A<!-- writercraft-evidence\n(.*?)\n-->\n", path.read_text(), re.DOTALL)
    if match is None or len(path.read_text()[match.end():].strip()) < 100:
        raise ValueError("Selected acceptance needs a receipt and substantive evidence narrative")
    receipt = json.loads(match[1])
    if not isinstance(receipt, dict) or receipt.get("scope") != STRUCTURED_SCOPE or receipt.get("version") != version:
        raise ValueError("Evidence report must name the selected scope and candidate version")
    if receipt.get("independent") is not True or receipt.get("reviewer") != reviewer:
        raise ValueError("Evidence report must identify its independent reviewer")
    return receipt


def checked_surface_review(observation, requirement, gate):
    visited = observation.get("visited_surfaces")
    unvisited = observation.get("unvisited_surfaces")
    for name, values in [("visited_surfaces", visited), ("unvisited_surfaces", unvisited)]:
        if not isinstance(values, list) or not all(isinstance(id_, str) and id_.strip() for id_ in values) or len(values) != len(set(values)):
            raise ValueError("F-52 needs an explicit distinct " + name + " list")
    if observation.get("surface_index_complete") is not True or not set(requirement["required_surfaces"]).issubset(visited) or set(visited) & set(unvisited):
        raise ValueError("F-52 must distinguish complete visited native coverage from unvisited areas")
    results = PARITY.indexed(observation.get("surface_results"), "F-52 surface_results")
    if set(results) != set(visited):
        raise ValueError("F-52 needs one disposition for every visited surface/dialog")
    images = {item["path"] for item in gate.get("evidence", []) if isinstance(item, dict) and isinstance(item.get("path"), str)}
    journeys = set()
    for id_, result in results.items():
        if result.get("outcome") != "passed" or result.get("provenance") != "native-os":
            raise ValueError("F-52 surface still needs actual independent native review: " + id_)
        captures = result.get("captures")
        if not isinstance(captures, list) or not captures or not all(isinstance(path, str) for path in captures) or not set(captures).issubset(images):
            raise ValueError("F-52 surface needs captures bound by the gate image hashes: " + id_)
        if not isinstance(result.get("assessment"), str) or len(result["assessment"].strip()) < 20:
            raise ValueError("F-52 surface needs a concrete usability/visual disposition: " + id_)
        ids = result.get("journeys")
        if not isinstance(ids, list) or not ids or not all(isinstance(value, str) for value in ids) or not set(ids).issubset(STRUCTURED_IDS - {"F52-VISUAL-01"}):
            raise ValueError("F-52 surface must identify its observed author scenario: " + id_)
        journeys.update(ids)
    if journeys != STRUCTURED_IDS - {"F52-VISUAL-01"}:
        raise ValueError("F-52 visual review must cover the surfaces of every selected author scenario")


def structured_issues(root: Path, gate, fingerprint):
    """Check exact selected observations, without weakening or satisfying full parity."""
    try:
        _, expected = structured_contract(root)
        if gate.get("scope") != STRUCTURED_SCOPE or not isinstance(fingerprint, str) or not re.fullmatch(r"[a-f0-9]{64}", fingerprint):
            raise ValueError("Structured acceptance needs its named scope and current source fingerprint")
        coverage = gate.get("coverage", {})
        if not isinstance(coverage, dict):
            raise ValueError("Structured coverage must be an object")
        actual = PARITY.indexed(coverage.get("journeys", []), "coverage.journeys")
        if set(actual) != set(expected):
            raise ValueError("Structured acceptance must cover exactly: " + ", ".join(sorted(expected)))
    except (OSError, ValueError, TypeError, KeyError) as error:
        return [str(error)]
    failures = []
    reports = set()
    for id_, requirement in expected.items():
        row = actual[id_]
        try:
            if row.get("status") != "accepted" or row.get("source_fingerprint") != fingerprint:
                raise ValueError("pending, rejected or stale observation")
            if row.get("independent") is not True or not isinstance(row.get("reviewer"), str) or not row["reviewer"].strip():
                raise ValueError("independent review is required")
            paths = row.get("covered_paths")
            if not isinstance(paths, list) or not all(isinstance(path, str) for path in paths) or len(paths) != len(set(paths)) or set(paths) != set(requirement["required_paths"]):
                raise ValueError("all selected success/return/preservation paths must be observed")
            operations = requirement.get("required_semantic_operations", [])
            required_kinds = {"native-journey", "semantic"} if operations else {"native-journey"}
            PARITY.checked_evidence(root, row.get("evidence"), required_kinds, fingerprint, id_, requirement["required_paths"], operations)
            for item in row["evidence"]:
                kind = item.get("kind")
                if kind not in required_kinds:
                    raise ValueError("Selected acceptance requires native author paths and declared hosted semantic corroboration")
                receipt = structured_receipt(root, item, gate.get("version"), row["reviewer"])
                observation = next(obs for obs in receipt["observations"] if isinstance(obs, dict) and obs.get("id") == id_ and obs.get("kind") == kind)
                if kind == "native-journey":
                    if observation.get("provenance") != "native-os":
                        raise ValueError("Selected author journeys require native OS interaction, not injected application events")
                    if id_ == "F52-VISUAL-01":
                        checked_surface_review(observation, requirement, gate)
                    reports.add(item["path"])
        except (OSError, ValueError, TypeError, KeyError) as error:
            failures.append(id_ + ": " + str(error))
    if gate.get("interaction_evidence") not in reports:
        failures.append("The main interaction report must substantiate selected native observations")
    return failures


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
            if record["scope"] in PARITY.SCOPES or record["scope"] == STRUCTURED_SCOPE:
                if record.get("interaction_evidence_sha256") != hashlib.sha256(report.read_bytes()).hexdigest():
                    raise ValueError("interaction report is unbound or changed after review")
                coverage_errors = (structured_issues(root, record, fingerprint) if record["scope"] == STRUCTURED_SCOPE
                                   else PARITY.issues(root, record, fingerprint))
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
    elif metadata["required_ux_scope"] == STRUCTURED_SCOPE:
        try:
            structured_contract(root)
        except (OSError, ValueError, TypeError, KeyError) as error:
            issues.append("Invalid structured-writing contract: " + str(error))
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
    coverage_blockers = []
    if metadata["required_ux_scope"] in PARITY.SCOPES:
        coverage_blockers = PARITY.issues(ROOT, gate, fingerprint)
    elif metadata["required_ux_scope"] == STRUCTURED_SCOPE:
        coverage_blockers = structured_issues(ROOT, gate, fingerprint)
    report = {"ok": not issues, "mode": args.mode, **metadata, "source_fingerprint": fingerprint,
              "release_ready": acceptance is not None and not issues, "ux_acceptance": acceptance,
              "ux_gate_status": gate.get("status", "pending"), "ux_open_blockers": gate.get("blockers", []),
              "ux_review_scope": gate.get("scope"),
              "coverage_blockers": coverage_blockers,
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
                   "observations": [{"kind": "inventory-audit", "id": "editor", "outcome": "passed", "provenance": "source-audit"},
                                    {"kind": "semantic", "id": "E01", "outcome": "passed", "provenance": "automated-test"},
                                    {"kind": "native", "id": "E01", "outcome": "passed", "provenance": "native-os"},
                                    {"kind": "native-journey", "id": "PT-NOV-01", "outcome": "passed", "provenance": "native-os",
                                     "paths": ["main", "cancel", "save-reopen"],
                                     "path_results": [{"id": id_, "outcome": "passed", "provenance": "native-os"}
                                                      for id_ in ["main", "cancel", "save-reopen"]]}]}
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


    def receipt(self, root, record):
        text = (root / record["interaction_evidence"]).read_text()
        return json.loads(text.split("\n", 1)[1].split("\n-->\n", 1)[0])

    def replace_receipt(self, root, record, receipt):
        report = root / record["interaction_evidence"]
        narrative = report.read_text().split("\n-->\n", 1)[1]
        report.write_text("<!-- writercraft-evidence\n" + json.dumps(receipt) + "\n-->\n" + narrative)
        checksum = hashlib.sha256(report.read_bytes()).hexdigest()
        record["interaction_evidence_sha256"] = checksum
        for rows in record["coverage"].values():
            for row in rows:
                for item in row["evidence"]:
                    item["sha256"] = checksum

    def require_operation_coverage(self, root, record, scope, path):
        operations = [
            {"id": "native-ime", "title": "Compose with the OS input method", "native_os_required": True},
            {"id": "render-gfm", "title": "Render GFM card content", "native_os_required": False},
        ]
        scope["capabilities"][0]["required_operations"] = operations
        path.write_text(json.dumps(scope))
        receipt = self.receipt(root, record)
        for observation in receipt["observations"]:
            if observation["id"] == "E01":
                observation["operation_results"] = [
                    {"id": op["id"], "outcome": "passed", "provenance": observation["provenance"]}
                    for op in operations]
        self.replace_receipt(root, record, receipt)
        return receipt

    def test_observations_require_success_and_explicit_compatible_provenance(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            self.make_coverage(root, record)
            original = self.receipt(root, record)
            for index in range(len(original["observations"])):
                for field, value in [("outcome", None), ("outcome", "failed"), ("outcome", "pending"),
                                     ("outcome", "skipped"), ("outcome", True), ("provenance", None),
                                     ("provenance", ""), ("provenance", "screenshot-implies-native")]:
                    changed = copy.deepcopy(original)
                    if value is None:
                        changed["observations"][index].pop(field)
                    else:
                        changed["observations"][index][field] = value
                    self.replace_receipt(root, record, changed)
                    with self.subTest(index=index, field=field, value=value):
                        self.assertTrue(PARITY.issues(root, record, "a" * 64))

    def test_each_journey_path_requires_its_own_success_and_provenance(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            self.make_coverage(root, record)
            original = self.receipt(root, record)
            for mutation in ["missing", "partial", "duplicate", "unknown", "failed", "no-outcome", "no-provenance", "conflicting-provenance"]:
                changed = copy.deepcopy(original)
                observation = changed["observations"][-1]
                results = observation["path_results"]
                if mutation == "missing": observation.pop("path_results")
                elif mutation == "partial": results.pop()
                elif mutation == "duplicate": results.append(copy.deepcopy(results[0]))
                elif mutation == "unknown": results[0]["id"] = "unknown"
                elif mutation == "failed": results[0]["outcome"] = "failed"
                elif mutation == "no-outcome": results[0].pop("outcome")
                elif mutation == "no-provenance": results[0].pop("provenance")
                else: results[0]["provenance"] = "injected-egui"
                self.replace_receipt(root, record, changed)
                with self.subTest(mutation=mutation):
                    self.assertTrue(PARITY.issues(root, record, "a" * 64))

    def test_complete_static_presentation_cannot_accept_an_interactive_journey(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            self.make_coverage(root, record)
            receipt = self.receipt(root, record)
            journey = receipt["observations"][-1]
            journey["provenance"] = "compiled-ui"
            for result in journey["path_results"]:
                result["provenance"] = "compiled-ui"
            self.replace_receipt(root, record, receipt)
            self.assertIn("compatible observation provenance", ";".join(PARITY.issues(root, record, "a" * 64)))

    def test_input_only_or_failed_operations_cannot_accept_a_complete_capability(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            scope, path = self.make_coverage(root, record)
            original = self.require_operation_coverage(root, record, scope, path)
            self.assertEqual(PARITY.issues(root, record, "a" * 64), [])
            for index in [1, 2]:
                for mutation in ["missing", "input-only", "duplicate", "unknown", "failed", "no-outcome", "no-provenance"]:
                    changed = copy.deepcopy(original)
                    observation = changed["observations"][index]
                    results = observation["operation_results"]
                    if mutation == "missing": observation.pop("operation_results")
                    elif mutation == "input-only": results.pop()
                    elif mutation == "duplicate": results.append(copy.deepcopy(results[0]))
                    elif mutation == "unknown": results[0]["id"] = "unknown"
                    elif mutation == "failed": results[0]["outcome"] = "failed"
                    elif mutation == "no-outcome": results[0].pop("outcome")
                    else: results[0].pop("provenance")
                    self.replace_receipt(root, record, changed)
                    with self.subTest(index=index, mutation=mutation):
                        self.assertTrue(PARITY.issues(root, record, "a" * 64))

    def test_injected_or_rendered_receipts_cannot_satisfy_native_os_operations(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            scope, path = self.make_coverage(root, record)
            original = self.require_operation_coverage(root, record, scope, path)
            for provenance in ["injected-egui", "compiled-ui"]:
                changed = copy.deepcopy(original)
                observation = changed["observations"][2]
                observation["provenance"] = provenance
                for result in observation["operation_results"]:
                    result["provenance"] = provenance
                self.replace_receipt(root, record, changed)
                self.assertIn("requires native OS input", ";".join(PARITY.issues(root, record, "a" * 64)))
                observation["operation_results"][0]["provenance"] = "native-os"
                self.replace_receipt(root, record, changed)
                self.assertIn("conflicts with observation provenance", ";".join(PARITY.issues(root, record, "a" * 64)))

    def test_non_os_operations_keep_explicit_injected_or_compiled_provenance(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            scope, path = self.make_coverage(root, record)
            original = self.require_operation_coverage(root, record, scope, path)
            scope["capabilities"][0]["required_operations"].pop(0)
            path.write_text(json.dumps(scope))
            for observation in original["observations"]:
                if observation["id"] == "E01":
                    observation["operation_results"].pop(0)
            for provenance in ["injected-egui", "compiled-ui"]:
                changed = copy.deepcopy(original)
                observation = changed["observations"][2]
                observation["provenance"] = provenance
                observation["operation_results"][0]["provenance"] = provenance
                self.replace_receipt(root, record, changed)
                self.assertEqual(PARITY.issues(root, record, "a" * 64), [])

    def test_operation_contract_requires_explicit_well_formed_subrequirements(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _ = self.make_review(root)
            scope, path = self.make_coverage(root, record)
            self.require_operation_coverage(root, record, scope, path)
            original = copy.deepcopy(scope)
            for mutation in ["null", "empty", "duplicate", "no-title", "no-os-policy", "invalid-os-policy"]:
                changed = copy.deepcopy(original)
                operations = changed["capabilities"][0]["required_operations"]
                if mutation == "null": changed["capabilities"][0]["required_operations"] = None
                elif mutation == "empty": operations.clear()
                elif mutation == "duplicate": operations.append(copy.deepcopy(operations[0]))
                elif mutation == "no-title": operations[0].pop("title")
                elif mutation == "no-os-policy": operations[0].pop("native_os_required")
                else: operations[0]["native_os_required"] = "yes"
                path.write_text(json.dumps(changed))
                with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                    PARITY.contract(root)

    def test_canvas_contract_keeps_gfm_board_history_and_recovery_beyond_input(self):
        required = PARITY.contract(ROOT)["capabilities"]
        b01 = {row["id"]: row for row in required["B01"]["required_operations"]}
        b26 = {row["id"]: row for row in required["B26"]["required_operations"]}
        self.assertTrue({"render-gfm", "reopen-edit", "native-ime", "native-clipboard"}.issubset(b01))
        self.assertTrue({"local-text-undo-redo", "board-undo-redo", "save-reopen", "recovery-reopen"}.issubset(b26))
        for operation in ["native-ime", "native-clipboard", "native-selection"]:
            self.assertIs(b01[operation]["native_os_required"], True)

    def make_structured_review(self, root):
        record, gate = self.make_review(root)
        record["scope"] = STRUCTURED_SCOPE
        selected = json.loads((ROOT / STRUCTURED_CONTRACT).read_text())
        contract = root / STRUCTURED_CONTRACT
        contract.write_text(json.dumps(selected))
        for relative in selected["contract_inputs"]:
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("Synthetic selected contract only: " + relative)
        receipt = {"scope": STRUCTURED_SCOPE, "version": record["version"],
                   "source_fingerprint": record["source_fingerprint"], "independent": True,
                   "reviewer": record["reviewer"], "build_ref": "https://example.invalid/synthetic-build",
                   "platform": "synthetic fixture; not actual acceptance", "observations": []}
        rows = []
        for requirement in selected["observations"]:
            paths = requirement["required_paths"]
            observation = {"id": requirement["id"], "kind": "native-journey", "outcome": "passed",
                           "provenance": "native-os", "paths": paths,
                           "path_results": [{"id": id_, "outcome": "passed", "provenance": "native-os"} for id_ in paths]}
            if requirement["id"] == "F52-VISUAL-01":
                observation.update({"surface_index_complete": True,
                                    "visited_surfaces": requirement["required_surfaces"],
                                    "unvisited_surfaces": ["full-canvas-authoring"],
                                    "surface_results": [
                                        {"id": id_, "outcome": "passed", "provenance": "native-os",
                                         "captures": [record["evidence"][index % 2]["path"]],
                                         "assessment": "Synthetic validator disposition; no real visual acceptance claimed.",
                                         "journeys": sorted(STRUCTURED_IDS - {"F52-VISUAL-01"})}
                                        for index, id_ in enumerate(requirement["required_surfaces"])]})
            receipt["observations"].append(observation)
            evidence = [{"kind": "native-journey", "path": record["interaction_evidence"], "sha256": ""}]
            operations = requirement.get("required_semantic_operations", [])
            if operations:
                receipt["observations"].append({
                    "id": requirement["id"], "kind": "semantic", "outcome": "passed",
                    "provenance": "automated-test",
                    "operation_results": [{"id": operation["id"], "outcome": "passed", "provenance": "automated-test"}
                                          for operation in operations]})
                evidence.append({"kind": "semantic", "path": record["interaction_evidence"], "sha256": ""})
            rows.append({"id": requirement["id"], "status": "accepted", "independent": True,
                         "reviewer": record["reviewer"], "source_fingerprint": record["source_fingerprint"],
                         "covered_paths": paths, "evidence": evidence})
        record["coverage"] = {"journeys": rows}
        report = root / record["interaction_evidence"]
        report.write_text("<!-- writercraft-evidence\n" + json.dumps(receipt) + "\n-->\n" + report.read_text())
        self.replace_receipt(root, record, receipt)
        gate.write_text(json.dumps(record))
        return record, gate, receipt, contract

    def test_structured_scope_accepts_complete_synthetic_gate_without_accepting_parity(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.make_structured_review(root)
            acceptance, reason = accepted_ux(root, "0.1.0-alpha.1", "a" * 64, STRUCTURED_SCOPE)
            self.assertEqual(acceptance["scope"], STRUCTURED_SCOPE, reason)
            self.assertIsNone(reason)
            self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64, "full-editor-parity")[0])
            (root / "Cargo.toml").write_text('[workspace.package]\nversion = "0.1.0-alpha.1"\nlicense = "MIT"\n[workspace.metadata.writercraft]\nrequired-ux-scope = "structured-writing-alpha"\n')
            self.assertEqual(workspace_metadata(root)["required_ux_scope"], STRUCTURED_SCOPE)

    def test_structured_each_author_scenario_requires_hosted_semantic_corroboration(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            original, gate, _, _ = self.make_structured_review(root)
            for id_ in sorted(STRUCTURED_IDS - {"F52-VISUAL-01"}):
                record = copy.deepcopy(original)
                row = next(row for row in record["coverage"]["journeys"] if row["id"] == id_)
                row["evidence"] = [item for item in row["evidence"] if item["kind"] != "semantic"]
                gate.write_text(json.dumps(record))
                acceptance, reason = accepted_ux(root, "0.1.0-alpha.1", "a" * 64, STRUCTURED_SCOPE)
                with self.subTest(id=id_):
                    self.assertIsNone(acceptance)
                    self.assertIn(id_ + ": Missing evidence kinds: semantic", reason)

    def test_structured_semantic_results_cannot_replace_native_paths_or_omit_preservation(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            original, gate, receipt, _ = self.make_structured_review(root)
            for mutation in ["native-evidence", "semantic-observation", "missing-operation", "failed-operation", "semantic-as-native", "native-as-semantic"]:
                record = copy.deepcopy(original)
                changed = copy.deepcopy(receipt)
                semantic = next(row for row in changed["observations"] if row["id"] == "A3-WRITE-01" and row["kind"] == "semantic")
                native = next(row for row in changed["observations"] if row["id"] == "A3-WRITE-01" and row["kind"] == "native-journey")
                if mutation == "native-evidence":
                    record["coverage"]["journeys"][0]["evidence"] = [item for item in record["coverage"]["journeys"][0]["evidence"] if item["kind"] == "semantic"]
                elif mutation == "semantic-observation": changed["observations"].remove(semantic)
                elif mutation == "missing-operation": semantic["operation_results"].pop()
                elif mutation == "failed-operation": semantic["operation_results"][0]["outcome"] = "failed"
                elif mutation == "semantic-as-native":
                    native["provenance"] = "automated-test"
                    for result in native["path_results"]:
                        result["provenance"] = "automated-test"
                else:
                    semantic["provenance"] = "native-os"
                    for result in semantic["operation_results"]:
                        result["provenance"] = "native-os"
                self.replace_receipt(root, record, changed)
                gate.write_text(json.dumps(record))
                with self.subTest(mutation=mutation):
                    self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64, STRUCTURED_SCOPE)[0])

    def test_structured_gate_refuses_missing_stale_incomplete_or_mismatched_coverage(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            original, gate, _, _ = self.make_structured_review(root)
            for mutation in ["missing", "duplicate", "unknown", "pending", "stale", "self-review", "no-evidence", "partial-paths", "scope", "main-hash"]:
                record = copy.deepcopy(original)
                rows = record["coverage"]["journeys"]
                if mutation == "missing": rows.pop()
                elif mutation == "duplicate": rows.append(copy.deepcopy(rows[0]))
                elif mutation == "unknown": rows[0]["id"] = "unselected"
                elif mutation == "pending": rows[0]["status"] = "pending"
                elif mutation == "stale": rows[0]["source_fingerprint"] = "b" * 64
                elif mutation == "self-review": rows[0]["independent"] = False
                elif mutation == "no-evidence": rows[0]["evidence"] = []
                elif mutation == "partial-paths": rows[0]["covered_paths"].pop()
                elif mutation == "scope": record["scope"] = "native-rich-editor-alpha"
                else: record.pop("interaction_evidence_sha256")
                gate.write_text(json.dumps(record))
                with self.subTest(mutation=mutation):
                    self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64, STRUCTURED_SCOPE)[0])

    def test_structured_report_cannot_relabel_missing_stale_or_non_native_observations(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            original, gate, receipt, _ = self.make_structured_review(root)
            for mutation in ["scope", "version", "source", "reviewer", "independent", "build", "platform", "missing-observation", "unrelated", "injected", "failed-path"]:
                changed = copy.deepcopy(receipt)
                record = copy.deepcopy(original)
                if mutation == "scope": changed["scope"] = "full-editor-parity"
                elif mutation == "version": changed["version"] = "0.1.0-alpha.2"
                elif mutation == "source": changed["source_fingerprint"] = "b" * 64
                elif mutation == "reviewer": changed["reviewer"] = "different reviewer"
                elif mutation == "independent": changed["independent"] = False
                elif mutation == "build": changed.pop("build_ref")
                elif mutation == "platform": changed["platform"] = ""
                elif mutation == "missing-observation": changed["observations"].pop(0)
                elif mutation == "unrelated": changed["observations"][0]["id"] = "unrelated"
                elif mutation == "injected":
                    changed["observations"][0]["provenance"] = "injected-egui"
                    for result in changed["observations"][0]["path_results"]:
                        result["provenance"] = "injected-egui"
                else: changed["observations"][0]["path_results"][0]["outcome"] = "failed"
                self.replace_receipt(root, record, changed)
                gate.write_text(json.dumps(record))
                with self.subTest(mutation=mutation):
                    self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64, STRUCTURED_SCOPE)[0])

    def test_structured_changed_or_missing_interaction_report_invalidates_gate(self):
        for mutation in ["changed", "missing", "row-hash"]:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                record, gate, _, _ = self.make_structured_review(root)
                report = root / record["interaction_evidence"]
                if mutation == "changed": report.write_text(report.read_text() + "Changed observation.")
                elif mutation == "missing": report.unlink()
                else:
                    record["coverage"]["journeys"][0]["evidence"][0]["sha256"] = "b" * 64
                    gate.write_text(json.dumps(record))
                self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64, STRUCTURED_SCOPE)[0])

    def test_structured_visual_review_needs_complete_captured_surface_dispositions(self):
        import copy
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            original, gate, receipt, _ = self.make_structured_review(root)
            for mutation in ["incomplete", "missing-surface", "missing-result", "unvisited", "assessment", "unbound-image", "failed-surface", "missing-journey"]:
                changed = copy.deepcopy(receipt)
                record = copy.deepcopy(original)
                visual = next(row for row in changed["observations"] if row["id"] == "F52-VISUAL-01")
                if mutation == "incomplete": visual["surface_index_complete"] = False
                elif mutation == "missing-surface":
                    visual["visited_surfaces"].pop()
                    visual["surface_results"].pop()
                elif mutation == "missing-result": visual["surface_results"].pop()
                elif mutation == "unvisited": visual["unvisited_surfaces"].append(visual["visited_surfaces"][0])
                elif mutation == "assessment": visual["surface_results"][0]["assessment"] = ""
                elif mutation == "unbound-image": visual["surface_results"][0]["captures"] = ["docs/images/unreviewed.png"]
                elif mutation == "failed-surface": visual["surface_results"][0]["outcome"] = "failed"
                else:
                    for row in visual["surface_results"]:
                        row["journeys"].remove("A3-REEDIT-01")
                self.replace_receipt(root, record, changed)
                gate.write_text(json.dumps(record))
                with self.subTest(mutation=mutation):
                    self.assertIsNone(accepted_ux(root, "0.1.0-alpha.1", "a" * 64, STRUCTURED_SCOPE)[0])

    def test_structured_plan_spec_visual_contract_and_validator_bind_the_fingerprint(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            record, _, _, contract = self.make_structured_review(root)
            (root / "Cargo.toml").write_text("synthetic workspace")
            (root / "Cargo.lock").write_text("synthetic lock")
            (root / "scripts").mkdir()
            for name in ["release-check.py", "parity_gate.py"]:
                (root / "scripts" / name).write_text("synthetic validator")
            for relative in sorted(STRUCTURED_INPUTS | {STRUCTURED_CONTRACT, "scripts/release-check.py"}):
                before = source_fingerprint(root)
                path = root / relative
                original = path.read_text()
                path.write_text(original + "\n")
                self.assertNotEqual(before, source_fingerprint(root), relative)
            before = source_fingerprint(root)
            report = root / record["interaction_evidence"]
            report.write_text(report.read_text() + "Mutable review prose.")
            self.assertEqual(before, source_fingerprint(root))
            selected = json.loads(contract.read_text())
            selected["observations"].pop()
            contract.write_text(json.dumps(selected))
            with self.assertRaises(ValueError):
                structured_contract(root)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, json.JSONDecodeError, plistlib.InvalidFileException) as error:
        print("Release check failed: " + str(error), file=sys.stderr)
        sys.exit(1)
