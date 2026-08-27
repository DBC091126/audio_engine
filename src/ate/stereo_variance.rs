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

    for i in 0..left.len() {
        let l = left[i] * l_gain + right[i] * crosstalk;
        let r = right[i] * r_gain + left[i] * crosstalk;
        left[i] = l;
        right[i] = r;
        if i % 2 == 1 {
            right[i] = right[i] * phase_rad.cos() - l * phase_rad.sin();
        }
    }
}
