use rayon::prelude::*;

use crate::dsd::DsdStream;

/// DSD output mode. The actual DSD bit rate is derived from the PCM family:
/// 44.1k family uses 2.8224/5.6448/11.2896 MHz and 48k family uses
/// 3.072/6.144/12.288 MHz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsdMode {
    DSD64,
    DSD128,
    DSD256,
}

impl DsdMode {
    fn multiplier(self) -> u32 {
        match self {
            DsdMode::DSD64 => 64,
            DsdMode::DSD128 => 128,
            DsdMode::DSD256 => 256,
        }
    }

    fn cutoff_hz(self) -> f32 {
        match self {
            DsdMode::DSD64 => 30_000.0,
            DsdMode::DSD128 => 40_000.0,
            DsdMode::DSD256 => 50_000.0,
        }
    }

    fn dsd_rate_for_base(self, base_rate: u32) -> u32 {
        base_rate * self.multiplier()
    }
}

/// Convert interleaved Float32 PCM to packed DSD.
///
/// The DSD family is taken from `pcm_rate`, so 44.1k-family PCM always outputs
/// 44.1k-family DSD and 48k-family PCM always outputs 48k-family DSD.
pub fn pcm_to_dsd(
    pcm: &[f32],
    pcm_rate: u32,
    channels: u16,
    dsd_target: DsdMode,
) -> Result<DsdStream, String> {
    let family_base = pcm_family_base(pcm_rate)?;
    pcm_to_dsd_with_family(pcm, pcm_rate, channels, dsd_target, family_base)
}

/// Convert PCM to DSD while explicitly requiring `dsd_family_base` to match
/// the PCM family. This is the strict family firewall used by GUI/config code
/// that might otherwise pair a 44.1k PCM rate with a 48k DSD rate.
pub fn pcm_to_dsd_with_family(
    pcm: &[f32],
    pcm_rate: u32,
    channels: u16,
    dsd_target: DsdMode,
    dsd_family_base: u32,
) -> Result<DsdStream, String> {
    if !matches!(dsd_family_base, 44_100 | 48_000) {
        return Err(format!(
            "DSD 家族基准采样率 {dsd_family_base} Hz 不合法，只允许 44100 或 48000"
        ));
    }

    let pcm_family = pcm_family_base(pcm_rate)?;
    if pcm_family != dsd_family_base {
        return Err(format!(
            "PCM {} Hz 属于 {} Hz 家族，不能输出 {} Hz 家族的 DSD（基频 {} Hz）",
            pcm_rate,
            pcm_family,
            dsd_family_base,
            dsd_target.dsd_rate_for_base(dsd_family_base)
        ));
    }

    validate_pcm_input(pcm, pcm_rate, channels)?;

    let pcm_multiplier = pcm_rate / pcm_family;
    if !matches!(pcm_multiplier, 2 | 4 | 8) {
        return Err(format!(
            "PCM 采样率 {pcm_rate} Hz 必须是基础频率的 2x/4x/8x（当前为 {pcm_multiplier}x）"
        ));
    }

    let dsd_rate = dsd_target.dsd_rate_for_base(dsd_family_base);
    if dsd_rate % pcm_rate != 0 {
        return Err(format!(
            "DSD 目标频率 {dsd_rate} Hz 不是 PCM {pcm_rate} Hz 的整数倍"
        ));
    }

    let frames = pcm.len() / usize::from(channels);
    let cutoff = dsd_target.cutoff_hz();
    let channel_bytes = (0..usize::from(channels))
        .into_par_iter()
        .map(|channel| {
            let mut channel_pcm = Vec::with_capacity(frames);
            for frame in 0..frames {
                channel_pcm.push(pcm[frame * usize::from(channels) + channel]);
            }
            modulate_channel(&channel_pcm, pcm_rate, dsd_rate, cutoff)
        })
        .collect::<Result<Vec<Vec<u8>>, String>>()?;

    let bytes_per_channel = channel_bytes
        .first()
        .map(Vec::len)
        .ok_or_else(|| "PCM 输入为空".to_string())?;
    let mut data = Vec::with_capacity(bytes_per_channel * usize::from(channels));
    for byte_index in 0..bytes_per_channel {
        for bytes in &channel_bytes {
            data.push(bytes[byte_index]);
        }
    }

    Ok(DsdStream {
        data,
        sample_rate: dsd_rate,
        channels,
    })
}

fn validate_pcm_input(pcm: &[f32], pcm_rate: u32, channels: u16) -> Result<(), String> {
    if pcm.is_empty() {
        return Err("PCM 输入为空".to_string());
    }
    if pcm_rate == 0 {
        return Err("PCM 采样率必须大于 0".to_string());
    }
    if channels == 0 {
        return Err("声道数必须大于 0".to_string());
    }
    if pcm.len() % usize::from(channels) != 0 {
        return Err(format!(
            "PCM 样本数 {} 不是声道数 {} 的整数倍",
            pcm.len(),
            channels
        ));
    }
    Ok(())
}

fn pcm_family_base(pcm_rate: u32) -> Result<u32, String> {
    if pcm_rate == 0 {
        return Err("PCM 采样率必须大于 0".to_string());
    }

    if pcm_rate % 44_100 == 0 {
        Ok(44_100)
    } else if pcm_rate % 48_000 == 0 {
        Ok(48_000)
    } else {
        Err(format!("PCM 采样率 {pcm_rate} Hz 不属于 44.1k 或 48k 家族"))
    }
}

