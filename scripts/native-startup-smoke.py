#!/usr/bin/env python3
"""Capture synthetic native review states; never certify editor/interaction acceptance."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import struct
import subprocess
import sys
import tempfile
import time
import zlib

TIMEOUT_SECONDS = 45
OUTPUT_NAME = "writercraft-native-startup-smoke"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def png_dimensions(path: Path) -> tuple[int, int]:
    """Check the actual app's bounded PNG output, including chunk CRCs and row data."""
    if not 0 < path.stat().st_size <= 32 * 1024 * 1024:
        raise ValueError("Missing or oversized startup PNG")
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("Startup capture is not PNG")
    offset, width, height = 8, 0, 0
    compressed = bytearray()
    ended = False
    while offset + 12 <= len(data):
        size = struct.unpack_from(">I", data, offset)[0]
        kind = data[offset + 4:offset + 8]
        payload = data[offset + 8:offset + 8 + size]
        end = offset + 12 + size
        if end > len(data):
            raise ValueError("Truncated PNG chunk")
        crc = struct.unpack_from(">I", data, offset + 8 + size)[0]
        if zlib.crc32(kind + payload) != crc:
            raise ValueError("Invalid PNG chunk checksum")
        if kind == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = struct.unpack(">IIBBBBB", payload)
            if not (0 < width <= 16384 and 0 < height <= 16384 and width * height <= 40_000_000):
                raise ValueError("Invalid startup image dimensions")
            if (depth, color, compression, filtering, interlace) != (8, 6, 0, 0, 0):
                raise ValueError("Expected the app's noninterlaced RGBA8 capture")
        elif kind == b"IDAT":
            compressed.extend(payload)
        elif kind == b"IEND":
            ended = size == 0 and end == len(data)
            break
        offset = end
    if not ended or not width or not compressed:
        raise ValueError("Incomplete startup PNG")
    expected = (width * 4 + 1) * height
    decoder = zlib.decompressobj()
    pixels = decoder.decompress(compressed, expected + 1)
    if not decoder.eof or decoder.unused_data or len(pixels) != expected:
        raise ValueError("Invalid startup PNG row data")
    if any(pixels[row * (width * 4 + 1)] > 4 for row in range(height)):
        raise ValueError("Invalid startup PNG row filter")
    return width, height


def run_child(command: list[str], env: dict[str, str], stdout: Path, stderr: Path, timeout: float = TIMEOUT_SECONDS) -> dict:
    """Own exactly one child PID; terminate/kill only that child on the bounded deadline."""
    start = time.monotonic()
    with stdout.open("wb") as out, stderr.open("wb") as err:
        child = subprocess.Popen(command, env=env, stdout=out, stderr=err)
        timed_out = False
        try:
            returncode = child.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            child.terminate()
            try:
                returncode = child.wait(timeout=3)
            except subprocess.TimeoutExpired:
                child.kill()
                returncode = child.wait(timeout=3)
    return {"returncode": returncode, "timed_out": timed_out, "elapsed_seconds": round(time.monotonic() - start, 3)}


def validate_capture_state(state: dict, target: str | None) -> None:
    """Reject vacuous readiness, target misses, failed renderers and mutated synthetic content."""
    ready = state.get("render_readiness") or {}
    if state.get("schema_version") != 1 or state.get("status") != "captured":
        raise ValueError("Native capture did not acknowledge a completed readback")
    if state.get("target_block") != target or state.get("canonical_unchanged") is not True:
        raise ValueError("Capture target or canonical preservation did not match")
    if state.get("stable_frames", 0) < 3 or not ready.get("settled") or not ready.get("target_visible"):
        raise ValueError("Visible native render/scroll state did not settle")
    if ready.get("pending_atoms") != 0 or ready.get("failed_atoms") != 0:
        raise ValueError("Valid synthetic native atoms remain pending or failed")
    if target and ready.get("visible_atoms", 0) <= 0:
        raise ValueError("Opaque target was not present in the observed native viewport")
    size = state.get("observed_inner_size_points")
    if not isinstance(size, list) or len(size) != 2 or not all(isinstance(v, (int, float)) and v > 0 for v in size):
        raise ValueError("Observed native viewport dimensions are missing")


