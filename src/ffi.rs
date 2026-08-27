use std::ffi::CStr;
use std::os::raw::c_char;

use anyhow::{anyhow, Error};

use crate::ate::{process_ate, AteConfig, AtePreset, OversamplingMode};
use crate::decode_file;
use crate::dsd::{encode_dff, encode_dsf};
use crate::dsd_modulator::pcm_to_dsd;
use crate::encoder::{PcmFormat, PcmStreamWriter};
use crate::pipeline::{dsd_mode_from_u32, dsd_working_rate, resample_to_vec};

const PROCESS_BLOCK_FRAMES: usize = 4096;

/// Unified entry point used by the Java/JNA GUI.
///
/// Returns 0 on success and -1 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn process_file(
    input_path: *const c_char,
    output_path: *const c_char,
    target_rate: u32,
    bit_depth: u16,
    output_format: u8,
    dsd_mode: u16,
    ate_enable: u8,
    ate_style: u8,
    ate_intensity: f32,
) -> i32 {
    let input = match cstr_to_string(input_path) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("[FFI ERROR] {message}");
            return -1;
        }
    };
    let output = match cstr_to_string(output_path) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("[FFI ERROR] {message}");
            return -1;
        }
    };

    let result = match output_format {
        0 | 1 => process_pcm_file(
            &input,
            &output,
            target_rate,
            bit_depth,
            output_format,
            ate_enable,
            ate_style,
            ate_intensity,
        ),
        2 | 3 => process_dsd_file(
            &input,
            &output,
            output_format,
            dsd_mode,
            ate_enable,
            ate_style,
            ate_intensity,
        ),
        format => Err(anyhow!("invalid output_format {format}")),
    };

    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("[FFI ERROR] {error:#}");
            -1
        }
    }
}

/// Return basic stream information for the Java/JNA GUI.
#[unsafe(no_mangle)]
pub extern "C" fn get_file_info(
    path: *const c_char,
    out_sample_rate: *mut u32,
    out_channels: *mut u16,
    out_bits: *mut u16,
    out_duration: *mut f64,
) -> i32 {
    let path = match cstr_to_string(path) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("[FFI ERROR] {message}");
            return -1;
        }
    };

    match decode_file(&path) {
        Ok(audio) => {
            unsafe {
                if !out_sample_rate.is_null() {
                    *out_sample_rate = audio.sample_rate;
                }
                if !out_channels.is_null() {
                    *out_channels = audio.channels;
                }
                if !out_bits.is_null() {
                    *out_bits = audio.bits_per_sample;
                }
                if !out_duration.is_null() {
                    *out_duration = if audio.sample_rate == 0 {
                        0.0
                    } else {
                        audio.total_frames as f64 / f64::from(audio.sample_rate)
                    };
                }
            }
            0
        }
        Err(error) => {
            eprintln!("[FFI ERROR] {error:#}");
            -1
        }
    }
}

/// Return container metadata as `key<TAB>value\n` lines for the Java GUI.
#[unsafe(no_mangle)]
pub extern "C" fn get_file_metadata(
    path: *const c_char,
    buffer: *mut c_char,
    buffer_size: usize,
) -> i32 {
    let path = match cstr_to_string(path) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("[FFI ERROR] {message}");
            return -1;
        }
    };

    let audio = match decode_file(&path) {
        Ok(audio) => audio,
        Err(error) => {
            eprintln!("[FFI ERROR] {error:#}");
            return -1;
        }
    };

    let mut bytes = Vec::new();
    let mut keys: Vec<&String> = audio.metadata.keys().collect();
    keys.sort();
    for key in keys {
        let value = &audio.metadata[key];
        bytes.extend_from_slice(key.as_bytes());
        bytes.push(b'\t');
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(b'\n');
    }

    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let copy_len = bytes.len().min(buffer_size - 1);
    unsafe {
        let dest = std::slice::from_raw_parts_mut(buffer as *mut u8, buffer_size);
        dest[..copy_len].copy_from_slice(&bytes[..copy_len]);
        dest[copy_len] = 0;
    }
    0
}

