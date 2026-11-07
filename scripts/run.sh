#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run --locked -p studio-desktop -- "$@"
