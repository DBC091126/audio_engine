use rayon::prelude::*;

use super::config::ChannelMismatch;
use super::noise::Lcg;

pub fn apply_channel_variance(
    left: &mut [f32],
    right: &mut [f32],
    params: &ChannelMismatch,
    seed: u64,
) {
    let mut rng = Lcg::new(seed);
    let l_gain_db = params.gain_db_l + (rng.next_f32() - 0.5) * 0.002;
    let r_gain_db = params.gain_db_r + (rng.next_f32() - 0.5) * 0.002;
    let phase_rad = params.phase_deg.to_radians() + (rng.next_f32() - 0.5) * 0.02;
    let crosstalk = super::noise::db_to_amp(params.crosstalk_db);
    let l_gain = super::noise::db_to_amp(l_gain_db);
    let r_gain = super::noise::db_to_amp(r_gain_db);
    let cos_phase = phase_rad.cos();
    let sin_phase = phase_rad.sin();

    left.par_iter_mut()
        .enumerate()
        .zip(right.par_iter_mut())
        .for_each(|((i, left_sample), right_sample)| {
            let l = *left_sample * l_gain + *right_sample * crosstalk;
            let r = *right_sample * r_gain + *left_sample * crosstalk;
            *left_sample = l;
            *right_sample = r;
            if i % 2 == 1 {
                *right_sample = *right_sample * cos_phase - l * sin_phase;
            }
        });
}
