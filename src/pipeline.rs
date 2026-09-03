use std::path::Path;

use anyhow::{anyhow, Context};

use crate::decode_file;
use crate::dsd::{encode_dff, encode_dsf};
use crate::dsd_modulator::{pcm_to_dsd, DsdMode};
use crate::encoder::{PcmFormat, PcmStreamWriter};
use crate::resampler::Resampler;

const BLOCK_FRAMES: usize = 4096;

struct PipelineArgs {
    input: String,
    output: String,
    target_rate: u32,
    bit_depth: u16,
    dsd_mode: u32,
}

pub fn run_pcm_pipeline(args: &[String]) -> anyhow::Result<()> {
    let options = parse_args(args, false)?;
    if !matches!(options.bit_depth, 16 | 24) {
        return Err(anyhow!(
            "bit depth must be 16 or 24, got {}",
            options.bit_depth
        ));
    }

    let audio = decode_file(&options.input)
        .with_context(|| format!("failed to decode input {}", options.input))?;
    let format = pcm_format_from_path(&options.output)?;
    let mut writer = PcmStreamWriter::create(
        &options.output,
        options.target_rate,
        audio.channels,
        options.bit_depth,
        format,
        &audio.metadata,
    )?;

    println!(
        "PCM pipeline: {} -> {} ({} Hz, {} bit, {} ch)",
        options.input, options.output, options.target_rate, options.bit_depth, audio.channels
    );

    resample_audio(
        &audio.samples,
        audio.sample_rate,
        options.target_rate,
        audio.channels,
        |block| writer.write_block(block),
    )?;
    writer.finish()?;
    println!("PCM pipeline complete: {}", options.output);
    Ok(())
}

pub fn run_dsd_pipeline(args: &[String]) -> anyhow::Result<()> {
    let options = parse_args(args, true)?;
    let mode = dsd_mode_from_u32(options.dsd_mode)?;
    let audio = decode_file(&options.input)
        .with_context(|| format!("failed to decode input {}", options.input))?;
    let working_rate = dsd_working_rate(audio.sample_rate)?;

    println!(
        "DSD pipeline: {} -> {} (mode {:?}, working PCM {} Hz)",
        options.input, options.output, mode, working_rate
    );

    let pcm = if working_rate == audio.sample_rate {
        audio.samples.clone()
    } else {
        let mut resampled = Vec::new();
        resample_audio(
            &audio.samples,
            audio.sample_rate,
            working_rate,
            audio.channels,
            |block| {
                resampled.extend_from_slice(block);
                Ok(())
            },
        )?;
        resampled
    };

    let dsd = pcm_to_dsd(&pcm, working_rate, audio.channels, mode).map_err(anyhow::Error::msg)?;
    let metadata = audio.metadata.clone();

    match dsd_format_from_path(&options.output)? {
        DsdOutput::Dsf => encode_dsf(&options.output, &dsd, &metadata)?,
        DsdOutput::Dff => encode_dff(&options.output, &dsd, &metadata)?,
    }
    println!(
        "DSD pipeline complete: {} ({} Hz, {} bytes)",
        options.output,
        dsd.sample_rate,
        dsd.data.len()
    );
    Ok(())
}

fn parse_args(args: &[String], dsd: bool) -> Result<PipelineArgs, anyhow::Error> {
    let mut input = None;
    let mut output = None;
    let mut target_rate = None;
    let mut bit_depth = None;
    let mut dsd_mode = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| anyhow!("missing value for {flag}"))?;
        match flag {
            "-i" => input = Some(value.clone()),
            "-o" => output = Some(value.clone()),
            "-r" => {
                target_rate = Some(
                    value
                        .parse::<u32>()
                        .with_context(|| format!("invalid target rate {value}"))?,
                )
            }
            "-b" => {
                bit_depth = Some(
                    value
                        .parse::<u16>()
                        .with_context(|| format!("invalid bit depth {value}"))?,
                )
            }
            "-d" => {
                dsd_mode = Some(
                    value
                        .parse::<u32>()
                        .with_context(|| format!("invalid DSD mode {value}"))?,
                )
            }
            other => return Err(anyhow!("unknown option {other}")),
        }
        index += 2;
    }

    let input = input.ok_or_else(|| anyhow!("missing -i <input>"))?;
    let output = output.ok_or_else(|| anyhow!("missing -o <output>"))?;

    if dsd {
        Ok(PipelineArgs {
            input,
            output,
            target_rate: 0,
            bit_depth: 0,
            dsd_mode: dsd_mode.ok_or_else(|| anyhow!("missing -d <dsd_mode>"))?,
        })
    } else {
        Ok(PipelineArgs {
            input,
            output,
            target_rate: target_rate.ok_or_else(|| anyhow!("missing -r <target_rate>"))?,
            bit_depth: bit_depth.ok_or_else(|| anyhow!("missing -b <bit_depth>"))?,
            dsd_mode: 0,
        })
    }
}

fn pcm_format_from_path(path: &str) -> Result<PcmFormat, anyhow::Error> {
    match extension(path)?.to_ascii_lowercase().as_str() {
        "wav" => Ok(PcmFormat::Wav),
        "flac" => Ok(PcmFormat::Flac),
        ext => Err(anyhow!(
            "unsupported PCM output extension .{ext}; expected .wav or .flac"
        )),
    }
}

