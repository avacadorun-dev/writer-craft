"""Validate completeness and task-journey evidence for the existing UX gate.

This checks traceability, not the truth of a human/native observation.
"""
import hashlib
import json
import re
from pathlib import Path

SCOPES = {"full-editor-parity", "full-parity"}
CONTRACT = "docs/migration/PARITY_SCOPE.json"
KINDS = ("inventories", "capabilities", "journeys")


def file_at(root, relative):
    if not isinstance(relative, str) or not relative or Path(relative).is_absolute():
        raise ValueError("Expected a repository-relative contract/evidence path")
    path = root / relative
    if path.is_symlink() or not path.resolve().is_relative_to(root.resolve()):
        raise ValueError("Contract/evidence path escapes repository: " + relative)
    if not path.is_file() or not path.stat().st_size:
        raise ValueError("Missing or empty contract/evidence: " + relative)
    return path


def indexed(rows, name):
    if not isinstance(rows, list):
        raise ValueError(name + " must be a list")
    result = {}
    for row in rows:
        if not isinstance(row, dict) or not isinstance(row.get("id"), str) or not row["id"].strip():
            raise ValueError(name + " needs stable nonempty IDs")
        if row["id"] in result:
            raise ValueError(name + " has duplicate ID: " + row["id"])
        result[row["id"]] = row
    return result


def contract(root, required_scope=None):
    data = json.loads(file_at(root, CONTRACT).read_text())
    if not isinstance(data, dict) or data.get("schema_version") != 1:
        raise ValueError("PARITY_SCOPE.json requires schema_version 1")
    inputs = data.get("contract_inputs")
    if not isinstance(inputs, list) or not inputs or not all(isinstance(path, str) for path in inputs) or len(inputs) != len(set(inputs)):
        raise ValueError("Parity contract needs distinct authoritative contract inputs")
    for relative in inputs:
        file_at(root, relative)
    scopes = data.get("review_scopes")
    if not isinstance(scopes, list) or not scopes or not all(isinstance(scope, str) and scope in SCOPES for scope in scopes):
        raise ValueError("Parity contract must declare supported review scopes")
    if required_scope is not None and required_scope not in scopes:
        raise ValueError("Parity inventory does not cover requested scope: " + required_scope)
    result = {kind: indexed(data.get(kind), kind) for kind in KINDS}
    for kind, rows in result.items():
        if not rows:
            raise ValueError("Parity contract cannot omit " + kind)
        for row in rows.values():
            file_at(root, row.get("contract"))
            if not row.get("title") or not row.get("issues") or not all(type(n) is int and n > 0 for n in row["issues"]):
                raise ValueError(row["id"] + " needs a title and tracked issues")
    for row in result["inventories"].values():
        sources = row.get("sources")
        if not isinstance(sources, list) or not sources:
            raise ValueError(row["id"] + " needs inspected source identities")
        for source in sources:
            if not isinstance(source, dict) or not source.get("path") or not re.fullmatch(r"[a-f0-9]{64}", source.get("sha256", "")):
                raise ValueError(row["id"] + " has an invalid source identity")
    for row in result["capabilities"].values():
        if row.get("inventory") not in result["inventories"]:
            raise ValueError(row["id"] + " has no known inventory")
    for row in result["journeys"].values():
        if row.get("origin") not in {"legacy", "requested-enhancement", "mixed"}:
            raise ValueError(row["id"] + " cannot require an unselected hypothesis")
        ids = row.get("capabilities", [])
        if not ids or any(id_ not in result["capabilities"] for id_ in ids):
            raise ValueError(row["id"] + " needs known capability links")
        paths = row.get("required_paths")
        if not isinstance(paths, list) or not paths or len(paths) != len(set(paths)):
            raise ValueError(row["id"] + " needs distinct required paths")
    for id_ in result["inventories"]:
        if not any(row["inventory"] == id_ for row in result["capabilities"].values()):
            raise ValueError(id_ + " has no capability requirements")
    return result


