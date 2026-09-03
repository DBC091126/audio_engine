use std::ffi::CStr;
use std::os::raw::c_char;

use anyhow::{anyhow, Error};

use crate::ate::{
    analyze_spectrum_mono, process_ate, AteConfig, AtePreset, OversamplingMode,
};
use crate::decode_file;
use crate::decoder::{decode_preview_seconds, probe_file, AudioData};
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

    match probe_file(&path) {
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

/// Return stream information and metadata from one header probe.
#[unsafe(no_mangle)]
pub extern "C" fn get_file_info_ex(
    path: *const c_char,
    out_sample_rate: *mut u32,
    out_channels: *mut u16,
    out_bits: *mut u16,
    out_duration: *mut f64,
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

    match probe_file(&path) {
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
            write_metadata(&audio, buffer, buffer_size);
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

    let audio = match probe_file(&path) {
        Ok(audio) => audio,
        Err(error) => {
            eprintln!("[FFI ERROR] {error:#}");
            return -1;
        }
    };

    write_metadata(&audio, buffer, buffer_size);
    0
}

/// Return a log-frequency response comparison for the first few seconds of a file.
#[unsafe(no_mangle)]
pub extern "C" fn get_ate_response_curve(
    input_path: *const c_char,
    ate_enable: u8,
    ate_style: u8,
    ate_intensity: f32,
    buffer: *mut c_char,
    buffer_size: usize,
) -> i32 {
    let input = match cstr_to_string(input_path) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("[FFI ERROR] {message}");
            return -1;
        }
    };

    let curve = match build_ate_response_curve(&input, ate_enable, ate_style, ate_intensity) {
        Ok(curve) => curve,
        Err(error) => {
            eprintln!("[FFI ERROR] {error:#}");
            return -1;
        }
    };
    write_text(&curve, buffer, buffer_size);
    0
}

fn build_ate_response_curve(
    input: &str,
    ate_enable: u8,
    ate_style: u8,
    ate_intensity: f32,
) -> Result<String, Error> {
    let audio = decode_preview_seconds(input, 4.0)?;
    if !matches!(audio.channels, 1 | 2) {
        return Err(anyhow!(
            "ATE response comparison currently supports mono or stereo, got {} channels",
            audio.channels
        ));
    }
    if audio.sample_rate == 0 {
        return Err(anyhow!("input sample rate is zero"));
    }

    let channel_count = usize::from(audio.channels);
    let total_frames = audio.samples.len() / channel_count;
    let frames = total_frames;
    if frames < audio.sample_rate as usize / 10 {
        return Err(anyhow!("input is too short for response comparison"));
    }

    let selected = &audio.samples[..frames * channel_count];
    let stereo: Vec<f32> = if channel_count == 1 {
        selected
            .iter()
            .flat_map(|&sample| [sample, sample])
            .collect()
    } else {
        selected.to_vec()
    };

    let mut processed = vec![0.0f32; stereo.len()];
    if ate_enable != 0 {
        let base = family_base(audio.sample_rate)?;
        let config = ate_config(ate_enable, ate_style, ate_intensity, audio.sample_rate);
        process_ate(&stereo, &mut processed, &config, audio.sample_rate, base, None);
    } else {
        processed.copy_from_slice(&stereo);
    }

    let before = to_mono(&stereo);
    let after = to_mono(&processed);
    let before_spectrum = analyze_spectrum_mono(&before, audio.sample_rate, 96);
    let after_spectrum = analyze_spectrum_mono(&after, audio.sample_rate, 96);

    let mut output = String::new();
    for point in 0..before_spectrum.len().min(after_spectrum.len()) {
        output.push_str(&format!(
            "{:.2}\t{:.1}\t{:.1}\n",
            before_spectrum[point].freq_hz,
            before_spectrum[point].level_db,
            after_spectrum[point].level_db
        ));
    }
    Ok(output)
}

fn to_mono(samples: &[f32]) -> Vec<f32> {
    samples
        .chunks_exact(2)
        .map(|frame| (frame[0] + frame[1]) * 0.5)
        .collect()
}

fn write_text(text: &str, buffer: *mut c_char, buffer_size: usize) {
    if buffer.is_null() || buffer_size == 0 {
        return;
    }
    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(buffer_size - 1);
    unsafe {
        let dest = std::slice::from_raw_parts_mut(buffer as *mut u8, buffer_size);
        dest[..copy_len].copy_from_slice(&bytes[..copy_len]);
        dest[copy_len] = 0;
    }
}

fn write_metadata(audio: &AudioData, buffer: *mut c_char, buffer_size: usize) {
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
        return;
    }
    let copy_len = bytes.len().min(buffer_size - 1);
    unsafe {
        let dest = std::slice::from_raw_parts_mut(buffer as *mut u8, buffer_size);
        dest[..copy_len].copy_from_slice(&bytes[..copy_len]);
        dest[copy_len] = 0;
    }
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
        let config = ate_config(ate_enable, ate_style, ate_intensity, target_rate);
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
        let config = ate_config(ate_enable, ate_style, ate_intensity, working_rate);
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

fn ate_config(enable: u8, style: u8, intensity: f32, sample_rate: u32) -> AteConfig {
    let preset = match style {
        0 => AtePreset::Tube,
        1 => AtePreset::Vinyl,
        2 => AtePreset::Hybrid,
        3 => AtePreset::SolidStateClassASingleEnded,
        4 => AtePreset::SolidStateClassAPushPull,
        5 => AtePreset::SolidStateClassAb,
        6 => AtePreset::SolidStateClassD,
        7 => AtePreset::VintageSolidState,
        _ => AtePreset::Hybrid,
    };
    AteConfig {
        enable: enable != 0,
        preset,
        intensity: intensity.clamp(0.0, 1.0),
        oversampling: oversampling_for_rate(sample_rate),
        stereo_variance_seed: 0x4154_455f_4656,
        enable_analyzer: false,
        custom_params: None,
    }
}

fn oversampling_for_rate(sample_rate: u32) -> OversamplingMode {
    if sample_rate >= 176_400 {
        OversamplingMode::None
    } else if sample_rate >= 88_200 {
        OversamplingMode::X2
    } else {
        OversamplingMode::X4
    }
}

fn family_base(rate: u32) -> Result<u32, Error> {
    if rate == 0 {
        return Err(anyhow!("sample rate must be greater than zero"));
    }

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