def main(bundle: Path) -> int:
    runner_temp = Path(os.environ["RUNNER_TEMP"]).resolve(strict=True)
    output = runner_temp / OUTPUT_NAME
    output.mkdir(exist_ok=True)
    evidence = {
        "source_commit": os.environ.get("GITHUB_SHA"),
        "run_id": os.environ.get("GITHUB_RUN_ID"),
        "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "runner_image": os.environ.get("ImageOS"),
        "runner_image_version": os.environ.get("ImageVersion"),
        "os_version": platform.mac_ver()[0],
        "architecture": platform.machine(),
        "capture_kind": "native WGPU viewport readback after visible render and scroll stability",
        "all_captures_settled": False,
        "editor_acceptance": False,
        "native_interaction_acceptance": False,
        "cases": [],
    }
    identity = output / "startup-evidence.json"
    def save():
        identity.write_text(json.dumps(evidence, indent=2) + "\n")
    save()
    try:
        if os.environ.get("GITHUB_ACTIONS") != "true" or sys.platform != "darwin":
            raise ValueError("Native startup capture runs only on the hosted macOS runner")
        bundle = bundle.resolve(strict=True)
        with (bundle / "Contents/Info.plist").open("rb") as stream:
            info = plistlib.load(stream)
        if info.get("CFBundleIdentifier") != "app.writercraft.editor-preview" or info.get("WriterCraftBuildCommit") != evidence["source_commit"]:
            raise ValueError("Preview bundle/source identity mismatch")
        executable = bundle / "Contents/MacOS/WriterCraft"
        evidence["executable_sha256"] = sha256(executable)
        evidence["bundle_version"] = info["WriterCraftVersion"]
        evidence["dependency_lock_sha256"] = sha256(Path(__file__).resolve().parent.parent / "Cargo.lock")
        probe = subprocess.run(["pgrep", "-x", "WindowServer"], capture_output=True, timeout=5)
        evidence["window_server_observed"] = probe.returncode == 0
        save()
        targets = {
            "compact-light-en-image": "showcase-image",
            "compact-dark-en-image": "showcase-image",
            "compact-light-en-flowchart": "showcase-flowchart",
            "compact-dark-en-sequence": "showcase-sequence",
            "compact-light-en-formula": "showcase-quadratic",
            "compact-dark-en-inline-formula": "showcase-inline-equation",
            "compact-dark-en-large-serif-image": "showcase-image",
            "wide-light-en-small-inter-flowchart": "showcase-flowchart",
        }
        profiles = {
            "compact-dark-en-large-serif-image": {"font_family": "source-serif", "font_size": 28.0, "line_height": 2.0, "paragraph_spacing": 1.1, "list_item_spacing": 0.5, "manuscript_width": 60},
            "wide-light-en-small-inter-flowchart": {"font_family": "inter", "font_size": 17.0, "line_height": 1.5, "paragraph_spacing": 0.65, "list_item_spacing": 0.0, "manuscript_width": 80},
        }
        for name, language, section, compact, flags in (
            ("compact-dark-en-manuscript", "en", None, True, []),
            ("compact-light-de-settings", "de", None, True, ["--light", "--settings"]),
            ("compact-light-fr-editor", "fr", "editor", True, ["--light", "--settings"]),
            ("compact-dark-ru-files", "ru", "files", True, ["--settings"]),
            ("compact-dark-ja-editor", "ja", "editor", True, ["--settings"]),
            ("compact-light-zh-max-text", "zh", None, True, ["--light", "--capture-max-text"]),
            ("wide-dark-es-focus", "es", None, False, ["--capture-focus"]),
            ("wide-light-pt-manuscript", "pt", None, False, ["--light"]),
            ("compact-dark-de-notes", "de", None, True, ["--capture-notes"]),
            ("wide-dark-en-editor", "en", "editor", False, ["--settings"]),
            ("compact-light-en-image", "en", None, True, ["--light"]),
            ("compact-dark-en-image", "en", None, True, []),
            ("compact-light-en-flowchart", "en", None, True, ["--light"]),
            ("compact-dark-en-sequence", "en", None, True, []),
            ("compact-light-en-formula", "en", None, True, ["--light"]),
            ("compact-dark-en-inline-formula", "en", None, True, []),
            ("compact-dark-en-large-serif-image", "en", None, True, []),
            ("wide-light-en-small-inter-flowchart", "en", None, False, ["--light"]),
        ):
            target = targets.get(name)
            case = {"target_block": target, "name": name, "language": language, "requested_inner_size_points": [1000, 680] if compact else [1440, 940], "flags": flags, "section": section, "status": "starting"}
            evidence["cases"].append(case)
            save()
            try:
                # The database/interface directory is created here; callers cannot supply a database.
                with tempfile.TemporaryDirectory(prefix="writercraft-startup-workspace-", dir=runner_temp) as scratch:
                    capture = output / (name + ".png")
                    capture.unlink(missing_ok=True)
                    capture_state = capture.with_suffix(".capture.json")
                    capture_state.unlink(missing_ok=True)
                    if name in profiles:
                        (Path(scratch) / "editor.json").write_text(json.dumps({"version": 1, "preferences": profiles[name]}))
                    env = os.environ.copy()
                    env.update(WRITERCRAFT_EDITOR_PREVIEW="1", WRITERCRAFT_DEMO="1", WRITERCRAFT_CAPTURE_LANGUAGE=language, WRITERCRAFT_CAPTURE_SECTION=section or "appearance", WRITERCRAFT_CAPTURE_BLOCK=target or "")
                    command = [str(executable), "--database", str(Path(scratch) / "workspace.sqlite"), "--demo", "--editor-preview", *(["--compact"] if compact else []), "--capture", str(capture), *flags]
                    case.update(run_child(command, env, output / (name + ".stdout.log"), output / (name + ".stderr.log")))
                    if case["timed_out"] or case["returncode"] != 0:
                        raise ValueError("Native startup timed out" if case["timed_out"] else "Native startup exited unsuccessfully")
                    case["capture_size_pixels"] = png_dimensions(capture)
                    case["capture_sha256"] = sha256(capture)
                    if not 0 < capture_state.stat().st_size <= 64 * 1024:
                        raise ValueError("Missing or oversized native capture state")
                    state = json.loads(capture_state.read_text())
                    validate_capture_state(state, target)
                    if state.get("image_size_pixels") != list(case["capture_size_pixels"]):
                        raise ValueError("Native state/readback pixel dimensions disagree")
                    case["native_state"] = state
                    case["state_sha256"] = sha256(capture_state)
                    observed = state["observed_inner_size_points"]
                    requested = case["requested_inner_size_points"]
                    case["requested_size_observed"] = all(abs(a - b) <= 4 for a, b in zip(observed, requested))
                    case["observed_layout_coverage"] = "wide" if observed[0] >= 1200 else "constrained"
                    if not compact and case["observed_layout_coverage"] != "wide":
                        case["coverage_limitation"] = "Hosted window-server viewport constrained the requested wide state; actual wide app acceptance remains pending."
                    case["status"] = "captured-settled-native-view"
            except (OSError, ValueError, subprocess.SubprocessError, zlib.error, struct.error) as error:
                case.update(status="failed", error=str(error))
            save()
        evidence["all_captures_settled"] = all(case["status"] == "captured-settled-native-view" for case in evidence["cases"])
        evidence["actual_wide_coverage"] = any(case.get("observed_layout_coverage") == "wide" for case in evidence["cases"])
        evidence["status"] = "startup-captures-complete" if all(case["status"] == "captured-settled-native-view" for case in evidence["cases"]) else "startup-capture-unavailable"
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        evidence.update(status="startup-capture-unavailable", error=str(error))
    save()
    print(json.dumps({"status": evidence["status"], "source_commit": evidence["source_commit"], "native_interaction_acceptance": False}))
    return 0 if evidence["status"] == "startup-captures-complete" else 1


