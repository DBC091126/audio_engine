use rayon::prelude::*;

mod analyzer;
mod config;
mod jitter;
mod noise;
mod nonlinear;
mod oversampling;
mod presets;
mod state_engine;
mod stereo_variance;

pub use analyzer::{analyze_spectrum_mono, db_from_amp, AnalyzerResult, AteAnalyzer, SpectrumPoint};
pub use config::{
    AteConfig, AteCustomParams, AtePreset, ChannelMismatch, CrossoverParams, JitterParams,
    JitterType, NoiseParams, OversamplingMode, StateParams,
};
pub use noise::{Lcg, NoiseSimulator};
pub use presets::{make_config, preset_params};
pub use state_engine::AteState;

struct AteStreamChannel {
    factor: usize,
    sample_rate: u32,
    working_rate: u32,
    preset: AtePreset,
    intensity: f32,
    params: AteCustomParams,
    up: Option<oversampling::PolyphaseUpsampler>,
    down: Option<oversampling::PolyphaseDownsampler>,
    dc_prev: Option<f32>,
    dc_y: f32,
    lowpass_y: Option<f32>,
    state: AteState,
    noise: NoiseSimulator,
    jitter: Option<jitter::JitterSimulator>,
    ultrasonic: Lcg,
}

impl AteStreamChannel {
    fn new(
        factor: usize,
        sample_rate: u32,
        working_rate: u32,
        preset: AtePreset,
        intensity: f32,
        params: AteCustomParams,
        seed: u64,
    ) -> Self {
        let jitter_enabled = params.jitter_params.enabled;
        Self {
            factor,
            sample_rate,
            working_rate,
            preset,
            intensity,
            params,
            up: (factor > 1).then(|| oversampling::PolyphaseUpsampler::new(factor)),
            down: (factor > 1).then(|| oversampling::PolyphaseDownsampler::new(factor)),
            dc_prev: None,
            dc_y: 0.0,
            lowpass_y: None,
            state: AteState::new(),
            noise: NoiseSimulator::new(seed),
            jitter: jitter_enabled
                .then(|| jitter::JitterSimulator::new(seed ^ 0x9E3779B9)),
            ultrasonic: Lcg::new(seed ^ 0xD1B54A32),
        }
    }

    fn process_block(&mut self, input: &[f32]) -> Vec<f32> {
        let mut samples = if self.factor > 1 {
            self.up.as_mut().unwrap().process(input)
        } else {
            input.to_vec()
        };

        let alpha = (2.0 * std::f32::consts::PI * 2.0 / self.working_rate as f32).min(0.999);
        let feedback = 1.0 - alpha;
        for sample in samples.iter_mut() {
            let x = *sample;
            let y = x - self.dc_prev.unwrap_or(x) + feedback * self.dc_y;
            self.dc_prev = Some(x);
            self.dc_y = y;
            *sample = oversampling::denormal_protect(y);
        }

        let cutoff = match self.preset {
            AtePreset::Tape | AtePreset::Hybrid => 38_000.0,
            AtePreset::Tube
            | AtePreset::VintageSolidState
            | AtePreset::TubePushPull
            | AtePreset::CathodeFollower
            | AtePreset::OpampPreamp
            | AtePreset::SolidStateClassASingleEnded
            | AtePreset::SolidStateClassAPushPull
            | AtePreset::SolidStateClassAb => 45_000.0,
            AtePreset::FerriteTape => 36_000.0,
            AtePreset::PhonoStage => 30_000.0,
            AtePreset::PhonoCartridgeResonance => 25_000.0,
            AtePreset::DacFilterRolloff => 22_000.0,
            AtePreset::PowerTransformerSaturation => 38_000.0,
            _ => 55_000.0,
        };
        if self.working_rate as f32 > cutoff * 2.0 {
            let alpha =
                1.0 - (-2.0 * std::f32::consts::PI * cutoff / self.working_rate as f32).exp();
            for sample in samples.iter_mut() {
                let x = *sample;
                let y = self.lowpass_y.unwrap_or(x) + alpha * (x - self.lowpass_y.unwrap_or(x));
                self.lowpass_y = Some(y);
                *sample = oversampling::denormal_protect(y);
            }
        }

        let mut out = Vec::with_capacity(samples.len());
        for sample in samples {
            out.push(oversampling::denormal_protect(nonlinear::apply_nonlinear(
                sample,
                &self.params.poly_a,
                &self.params.crossover,
                &mut self.state,
                &self.params.state_params,
                &self.params.mem_poly,
                self.intensity,
            )));
        }

        self.noise
            .add_noise(&mut out, &self.params.noise_params, self.working_rate as f32);

        if let Some(jitter_sim) = self.jitter.as_mut() {
            out = jitter_sim.apply_jitter(
                &out,
                &self.params.jitter_params,
                self.working_rate as f32,
            );
        }

        if self.sample_rate >= 176_400 {
            let level = noise::db_to_amp(-110.0);
            let sample_rate = self.working_rate as f32;
            let mut index = 0usize;
            for sample in out.iter_mut() {
                let t = index as f32 / sample_rate;
                let envelope = 0.5 + 0.5 * (2.0 * std::f32::consts::PI * 40_000.0 * t).sin();
                *sample += self.ultrasonic.gaussian() * level * envelope;
                index += 1;
            }
        }

        if self.factor > 1 {
            out = self.down.as_mut().unwrap().process(&out);
        }
        out
    }
}

