# audio_engine

Batch 1: Rust universal decoder.
Batch 2: Rust PCM encoder.
Batch 3: Rust DSD container encoder (DSF + DFF).
Batch 4: Rust libsamplerate resampler with family validation.
Batch 5: Rust PCM-to-DSD modulator with family firewall.
Batch 6: PCM command-line pipeline (decode -> SRC -> encode).
Batch 7: DSD command-line pipeline (decode -> SRC -> DSD modulation -> DSF/DFF).
Batch 8: ATE analog texture engine (optional post-processing).
Batch 9: Unified FFI entry for Java/JNA.
Batch 10: JavaFX GUI with JNA wrapper and batch processing.

## Scope

- Inputs: WAV / FLAC / MP3 / OGG Vorbis / Opus / AAC(M4A)
- Output: interleaved `Vec<f32>`, original sample rate, channel count, frame count, and metadata
- Primary decoder: Symphonia
- Fallback decoder: FFmpeg (used for Opus and AAC/M4A; also used when Symphonia rejects a file)
- Encoder: WAV (16/24-bit) and FLAC (16/24-bit) with hard clamp, TPDF dither, and metadata
- DSD containers: DSF and DFF for packed 1-bit DSD streams
- Resampler: libsamplerate state machine gated by 44.1k/48k family validation
- DSD modulator: lowpass, zero-fill oversampling, 5th-order noise shaping, rayon blocks
- ATE: presets, oversampling, nonlinear state models, noise/jitter, analyzer
- FFI: `process_file` and `get_file_info` C ABI exports
- GUI: JavaFX batch interface with drag/drop, config, ATE panel, progress, logs

## Build

```bash
cargo build --release
```

## Test all six formats

```bash
bash scripts/test_all_formats.sh
```

The script generates one 1-second sample per format under `test_audio/` using FFmpeg, then prints
sample rate, channel count, total frames, first five Float32 samples, and metadata for each file.

## Test PCM encoder

```bash
bash scripts/test_pcm_encoder.sh
```

The script writes `test.wav` and `test.flac`, re-reads them with the Batch 1 decoder, and uses
`ffprobe` when available to verify the embedded tags.

## Test DSD encoder

```bash
bash scripts/test_dsd_encoder.sh
```

The script writes `test.dsf` and `test.dff`, then uses `ffmpeg -i` and `ffprobe` when available
to verify stream parameters and metadata.

## Test resampler

```bash
bash scripts/test_resampler.sh
```

The script runs the family validation cases and performs an FFI 44.1 kHz to 88.2 kHz conversion
through `create_src` / `process_src` / `destroy_src`.

## Test DSD modulator

```bash
bash scripts/test_dsd_modulator.sh
```

The script covers 44.1k/48k DSD256 conversion, DSD64 conversion, 192 kHz DSD256, and explicit
cross-family rejection through the DSD family firewall.

## PCM pipeline

```bash
cargo run --release -- -i test.flac -o out.wav -r 176400 -b 24
cargo run --release -- -i test.flac -o out.flac -r 88200 -b 16
```

The pipeline decodes the input, resamples in 4096-frame blocks through libsamplerate, and writes
PCM output incrementally.

## DSD pipeline

```bash
cargo run --release -- -i test.flac -o out.dsf -d 256
cargo run --release -- -i test.flac -o out.dff -d 64
```

The pipeline resamples PCM to the correct 352.8k/384k working rate when needed, then modulates to
DSD and writes DSF/DFF.

## ATE test

```bash
cargo run --release -- --ate-test
```

The test runs a Tube preset harmonic analysis and a Vintage DAC IMD test. ATE is disabled by
default; enable it through `AteConfig.enable`.

## FFI test

```bash
cargo run --release -- --ffi-test
```

The test calls `process_file` for PCM WAV and DSD DSF+ATE paths, then calls `get_file_info` to
verify sample rate, channels, bit depth, and duration.

## JavaFX GUI

```bash
bash scripts/run_gui.sh
```

Build and JNA smoke test:

```bash
bash scripts/test_gui.sh
```

The GUI requires JDK 21+ and JavaFX. It stores settings in `~/.audio_engine/config.toml`.

## Packaging

```bash
bash scripts/package.sh
```

On Linux this produces a `.deb`, on macOS a `.dmg`, and on Windows `package.bat` produces an
`.exe`. Set `PACKAGE_TYPE=app-image` for a portable app directory.

Rust cross-compile targets can be selected with:

```bash
RUST_TARGET=x86_64-pc-windows-gnu bash scripts/package.sh
RUST_TARGET=aarch64-apple-darwin bash scripts/package.sh
```

The Maven Assembly descriptor packs the Rust dynamic library into the app directory as
`native/`, and the GUI loads it from the JAR's ProtectionDomain-relative `native/` folder.