def self_test() -> None:
    import unittest
    class Tests(unittest.TestCase):
        def test_png_valid_crc_and_pixels(self):
            def chunk(kind, payload):
                return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload))
            data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(b"\0\xff\0\0\xff")) + chunk(b"IEND", b"")
            with tempfile.TemporaryDirectory(prefix="writercraft-smoke-test-") as directory:
                path = Path(directory) / "capture.png"
                path.write_bytes(data)
                self.assertEqual(png_dimensions(path), (1, 1))
                path.write_bytes(data[:-1] + b"\xff")
                with self.assertRaises(ValueError):
                    png_dimensions(path)
                path.write_bytes(data[:40])
                with self.assertRaises(ValueError):
                    png_dimensions(path)
        def test_capture_state_refuses_unready_empty_failed_or_changed_content(self):
            good = {"schema_version": 1, "status": "captured", "target_block": "image", "canonical_unchanged": True, "stable_frames": 3, "observed_inner_size_points": [1024, 684], "render_readiness": {"settled": True, "target_visible": True, "visible_atoms": 1, "pending_atoms": 0, "failed_atoms": 0}}
            validate_capture_state(good, "image")
            for field, value in (("pending_atoms", 1), ("failed_atoms", 1), ("visible_atoms", 0), ("target_visible", False), ("settled", False)):
                bad = dict(good, render_readiness=dict(good["render_readiness"], **{field: value}))
                with self.assertRaises(ValueError): validate_capture_state(bad, "image")
            for field, value in (("canonical_unchanged", False), ("stable_frames", 2), ("target_block", "different"), ("observed_inner_size_points", None)):
                with self.assertRaises(ValueError): validate_capture_state(dict(good, **{field: value}), "image")
        def test_child_exit_and_timeout_are_recorded(self):
            with tempfile.TemporaryDirectory(prefix="writercraft-smoke-test-") as directory:
                output = Path(directory)
                result = run_child([sys.executable, "-c", "raise SystemExit(3)"], os.environ.copy(), output / "out", output / "err")
                self.assertEqual(result["returncode"], 3)
                self.assertFalse(result["timed_out"])
                result = run_child([sys.executable, "-c", "import time; time.sleep(10)"], os.environ.copy(), output / "out", output / "err", timeout=0.1)
                self.assertTrue(result["timed_out"])
                self.assertNotEqual(result["returncode"], 0)
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Tests)
    if not unittest.TextTestRunner().run(suite).wasSuccessful():
        raise SystemExit(1)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path)
    parser.add_argument("--self-test", action="store_true")
    arguments = parser.parse_args()
    if arguments.self_test:
        self_test()
    elif arguments.bundle is None:
        parser.error("--bundle is required")
    else:
        raise SystemExit(main(arguments.bundle))
