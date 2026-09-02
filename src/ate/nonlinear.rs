use super::config::CrossoverParams;
use super::state_engine::AteState;

pub fn polynomial(x: f32, coeffs: &[f32; 6]) -> f32 {
    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x3 * x;
    let x5 = x4 * x;
    coeffs[0] + coeffs[1] * x + coeffs[2] * x2 + coeffs[3] * x3 + coeffs[4] * x4 + coeffs[5] * x5
}

pub fn harmonics_to_poly(h2: f32, h3: f32, h4: f32, h5: f32) -> [f32; 6] {
    let a5 = 16.0 * h5;
    let a4 = 8.0 * h4;
    let a2 = 2.0 * h2 - 8.0 * h4;
    let a3 = 4.0 * (h3 - 5.0 * h5);
    let a1 = 1.0 - 3.0 * a3 / 4.0 - 5.0 * a5 / 8.0;
    let a0 = -(a2 / 2.0 + 3.0 * a4 / 8.0);
    [a0, a1, a2, a3, a4, a5]
}

pub fn apply_crossover(x: f32, params: &CrossoverParams) -> f32 {
    if !params.enabled {
        return x;
    }

    let inner = if x < 0.0 {
        params.negative_inner_gain
    } else {
        params.inner_gain
    };
    let theta = if x < 0.0 {
        params.negative_theta
    } else {
        params.theta
    };
    let g = inner + (1.0 - inner) * smoothstep(x.abs(), 0.0, theta);
    x * g
}

pub fn smoothstep(x: f32, edge0: f32, edge1: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn apply_memory_polynomial(
    x: f32,
    history: &[f32],
    coeffs: &[f32],
    k_max: usize,
    m_max: usize,
) -> f32 {
    if coeffs.is_empty() {
        return x;
    }

    let mut y = 0.0f32;
    for k in 1..=k_max {
        for m in 0..=m_max {
            let sample = if m == 0 {
                x
            } else {
                history.get(m - 1).copied().unwrap_or(0.0)
            };
            let index = (k - 1) * (m_max + 1) + m;
            let Some(&coeff) = coeffs.get(index) else {
                continue;
            };
            y += coeff * sample * sample.abs().powf((k - 1) as f32);
        }
    }
    y
}

pub fn apply_nonlinear(
    x: f32,
    coeffs: &[f32; 6],
    crossover: &CrossoverParams,
    state: &mut AteState,
    state_params: &super::config::StateParams,
    mem_poly: &[f32],
    intensity: f32,
) -> f32 {
    state.update(x, state_params);

    let bias_modulated = coeffs[0] + state.bias * 0.001 * intensity;
    let thermal_gain = 1.0 + (state.thermal - 0.25) * 0.004 * intensity;
    let flux_compression = 1.0 / (1.0 + state.flux.abs() * 0.02 * intensity);

    let mut poly = [
        bias_modulated,
        coeffs[1],
        coeffs[2],
        coeffs[3],
        coeffs[4],
        coeffs[5],
    ];
    poly[1] *= thermal_gain;

    let mem = apply_memory_polynomial(x, &[], mem_poly, 5, 2);
    let linear = x + (mem - x) * intensity.min(1.0);
    let shaped = polynomial(linear * flux_compression, &poly);
    let shaped = if intensity >= 1.0 {
        shaped
    } else {
        x + (shaped - x) * intensity
    };
    apply_crossover(shaped, crossover)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ate::analyzer::goertzel;
    use crate::ate::noise::db_to_amp;

    #[test]
    fn tube_poly_has_h2() {
        let coeffs = harmonics_to_poly(
            db_to_amp(-30.0),
            db_to_amp(-50.0),
            db_to_amp(-70.0),
            db_to_amp(-80.0),
        );
        let samples: Vec<f32> = (0..4410)
            .map(|i| {
                let t = i as f32 / 44_100.0;
                polynomial(
                    (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.5,
                    &coeffs,
                )
            })
            .collect();
        let h2 = goertzel(&samples, 2000.0, 44_100);
        assert!(h2 > 0.001, "h2={h2}");
    }

    #[test]
    fn tube_nonlinear_keeps_fundamental() {
        use crate::ate::state_engine::AteState;
        let coeffs = harmonics_to_poly(
            db_to_amp(-30.0),
            db_to_amp(-50.0),
            db_to_amp(-70.0),
            db_to_amp(-80.0),
        );
        let crossover = CrossoverParams::default();
        let state_params = crate::ate::config::StateParams::default();
        let mut state = AteState::new();
        let samples: Vec<f32> = (0..4410)
            .map(|i| {
                let t = i as f32 / 44_100.0;
                let x = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.5;
                apply_nonlinear(x, &coeffs, &crossover, &mut state, &state_params, &[], 0.5)
            })
            .collect();
        let fund = goertzel(&samples, 1000.0, 44_100);
        assert!(fund > 0.3, "fund={fund}");
    }
}
