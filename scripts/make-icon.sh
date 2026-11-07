#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p artifacts/WriterCraft.iconset artifacts/swift-cache
swift -module-cache-path artifacts/swift-cache scripts/MakeIcon.swift artifacts/WriterCraft.iconset
iconutil -c icns artifacts/WriterCraft.iconset -o assets/WriterCraft.icns
