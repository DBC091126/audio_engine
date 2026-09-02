use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;

use audio_engine::ate::{
    make_config, process_ate, AteAnalyzer, AteConfig, AtePreset, OversamplingMode,
};
use audio_engine::decode_file;
use audio_engine::dsd::{encode_dff, encode_dsf, DsdStream};
use audio_engine::dsd_modulator::{pcm_to_dsd, pcm_to_dsd_with_family, DsdMode};
use audio_engine::encoder::{encode_pcm, PcmFormat};
use audio_engine::ffi::{get_ate_response_curve, get_file_info, process_file};
use audio_engine::pipeline::{run_dsd_pipeline, run_pcm_pipeline};
use audio_engine::resampler::{
    create_src, destroy_src, get_recommended_rates, process_src, validate_rate_family,
};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: audio_engine <input...>\n       audio_engine --all\n       audio_engine --pcm-test\n       audio_engine --dsd-test\n       audio_engine --resample-test\n       audio_engine --dsd-modulator-test\n       audio_engine --ate-test\n       audio_engine --ffi-test\n       audio_engine -i <input> -o <output.wav/flac> -r <rate> -b <bits>\n       audio_engine -i <input> -o <output.dsf/dff> -d <64|128|256>"
        );
        return Ok(());
    }

    if args[1] == "--pcm-test" {
        return run_pcm_test();
    }
    if args[1] == "--dsd-test" {
        return run_dsd_test();
    }
    if args[1] == "--resample-test" {
        return run_resample_test();
    }
    if args[1] == "--dsd-modulator-test" {
        return run_dsd_modulator_test();
    }
    if args[1] == "--ate-test" {
        return run_ate_test();
    }
    if args[1] == "--ffi-test" {
        return run_ffi_test();
    }
    if args[1] == "-i" {
        if args.iter().any(|arg| arg == "-d") {
            return run_dsd_pipeline(&args[1..]);
        }
        return run_pcm_pipeline(&args[1..]);
    }

    let paths: Vec<String> = if args[1] == "--all" {
        ["wav", "flac", "mp3", "ogg", "opus", "m4a"]
            .iter()
            .map(|ext| format!("test_audio/sample.{ext}"))
            .collect()
    } else {
        args[1..].to_vec()
    };

    for path in paths {
        let label = PathBuf::from(&path).display().to_string();
        match decode_file(&path) {
            Ok(data) => print_summary(&label, &data),
            Err(err) => eprintln!("FAILED {label}: {err:#}"),
        }
    }

    Ok(())
}

fn run_ffi_test() -> anyhow::Result<()> {
    let input = CString::new("test.flac").map_err(anyhow::Error::msg)?;
    let out_wav = CString::new("ffi_out.wav").map_err(anyhow::Error::msg)?;
    let out_dsf = CString::new("ffi_out.dsf").map_err(anyhow::Error::msg)?;

    let rc = process_file(
        input.as_ptr(),
        out_wav.as_ptr(),
        176_400,
        24,
        0,
        0,
        0,
        0,
        0.0,
    );
    if rc != 0 {
        return Err(anyhow::anyhow!("process_file PCM failed with code {rc}"));
    }

    let rc = process_file(input.as_ptr(), out_dsf.as_ptr(), 0, 0, 2, 256, 1, 0, 0.25);
    if rc != 0 {
        return Err(anyhow::anyhow!(
            "process_file DSD+ATE failed with code {rc}"
        ));
    }

    for ate_style in 3..=7u8 {
        let out = CString::new(format!("/tmp/ffi_ate_style_{ate_style}.wav"))
            .map_err(anyhow::Error::msg)?;
        let rc = process_file(
            input.as_ptr(),
            out.as_ptr(),
            176_400,
            24,
            0,
            0,
            1,
            ate_style,
            0.5,
        );
        if rc != 0 {
            return Err(anyhow::anyhow!(
                "process_file solid-state ATE style {ate_style} failed with code {rc}"
            ));
        }
        println!("FFI solid-state ATE style {ate_style}: WAV ok");
    }

    let mut curve_buffer = vec![0u8; 65_536];
    let rc = get_ate_response_curve(
        input.as_ptr(),
        1,
        3,
        0.5,
        curve_buffer.as_mut_ptr() as *mut c_char,
        curve_buffer.len(),
    );
    if rc != 0 {
        return Err(anyhow::anyhow!(
            "get_ate_response_curve failed with code {rc}"
        ));
    }
    let curve = unsafe { CStr::from_ptr(curve_buffer.as_ptr() as *const c_char) }
        .to_string_lossy()
        .to_string();
    let mut lines = curve.lines();
    let first = lines.next().unwrap_or_default();
    println!(
        "FFI ATE response curve: {} points, first={first}",
        curve.lines().count()
    );

    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut bits = 0u16;
    let mut duration = 0.0f64;
    let rc = get_file_info(
        input.as_ptr(),
        &mut sample_rate,
        &mut channels,
        &mut bits,
        &mut duration,
    );
    if rc != 0 {
        return Err(anyhow::anyhow!("get_file_info failed with code {rc}"));
    }

    println!("FFI process_file: PCM WAV ok, DSD DSF+ATE ok");
    println!(
        "FFI get_file_info: {} Hz, {} ch, {} bit, {:.3}s",
        sample_rate, channels, bits, duration
    );
    Ok(())
}

