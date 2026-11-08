#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

profile=debug
editor_preview=${WRITERCRAFT_EDITOR_PREVIEW:-0}
if [[ "$editor_preview" != 0 && "$editor_preview" != 1 ]]; then
  echo 'WRITERCRAFT_EDITOR_PREVIEW must be 0 or 1.' >&2; exit 2
fi
output=
rust_target=
while (($#)); do
  case "$1" in
    --debug) profile=debug; shift ;;
    --release) profile=release; shift ;;
    --output)
      if (($# < 2)); then echo '--output requires an app path.' >&2; exit 2; fi
      output="$2"; shift 2 ;;
    --target)
      if (($# < 2)); then echo '--target requires a Rust target triple.' >&2; exit 2; fi
      rust_target="$2"; shift 2 ;;
    --help|-h)
      echo "Usage: $0 [--debug|--release] [--target aarch64-apple-darwin] [--output APP_PATH]"
      echo 'Local builds default to debug. Optimized release builds run on GitHub Actions.'
      echo 'WRITERCRAFT_EDITOR_PREVIEW=1 builds an isolated editor engineering preview.'
      exit 0 ;;
    *) echo "Unknown option: $1. Use --help." >&2; exit 2 ;;
  esac
done
if [[ "$(uname -s)" != Darwin ]]; then
  echo 'macOS packaging requires macOS.' >&2; exit 1
fi
if [[ "$profile" == release && "${GITHUB_ACTIONS:-false}" != true ]]; then
  echo 'Optimized distribution builds run on GitHub Actions. Use --debug for a local preview.' >&2
  exit 1
fi
if [[ -n "$rust_target" && "$rust_target" != aarch64-apple-darwin ]]; then
  echo 'The current packaging target is Apple silicon (aarch64-apple-darwin).' >&2; exit 2
fi
if [[ -z "$output" ]]; then
  app_name=WriterCraft
  if [[ "$editor_preview" == 1 ]]; then app_name="WriterCraft Editor Preview"; fi
  if [[ "$profile" == release ]]; then output="artifacts/$app_name.app"; else output="artifacts/debug/$app_name.app"; fi
fi
if [[ "$output" != *.app ]]; then
  echo '--output must name a .app bundle.' >&2; exit 2
fi
version="$(python3 scripts/release-check.py --print-version)"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-14.0}"

build=(cargo build --locked -p studio-desktop)
if [[ "$editor_preview" == 1 ]]; then build+=(--features editor-preview); fi
if [[ "$profile" == release ]]; then build+=(--release); fi
if [[ -n "$rust_target" ]]; then build+=(--target "$rust_target"); fi
"${build[@]}"

python3 - "$output" "$profile" "$rust_target" "$version" "$editor_preview" <<'PYTHON'
import os
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

output = Path(sys.argv[1]).resolve()
profile, rust_target, version, preview_flag = sys.argv[2:]
editor_preview = preview_flag == "1"
app_name = "WriterCraft Editor Preview" if editor_preview else "WriterCraft"
bundle_id = "app.writercraft.editor-preview" if editor_preview else "app.writercraft.desktop"
marketing_version = version.split("-", 1)[0].split("+", 1)[0]
if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", marketing_version):
    raise SystemExit("Invalid workspace version for Apple bundle metadata")
run_number = os.environ.get("GITHUB_RUN_NUMBER", "1")
run_attempt = os.environ.get("GITHUB_RUN_ATTEMPT", "1")
build_number = run_number + "." + run_attempt
if not re.fullmatch(r"[0-9]+\.[0-9]+", build_number):
    raise SystemExit("Invalid numeric CI build identity")
processes = subprocess.run(["ps", "-axo", "command="], text=True, capture_output=True, check=True).stdout
running_binary = str(output / "Contents/MacOS/WriterCraft")
if any(line.strip() == running_binary or line.strip().startswith(running_binary + " ") for line in processes.splitlines()):
    raise SystemExit("That preview is running. Choose a different --output; do not replace the owner's active app.")
target_dir = Path(os.environ.get("CARGO_TARGET_DIR", "target"))
if rust_target:
    target_dir /= rust_target
binary = target_dir / profile / "writercraft"
if not binary.is_file():
    raise SystemExit("Built executable is missing: " + str(binary))
output.parent.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix=".writercraft-package-", dir=output.parent) as staging:
    app = Path(staging) / (app_name + ".app")
    contents = app / "Contents"
    (contents / "MacOS").mkdir(parents=True)
    resources = contents / "Resources"
    resources.mkdir()
    shutil.copy2(binary, contents / "MacOS/WriterCraft")
    icon = Path("assets/WriterCraft.icns")
    if not icon.is_file():
        raise SystemExit("Application icon is missing; prepare the checked-in runtime assets first")
    shutil.copy2(icon, resources / "WriterCraft.icns")
    licenses = resources / "licenses"
    licenses.mkdir()
    shutil.copy2("LICENSE", licenses / "LICENSE")
    for notice in Path("assets").rglob("*"):
        if notice.is_file() and ("LICENSE" in notice.name or notice.name == "NOTICE.md"):
            destination = licenses / notice.relative_to("assets")
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(notice, destination)
    notice_command = [sys.executable, "scripts/generate-notices.py",
                      "--output", str(licenses / "THIRD_PARTY_NOTICES.txt")]
    if rust_target:
        notice_command += ["--target", rust_target]
    if editor_preview:
        notice_command += ["--features", "studio-desktop/editor-preview"]
    subprocess.run(notice_command, check=True)
    info = {
        "CFBundleExecutable": "WriterCraft", "CFBundleIdentifier": bundle_id,
        "CFBundleName": app_name, "CFBundleDisplayName": app_name, "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": marketing_version, "CFBundleVersion": build_number,
        "CFBundleGetInfoString": app_name + " " + version, "WriterCraftVersion": version,
        "WriterCraftBuildCommit": os.environ.get("GITHUB_SHA", "local-debug"),
        "LSMinimumSystemVersion": os.environ.get("MACOSX_DEPLOYMENT_TARGET", "14.0"),
        "NSHighResolutionCapable": True, "CFBundleIconFile": "WriterCraft.icns",
        "CFBundleDevelopmentRegion": "en", "CFBundleLocalizations": ["ru", "en", "de", "fr", "es", "pt", "zh-Hans", "ja"],
    }
    if editor_preview:
        info["LSEnvironment"] = {"WRITERCRAFT_EDITOR_PREVIEW": "1"}
        info["WriterCraftChannel"] = "editor-engineering-preview"
    with (contents / "Info.plist").open("wb") as stream:
        plistlib.dump(info, stream)
    identity = os.environ.get("WRITERCRAFT_SIGN_IDENTITY", "-")
    subprocess.run(["codesign", "--force", "--deep", "--sign", identity, str(app)], check=True)
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
    if output.exists():
        shutil.rmtree(output)
    shutil.move(str(app), str(output))
print("Packaged " + str(output) + " — " + version + " (" + profile + ")")
PYTHON
