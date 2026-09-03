use std::collections::HashMap;
use std::sync::Once;

use anyhow::{anyhow, Context};
use ffmpeg_next::format::{sample::Type as SampleType, Sample};
use ffmpeg_next::frame::Audio as AudioFrame;
use ffmpeg_next::software::resampling::Context as ResamplingContext;
use ffmpeg_next::{codec, format, media, ChannelLayout};

use crate::decoder::AudioData;

static FFMPEG_INIT: Once = Once::new();

pub(crate) fn probe_sample_rate(path: &str) -> Result<u32, anyhow::Error> {
    let mut init_error: Option<ffmpeg_next::Error> = None;
    FFMPEG_INIT.call_once(|| {
        if let Err(err) = ffmpeg_next::init() {
            init_error = Some(err);
        }
    });
    if let Some(err) = init_error {
        return Err(err.into());
    }

    let context = format::input(path).with_context(|| format!("ffmpeg failed to open {path}"))?;
    let stream = context
        .streams()
        .best(media::Type::Audio)
        .ok_or_else(|| anyhow!("no audio stream in {path}"))?;
    let decoder_context = codec::context::Context::from_parameters(stream.parameters())
        .with_context(|| format!("ffmpeg could not create decoder context for {path}"))?;
    let decoder = decoder_context
        .decoder()
        .audio()
        .with_context(|| format!("ffmpeg audio decoder unavailable for {path}"))?;
    let sample_rate = decoder.rate();
    if sample_rate == 0 {
        return Err(anyhow!("ffmpeg reported zero sample rate for {path}"));
    }
    Ok(sample_rate)
}

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
    let mut init_error: Option<ffmpeg_next::Error> = None;
    FFMPEG_INIT.call_once(|| {
        if let Err(err) = ffmpeg_next::init() {
            init_error = Some(err);
        }
    });
    if let Some(err) = init_error {
        return Err(err.into());
    }

    let mut context =
        format::input(path).with_context(|| format!("ffmpeg failed to open {path}"))?;
    let stream = context
        .streams()
        .best(media::Type::Audio)
        .ok_or_else(|| anyhow!("no audio stream in {path}"))?;
    let stream_index = stream.index();

    let mut metadata = HashMap::new();
    for (key, value) in context.metadata().iter() {
        metadata.insert(key.to_string(), value.to_string());
    }
    for (key, value) in stream.metadata().iter() {
        metadata.insert(key.to_string(), value.to_string());
    }

    let decoder_context = codec::context::Context::from_parameters(stream.parameters())
        .with_context(|| format!("ffmpeg could not create decoder context for {path}"))?;
    let mut decoder = decoder_context
        .decoder()
        .audio()
        .with_context(|| format!("ffmpeg audio decoder unavailable for {path}"))?;
    decoder
        .set_parameters(stream.parameters())
        .with_context(|| format!("ffmpeg could not initialize audio decoder for {path}"))?;

    let sample_rate = decoder.rate();
    let channel_layout = decoder.channel_layout();
    let channel_count = channel_layout.channels();
    let output_layout = if channel_count > 0 {
        channel_layout
    } else {
        ChannelLayout::default(decoder.channels().into())
    };
    let input_layout = if channel_count > 0 {
        channel_layout
    } else {
        output_layout
    };
    let channels = if output_layout.channels() > 0 {
        output_layout.channels() as u16
    } else {
        return Err(anyhow!("ffmpeg reported zero audio channels for {path}"));
    };

    let mut samples: Vec<f32> = Vec::new();
    if let Some(capacity) = estimate_sample_capacity(
        stream.frames(),
        context.duration(),
        sample_rate,
        channels,
    ) {
        let _ = samples.try_reserve(capacity);
    }

    let mut resampler = ResamplingContext::get(
        decoder.format(),
        input_layout,
        sample_rate,
        Sample::F32(SampleType::Packed),
        output_layout,
        sample_rate,
    )
    .with_context(|| format!("ffmpeg resampler setup failed for {path}"))?;

    let mut decoded_frame = AudioFrame::empty();
    let mut resampled_frame = AudioFrame::empty();
    let max_samples = max_frames.map(|frames| frames.saturating_mul(usize::from(channels)));
    let mut limit_reached = false;

    'packets: for (packet_stream, packet) in context.packets() {
        if packet_stream.index() != stream_index {
            continue;
        }
        decoder
            .send_packet(&packet)
            .with_context(|| format!("ffmpeg send_packet failed for {path}"))?;

        while decoder.receive_frame(&mut decoded_frame).is_ok() {
            resampler
                .run(&decoded_frame, &mut resampled_frame)
                .with_context(|| format!("ffmpeg resample failed for {path}"))?;
            if append_resampled(&resampled_frame, channels, max_samples, &mut samples)? {
                limit_reached = true;
                break 'packets;
            }
        }
    }

    if !limit_reached {
        decoder
            .send_eof()
            .with_context(|| format!("ffmpeg send_eof failed for {path}"))?;
        while decoder.receive_frame(&mut decoded_frame).is_ok() {
            resampler
                .run(&decoded_frame, &mut resampled_frame)
                .with_context(|| format!("ffmpeg final resample failed for {path}"))?;
            if append_resampled(&resampled_frame, channels, max_samples, &mut samples)? {
                limit_reached = true;
                break;
            }
        }

        if !limit_reached {
            let _ = flush_resampler(&mut resampler, channels, max_samples, &mut samples)?;
        }
    }

    if samples.is_empty() {
        return Err(anyhow!("ffmpeg decoded no audio frames from {path}"));
    }

    let total_frames = samples.len() as u64 / u64::from(channels);
    Ok(AudioData {
        samples,
        sample_rate,
        channels,
        bits_per_sample: 0,
        total_frames,
        metadata,
    })
}

