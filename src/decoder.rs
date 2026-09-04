use std::path::Path;
use std::collections::HashMap;

use anyhow::{anyhow, Context};

#[cfg(feature = "ffmpeg")]
use crate::ffmpeg_decoder;
use crate::symphonia_decoder;
use crate::dsd::{decode_dff, decode_dsf, dsd_to_pcm};

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

    if ext == "dsf" || ext == "dff" {
        let stream = if ext == "dsf" {
            decode_dsf(path)?
        } else {
            decode_dff(path)?
        };
        let output_rate = stream.sample_rate / 64;
        return dsd_to_pcm(&stream, output_rate);
    }

    if matches!(ext.as_str(), "m4a" | "aac" | "mp4") {
        match symphonia_decoder::decode(path) {
            Ok(data) => return Ok(data),
            Err(symphonia_err) => {
                #[cfg(feature = "ffmpeg")]
                return ffmpeg_decoder::decode(path).with_context(|| {
                    format!(
                        "symphonia failed ({symphonia_err:#}); ffmpeg fallback also failed for {path}"
                    )
                });
                #[cfg(not(feature = "ffmpeg"))]
                return Err(symphonia_err).with_context(|| {
                    format!("symphonia decode failed for {path}")
                });
            }
        }
    }

    if matches!(ext.as_str(), "opus") {
        #[cfg(feature = "ffmpeg")]
        return ffmpeg_decoder::decode(path)
            .with_context(|| format!("ffmpeg decoder failed for {path}"));
        #[cfg(not(feature = "ffmpeg"))]
        return Err(anyhow!(
            "Opus depends on FFmpeg and is unavailable in this build: {path}"
        ));
    }

    match symphonia_decoder::decode(path) {
        Ok(data) => Ok(data),
        #[cfg(feature = "ffmpeg")]
        Err(symphonia_err) => ffmpeg_decoder::decode(path).with_context(|| {
            format!("symphonia failed ({symphonia_err:#}); ffmpeg fallback also failed for {path}")
        }),
        #[cfg(not(feature = "ffmpeg"))]
        Err(symphonia_err) => Err(symphonia_err).with_context(|| {
            format!("symphonia decode failed for {path}")
        }),
    }
}

pub(crate) fn decode_preview_seconds(
    path: &str,
    seconds: f64,
) -> Result<AudioData, anyhow::Error> {
    let ext = extension(path).ok_or_else(|| anyhow!("cannot determine file extension: {path}"))?;
    if ext == "dsf" || ext == "dff" {
        let stream = if ext == "dsf" {
            decode_dsf(path)?
        } else {
            decode_dff(path)?
        };
        let output_rate = stream.sample_rate / 64;
        let mut pcm = dsd_to_pcm(&stream, output_rate)?;
        let channels = usize::from(pcm.channels);
        let max_frames = ((f64::from(output_rate)) * seconds).max(1.0) as usize;
        let frames = pcm.samples.len() / channels;
        let take = max_frames.min(frames);
        pcm.samples.truncate(take * channels);
        pcm.total_frames = take as u64;
        return Ok(pcm);
    }
    let ffmpeg_first = matches!(ext.as_str(), "m4a" | "aac" | "mp4" | "opus");
    let symphonia_first = matches!(ext.as_str(), "m4a" | "aac" | "mp4")
        && symphonia_decoder::probe_sample_rate_only(path).is_ok();

    let sample_rate = if symphonia_first {
        symphonia_decoder::probe_sample_rate_only(path)?
    } else if ffmpeg_first {
        #[cfg(feature = "ffmpeg")]
        {
            ffmpeg_decoder::probe_sample_rate(path)?
        }
        #[cfg(not(feature = "ffmpeg"))]
        {
            return Err(anyhow!(
                "FFmpeg-dependent format {} is unavailable in this build",
                ext
            ));
        }
    } else {
        match symphonia_decoder::probe(path) {
            Ok(data) if data.sample_rate > 0 => data.sample_rate,
            #[cfg(feature = "ffmpeg")]
            _ => ffmpeg_decoder::probe_sample_rate(path)?,
            #[cfg(not(feature = "ffmpeg"))]
            _ => return Err(anyhow!(
                "cannot determine sample rate for preview: {path}"
            )),
        }
    };
    if sample_rate == 0 {
        return Err(anyhow!("cannot determine sample rate for preview: {path}"));
    }

    let max_frames = ((f64::from(sample_rate)) * seconds).max(1.0) as usize;
    if symphonia_first {
        return symphonia_decoder::decode_preview(path, max_frames)
            .with_context(|| format!("symphonia preview decoder failed for {path}"));
    }
    if ffmpeg_first {
        #[cfg(feature = "ffmpeg")]
        {
            return ffmpeg_decoder::decode_preview(path, max_frames)
                .with_context(|| format!("ffmpeg preview decoder failed for {path}"));
        }
        #[cfg(not(feature = "ffmpeg"))]
        {
            return Err(anyhow!(
                "FFmpeg-dependent format {} is unavailable in this build",
                ext
            ));
        }
    }

    match symphonia_decoder::decode_preview(path, max_frames) {
        Ok(data) => Ok(data),
        #[cfg(feature = "ffmpeg")]
        Err(symphonia_err) => ffmpeg_decoder::decode_preview(path, max_frames).with_context(|| {
            format!(
                "symphonia preview failed ({symphonia_err:#}); ffmpeg fallback also failed for {path}"
            )
        }),
        #[cfg(not(feature = "ffmpeg"))]
        Err(symphonia_err) => Err(symphonia_err).with_context(|| {
            format!("symphonia preview failed for {path}")
        }),
    }
}

