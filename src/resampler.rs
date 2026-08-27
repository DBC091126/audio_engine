use std::slice;

use libc::{c_int, c_long, c_void};
use libsamplerate_sys::{
    src_delete, src_error, src_new, src_process, src_set_ratio, src_strerror, SRC_DATA,
    SRC_SINC_BEST_QUALITY, SRC_STATE,
};

const ALLOWED_RATIOS: [f64; 7] = [0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0];

/// Check that both rates belong to the same family and the ratio is one of
/// 1/8x, 1/4x, 1/2x, 1x, 2x, 4x or 8x.
pub fn validate_rate_family(src_rate: u32, tgt_rate: u32) -> Result<(), String> {
    let src_is_44k = src_rate % 44100 == 0;
    let src_is_48k = src_rate % 48000 == 0;

    if !src_is_44k && !src_is_48k {
        return Err(format!("源采样率 {} Hz 不属于 44.1k 或 48k 家族", src_rate));
    }

    let tgt_is_44k = tgt_rate % 44100 == 0;
    let tgt_is_48k = tgt_rate % 48000 == 0;

    if src_is_44k && !tgt_is_44k {
        return Err(format!(
            "源采样率 {} Hz (44.1k 家族) 只能升/降频到 88.2/176.4/352.8 kHz，目标 {} Hz 不属于该家族",
            src_rate, tgt_rate
        ));
    }

    if src_is_48k && !tgt_is_48k {
        return Err(format!(
            "源采样率 {} Hz (48k 家族) 只能升/降频到 96/192/384 kHz，目标 {} Hz 不属于该家族",
            src_rate, tgt_rate
        ));
    }

    let ratio = tgt_rate as f64 / src_rate as f64;
    if !ALLOWED_RATIOS
        .iter()
        .any(|&allowed| (ratio - allowed).abs() < 0.001)
    {
        return Err(format!(
            "目标采样率 {} Hz 与源采样率 {} Hz 的比值 {:.2}x 不是允许的整数倍（仅支持 1/8x, 1/4x, 1/2x, 1x, 2x, 4x, 8x）",
            tgt_rate, src_rate, ratio
        ));
    }

    Ok(())
}

/// Return the family-compatible target rates used by the GUI.
pub fn get_recommended_rates(input_rate: u32) -> Vec<u32> {
    if input_rate % 44100 == 0 {
        vec![88200, 176400, 352800]
    } else if input_rate % 48000 == 0 {
        vec![96000, 192000, 384000]
    } else {
        Vec::new()
    }
}

/// Owning wrapper around a libsamplerate state machine.
#[derive(Debug)]
pub struct Resampler {
    state: *mut SRC_STATE,
    channels: c_int,
    ratio: f64,
}

impl Resampler {
    pub fn new(src_rate: u32, tgt_rate: u32, channels: i32) -> Result<Self, String> {
        validate_rate_family(src_rate, tgt_rate)?;
        if channels <= 0 {
            return Err(format!("声道数必须大于 0，当前为 {channels}"));
        }

        let ratio = tgt_rate as f64 / src_rate as f64;
        let mut error: c_int = 0;
        let state = unsafe { src_new(SRC_SINC_BEST_QUALITY as c_int, channels, &mut error) };
        if state.is_null() {
            let message = unsafe { src_strerror(error) };
            let message = if message.is_null() {
                "unknown libsamplerate error".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(message) }
                    .to_string_lossy()
                    .into_owned()
            };
            return Err(format!("src_new 创建失败: {message}"));
        }

        Ok(Self {
            state,
            channels,
            ratio,
        })
    }

    pub fn channels(&self) -> i32 {
        self.channels
    }

    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    pub fn process(
        &mut self,
        input: &[f32],
        output: &mut [f32],
        input_frames: usize,
        output_frames: usize,
        end_of_input: i32,
    ) -> Result<(c_long, c_long), String> {
        let channels = self.channels as usize;
        let input_len = input_frames
            .checked_mul(channels)
            .ok_or_else(|| "输入帧数溢出".to_string())?;
        if input.len() < input_len {
            return Err(format!(
                "输入缓冲区不足：需要 {input_len} 个 Float32，实际 {} 个",
                input.len()
            ));
        }

        let output_len = output_frames
            .checked_mul(channels)
            .ok_or_else(|| "输出帧数溢出".to_string())?;
        if output.len() < output_len {
            return Err(format!(
                "输出缓冲区不足：需要 {output_len} 个 Float32，实际 {} 个",
                output.len()
            ));
        }

        let mut data = SRC_DATA {
            data_in: input.as_ptr(),
            data_out: output.as_mut_ptr(),
            input_frames: input_frames as c_long,
            output_frames: output_frames as c_long,
            input_frames_used: 0,
            output_frames_gen: 0,
            end_of_input,
            src_ratio: self.ratio,
        };

        let error = unsafe { src_process(self.state, &mut data) };
        if error != 0 {
            let message = unsafe { src_strerror(error) };
            let message = if message.is_null() {
                "unknown libsamplerate error".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(message) }
                    .to_string_lossy()
                    .into_owned()
            };
            return Err(format!("src_process 失败: {message}"));
        }

        Ok((data.input_frames_used, data.output_frames_gen))
    }

    pub fn set_ratio(&mut self, ratio: f64) -> Result<(), String> {
        let error = unsafe { src_set_ratio(self.state, ratio) };
        if error != 0 {
            let message = unsafe { src_strerror(error) };
            let message = if message.is_null() {
                "unknown libsamplerate error".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(message) }
                    .to_string_lossy()
                    .into_owned()
            };
            return Err(format!("src_set_ratio 失败: {message}"));
        }
        self.ratio = ratio;
        Ok(())
    }
}

