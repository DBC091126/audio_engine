#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v cargo >/dev/null 2>&1 && [ -x /tmp/cargo/bin/cargo ]; then
  export PATH="/tmp/cargo/bin:$PATH"
  export CARGO_HOME="${CARGO_HOME:-/tmp/cargo}"
  export RUSTUP_HOME="${RUSTUP_HOME:-/tmp/rustup}"
fi

cargo run --release -- --pcm-test

if command -v ffprobe >/dev/null 2>&1; then
  echo "ffprobe WAV tags:"
  ffprobe -v error -show_entries format_tags -of default=noprint_wrappers=1 test.wav
  echo "ffprobe FLAC tags:"
  ffprobe -v error -show_entries format_tags -of default=noprint_wrappers=1 test.flac
fi
