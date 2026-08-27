use super::config::{JitterParams, JitterType};
use super::noise::Lcg;

#[derive(Debug, Clone)]
pub struct JitterSimulator {
    rng: Lcg,
    correlated_state: f32,
}

impl JitterSimulator {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Lcg::new(seed),
            correlated_state: 0.0,
        }
    }

    pub fn apply_jitter(
        &mut self,
        samples: &[f32],
        params: &JitterParams,
        sample_rate: f32,
    ) -> Vec<f32> {
        if !params.enabled {
            return samples.to_vec();
        }

        let sigma_samples = params.rms_ps * 1.0e-12 * sample_rate;
        let periodic_amp = super::noise::db_to_amp(params.periodic_db);
        let correlated_amp = super::noise::db_to_amp(params.correlated_db);
        let mut out = vec![0.0f32; samples.len()];

        for i in 0..samples.len() {
            let white = self.rng.gaussian();
            self.correlated_state += 0.05 * white;
            self.correlated_state *= 0.95;
            let t = i as f32 / sample_rate;
            let periodic = (2.0 * std::f32::consts::PI * params.periodic_hz * t).sin();
            let offset = match params.jitter_type {
                JitterType::White => white,
                JitterType::Periodic => periodic,
                JitterType::Correlated => self.correlated_state,
                JitterType::Mixed => {
                    white * 0.6
                        + periodic * periodic_amp * 1000.0
                        + self.correlated_state * correlated_amp * 1000.0
                }
            };
            let offset = (offset * sigma_samples).clamp(-0.5, 0.5);
            out[i] = cubic_interpolate(samples, i as f32 + offset);
        }
        out
    }
}

fn cubic_interpolate(samples: &[f32], position: f32) -> f32 {
    let index = position.floor() as isize;
    let frac = position - index as f32;
    let x0 = at(samples, index - 1);
    let x1 = at(samples, index);
    let x2 = at(samples, index + 1);
    let x3 = at(samples, index + 2);

    let c0 = x1;
    let c1 = 0.5 * (x2 - x0);
    let c2 = x0 - 2.5 * x1 + 2.0 * x2 - 0.5 * x3;
    let c3 = 0.5 * (x3 - x0) + 1.5 * (x1 - x2);
    ((c3 * frac + c2) * frac + c1) * frac + c0
}

fn at(samples: &[f32], index: isize) -> f32 {
    if index < 0 || index >= samples.len() as isize {
        0.0
    } else {
        samples[index as usize]
    }
}
