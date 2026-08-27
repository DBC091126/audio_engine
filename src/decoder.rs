use std::path::Path;

use anyhow::{anyhow, Context};

use crate::{ffmpeg_decoder, symphonia_decoder};

/// Fully decoded, interleaved audio in the Float32 domain.
#[derive(Debug, Clone, Default)]
pub struct AudioData {
    /// Interleaved samples: L, R, L, R, ...
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    /// Source bit depth when known; 0 when the decoder cannot report it.
    pub bits_per_sample: u16,
    pub total_frames: u64,
    /// Album, artist, title, and other container tags.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Decode WAV / FLAC / MP3 / OGG Vorbis / Opus / AAC(M4A) into Float32.
///
/// The primary decoder is Symphonia. AAC/M4A and Opus are routed to FFmpeg:
/// AAC because Symphonia may not cover every MP4/AAC flavor, and Opus because
/// `symphonia-bundle-opus` is not published on crates.io.
pub fn decode_file(path: &str) -> Result<AudioData, anyhow::Error> {
    let ext = extension(path).ok_or_else(|| anyhow!("cannot determine file extension: {path}"))?;

    if matches!(ext.as_str(), "m4a" | "aac" | "mp4" | "opus") {
        return ffmpeg_decoder::decode(path)
            .with_context(|| format!("ffmpeg decoder failed for {path}"));
    }

    match symphonia_decoder::decode(path) {
        Ok(data) => Ok(data),
        Err(symphonia_err) => ffmpeg_decoder::decode(path).with_context(|| {
            format!("symphonia failed ({symphonia_err:#}); ffmpeg fallback also failed for {path}")
        }),
    }
}

fn extension(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}
