#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release
ls -lh target/release/gemini-file-viewer-linux

