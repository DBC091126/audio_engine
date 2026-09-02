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

#[derive(Debug, Clone, Copy)]
pub struct SpectrumPoint {
    pub freq_hz: f32,
    pub level_db: f32,
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

pub fn analyze_spectrum_mono(samples: &[f32], sample_rate: u32, points: usize) -> Vec<SpectrumPoint> {
    let points = points.max(8).min(256);
    if samples.is_empty() || sample_rate == 0 {
        return Vec::new();
    }

    let nyquist = sample_rate as f32 * 0.5;
    let max_hz = 20_000.0_f32.min(nyquist * 0.98).max(20.0);
    if max_hz <= 20.0 {
        return Vec::new();
    }

    let n = next_pow2(samples.len().max(4096));
    let mut re = vec![0.0f64; n];
    let mut im = vec![0.0f64; n];
    for (i, &sample) in samples.iter().enumerate().take(samples.len().min(n)) {
        let position = if n == 1 { 0.0 } else {
            std::f64::consts::TAU * i as f64 / (n - 1) as f64
        };
        re[i] = f64::from(sample) * 0.5 * (1.0 - position.cos());
    }
    fft(&mut re, &mut im);

    let centers = log_centers(20.0, max_hz, points);
    let mut levels = Vec::with_capacity(points);
    let scale = (n as f64).powi(2);
    let bin_width = sample_rate as f64 / n as f64;
    let usable = n / 2 + 1;

    for point in 0..points {
        let center = f64::from(centers[point]);
        let lower = if point == 0 {
            f64::from(20.0_f32.min(max_hz))
        } else {
            (f64::from(centers[point - 1]) * center).sqrt()
        };
        let upper = if point + 1 == points {
            f64::from(max_hz)
        } else {
            (center * f64::from(centers[point + 1])).sqrt()
        };

        let first_bin = (lower / bin_width).floor() as usize;
        let last_bin = (upper / bin_width).ceil() as usize;
        let mut sum = 0.0f64;
        let mut count = 0usize;
        for bin in first_bin..=last_bin.min(usable.saturating_sub(1)) {
            let freq = bin as f64 * bin_width;
            if freq < lower || freq >= upper {
                continue;
            }
            let power = (re[bin] * re[bin] + im[bin] * im[bin]) * 2.0 / scale;
            sum += power;
            count += 1;
        }
        let rms = if count == 0 {
            0.0
        } else {
            (sum / count as f64).sqrt()
        };
        levels.push(SpectrumPoint {
            freq_hz: centers[point],
            level_db: db_from_amp(rms.max(1.0e-14) as f32),
        });
    }
    levels
}

fn log_centers(min_hz: f32, max_hz: f32, points: usize) -> Vec<f32> {
    let min_log = min_hz.log10();
    let max_log = max_hz.log10();
    (0..points)
        .map(|i| {
            let t = i as f32 / (points - 1) as f32;
            10.0f32.powf(min_log + (max_log - min_log) * t)
        })
        .collect()
}

fn next_pow2(mut value: usize) -> usize {
    value = value.saturating_sub(1);
    value |= value >> 1;
    value |= value >> 2;
    value |= value >> 4;
    value |= value >> 8;
    value |= value >> 16;
    value |= value >> 32;
    value.saturating_add(1)
}

fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    if n < 2 || !n.is_power_of_two() {
        return;
    }

    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    let mut len = 2usize;
    while len <= n {
        let angle = -std::f64::consts::TAU / len as f64;
        let wlen_re = angle.cos();
        let wlen_im = angle.sin();
        let half = len / 2;
        for start in (0..n).step_by(len) {
            let mut w_re = 1.0f64;
            let mut w_im = 0.0f64;
            for offset in 0..half {
                let a = start + offset;
                let b = a + half;
                let u_re = re[a];
                let u_im = im[a];
                let v_re = re[b] * w_re - im[b] * w_im;
                let v_im = re[b] * w_im + im[b] * w_re;
                re[a] = u_re + v_re;
                im[a] = u_im + v_im;
                re[b] = u_re - v_re;
                im[b] = u_im - v_im;
                let next_re = w_re * wlen_re - w_im * wlen_im;
                let next_im = w_re * wlen_im + w_im * wlen_re;
                w_re = next_re;
                w_im = next_im;
            }
        }
        len <<= 1;
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

    #[test]
    fn spectrum_reports_low_and_high_band_power() {
        let sample_rate = 44_100u32;
        let samples: Vec<f32> = (0..sample_rate as usize)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
                    + (2.0 * std::f32::consts::PI * 9_000.0 * t).sin() * 0.3
            })
            .collect();
        let spectrum = analyze_spectrum_mono(&samples, sample_rate, 96);
        assert!(spectrum.len() == 96);
        let low = spectrum
            .iter()
            .find(|point| (point.freq_hz - 440.0).abs() < 120.0)
            .map(|point| point.level_db)
            .unwrap_or(-200.0);
        let high = spectrum
            .iter()
            .find(|point| (point.freq_hz - 9_000.0).abs() < 1_000.0)
            .map(|point| point.level_db)
            .unwrap_or(-200.0);
        assert!(low > -75.0, "low={low}");
        assert!(high > -95.0, "high={high}");
    }
}