impl Drop for Resampler {
    fn drop(&mut self) {
        unsafe {
            src_delete(self.state);
        }
    }
}

/// Create a libsamplerate state machine after family validation.
///
/// Returns a null pointer when validation or creation fails.
#[unsafe(no_mangle)]
pub extern "C" fn create_src(src_rate: c_int, tgt_rate: c_int, channels: c_int) -> *mut c_void {
    if src_rate <= 0 || tgt_rate <= 0 || channels <= 0 {
        eprintln!("[ERROR] 采样率与声道数必须大于 0");
        return std::ptr::null_mut();
    }

    match Resampler::new(src_rate as u32, tgt_rate as u32, channels) {
        Ok(resampler) => Box::into_raw(Box::new(resampler)) as *mut c_void,
        Err(message) => {
            eprintln!("[ERROR] {message}");
            std::ptr::null_mut()
        }
    }
}

/// Run one libsamplerate processing pass.
///
/// Returns 0 on success, a libsamplerate error code on processing failure, or
/// -1 when arguments are invalid.
#[unsafe(no_mangle)]
pub extern "C" fn process_src(
    state: *mut c_void,
    input: *const f32,
    output: *mut f32,
    input_frames: c_long,
    output_frames: c_long,
    end_of_input: c_int,
    input_frames_used: *mut c_long,
    output_frames_gen: *mut c_long,
) -> c_int {
    if state.is_null()
        || input.is_null()
        || output.is_null()
        || input_frames_used.is_null()
        || output_frames_gen.is_null()
    {
        return -1;
    }
    if input_frames < 0 || output_frames < 0 {
        return -1;
    }

    let resampler = unsafe { &mut *(state as *mut Resampler) };
    let channels = resampler.channels() as usize;
    let input_frames = input_frames as usize;
    let output_frames = output_frames as usize;

    let input_len = match input_frames.checked_mul(channels) {
        Some(len) => len,
        None => return -1,
    };
    let output_len = match output_frames.checked_mul(channels) {
        Some(len) => len,
        None => return -1,
    };

    let input_slice = unsafe { slice::from_raw_parts(input, input_len) };
    let output_slice = unsafe { slice::from_raw_parts_mut(output, output_len) };

    match resampler.process(
        input_slice,
        output_slice,
        input_frames,
        output_frames,
        end_of_input,
    ) {
        Ok((used, generated)) => {
            unsafe {
                *input_frames_used = used;
                *output_frames_gen = generated;
            }
            0
        }
        Err(message) => {
            eprintln!("[ERROR] {message}");
            -2
        }
    }
}

/// Destroy a state machine created by `create_src`.
#[unsafe(no_mangle)]
pub extern "C" fn destroy_src(state: *mut c_void) {
    if !state.is_null() {
        unsafe {
            drop(Box::from_raw(state as *mut Resampler));
        }
    }
}

/// Change the conversion ratio of an existing state machine.
#[unsafe(no_mangle)]
pub extern "C" fn set_src_ratio(state: *mut c_void, ratio: f64) -> c_int {
    if state.is_null() {
        return -1;
    }
    let resampler = unsafe { &mut *(state as *mut Resampler) };
    match resampler.set_ratio(ratio) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("[ERROR] {message}");
            -2
        }
    }
}

/// Expose the libsamplerate state error code for diagnostics.
#[unsafe(no_mangle)]
pub extern "C" fn get_src_error(state: *mut c_void) -> c_int {
    if state.is_null() {
        return -1;
    }
    let resampler = unsafe { &*(state as *const Resampler) };
    unsafe { src_error(resampler.state) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_validation_cases() {
        assert!(validate_rate_family(44_100, 88_200).is_ok());
        assert!(validate_rate_family(44_100, 96_000).is_err());
        assert!(validate_rate_family(48_000, 192_000).is_ok());
        assert!(validate_rate_family(48_000, 176_400).is_err());
        assert!(validate_rate_family(44_100, 352_800).is_ok());
        assert!(validate_rate_family(44_100, 705_600).is_err());
    }

    #[test]
    fn recommended_rates_follow_family() {
        assert_eq!(
            get_recommended_rates(44_100),
            vec![88_200, 176_400, 352_800]
        );
        assert_eq!(
            get_recommended_rates(48_000),
            vec![96_000, 192_000, 384_000]
        );
        assert!(get_recommended_rates(22_050).is_empty());
    }
}
