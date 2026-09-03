use std::time::Instant;

use audio_engine::ate::{make_config, process_ate, AteConfig, AtePreset, OversamplingMode};

fn main() {
    let sample_rate = 44_100u32;
    let seconds = 10.0f32;
    let frames = (sample_rate as f32 * seconds) as usize;
    let mut pcm = Vec::with_capacity(frames * 2);
    for frame in 0..frames {
        let t = frame as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * 997.0 * t).sin() * 0.5
            + (2.0 * std::f32::consts::PI * 5000.0 * t).sin() * 0.25;
        pcm.push(sample);
        pcm.push(sample * 0.98);
    }

    let presets = [
        (AtePreset::Tube, "tube-x4"),
        (AtePreset::SolidStateClassASingleEnded, "class-a-x4"),
        (AtePreset::SolidStateClassD, "class-d-x4"),
        (AtePreset::VintageSolidState, "vintage-solid-state-x4"),
    ];

    let mut output = vec![0.0f32; pcm.len()];
    for (preset, name) in presets {
        let config = make_config(preset, OversamplingMode::X4, 0.5, 42, false);
        let started = Instant::now();
        process_ate(&pcm, &mut output, &config, sample_rate, 44_100, None);
        let elapsed = started.elapsed();
        let rms = output
            .iter()
            .step_by(2)
            .take(frames)
            .map(|sample| sample * sample)
            .sum::<f32>()
            .sqrt()
            / frames as f32;
        println!("{name}: {:.3}s rms={rms:.4}", elapsed.as_secs_f64());
    }

    let disabled = AteConfig {
        enable: false,
        ..AteConfig::default()
    };
    let started = Instant::now();
    process_ate(&pcm, &mut output, &disabled, sample_rate, 44_100, None);
    println!("disabled: {:.3}s", started.elapsed().as_secs_f64());
}
