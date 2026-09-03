#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/test_audio"
mkdir -p "$OUT"

if ! command -v cargo >/dev/null 2>&1 && [ -x /tmp/cargo/bin/cargo ]; then
  export PATH="/tmp/cargo/bin:$PATH"
  export CARGO_HOME="${CARGO_HOME:-/tmp/cargo}"
  export RUSTUP_HOME="${RUSTUP_HOME:-/tmp/rustup}"
fi

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "sine=frequency=440:sample_rate=48000:duration=1" \
  -metadata title="All-in Test" \
  -metadata artist="audio_engine" \
  -metadata album="Batch 1" \
  -c:a pcm_s16le "$OUT/sample.wav"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "sine=frequency=440:sample_rate=48000:duration=1" \
  -metadata title="All-in Test" \
  -metadata artist="audio_engine" \
  -metadata album="Batch 1" \
  -c:a flac "$OUT/sample.flac"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "sine=frequency=440:sample_rate=44100:duration=1" \
  -metadata title="All-in Test" \
  -metadata artist="audio_engine" \
  -metadata album="Batch 1" \
  -c:a libmp3lame -b:a 192k "$OUT/sample.mp3"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "sine=frequency=440:sample_rate=44100:duration=1" \
  -metadata title="All-in Test" \
  -metadata artist="audio_engine" \
  -metadata album="Batch 1" \
  -c:a libvorbis -q:a 5 "$OUT/sample.ogg"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "sine=frequency=440:sample_rate=48000:duration=1" \
  -metadata title="All-in Test" \
  -metadata artist="audio_engine" \
  -metadata album="Batch 1" \
  -c:a libopus -b:a 160k "$OUT/sample.opus"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "sine=frequency=440:sample_rate=44100:duration=1" \
  -metadata title="All-in Test" \
  -metadata artist="audio_engine" \
  -metadata album="Batch 1" \
  -c:a aac -b:a 192k "$OUT/sample.m4a"

cd "$ROOT"
ENGINE_LOG="$OUT/summary.log"
cargo run --release -- "$OUT/sample.wav" "$OUT/sample.flac" "$OUT/sample.mp3" \
  "$OUT/sample.ogg" "$OUT/sample.opus" "$OUT/sample.m4a" >"$ENGINE_LOG" 2>&1
cat "$ENGINE_LOG"

opus_frames="$(awk '
  /\[.*sample\.opus\]/ { in_opus = 1; next }
  in_opus && /total_frames:/ { print $2; exit }
' "$ENGINE_LOG")"
if [[ "$opus_frames" != "48000" ]]; then
  echo "Opus frame count mismatch: expected 48000, got ${opus_frames:-missing}" >&2
  exit 1
fi
