use std::collections::HashMap;
use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{anyhow, Context};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::decoder::AudioData;

pub(crate) fn decode(path: &str) -> Result<AudioData, anyhow::Error> {
    decode_with_limit(path, None)
}

pub(crate) fn decode_preview(
    path: &str,
    max_frames: usize,
) -> Result<AudioData, anyhow::Error> {
    decode_with_limit(path, Some(max_frames))
}

fn decode_with_limit(
    path: &str,
    max_frames: Option<usize>,
) -> Result<AudioData, anyhow::Error> {
    let file = File::open(path).with_context(|| format!("failed to open {path}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|ext| ext.to_str()) {
        hint.with_extension(ext);
    }

    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .with_context(|| format!("symphonia failed to probe {path}"))?;

    let mut metadata = HashMap::new();
    if let Some(probed_metadata) = probed.metadata.get() {
        collect_metadata(probed_metadata.current(), &mut metadata);
    }

    let mut format = probed.format;
    collect_metadata(format.metadata().current(), &mut metadata);

    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("no supported audio track in {path}"))?;
    let bits_per_sample = track.codec_params.bits_per_sample.unwrap_or(0) as u16;
    let estimated_frames = track.codec_params.n_frames;

    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .with_context(|| format!("unsupported codec in {path}"))?;

    let mut samples: Vec<f32> = Vec::new();
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut channels: u16 = 0;
    let mut sample_rate: u32 = 0;
    let mut limit_reached = false;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(err)) if err.kind() == ErrorKind::UnexpectedEof => break,
            Err(SymphoniaError::ResetRequired) => {
                return Err(anyhow!(
                    "chained or resetting streams are not supported: {path}"
                ));
            }
            Err(SymphoniaError::IoError(err)) => return Err(err.into()),
            Err(err) => return Err(anyhow!("symphonia packet read failed for {path}: {err}")),
        };

        consume_format_metadata(&mut format, &mut metadata);

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                channels = spec.channels.count() as u16;
                sample_rate = spec.rate;

                let buffer = sample_buf.get_or_insert_with(|| {
                    SampleBuffer::<f32>::new(decoded.capacity() as u64, spec)
                });
                buffer.copy_interleaved_ref(decoded);
                let decoded_samples = buffer.samples();
                if samples.capacity() == 0 {
                    if let Some(frames) = estimated_frames {
                        let frames = usize::try_from(frames).unwrap_or(usize::MAX);
                        let capacity = frames.saturating_mul(usize::from(channels));
                        let _ = samples.try_reserve(capacity);
                    }
                }
                let remaining = max_frames
                    .map(|frames| {
                        usize::from(channels)
                            .checked_mul(frames)
                            .unwrap_or(usize::MAX)
                            .saturating_sub(samples.len())
                    })
                    .unwrap_or(usize::MAX);
                let take = remaining.min(decoded_samples.len());
                samples.extend(
                    decoded_samples[..take]
                        .iter()
                        .map(|&sample| normalize(sample)),
                );
                if let Some(frames) = max_frames {
                    if samples.len() >= frames.saturating_mul(usize::from(channels)) {
                        limit_reached = true;
                    }
                }
            }
            Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::DecodeError(_)) => {
                // Skip corrupt packets; the next packet may still decode cleanly.
            }
            Err(err) => {
                return Err(anyhow!("symphonia decode failed for {path}: {err}"));
            }
        }
        if limit_reached {
            break;
        }
    }

    if samples.is_empty() {
        return Err(anyhow!("no audio frames decoded from {path}"));
    }
    if channels == 0 {
        return Err(anyhow!("decoder reported zero channels for {path}"));
    }

    let total_frames = samples.len() as u64 / u64::from(channels);
    Ok(AudioData {
        samples,
        sample_rate,
        channels,
        bits_per_sample,
        total_frames,
        metadata,
    })
}

pub(crate) fn probe(path: &str) -> Result<AudioData, anyhow::Error> {
    let file = File::open(path).with_context(|| format!("failed to open {path}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|ext| ext.to_str()) {
        hint.with_extension(ext);
    }

    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .with_context(|| format!("symphonia failed to probe {path}"))?;

    let mut metadata = HashMap::new();
    if let Some(probed_metadata) = probed.metadata.get() {
        collect_metadata(probed_metadata.current(), &mut metadata);
    }

    let mut format = probed.format;
    consume_format_metadata(&mut format, &mut metadata);

    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("no supported audio track in {path}"))?;

    let sample_rate = track.codec_params.sample_rate.unwrap_or(0);
    let channels = track
        .codec_params
        .channels
        .map(|channels| channels.count() as u16)
        .unwrap_or(0);
    let bits_per_sample = track.codec_params.bits_per_sample.unwrap_or(0) as u16;
    let total_frames = track.codec_params.n_frames.unwrap_or(0);

    if sample_rate == 0 || channels == 0 {
        return Err(anyhow!("probe could not determine stream parameters for {path}"));
    }

    Ok(AudioData {
        samples: Vec::new(),
        sample_rate,
        channels,
        bits_per_sample,
        total_frames,
        metadata,
    })
}

fn collect_metadata(
    revision: Option<&symphonia::core::meta::MetadataRevision>,
    metadata: &mut HashMap<String, String>,
) {
    if let Some(revision) = revision {
        for tag in revision.tags() {
            let key = tag.key.trim_matches('\0').to_string();
            let value = tag.value.to_string();
            metadata.insert(key, value.trim_matches('\0').to_string());
        }
    }
}

fn consume_format_metadata(
    format: &mut Box<dyn symphonia::core::formats::FormatReader>,
    metadata: &mut HashMap<String, String>,
) {
    while !format.metadata().is_latest() {
        collect_metadata(format.metadata().current(), metadata);
        format.metadata().pop();
    }
    collect_metadata(format.metadata().current(), metadata);
}

fn normalize(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}
