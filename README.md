# Audio Engine

Cross-platform high-resolution audio conversion and analog tone processing.

Current version: **1.2.1**

The engine decodes common compressed and lossless formats, resamples with the
high-quality `SRC_SINC_BEST_QUALITY` algorithm, converts PCM to DSD, and applies
the ATE analog texture engine. It ships as a Rust core with a JavaFX GUI.

The product is organized as a music player first: the library, search, and
playback UI is the main screen, while ATE tone processing and PCM/DSD conversion
are secondary tools accessible from the same navigation.

## Highlights

- Decode WAV, FLAC, MP3, OGG Vorbis, Opus, and AAC/M4A into Float32.
- Resample only inside the 44.1 kHz or 48 kHz family, with 1/8x to 8x ratios.
- Encode PCM WAV/FLAC at 16-bit or 24-bit with TPDF dither and metadata.
- Encode packed DSF and DFF containers for DSD64, DSD128, and DSD256.
- Read DSF/DFF DSD input and convert it to PCM for ATE or further processing.
- ATE analog presets: tube, vinyl, tape, hybrid, and solid-state A/AB/D.
- Before/after frequency-response comparison for the ATE pipeline.
- Reference-tone matching that derives even/odd harmonic settings from a track.
- JavaFX dashboard with batch conversion, drag/drop, and cached analysis.
- Music-player view with folder scanning, search-based library, play/pause,
  previous/next, seeking, and engine-backed playback for FLAC/OGG/DSD etc.
- Real-time ATE DSP through `SourceDataLine`: when ATE is enabled in the player,
  decoded PCM is streamed through the Rust ATE engine before it reaches the
  sound card.
- Live player visualizer: a 64-bar spectrum-style canvas is driven by the
  real-time DSP audio buffer while playing with ATE enabled.
- Playlist import/export via M3U and local cover-art display from common
  folder names (`cover.jpg`, `folder.jpg`, etc).
- Batch per-file ATE overrides, skip selection, queue reorder, parallel conversion,
  and automatic conflict renaming.
- Playback preview that turns FLAC/OGG/DSD etc. into a temporary WAV.
- One-click update check that opens the GitHub release page.
- Multi-threaded decode, ATE filtering, DSD modulation, and CLI batch decode.
- Streaming DSF/DFF writers and a streaming DSD packer to reduce memory.
- Streaming DSD-to-PCM conversion that writes blocks directly to the output.
- Stateful streaming ATE for same-rate conversions, keeping processed audio
  buffered in small blocks instead of a full second copy.
- SHA-256 checksums published with every GitHub release installer.

## Releases

Official installers and checksums are published on GitHub:

<https://github.com/DBC091126/audio_engine/releases/tag/v1.2.1>

Available artifacts:

| Platform | Artifact |
| --- | --- |
| Linux x86_64 | `audio-engine_1.2.1_amd64.deb` |
| Linux arm64 | `audio-engine_1.2.1_arm64.deb` |
| macOS Intel | `audio-engine_1.2.1_macos-x86_64.dmg` |
| macOS Apple Silicon | `audio-engine_1.2.1_macos-arm64.dmg` |
| Windows x86_64 | `AudioEngine-1.2.1.exe` |

Each artifact has a matching `.sha256` file. On Linux you can install a `.deb`
with `dpkg -i`; on macOS open the `.dmg` and drag `AudioEngine` into
`Applications`; on Windows run the `.exe` installer.

## Build From Source

### Required Toolchain

- Rust stable
- JDK 21 or newer
- Maven, or use `gui/mvnw`
- FFmpeg development headers on Linux/macOS for Opus and AAC/M4A
- `pkg-config`
- CMake for the bundled `libsamplerate` build

On Debian/Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y \
  pkg-config libavformat-dev libavcodec-dev libavutil-dev \
  libavfilter-dev libswresample-dev libswscale-dev libavdevice-dev