fn process_pcm_file(
    input: &str,
    output: &str,
    target_rate: u32,
    bit_depth: u16,
    output_format: u8,
    ate_enable: u8,
    ate_style: u8,
    ate_intensity: f32,
) -> Result<(), Error> {
    if target_rate == 0 {
        return Err(anyhow!("PCM target_rate must be greater than zero"));
    }
    if !matches!(bit_depth, 16 | 24) {
        return Err(anyhow!("PCM bit_depth must be 16 or 24"));
    }

    let audio = decode_file(input)?;
    let format = match output_format {
        0 => PcmFormat::Wav,
        1 => PcmFormat::Flac,
        _ => return Err(anyhow!("invalid PCM output format")),
    };

    let mut pcm = if audio.sample_rate == target_rate {
        audio.samples.clone()
    } else {
        resample_to_vec(
            &audio.samples,
            audio.sample_rate,
            target_rate,
            audio.channels,
        )?
    };

    if ate_enable != 0 {
        if audio.channels != 2 {
            return Err(anyhow!("ATE currently requires stereo input"));
        }
        let base = family_base(target_rate)?;
        let config = ate_config(ate_enable, ate_style, ate_intensity);
        let mut processed = vec![0.0f32; pcm.len()];
        process_ate(&pcm, &mut processed, &config, target_rate, base, None);
        pcm = processed;
    }

    let mut writer = PcmStreamWriter::create(
        output,
        target_rate,
        audio.channels,
        bit_depth,
        format,
        &audio.metadata,
    )?;
    for block in pcm.chunks(PROCESS_BLOCK_FRAMES * usize::from(audio.channels)) {
        writer.write_block(block)?;
    }
    writer.finish()
}

fn process_dsd_file(
    input: &str,
    output: &str,
    output_format: u8,
    dsd_mode: u16,
    ate_enable: u8,
    ate_style: u8,
    ate_intensity: f32,
) -> Result<(), Error> {
    let audio = decode_file(input)?;
    let working_rate = dsd_working_rate(audio.sample_rate)?;
    let mode = dsd_mode_from_u32(u32::from(dsd_mode))?;

    let mut pcm = if working_rate == audio.sample_rate {
        audio.samples.clone()
    } else {
        resample_to_vec(
            &audio.samples,
            audio.sample_rate,
            working_rate,
            audio.channels,
        )?
    };

    if ate_enable != 0 {
        if audio.channels != 2 {
            return Err(anyhow!("ATE currently requires stereo input"));
        }
        let base = family_base(working_rate)?;
        let config = ate_config(ate_enable, ate_style, ate_intensity);
        let mut processed = vec![0.0f32; pcm.len()];
        process_ate(&pcm, &mut processed, &config, working_rate, base, None);
        pcm = processed;
    }

    let dsd = pcm_to_dsd(&pcm, working_rate, audio.channels, mode).map_err(Error::msg)?;
    match output_format {
        2 => encode_dsf(output, &dsd, &audio.metadata),
        3 => encode_dff(output, &dsd, &audio.metadata),
        _ => Err(anyhow!("invalid DSD output format")),
    }
}

fn ate_config(enable: u8, style: u8, intensity: f32) -> AteConfig {
    let preset = match style {
        0 => AtePreset::Tube,
        1 => AtePreset::Vinyl,
        _ => AtePreset::Hybrid,
    };
    AteConfig {
        enable: enable != 0,
        preset,
        intensity: intensity.clamp(0.0, 1.0),
        oversampling: OversamplingMode::X4,
        stereo_variance_seed: 0x4154_455f_4656,
        enable_analyzer: false,
        custom_params: None,
    }
}

fn family_base(rate: u32) -> Result<u32, Error> {
    if rate % 44_100 == 0 {
        Ok(44_100)
    } else if rate % 48_000 == 0 {
        Ok(48_000)
    } else {
        Err(anyhow!(
            "sample rate {rate} Hz does not belong to 44.1k or 48k family"
        ))
    }
}

fn cstr_to_string(ptr: *const c_char) -> Result<String, &'static str> {
    if ptr.is_null() {
        return Err("null string pointer");
    }
    let cstr = unsafe { CStr::from_ptr(ptr) };
    cstr.to_str()
        .map(str::to_string)
        .map_err(|_| "string is not valid UTF-8")
}
