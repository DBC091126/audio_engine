#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v cargo >/dev/null 2>&1 && [ -x /tmp/cargo/bin/cargo ]; then
  export PATH="/tmp/cargo/bin:$PATH"
  export CARGO_HOME="${CARGO_HOME:-/tmp/cargo}"
  export RUSTUP_HOME="${RUSTUP_HOME:-/tmp/rustup}"
fi

cargo run --release -- --dsd-test

if command -v ffmpeg >/dev/null 2>&1; then
  echo "ffmpeg DSF info:"
  ffmpeg -hide_banner -i test.dsf 2>&1 | sed -n '1,30p' || true
  echo "ffmpeg DFF info:"
  ffmpeg -hide_banner -i test.dff 2>&1 | sed -n '1,30p' || true
fi

if command -v ffprobe >/dev/null 2>&1; then
  echo "ffprobe DSF stream:"
  ffprobe -v error -show_entries stream=codec_name,sample_rate,channels,duration \
    -of default=noprint_wrappers=1 test.dsf
  echo "ffprobe DFF stream:"
  ffprobe -v error -show_entries stream=codec_name,sample_rate,channels,duration \
    -of default=noprint_wrappers=1 test.dff
  echo "ffprobe DSF tags:"
  ffprobe -v error -show_entries format_tags -of default=noprint_wrappers=1 test.dsf
  echo "ffprobe DFF tags:"
  ffprobe -v error -show_entries format_tags -of default=noprint_wrappers=1 test.dff
fi