```

On macOS with Homebrew:

```bash
brew install ffmpeg pkg-config
```

### Build the Core

```bash
cargo build --release
./target/release/audio_engine --version
```

### Build the GUI

```bash
bash scripts/test_gui.sh
```

### Package an Installer

```bash
bash scripts/package.sh
```

The script produces `.deb` on Linux, `.dmg` on macOS, and `.exe` on Windows.
`PACKAGE_TYPE=app-image` produces a portable app directory.

## Command Line

### Decode and Inspect Files

```bash
audio_engine song.flac
audio_engine song.flac song.mp3 song.m4a
audio_engine --all
```

The CLI decodes input files in parallel and prints sample rate, channel count,
frame count, the first samples, and container metadata.

### PCM Conversion

```bash
audio_engine -i song.flac -o out.wav -r 176400 -b 24
audio_engine -i song.flac -o out.flac -r 88200 -b 16
```

Only family-compatible rates are accepted:

| Source family | Allowed targets |
| --- | --- |
| 44.1 kHz | 88.2 / 176.4 / 352.8 kHz |
| 48 kHz | 96 / 192 / 384 kHz |

### DSD Conversion

```bash
audio_engine -i song.flac -o out.dsf -d 256
audio_engine -i song.flac -o out.dff -d 64
```

`-d` accepts `64`, `128`, or `256`. Intermediate working rates are inserted
automatically, then the PCM is modulated to packed 1-bit DSD.

### Diagnostics

```bash
audio_engine --version
audio_engine --pcm-test
audio_engine --dsd-test
audio_engine --resample-test
audio_engine --dsd-modulator-test
audio_engine --ate-test
audio_engine --ffi-test
```

## GUI

The JavaFX GUI contains:

- Main dashboard with animated DSP console and navigation cards.
- Batch PCM/DSD converter with drag/drop and file metadata cache.
- ATE tone lab with analog preset selection and intensity control.
- Before/after frequency-response comparison chart.
- Progress, log, queue, output-folder, and language controls.
- English, Simplified Chinese, and Traditional Chinese localization.

Settings are stored in `~/.audio_engine/config.toml`.

## ATE Presets

The ATE engine models nonlinear state, crossover distortion, channel mismatch,
noise, jitter, and oversampling. Available presets include:

- Tube
- Vinyl
- Tape
- Hybrid
- Vintage DAC
- Solid-State Class A single-ended
- Solid-State Class A push-pull
- Solid-State Class AB
- Solid-State Class D
- Vintage solid state
- Tube push-pull
- Ferrite tape
- Phono stage
- Power transformer saturation
- Cathode follower
- Opamp preamp
- Phono cartridge resonance
- DAC filter rolloff

The audio-synthesis path is deliberately deterministic with a fixed random
seed so the same file and preset produce the same result.

The advanced ATE lab also exposes a custom control set in the GUI:

- Thermal/pink/tape noise floor
- Clock jitter amount
- Channel phase offset
- Crossover depth
- Even-harmonic and odd-harmonic scaling

Custom controls are overlaid on the selected preset; leaving a control at its
default keeps the preset character intact.

## Performance Notes

The release is optimized without degrading the requested sound path:

- ATE FIR oversampling uses a polyphase implementation and Rayon parallelism,
  reducing operations while keeping the same filter response.
- DC blocking and low-pass stages run in place to reduce allocations.
- Gaussian noise uses a static lookup table instead of per-sample
  transcendental functions.
- DSD modulation runs both channels in parallel and packs bits incrementally.
- DSF and DFF writers stream to disk instead of building a second full-file
  `Vec<u8>` in memory.
- M4A/Opus metadata is probed from headers instead of fully decoding the file.
- The CLI decodes multiple input files in parallel.

Local benchmark on a 10-second stereo file:

| Path | Time | Peak RSS |
| --- | --- | --- |
| ATE 4x oversampling (tube) | about 0.22 s | small |
| PCM 44.1 kHz to 176.4 kHz | about 1.6 s | moderate |
| PCM to DSD256 | about 5.5 s | about 160 MB |

Results vary by CPU, core count, and file length.

## Testing

Run the Rust unit suite:

```bash
cargo test --release
```

Run the format and pipeline scripts:

```bash
bash scripts/test_all_formats.sh
bash scripts/test_ffi.sh
bash scripts/test_dsd_pipeline.sh
bash scripts/test_ate.sh
bash scripts/test_gui.sh
```

`scripts/test_all_formats.sh` asserts that Opus decodes to the expected frame
count, which guards the FFmpeg resampler flush logic.

Run the test suite and the ATE benchmark together:

```bash
bash scripts/bench.sh
```

## Layout

```text
src/
  decoder.rs            Unified decode entry point
  ffmpeg_decoder.rs     FFmpeg fallback and header probing
  symphonia_decoder.rs  Symphonia decoder
  resampler.rs          libsamplerate family validation
  pipeline.rs           PCM/DSD command-line pipelines
  encoder.rs            PCM WAV/FLAC writers
  dsd.rs                DSF/DFF container writers
  dsd_modulator.rs      PCM-to-DSD modulator
  ffi.rs                C ABI for Java/JNA
  ate/                  Analog texture engine
gui/
  src/main/java/        JavaFX application
  src/main/resources/   Theme CSS
.github/workflows/      Cross-platform release builds
examples/ate_bench.rs   Repeatable ATE performance benchmark
```

## Windows Note

The Windows installer now bundles the MSYS2 FFmpeg runtime DLLs, so it supports
the same format family as the Linux and macOS builds, including OGG with Opus
and AAC/M4A. The installer is larger because the FFmpeg runtime is included.

## License

MIT