pub struct AteStream {
    left: AteStreamChannel,
    right: AteStreamChannel,
}

impl AteStream {
    pub fn new(config: &AteConfig, sample_rate: u32) -> Result<Self, String> {
        if !config.enable {
            return Err("AteStream requires enabled ATE config".to_string());
        }
        let params = config
            .custom_params
            .clone()
            .unwrap_or_else(|| presets::preset_params(config.preset));
        let factor = config.oversampling.factor();
        let working_rate = sample_rate.saturating_mul(factor as u32);
        let seed = config.stereo_variance_seed;
        Ok(Self {
            left: AteStreamChannel::new(
                factor,
                sample_rate,
                working_rate,
                config.preset,
                config.intensity,
                params.clone(),
                seed,
            ),
            right: AteStreamChannel::new(
                factor,
                sample_rate,
                working_rate,
                config.preset,
                config.intensity,
                params,
                seed ^ 0x5DEECE66D,
            ),
        })
    }

    pub fn process_stereo_block(&mut self, input: &[f32]) -> Vec<f32> {
        let frames = input.len() / 2;
        let mut left = self.left.process_block(&deinterleave(input, 0, frames));
        let mut right = self.right.process_block(&deinterleave(input, 1, frames));
        let params = self.left.params.clone();
        stereo_variance::apply_channel_variance(
            &mut left,
            &mut right,
            &params.channel_mismatch,
            0x4154_455f_4656,
        );
        let mut out = vec![0.0f32; left.len() + right.len()];
        interleave(&left, &right, &mut out);
        out
    }
}