fn run_ate_test() -> anyhow::Result<()> {
    let input = make_test_pcm(44_100, 0.1, 2);
    let mut output = vec![0.0f32; input.len()];
    let config = make_config(AtePreset::Tube, OversamplingMode::X4, 0.5, 42, true);
    let mut progress = |p: f32| {
        if p > 0.99 {
            println!("ATE progress: 100%");
        }
    };
    process_ate(
        &input,
        &mut output,
        &config,
        44_100,
        44_100,
        Some(&mut progress),
    );

    let left: Vec<f32> = output.iter().step_by(2).copied().collect();
    let result = AteAnalyzer::analyze_harmonics(&left, 44_100, 1000.0);
    println!("ATE Tube harmonic test:");
    println!("  THD  : {:.3}%", result.thd_percent);
    println!("  Fund : {:.1} dB", result.fingerprint[0]);
    println!("  H2/H3: {:.1} dB / {:.1} dB", result.h2_db, result.h3_db);
    println!("  H4/H5: {:.1} dB / {:.1} dB", result.h4_db, result.h5_db);

    let solid_state_presets = [
        AtePreset::SolidStateClassASingleEnded,
        AtePreset::SolidStateClassAPushPull,
        AtePreset::SolidStateClassAb,
        AtePreset::SolidStateClassD,
        AtePreset::VintageSolidState,
    ];
    for preset in solid_state_presets {
        let mut preset_output = vec![0.0f32; input.len()];
        let preset_config =
            make_config(preset, OversamplingMode::X4, 0.5, 42, true);
        process_ate(
            &input,
            &mut preset_output,
            &preset_config,
            44_100,
            44_100,
            None,
        );
        let preset_left: Vec<f32> = preset_output.iter().step_by(2).copied().collect();
        let preset_result =
            AteAnalyzer::analyze_harmonics(&preset_left, 44_100, 1000.0);
        println!("ATE {preset:?} harmonic test:");
        println!("  THD  : {:.3}%", preset_result.thd_percent);
        println!(
            "  H2/H3: {:.1} dB / {:.1} dB",
            preset_result.h2_db, preset_result.h3_db
        );
        println!(
            "  H4/H5: {:.1} dB / {:.1} dB",
            preset_result.h4_db, preset_result.h5_db
        );
    }

    let identity_config = AteConfig {
        enable: false,
        ..AteConfig::default()
    };
    let mut identity = vec![0.0f32; input.len()];
    process_ate(
        &input,
        &mut identity,
        &identity_config,
        44_100,
        44_100,
        None,
    );
    if identity != input {
        return Err(anyhow::anyhow!("disabled ATE is not an identity copy"));
    }
    println!("ATE disabled path: identity copy ok");

    let imd_input = make_dual_tone_pcm(44_100, 0.1, 19_000.0, 20_000.0, 2);
    let mut imd_output = vec![0.0f32; imd_input.len()];
    let imd_config = make_config(AtePreset::VintageDac, OversamplingMode::X4, 0.4, 7, false);
    process_ate(
        &imd_input,
        &mut imd_output,
        &imd_config,
        44_100,
        44_100,
        None,
    );
    let imd_left: Vec<f32> = imd_output.iter().step_by(2).copied().collect();
    let imd_db = AteAnalyzer::analyze_imd(&imd_left, 44_100, 19_000.0, 20_000.0);
    println!("ATE IMD test (19k + 20k): {imd_db:.1} dB");

    Ok(())
}

fn make_dual_tone_pcm(sample_rate: u32, seconds: f32, f1: f32, f2: f32, channels: u16) -> Vec<f32> {
    let frames = (sample_rate as f32 * seconds) as usize;
    let mut pcm = Vec::with_capacity(frames * usize::from(channels));
    for frame in 0..frames {
        let t = frame as f32 / sample_rate as f32;
        let sample = ((2.0 * std::f32::consts::PI * f1 * t).sin()
            + (2.0 * std::f32::consts::PI * f2 * t).sin())
            * 0.5;
        for _ in 0..channels {
            pcm.push(sample);
        }
    }
    pcm
}