enum DsdOutput {
    Dsf,
    Dff,
}

fn dsd_format_from_path(path: &str) -> Result<DsdOutput, anyhow::Error> {
    match extension(path)?.to_ascii_lowercase().as_str() {
        "dsf" => Ok(DsdOutput::Dsf),
        "dff" => Ok(DsdOutput::Dff),
        ext => Err(anyhow!(
            "unsupported DSD output extension .{ext}; expected .dsf or .dff"
        )),
    }
}

fn extension(path: &str) -> Result<String, anyhow::Error> {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("output path has no extension: {path}"))
}

pub(crate) fn dsd_mode_from_u32(mode: u32) -> Result<DsdMode, anyhow::Error> {
    match mode {
        64 => Ok(DsdMode::DSD64),
        128 => Ok(DsdMode::DSD128),
        256 => Ok(DsdMode::DSD256),
        _ => Err(anyhow!(
            "unsupported DSD mode {mode}; expected 64, 128 or 256"
        )),
    }
}

pub(crate) fn dsd_working_rate(input_rate: u32) -> Result<u32, anyhow::Error> {
    if input_rate == 0 {
        return Err(anyhow!("input sample rate must be greater than zero"));
    }

    let base = if input_rate % 44_100 == 0 {
        44_100
    } else if input_rate % 48_000 == 0 {
        48_000
    } else {
        return Err(anyhow!(
            "input sample rate {input_rate} Hz does not belong to 44.1k or 48k family"
        ));
    };

    let multiplier = input_rate / base;
    if matches!(multiplier, 2 | 4 | 8) {
        Ok(input_rate)
    } else {
        Ok(base * 8)
    }
}

pub(crate) fn resample_to_vec(
    samples: &[f32],
    src_rate: u32,
    tgt_rate: u32,
    channels: u16,
) -> Result<Vec<f32>, anyhow::Error> {
    let mut out = Vec::new();
    resample_audio(samples, src_rate, tgt_rate, channels, |block| {
        out.extend_from_slice(block);
        Ok(())
    })?;
    Ok(out)
}

fn resample_audio<F>(
    samples: &[f32],
    src_rate: u32,
    tgt_rate: u32,
    channels: u16,
    mut on_block: F,
) -> Result<(), anyhow::Error>
where
    F: FnMut(&[f32]) -> Result<(), anyhow::Error>,
{
    let total_frames = samples.len() / usize::from(channels);
    let mut next_percent = 10u8;

    if src_rate == tgt_rate {
        let mut processed = 0usize;
        for block in samples.chunks(BLOCK_FRAMES * usize::from(channels)) {
            on_block(block)?;
            processed += block.len() / usize::from(channels);
            report_progress(processed, total_frames, &mut next_percent);
        }
        report_progress(total_frames, total_frames, &mut next_percent);
        return Ok(());
    }

    let mut resampler =
        Resampler::new(src_rate, tgt_rate, i32::from(channels)).map_err(anyhow::Error::msg)?;
    let ratio = f64::from(tgt_rate) / f64::from(src_rate);
    let capacity = ((BLOCK_FRAMES as f64 * ratio).ceil() as usize + 8192).max(BLOCK_FRAMES);
    let mut output = vec![0f32; capacity * usize::from(channels)];
    let mut input_pos = 0usize;
    let mut processed = 0usize;

    while input_pos < total_frames {
        let take = BLOCK_FRAMES.min(total_frames - input_pos);
        let input =
            &samples[input_pos * usize::from(channels)..(input_pos + take) * usize::from(channels)];
        let mut consumed = 0usize;

        while consumed < take {
            let remaining = take - consumed;
            let chunk = &input[consumed * usize::from(channels)..];
            let (used, generated) = resampler
                .process(chunk, &mut output, remaining, capacity, 0)
                .map_err(anyhow::Error::msg)?;

            if generated > 0 {
                on_block(&output[..generated as usize * usize::from(channels)])?;
            }
            consumed += used as usize;
            processed += used as usize;
            report_progress(processed, total_frames, &mut next_percent);

            if used == 0 && generated == 0 {
                return Err(anyhow!("libsamplerate made no progress"));
            }
        }

        input_pos += take;
    }

    loop {
        let (_, generated) = resampler
            .process(&[], &mut output, 0, capacity, 1)
            .map_err(anyhow::Error::msg)?;
        if generated == 0 {
            break;
        }
        on_block(&output[..generated as usize * usize::from(channels)])?;
        if (generated as usize) < capacity {
            break;
        }
    }

    report_progress(total_frames, total_frames, &mut next_percent);
    Ok(())
}

fn report_progress(processed: usize, total: usize, next_percent: &mut u8) {
    if total == 0 {
        return;
    }
    let percent = (processed as u64 * 100 / total as u64) as u8;
    while *next_percent <= 100 && percent >= *next_percent {
        println!("  progress: {next_percent}%");
        if *next_percent == 100 {
            *next_percent = 101;
            break;
        }
        *next_percent += 10;
    }
}