/// Process interleaved stereo Float32 audio through the ATE pipeline.
///
/// The function is a no-op copy when `config.enable` is false. `base_family`
/// is used for preset-dependent noise/power-line behavior and should be
/// 44100 or 48000.
pub fn process_ate(
    input: &[f32],
    output: &mut [f32],
    config: &AteConfig,
    sample_rate: u32,
    base_family: u32,
    mut progress_callback: Option<&mut dyn FnMut(f32)>,
) {
    if input.len() != output.len() {
        return;
    }
    if !config.enable {
        output.copy_from_slice(input);
        if let Some(callback) = progress_callback {
            callback(1.0);
        }
        return;
    }

    let frames = input.len() / 2;
    if frames == 0 {
        return;
    }

    let params = config
        .custom_params
        .clone()
        .unwrap_or_else(|| presets::preset_params(config.preset));
    let factor = config.oversampling.factor();
    let working_rate = sample_rate.saturating_mul(factor as u32);
    let family_hz = if base_family == 48_000 {
        48_000.0
    } else {
        44_100.0
    };

    if let Some(callback) = progress_callback.as_mut() {
        callback(0.05);
    }

    let left_input = deinterleave(input, 0, frames);
    let right_input = deinterleave(input, 1, frames);
    let (mut processed_left, mut processed_right) = rayon::join(
        || {
            process_ate_channel(
                left_input,
                &params,
                factor,
                sample_rate,
                working_rate,
                config.preset,
                config.intensity,
                config.stereo_variance_seed,
            )
        },
        || {
            process_ate_channel(
                right_input,
                &params,
                factor,
                sample_rate,
                working_rate,
                config.preset,
                config.intensity,
                config.stereo_variance_seed ^ 0x5DEECE66D,
            )
        },
    );

    if let Some(callback) = progress_callback.as_mut() {
        callback(0.55);
    }

    stereo_variance::apply_channel_variance(
        &mut processed_left,
        &mut processed_right,
        &params.channel_mismatch,
        config.stereo_variance_seed,
    );

    if let Some(callback) = progress_callback.as_mut() {
        callback(0.9);
    }

    interleave(&processed_left, &processed_right, output);

    if config.enable_analyzer {
        let _ = AteAnalyzer::analyze_harmonics(&processed_left, sample_rate, 1000.0);
        let _ = family_hz;
    }

    if let Some(callback) = progress_callback {
        callback(1.0);
    }
}

fn process_ate_channel(
    mut samples: Vec<f32>,
    params: &AteCustomParams,
    factor: usize,
    sample_rate: u32,
    working_rate: u32,
    preset: AtePreset,
    intensity: f32,
    seed: u64,
) -> Vec<f32> {
    if factor > 1 {
        samples = oversampling::upsample_channel(&samples, factor, sample_rate);
    }
    oversampling::apply_dc_block(&mut samples, working_rate);
    apply_linear_stage(&mut samples, working_rate, preset);

    let mut state = AteState::new();
    let mut out = Vec::with_capacity(samples.len());
    for sample in samples {
        let processed = nonlinear::apply_nonlinear(
            sample,
            &params.poly_a,
            &params.crossover,
            &mut state,
            &params.state_params,
            &params.mem_poly,
            intensity,
        );
        out.push(oversampling::denormal_protect(processed));
    }

    let mut noise_sim = NoiseSimulator::new(seed);
    noise_sim.add_noise(&mut out, &params.noise_params, working_rate as f32);

    if params.jitter_params.enabled {
        let mut jitter_sim = jitter::JitterSimulator::new(seed ^ 0x9E3779B9);
        out = jitter_sim.apply_jitter(&out, &params.jitter_params, working_rate as f32);
    }

    if sample_rate >= 176_400 {
        add_ultrasonic_noise(&mut out, seed ^ 0xD1B54A32, working_rate as f32);
    }

    if factor > 1 {
        out = oversampling::downsample_channel(&out, factor, sample_rate);
    }
    out
}

fn add_ultrasonic_noise(samples: &mut [f32], seed: u64, sample_rate: f32) {
    let mut rng = Lcg::new(seed ^ 0xA7E3_0000);
    let level = noise::db_to_amp(-110.0);
    for (i, sample) in samples.iter_mut().enumerate() {
        let t = i as f32 / sample_rate;
        let envelope = 0.5 + 0.5 * (2.0 * std::f32::consts::PI * 40_000.0 * t).sin();
        *sample += rng.gaussian() * level * envelope;
    }
}