fn run_dsd_modulator_test() -> anyhow::Result<()> {
    let pcm_352 = make_test_pcm(352_800, 0.01, 2);
    let pcm_384 = make_test_pcm(384_000, 0.01, 2);
    let pcm_192 = make_test_pcm(192_000, 0.02, 2);

    let dsd256_44 =
        pcm_to_dsd(&pcm_352, 352_800, 2, DsdMode::DSD256).map_err(anyhow::Error::msg)?;
    let dsd64_44 = pcm_to_dsd(&pcm_352, 352_800, 2, DsdMode::DSD64).map_err(anyhow::Error::msg)?;
    let dsd256_48 =
        pcm_to_dsd(&pcm_384, 384_000, 2, DsdMode::DSD256).map_err(anyhow::Error::msg)?;
    let dsd256_192 =
        pcm_to_dsd(&pcm_192, 192_000, 2, DsdMode::DSD256).map_err(anyhow::Error::msg)?;

    println!(
        "352.8k PCM -> DSD256 (44.1k): {} Hz, {} bytes",
        dsd256_44.sample_rate,
        dsd256_44.data.len()
    );
    println!(
        "352.8k PCM -> DSD64 (44.1k): {} Hz, {} bytes",
        dsd64_44.sample_rate,
        dsd64_44.data.len()
    );
    println!(
        "384k PCM -> DSD256 (48k): {} Hz, {} bytes",
        dsd256_48.sample_rate,
        dsd256_48.data.len()
    );
    println!(
        "192k PCM -> DSD256 (48k): {} Hz, {} bytes",
        dsd256_192.sample_rate,
        dsd256_192.data.len()
    );

    let reject_44_to_48 = pcm_to_dsd_with_family(&pcm_352, 352_800, 2, DsdMode::DSD128, 48_000);
    let reject_48_to_44 = pcm_to_dsd_with_family(&pcm_384, 384_000, 2, DsdMode::DSD128, 44_100);
    if reject_44_to_48.is_ok() || reject_48_to_44.is_ok() {
        return Err(anyhow::anyhow!("DSD 家族防火墙未拒绝混族调用"));
    }
    println!(
        "352.8k PCM -> DSD128 (48k base): Err: {}",
        reject_44_to_48.unwrap_err()
    );
    println!(
        "384k PCM -> DSD128 (44.1k base): Err: {}",
        reject_48_to_44.unwrap_err()
    );

    Ok(())
}

fn make_test_pcm(sample_rate: u32, seconds: f32, channels: u16) -> Vec<f32> {
    let frames = (sample_rate as f32 * seconds) as usize;
    let mut pcm = Vec::with_capacity(frames * usize::from(channels));
    for frame in 0..frames {
        let t = frame as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * 1_000.0 * t).sin() * 0.5;
        for _ in 0..channels {
            pcm.push(sample);
        }
    }
    pcm
}

fn run_resample_test() -> anyhow::Result<()> {
    let cases = [
        (44_100u32, 88_200u32, true),
        (44_100, 96_000, false),
        (48_000, 192_000, true),
        (48_000, 176_400, false),
        (44_100, 352_800, true),
        (44_100, 705_600, false),
    ];

    for (src_rate, tgt_rate, expected_ok) in cases {
        let ok = validate_rate_family(src_rate, tgt_rate).is_ok();
        if ok != expected_ok {
            return Err(anyhow::anyhow!(
                "validate_rate_family({src_rate}, {tgt_rate}) returned ok={ok}, expected {expected_ok}"
            ));
        }
        println!(
            "{src_rate} -> {tgt_rate}: {}",
            if ok { "Ok" } else { "Err" }
        );
    }

    let recommended = get_recommended_rates(44_100);
    println!("44.1k 推荐列表: {recommended:?}");

    let state = create_src(44_100, 88_200, 1);
    if state.is_null() {
        return Err(anyhow::anyhow!(
            "create_src returned null for 44.1k -> 88.2k"
        ));
    }

    let input: Vec<f32> = (0..44_100)
        .map(|i| {
            let t = i as f32 / 44_100.0;
            (2.0 * std::f32::consts::PI * 1_000.0 * t).sin()
        })
        .collect();
    let mut output = vec![0f32; 100_000];
    let mut input_frames_used = 0;
    let mut output_frames_gen = 0;

    let error = process_src(
        state,
        input.as_ptr(),
        output.as_mut_ptr(),
        44_100,
        100_000,
        1,
        &mut input_frames_used,
        &mut output_frames_gen,
    );
    if error != 0 {
        destroy_src(state);
        return Err(anyhow::anyhow!("process_src failed with code {error}"));
    }
    println!(
        "44.1k -> 88.2k FFI: input_frames_used={input_frames_used}, output_frames_gen={output_frames_gen}"
    );
    destroy_src(state);

    let rejected = create_src(44_100, 96_000, 1);
    if !rejected.is_null() {
        destroy_src(rejected);
        return Err(anyhow::anyhow!(
            "create_src unexpectedly accepted 44.1k -> 96k"
        ));
    }
    println!("44.1k -> 96k FFI: rejected with null pointer");

    Ok(())
}