def checked_evidence(root, items, required, fingerprint, row_id, required_paths):
    if not isinstance(items, list):
        raise ValueError("Evidence must be a list")
    kinds = set()
    for item in items:
        if not isinstance(item, dict):
            raise ValueError("Evidence needs kind, path and sha256")
        path = file_at(root, item.get("path"))
        if path.suffix != ".md":
            raise ValueError("Evidence must be a narrative record with exact build/test references")
        if hashlib.sha256(path.read_bytes()).hexdigest() != item.get("sha256"):
            raise ValueError("Evidence changed after review: " + item["path"])
        receipt_match = re.match(r"\A<!-- writercraft-evidence\n(.*?)\n-->\n", path.read_text(), re.DOTALL)
        if receipt_match is None:
            raise ValueError("Evidence has no structured candidate/observation receipt: " + item["path"])
        receipt = json.loads(receipt_match[1])
        if not isinstance(receipt, dict) or receipt.get("source_fingerprint") != fingerprint:
            raise ValueError("Report describes a different candidate: " + item["path"])
        kind = item.get("kind")
        observations = receipt.get("observations")
        if not isinstance(observations, list):
            raise ValueError("Report needs explicit observed requirement IDs")
        matching = [obs for obs in observations if isinstance(obs, dict) and obs.get("id") == row_id and obs.get("kind") == kind]
        if len(matching) != 1:
            raise ValueError("Report does not uniquely cover " + str(kind) + "/" + row_id)
        if kind == "native-journey":
            paths = matching[0].get("paths", [])
            if not isinstance(paths, list) or len(paths) != len(set(paths)) or set(paths) != set(required_paths):
                raise ValueError("Report does not observe every required task path: " + row_id)
        if kind in {"native", "native-journey", "semantic"}:
            if not isinstance(receipt.get("build_ref"), str) or not receipt["build_ref"].startswith("https://"):
                raise ValueError("Report needs the exact hosted candidate/test build reference")
            if not isinstance(receipt.get("platform"), str) or not receipt["platform"].strip():
                raise ValueError("Report must name the tested platform")
        kinds.add(kind)
    if not required.issubset(kinds):
        raise ValueError("Missing evidence kinds: " + ", ".join(sorted(required - kinds)))


def issues(root, gate, fingerprint):
    """Both axes are mandatory even for a pre-1.0 parity candidate."""
    try:
        expected = contract(root, gate.get("scope"))
        coverage = gate.get("coverage", {})
        if not isinstance(coverage, dict):
            raise ValueError("UX_GATE coverage must be an object")
        if not isinstance(fingerprint, str) or not re.fullmatch(r"[a-f0-9]{64}", fingerprint):
            raise ValueError("Coverage needs a current source/contract fingerprint")
    except (OSError, ValueError, TypeError, KeyError) as error:
        return [str(error)]
    failures = []
    for kind in KINDS:
        try:
            actual = indexed(coverage.get(kind, []), "coverage." + kind)
        except ValueError as error:
            failures.append(str(error))
            continue
        missing = set(expected[kind]) - set(actual)
        unknown = set(actual) - set(expected[kind])
        if missing:
            failures.append(kind + " missing acceptance: " + ", ".join(sorted(missing)))
        if unknown:
            failures.append(kind + " has unknown IDs: " + ", ".join(sorted(unknown)))
        for id_ in set(actual) & set(expected[kind]):
            row, requirement = actual[id_], expected[kind][id_]
            try:
                if row.get("status") != "accepted" or row.get("source_fingerprint") != fingerprint:
                    raise ValueError("pending, rejected or stale evidence")
                if row.get("independent") is not True or not isinstance(row.get("reviewer"), str) or not row["reviewer"].strip():
                    raise ValueError("independent review is required")
                if kind == "inventories":
                    if row.get("complete") is not True:
                        raise ValueError("source inventory has not been reviewed for omissions")
                    required = {"inventory-audit"}
                elif kind == "capabilities":
                    if not isinstance(row.get("native_entry"), str) or not row["native_entry"].strip():
                        raise ValueError("reachable native entry point is required")
                    required = {"semantic", "native"}
                else:
                    paths = row.get("covered_paths", [])
                    if not isinstance(paths, list) or len(paths) != len(set(paths)) or set(paths) != set(requirement["required_paths"]):
                        raise ValueError("required success/return/failure paths are not covered")
                    required = {"native-journey"}
                checked_evidence(root, row.get("evidence"), required, fingerprint, id_, requirement.get("required_paths", []))
            except (OSError, ValueError, TypeError) as error:
                failures.append(kind + "/" + id_ + ": " + str(error))
    return failures