/// Simple intensity calibration helper: scales `config.intensity` toward a
/// target H2 level based on the current analyzer measurement.
pub fn calibrate_ate(
    mut config: AteConfig,
    current: &AnalyzerResult,
    target_h2_db: f32,
) -> AteConfig {
    let correction = target_h2_db - current.h2_db;
    if correction > 0.0 {
        let factor = 10.0f32.powf(correction / 40.0);
        config.intensity = (config.intensity * factor).clamp(0.0, 1.0);
    }
    config
}

fn apply_linear_stage(samples: &mut [f32], sample_rate: u32, preset: AtePreset) {
    let cutoff = match preset {
        AtePreset::Tape | AtePreset::Hybrid => 38_000.0,
        AtePreset::Tube
        | AtePreset::VintageSolidState
        | AtePreset::TubePushPull
        | AtePreset::CathodeFollower
        | AtePreset::OpampPreamp
        | AtePreset::SolidStateClassASingleEnded
        | AtePreset::SolidStateClassAPushPull
        | AtePreset::SolidStateClassAb => 45_000.0,
        AtePreset::FerriteTape => 36_000.0,
        AtePreset::PhonoStage => 30_000.0,
        AtePreset::PhonoCartridgeResonance => 25_000.0,
        AtePreset::DacFilterRolloff => 22_000.0,
        AtePreset::PowerTransformerSaturation => 38_000.0,
        _ => 55_000.0,
    };
    if sample_rate as f32 > cutoff * 2.0 {
        oversampling::apply_lowpass(samples, sample_rate, cutoff);
    }
}

fn deinterleave(input: &[f32], channel: usize, frames: usize) -> Vec<f32> {
    input
        .par_chunks_exact(2)
        .take(frames)
        .map(|frame| frame[channel])
        .collect()
}

fn interleave(left: &[f32], right: &[f32], output: &mut [f32]) {
    let frames = left.len().min(right.len()).min(output.len() / 2);
    let front = &mut output[..frames * 2];
    front
        .par_chunks_exact_mut(2)
        .enumerate()
        .for_each(|(frame, pair)| {
            pair[0] = left[frame];
            pair[1] = right[frame];
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_ate_is_identity() {
        let input = [0.1f32, -0.2, 0.3, -0.4];
        let mut output = [0.0f32; 4];
        let config = AteConfig {
            enable: false,
            ..AteConfig::default()
        };
        process_ate(&input, &mut output, &config, 44_100, 44_100, None);
        assert_eq!(output, input);
    }

    #[test]
    fn tube_ate_produces_audio() {
        let input = vec![0.25f32; 8192];
        let mut output = vec![0.0f32; input.len()];
        let config = presets::make_config(AtePreset::Tube, OversamplingMode::X4, 0.5, 1, false);
        process_ate(&input, &mut output, &config, 44_100, 44_100, None);
        assert!(output.iter().any(|&sample| sample.abs() > 1.0e-6));
    }

    #[test]
    fn tube_ate_keeps_fundamental() {
        let input: Vec<f32> = (0..4410)
            .flat_map(|i| {
                let t = i as f32 / 44_100.0;
                let v = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.5;
                [v, v]
            })
            .collect();
        let mut output = vec![0.0f32; input.len()];
        let config = presets::make_config(AtePreset::Tube, OversamplingMode::X4, 0.5, 42, false);
        process_ate(&input, &mut output, &config, 44_100, 44_100, None);
        let left: Vec<f32> = output.iter().step_by(2).copied().collect();
        let fund = analyzer::goertzel(&left, 1000.0, 44_100);
        assert!(fund > 0.1, "fund={fund}");
    }

    #[test]
    fn ate_stream_produces_audio() {
        let input = vec![0.1f32; 8192 * 2];
        let config = presets::make_config(AtePreset::Tube, OversamplingMode::X4, 0.5, 1, false);
        let mut stream = AteStream::new(&config, 44_100).unwrap();
        let output = stream.process_stereo_block(&input);
        assert_eq!(output.len(), input.len());
        assert!(output.iter().any(|&sample| sample.is_finite() && sample.abs() > 1.0e-6));
    }
}
