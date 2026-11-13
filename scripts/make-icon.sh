#!/usr/bin/env bash
set -euo pipefail

# No build output or Swift module cache is stored in the repository.
# An optional staging directory retains the comparison, receipt and iconset for review.
if (( $# > 1 )); then
  echo "Usage: scripts/make-icon.sh [STAGING_DIRECTORY]" >&2
  exit 2
fi
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/writercraft-icon.XXXXXX")"
trap 'rm -rf "$work"' EXIT
source_png="$repo_root/assets/brand/lion-logo-approved.png"
source_svg="$repo_root/assets/brand/writercraft-mark.svg"
expected_svg=9475584f66032e04efb8a1a249a681102ea4aea5df91dfa3455fb75cddfcc95c
actual_svg="$(shasum -a 256 "$source_svg")"
if [[ "${actual_svg%% *}" != "$expected_svg" ]]; then
  echo "The approved source SVG must remain byte-for-byte unchanged." >&2
  exit 1
fi
swift -module-cache-path "$work/swift-cache" "$repo_root/scripts/MakeIcon.swift" "$source_png" "$work/output"
iconutil -c icns "$work/output/WriterCraft.iconset" -o "$work/output/WriterCraft.icns"
if (( $# == 1 )); then
  mkdir -p "$1"
  cp -R "$work/output/." "$1/"
else
  cp "$work/output/WriterCraft.icns" "$repo_root/assets/WriterCraft.icns"
  cp "$work/output/WriterCraft.png" "$repo_root/assets/WriterCraft.png"
  cp "$work/output/writercraft-mark.png" "$repo_root/assets/brand/writercraft-mark.png"
  for size in 32 64 128; do
    cp "$work/output/writercraft-mark-$size.png" "$repo_root/assets/brand/writercraft-mark-$size.png"
  done
fi
