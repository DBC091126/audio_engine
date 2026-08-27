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
cargo run --release -- -i test.flac -o out.dsf -d 256
cargo run --release -- -i test.flac -o out.dff -d 64

if command -v ffprobe >/dev/null 2>&1; then
  echo "ffprobe out.dsf:"
  ffprobe -v error -show_entries stream=codec_name,sample_rate,channels \
    -show_entries format=duration,format_name -of default=noprint_wrappers=1 out.dsf
  echo "ffprobe out.dff:"
  ffprobe -v error -show_entries stream=codec_name,sample_rate,channels \
    -show_entries format=duration,format_name -of default=noprint_wrappers=1 out.dff
fi
