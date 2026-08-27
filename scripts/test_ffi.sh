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
cargo run --release -- --ffi-test

if command -v ffprobe >/dev/null 2>&1; then
  echo "ffprobe ffi_out.wav:"
  ffprobe -v error -show_entries stream=codec_name,sample_rate,channels,bits_per_sample \
    -of default=noprint_wrappers=1 ffi_out.wav
  echo "ffprobe ffi_out.dsf:"
  ffprobe -v error -show_entries stream=codec_name,sample_rate,channels \
    -of default=noprint_wrappers=1 ffi_out.dsf
fi
