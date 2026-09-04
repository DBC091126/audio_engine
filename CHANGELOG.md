# Changelog

## 1.2.1

- Added tube push-pull, ferrite tape, and phono-stage ATE presets.
- Exposed advanced ATE noise, jitter, phase, crossover, and even/odd harmonic
  controls through the GUI and the new `process_file_custom` FFI entry point.
- Fixed global language synchronization so dynamic file information, memory,
  cache, queue, and output labels are re-rendered after switching language.
- Added the "Audio Engine" window title and file-family labels to the language
  switch path.
- Made M4A/AAC response curves work without FFmpeg by probing the sample rate
  from Symphonia and decoding the preview through Symphonia.
- Surface native response-curve errors back to the GUI instead of returning a
  bare `code -1`.
- Re-enabled the MSYS2 FFmpeg build for Windows and bundled its runtime DLLs,
  so OGG/Opus and other FFmpeg-backed formats are recognized on Windows.
- Added DSF/DFF DSD input decoding to PCM, including GUI file selection and
  response-curve preview support.
- Added reference-tone matching that analyzes even/odd harmonics and applies
  them through the advanced ATE controls.
- Upgraded batch conversion with per-file ATE overrides, skip selection, queue
  reorder, configurable parallelism, and conflict-free output naming.
- Added engine-backed playback preview using a temporary WAV.
- Added five more ATE device presets: power transformer, cathode follower,
  opamp preamp, phono cartridge resonance, and DAC filter rolloff.
- Added a one-click update check button in the toolbar.
- Bumped the project, Maven, installer, and release version to `1.2.1`.

## 1.1.0

### Performance

- Added polyphase ATE oversampling/decimation, reducing FIR work at 4x.
- Parallelized ATE FIR filtering, channel deinterleaving, and interleaving.
- Parallelized solid-state channel-mismatch processing.
- Replaced per-sample Box-Muller noise with a static Gaussian lookup table.
- Replaced separate harmonic Goertzel passes with a single-pass multi-tone
  computation.
- Parallelized DSD channel modulation.
- Introduced a streaming DSD packer that keeps noise-shaping state continuous
  and avoids materializing the full oversampled Float32 buffer.
- Streaming DSF/DFF writers now write directly to disk through `BufWriter`
  instead of building a complete file copy in memory.
- Pre-reserved decoder buffers for Symphonia and FFmpeg.
- Probed M4A/Opus metadata from headers instead of decoding the full file.
- Parallelized multi-file CLI decoding.
- Added Cargo and Maven dependency caching to release workflows.
- Added the `examples/ate_bench.rs` repeatable ATE benchmark.
- Added tube push-pull, ferrite tape, and phono-stage presets.
- Exposed advanced ATE noise, jitter, phase, crossover, and harmonic controls
  through the GUI and new `process_file_custom` FFI entry point.
- Localized the advanced ATE lab controls into English, Simplified Chinese,
  and Traditional Chinese.

### Correctness

- Flushed the FFmpeg `swr` resampler so Opus no longer loses tail samples.
- Limited Symphonia preview decoding to the requested frame count.
- Removed NUL bytes from WAV/MP3 metadata for display.
- Avoided mutating shared GUI conversion settings in mixed-rate batches.
- Added zero-sample-rate guards across resampler, DSD, and ATE family checks.
- Fixed invalid DFF output caused by a missing root block marker.

### Tooling

- Bumped project, Maven, and installer version to `1.1.0`.
- Added `audio_engine --version`.
- Made packaging and GUI test scripts auto-detect Java/Maven instead of relying
  on machine-specific SDKMAN paths.
- Added SHA-256 checksum uploads to every release installer.

## 0.1.0

- Initial cross-platform audio engine, JavaFX GUI, ATE presets, PCM/DSD pipeline,
  and release workflow.