/// Read stream parameters and metadata from container headers when possible.
///
/// The GUI uses this to show file information without decoding the full audio. Formats without
/// reliable header duration still fall back to the full decoder.
pub(crate) fn probe_file(path: &str) -> Result<AudioData, anyhow::Error> {
    let ext = extension(path).ok_or_else(|| anyhow!("cannot determine file extension: {path}"))?;

    if ext == "dsf" || ext == "dff" {
        let stream = if ext == "dsf" {
            decode_dsf(path)?
        } else {
            decode_dff(path)?
        };
        let channels = usize::from(stream.channels);
        let bytes_per_channel = stream.data.len() / channels;
        let output_rate = stream.sample_rate / 64;
        let total_frames = (bytes_per_channel * 8 / 64) as u64;
        return Ok(AudioData {
            samples: Vec::new(),
            sample_rate: output_rate,
            channels: stream.channels,
            bits_per_sample: 1,
            total_frames,
            metadata: HashMap::new(),
        });
    }

    if matches!(ext.as_str(), "m4a" | "aac" | "mp4" | "opus") {
        if matches!(ext.as_str(), "m4a" | "aac" | "mp4") {
            if let Ok(data) = symphonia_decoder::probe(path) {
                if data.sample_rate > 0 && data.channels > 0 && data.total_frames > 0 {
                    return Ok(data);
                }
            }
            #[cfg(feature = "ffmpeg")]
            {
                return match ffmpeg_decoder::probe_info(path) {
                    Ok(data) if data.total_frames > 0 => Ok(data),
                    Ok(_) => decode_file(path),
                    Err(_) => decode_file(path),
                };
            }
            #[cfg(not(feature = "ffmpeg"))]
            return decode_file(path);
        }
        if matches!(ext.as_str(), "opus") {
            #[cfg(feature = "ffmpeg")]
            {
                return match ffmpeg_decoder::probe_info(path) {
                    Ok(data) if data.total_frames > 0 => Ok(data),
                    Ok(_) => decode_file(path),
                    Err(_) => decode_file(path),
                };
            }
            #[cfg(not(feature = "ffmpeg"))]
            return Err(anyhow!(
                "Opus depends on FFmpeg and is unavailable in this build: {path}"
            ));
        }
    }

    match symphonia_decoder::probe(path) {
        Ok(data) if data.sample_rate > 0 && data.channels > 0 && data.total_frames > 0 => Ok(data),
        Ok(_) => decode_file(path),
        Err(symphonia_err) => decode_file(path).with_context(|| {
            format!("symphonia probe failed ({symphonia_err:#}); full decode also failed for {path}")
        }),
    }
}

fn extension(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}