fn estimate_sample_capacity(
    stream_frames: i64,
    context_duration_us: i64,
    sample_rate: u32,
    channels: u16,
) -> Option<usize> {
    let estimated_frames = if stream_frames > 0 {
        u64::try_from(stream_frames).ok()?
    } else if context_duration_us > 0 {
        let duration_us = u64::try_from(context_duration_us).ok()?;
        duration_us
            .saturating_mul(u64::from(sample_rate))
            .saturating_div(1_000_000)
    } else {
        return None;
    };

    let frames = usize::try_from(estimated_frames).unwrap_or(usize::MAX);
    Some(frames.saturating_mul(usize::from(channels)))
}

fn flush_resampler(
    resampler: &mut ResamplingContext,
    channels: u16,
    max_samples: Option<usize>,
    output: &mut Vec<f32>,
) -> Result<bool, anyhow::Error> {
    loop {
        let output_def = resampler.output();
        let mut flushed_frame =
            AudioFrame::new(output_def.format, 8192, output_def.channel_layout);
        let delay = resampler
            .flush(&mut flushed_frame)
            .map_err(|err| anyhow!("ffmpeg resampler flush failed: {err}"))?;
        if flushed_frame.samples() == 0 {
            break;
        }
        if append_resampled(&flushed_frame, channels, max_samples, output)? {
            return Ok(true);
        }
        if delay.is_none() {
            break;
        }
    }
    Ok(false)
}

fn append_resampled(
    frame: &AudioFrame,
    channels: u16,
    max_samples: Option<usize>,
    output: &mut Vec<f32>,
) -> Result<bool, anyhow::Error> {
    let total_samples = frame
        .samples()
        .checked_mul(usize::from(channels))
        .ok_or_else(|| anyhow!("frame sample count overflow"))?;
    let bytes = frame.data(0);
    let needed = total_samples
        .checked_mul(4)
        .ok_or_else(|| anyhow!("frame byte count overflow"))?;
    if bytes.len() < needed {
        return Err(anyhow!(
            "resampled frame too small: got {} bytes, need {needed}",
            bytes.len()
        ));
    }

    let floats = unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const f32, total_samples) };
    let take = max_samples
        .map(|max| max.saturating_sub(output.len()).min(floats.len()))
        .unwrap_or(floats.len());
    output.extend(floats[..take].iter().map(|&sample| normalize(sample)));
    Ok(max_samples.is_some_and(|max| output.len() >= max))
}

fn normalize(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}
