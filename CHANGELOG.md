# Changelog

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