fn modulate_channel(
    samples: &[f32],
    pcm_rate: u32,
    dsd_rate: u32,
    cutoff_hz: f32,
) -> Result<Vec<u8>, String> {
    let working_rate = internal_src_rate(pcm_rate, dsd_rate);
    let working_ratio = (dsd_rate / working_rate) as usize;
    let mut working = if working_rate == pcm_rate * 2 {
        zero_fill_2x(samples)
    } else {
        samples.to_vec()
    };

    let mut filter = ButterLowpass::new(working_rate, cutoff_hz);
    filter.process(&mut working);

    let total_samples = working
        .len()
        .checked_mul(working_ratio)
        .ok_or_else(|| "DSD 过采样样本数溢出".to_string())?;
    if total_samples % 8 != 0 {
        return Err(format!(
            "过采样样本数 {} 不是 8 的整数倍，无法打包 1-bit 字节流",
            total_samples
        ));
    }

    let mut packer = DsdPacker::with_capacity(total_samples / 8);
    for &sample in &working {
        packer.push(sample);
        for _ in 0..working_ratio - 1 {
            packer.push(0.0);
        }
    }
    Ok(packer.finish())
}

fn internal_src_rate(pcm_rate: u32, dsd_rate: u32) -> u32 {
    if dsd_rate / pcm_rate == 64 && (pcm_rate == 176_400 || pcm_rate == 192_000) {
        pcm_rate * 2
    } else {
        pcm_rate
    }
}

fn zero_fill_2x(input: &[f32]) -> Vec<f32> {
    let mut out = Vec::with_capacity(input.len() * 2);
    for &sample in input {
        out.push(sample);
        out.push(0.0);
    }
    out
}

struct DsdPacker {
    error: [f32; 5],
    bytes: Vec<u8>,
    byte: u8,
    bits: u8,
}

impl DsdPacker {
    fn with_capacity(bytes: usize) -> Self {
        Self {
            error: [0.0; 5],
            bytes: Vec::with_capacity(bytes),
            byte: 0,
            bits: 0,
        }
    }

    fn push(&mut self, sample: f32) {
        let mut y = sample;
        y += self.error[0] * 0.5
            + self.error[1] * 0.25
            + self.error[2] * 0.125
            + self.error[3] * 0.0625
            + self.error[4] * 0.03125;
        let output = if y > 0.0 { 1.0 } else { -1.0 };
        let new_error = y - output;

        self.error[4] = self.error[3];
        self.error[3] = self.error[2];
        self.error[2] = self.error[1];
        self.error[1] = self.error[0];
        self.error[0] = new_error;

        if output > 0.0 {
            self.byte |= 1 << (7 - self.bits);
        }
        self.bits += 1;
        if self.bits == 8 {
            self.bytes.push(self.byte);
            self.byte = 0;
            self.bits = 0;
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

struct ButterLowpass {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl ButterLowpass {
    fn new(sample_rate: u32, cutoff_hz: f32) -> Self {
        let fs = sample_rate as f32;
        let nyquist = fs * 0.5;
        let cutoff = cutoff_hz.min(nyquist * 0.95).max(1.0);
        let k = (std::f32::consts::PI * cutoff / fs).tan();
        let k2 = k * k;
        let sqrt2 = std::f32::consts::SQRT_2;
        let a0 = 1.0 + sqrt2 * k + k2;

        Self {
            b0: k2 / a0,
            b1: 2.0 * k2 / a0,
            b2: k2 / a0,
            a1: 2.0 * (k2 - 1.0) / a0,
            a2: (1.0 - sqrt2 * k + k2) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, samples: &mut [f32]) {
        for sample in samples {
            let x = *sample;
            let y = self.b0 * x + self.z1;
            self.z1 = self.b1 * x - self.a1 * y + self.z2;
            self.z2 = self.b2 * x - self.a2 * y;
            *sample = y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_pcm(frames: usize, channels: u16) -> Vec<f32> {
        let mut pcm = Vec::with_capacity(frames * usize::from(channels));
        for frame in 0..frames {
            let value = (frame as f32 * 0.001).sin() * 0.5;
            for _ in 0..channels {
                pcm.push(value);
            }
        }
        pcm
    }

    #[test]
    fn family_mismatches_are_rejected() {
        let pcm = small_pcm(64, 1);
        assert!(pcm_to_dsd_with_family(&pcm, 352_800, 1, DsdMode::DSD128, 48_000).is_err());
        assert!(pcm_to_dsd_with_family(&pcm, 384_000, 1, DsdMode::DSD128, 44_100).is_err());
    }

    #[test]
    fn modulator_produces_packed_stereo_dsd() {
        let pcm = small_pcm(128, 2);
        let stream = pcm_to_dsd(&pcm, 352_800, 2, DsdMode::DSD64).unwrap();
        assert_eq!(stream.sample_rate, 2_822_400);
        assert_eq!(stream.channels, 2);
        assert_eq!(stream.data.len(), (128 * 8 / 8) * 2);
        assert!(!stream.data.is_empty());
    }
}