fn run_dsd_test() -> anyhow::Result<()> {
    let sample_rate = 2_822_400u32;
    let channels = 2u16;
    let block_size = 4096usize;
    let blocks_per_channel = 86usize;
    let bytes_per_channel = block_size * blocks_per_channel;

    let mut data = Vec::with_capacity(bytes_per_channel * usize::from(channels));
    for byte_index in 0..bytes_per_channel {
        let mut byte = 0u8;
        for bit in 0..8 {
            let sample_index = byte_index * 8 + bit;
            let t = sample_index as f64 / f64::from(sample_rate);
            if (2.0 * std::f64::consts::PI * 1_000.0 * t).sin() >= 0.0 {
                byte |= 1 << (7 - bit);
            }
        }
        data.push(byte);
        data.push(byte);
    }

    let stream = DsdStream {
        data,
        sample_rate,
        channels,
    };

    let mut metadata = HashMap::new();
    metadata.insert("title".to_string(), "1 kHz DSD Encoder Test".to_string());
    metadata.insert("artist".to_string(), "audio_engine".to_string());
    metadata.insert("album".to_string(), "Batch 3".to_string());
    metadata.insert(
        "comment".to_string(),
        "Generated by audio_engine".to_string(),
    );

    encode_dsf("test.dsf", &stream, &metadata)?;
    encode_dff("test.dff", &stream, &metadata)?;

    let samples_per_channel = stream.data.len() / usize::from(channels) * 8;
    println!("wrote test.dsf and test.dff");
    println!(
        "  dsd64 stereo, {samples_per_channel} samples/channel ({:.3}s)",
        samples_per_channel as f64 / f64::from(stream.sample_rate)
    );
    Ok(())
}

fn run_pcm_test() -> anyhow::Result<()> {
    let sample_rate = 44_100u32;
    let channels = 2u16;
    let frames = sample_rate as usize;
    let mut samples = Vec::with_capacity(frames * usize::from(channels));
    for frame in 0..frames {
        let t = frame as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * 1_000.0 * t).sin() * 0.5;
        samples.push(sample);
        samples.push(sample);
    }

    let mut metadata = HashMap::new();
    metadata.insert("title".to_string(), "1 kHz PCM Encoder Test".to_string());
    metadata.insert("artist".to_string(), "audio_engine".to_string());
    metadata.insert("album".to_string(), "Batch 2".to_string());
    metadata.insert(
        "comment".to_string(),
        "Generated by audio_engine".to_string(),
    );

    encode_pcm(
        "test.wav",
        &samples,
        sample_rate,
        channels,
        24,
        PcmFormat::Wav,
        &metadata,
    )?;
    encode_pcm(
        "test.flac",
        &samples,
        sample_rate,
        channels,
        16,
        PcmFormat::Flac,
        &metadata,
    )?;

    println!("wrote test.wav (24-bit WAV) and test.flac (16-bit FLAC)");
    for path in ["test.wav", "test.flac"] {
        match decode_file(path) {
            Ok(data) => print_summary(path, &data),
            Err(err) => eprintln!("FAILED re-read {path}: {err:#}"),
        }
    }
    Ok(())
}

fn print_summary(path: &str, data: &audio_engine::AudioData) {
    println!("[{path}]");
    println!("  sample_rate : {}", data.sample_rate);
    println!("  channels    : {}", data.channels);
    println!("  total_frames: {}", data.total_frames);
    println!(
        "  first 5      : {:?}",
        data.samples.iter().take(5).collect::<Vec<_>>()
    );
    println!("  metadata     :");
    let mut keys: Vec<&String> = data.metadata.keys().collect();
    keys.sort();
    for key in keys {
        println!("    {key} = {}", data.metadata.get(key).unwrap());
    }
}
