use rayon::prelude::*;

pub fn upsample_channel(input: &[f32], factor: usize, _sample_rate: u32) -> Vec<f32> {
    if factor <= 1 {
        return input.to_vec();
    }

    let mut zero_stuffed = Vec::with_capacity(input.len() * factor);
    for &sample in input {
        zero_stuffed.push(sample);
        zero_stuffed.resize(zero_stuffed.len() + factor - 1, 0.0);
    }

    let cutoff = 0.45 / factor as f32;
    let fir = make_lowpass_fir(65, cutoff, factor as f32);
    fir_filter(&zero_stuffed, &fir)
}

pub fn downsample_channel(input: &[f32], factor: usize, _sample_rate: u32) -> Vec<f32> {
    if factor <= 1 {
        return input.to_vec();
    }

    let cutoff = 0.45 / factor as f32;
    let fir = make_lowpass_fir(65, cutoff, 1.0);
    let filtered = fir_filter(input, &fir);
    filtered
        .iter()
        .skip(factor / 2)
        .step_by(factor)
        .copied()
        .collect()
}

pub fn make_lowpass_fir(taps: usize, cutoff: f32, gain: f32) -> Vec<f32> {
    let mut coeffs = Vec::with_capacity(taps);
    let center = (taps as f32 - 1.0) * 0.5;
    let mut sum = 0.0f32;

    for i in 0..taps {
        let x = i as f32 - center;
        let sinc = if x.abs() < 1.0e-6 {
            2.0 * cutoff
        } else {
            (2.0 * std::f32::consts::PI * cutoff * x).sin() / (std::f32::consts::PI * x)
        };
        let t = i as f32 / (taps as f32 - 1.0);
        let window = 0.42 - 0.5 * (2.0 * std::f32::consts::PI * t).cos()
            + 0.08 * (4.0 * std::f32::consts::PI * t).cos();
        let value = sinc * window;
        coeffs.push(value);
        sum += value;
    }

    if sum.abs() > 1.0e-12 {
        for coeff in &mut coeffs {
            *coeff = *coeff * gain / sum;
        }
    }
    coeffs
}

pub fn fir_filter(input: &[f32], coeffs: &[f32]) -> Vec<f32> {
    let half = coeffs.len() as isize / 2;
    input
        .par_iter()
        .enumerate()
        .map(|(i, _)| {
            let mut sum = 0.0f32;
            for (j, &coeff) in coeffs.iter().enumerate() {
                let index = i as isize + j as isize - half;
                let value = if index < 0 || index >= input.len() as isize {
                    0.0
                } else {
                    input[index as usize]
                };
                sum += coeff * value;
            }
            sum
        })
        .collect()
}

pub fn apply_dc_block(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    let alpha = (2.0 * std::f32::consts::PI * 2.0 / sample_rate as f32).min(0.999);
    let feedback = 1.0 - alpha;
    let mut previous = samples.first().copied().unwrap_or(0.0);
    let mut y_previous = 0.0f32;
    let mut out = Vec::with_capacity(samples.len());

    for &x in samples {
        let y = x - previous + feedback * y_previous;
        previous = x;
        y_previous = y;
        out.push(denormal_protect(y));
    }
    out
}

pub fn apply_lowpass(samples: &[f32], sample_rate: u32, cutoff_hz: f32) -> Vec<f32> {
    let alpha = 1.0 - (-2.0 * std::f32::consts::PI * cutoff_hz / sample_rate as f32).exp();
    let mut y = samples.first().copied().unwrap_or(0.0);
    let mut out = Vec::with_capacity(samples.len());
    for &x in samples {
        y += alpha * (x - y);
        out.push(denormal_protect(y));
    }
    out
}

pub fn denormal_protect(x: f32) -> f32 {
    if x.abs() < 1.0e-30 {
        0.0
    } else {
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ate::analyzer::goertzel;

    #[test]
    fn round_trip_preserves_1k_amplitude() {
        let sample_rate = 44_100u32;
        let input: Vec<f32> = (0..4410)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * 1000.0 * t).sin()
            })
            .collect();
        let up = upsample_channel(&input, 4, sample_rate);
        let down = downsample_channel(&up, 4, sample_rate);
        let amp = goertzel(&down, 1000.0, sample_rate);
        assert!((amp - 1.0).abs() < 0.1, "amp={amp}");
    }
}
