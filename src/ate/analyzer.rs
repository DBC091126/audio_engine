#[derive(Debug, Clone, Copy, Default)]
pub struct AnalyzerResult {
    pub thd_percent: f32,
    pub thdn_percent: f32,
    pub h2_db: f32,
    pub h3_db: f32,
    pub h4_db: f32,
    pub h5_db: f32,
    pub imd_db: f32,
    pub noise_floor_db: f32,
    pub fingerprint: [f32; 8],
}

pub struct AteAnalyzer;

impl AteAnalyzer {
    pub fn analyze_harmonics(
        samples: &[f32],
        sample_rate: u32,
        fundamental_hz: f32,
    ) -> AnalyzerResult {
        let fundamental = goertzel(samples, fundamental_hz, sample_rate);
        let h2 = goertzel(samples, fundamental_hz * 2.0, sample_rate);
        let h3 = goertzel(samples, fundamental_hz * 3.0, sample_rate);
        let h4 = goertzel(samples, fundamental_hz * 4.0, sample_rate);
        let h5 = goertzel(samples, fundamental_hz * 5.0, sample_rate);
        let noise_floor_db = estimate_noise_floor_db(samples, sample_rate, fundamental_hz);

        let harmonic_rms = (h2 * h2 + h3 * h3 + h4 * h4 + h5 * h5).sqrt();
        let thd_percent = if fundamental.abs() > 1.0e-12 {
            harmonic_rms / fundamental * 100.0
        } else {
            0.0
        };
        let thdn_percent = (thd_percent.powi(2)
            + super::noise::db_to_amp(noise_floor_db).powi(2) * 10_000.0)
            .sqrt();

        let fingerprint = [
            db_from_amp(fundamental),
            db_from_amp(h2),
            db_from_amp(h3),
            db_from_amp(h4),
            db_from_amp(h5),
            noise_floor_db,
            0.0,
            0.0,
        ];

        AnalyzerResult {
            thd_percent,
            thdn_percent,
            h2_db: db_from_amp(h2),
            h3_db: db_from_amp(h3),
            h4_db: db_from_amp(h4),
            h5_db: db_from_amp(h5),
            imd_db: 0.0,
            noise_floor_db,
            fingerprint,
        }
    }

    pub fn analyze_imd(samples: &[f32], sample_rate: u32, f1: f32, f2: f32) -> f32 {
        let diff = goertzel(samples, (f2 - f1).abs(), sample_rate);
        let sum = goertzel(samples, f1 + f2, sample_rate);
        let reference = goertzel(samples, f1, sample_rate).max(goertzel(samples, f2, sample_rate));
        if reference.abs() < 1.0e-12 {
            -200.0
        } else {
            db_from_amp((diff + sum) / reference)
        }
    }
}

pub fn goertzel(samples: &[f32], freq_hz: f32, sample_rate: u32) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let w = 2.0 * std::f32::consts::PI * freq_hz / sample_rate as f32;
    let coeff = 2.0 * w.cos();
    let mut s_prev = 0.0f32;
    let mut s_prev2 = 0.0f32;
    for &sample in samples {
        let s = sample + coeff * s_prev - s_prev2;
        s_prev2 = s_prev;
        s_prev = s;
    }
    let power = s_prev2 * s_prev2 + s_prev * s_prev - coeff * s_prev * s_prev2;
    2.0 * power.sqrt() / samples.len() as f32
}

fn estimate_noise_floor_db(samples: &[f32], sample_rate: u32, fundamental_hz: f32) -> f32 {
    let mut sum_sq = 0.0f32;
    let mut count = 0usize;
    for (i, &sample) in samples.iter().enumerate() {
        let freq = i as f32 / samples.len() as f32 * sample_rate as f32;
        let near = (freq - fundamental_hz).abs() < fundamental_hz * 1.5
            || (freq - 2.0 * fundamental_hz).abs() < fundamental_hz
            || (freq - 3.0 * fundamental_hz).abs() < fundamental_hz
            || (freq - 4.0 * fundamental_hz).abs() < fundamental_hz
            || (freq - 5.0 * fundamental_hz).abs() < fundamental_hz;
        if !near {
            sum_sq += sample * sample;
            count += 1;
        }
    }
    let rms = if count == 0 {
        0.0
    } else {
        (sum_sq / count as f32).sqrt()
    };
    db_from_amp(rms)
}

pub fn db_from_amp(amp: f32) -> f32 {
    if amp <= 1.0e-12 {
        -200.0
    } else {
        20.0 * amp.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goertzel_measures_sine_amplitude() {
        let sample_rate = 44_100u32;
        let samples: Vec<f32> = (0..4410)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * 1000.0 * t).sin()
            })
            .collect();
        let amp = goertzel(&samples, 1000.0, sample_rate);
        assert!((amp - 1.0).abs() < 0.05, "amp={amp}");
    }
}
