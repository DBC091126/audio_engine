use super::config::{
    AteConfig, AteCustomParams, AtePreset, ChannelMismatch, CrossoverParams, JitterParams,
    JitterType, NoiseParams, OversamplingMode, StateParams,
};
use super::noise::db_to_amp;
use super::nonlinear::harmonics_to_poly;

pub fn preset_params(preset: AtePreset) -> AteCustomParams {
    match preset {
        AtePreset::CleanAnalog => {
            let poly = harmonics_to_poly(
                db_to_amp(-50.0),
                db_to_amp(-70.0),
                db_to_amp(-85.0),
                db_to_amp(-95.0),
            );
            AteCustomParams {
                poly_a: poly,
                state_params: StateParams {
                    env_alpha: 0.0001,
                    ..StateParams::default()
                },
                noise_params: NoiseParams {
                    thermal_db: -120.0,
                    ..NoiseParams::default()
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::Tube => {
            let poly = harmonics_to_poly(
                db_to_amp(-30.0),
                db_to_amp(-50.0),
                db_to_amp(-70.0),
                db_to_amp(-80.0),
            );
            AteCustomParams {
                poly_a: poly,
                state_params: StateParams {
                    flux_alpha: 0.01,
                    flux_beta: 0.002,
                    bias_alpha: 0.0005,
                    ..StateParams::default()
                },
                noise_params: NoiseParams {
                    thermal_db: -110.0,
                    pink_db: -95.0,
                    hum_db: -85.0,
                    hum_hz: 50.0,
                    ..NoiseParams::default()
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::VintageSolidState => {
            let poly = harmonics_to_poly(
                db_to_amp(-60.0),
                db_to_amp(-35.0),
                db_to_amp(-75.0),
                db_to_amp(-55.0),
            );
            AteCustomParams {
                poly_a: poly,
                crossover: CrossoverParams {
                    enabled: true,
                    inner_gain: 0.04,
                    theta: 0.04,
                    negative_inner_gain: 0.03,
                    negative_theta: 0.05,
                },
                state_params: StateParams {
                    thermal_alpha: 0.001,
                    ..StateParams::default()
                },
                noise_params: NoiseParams {
                    thermal_db: -115.0,
                    ..NoiseParams::default()
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::VintageDac => {
            let poly = harmonics_to_poly(
                db_to_amp(-70.0),
                db_to_amp(-50.0),
                db_to_amp(-80.0),
                db_to_amp(-70.0),
            );
            AteCustomParams {
                poly_a: poly,
                jitter_params: JitterParams {
                    enabled: true,
                    rms_ps: 30.0,
                    periodic_hz: 1000.0,
                    periodic_db: -90.0,
                    correlated_db: -100.0,
                    jitter_type: JitterType::Mixed,
                },
                noise_params: NoiseParams {
                    thermal_db: -130.0,
                    pink_db: -120.0,
                    ..NoiseParams::default()
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::Vinyl => AteCustomParams {
            poly_a: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            noise_params: NoiseParams {
                thermal_db: -130.0,
                pink_db: -80.0,
                hum_db: -85.0,
                hum_hz: 60.0,
                vinyl_db: -75.0,
                crackle_db: -55.0,
                tape_db: -130.0,
            },
            ..AteCustomParams::default()
        },
        AtePreset::Tape => {
            let poly = harmonics_to_poly(
                db_to_amp(-40.0),
                db_to_amp(-40.0),
                db_to_amp(-75.0),
                db_to_amp(-75.0),
            );
            AteCustomParams {
                poly_a: poly,
                state_params: StateParams {
                    flux_alpha: 0.02,
                    flux_beta: 0.004,
                    recovery_alpha: 0.002,
                    recovery_decay: 0.999,
                    ..StateParams::default()
                },
                noise_params: NoiseParams {
                    thermal_db: -115.0,
                    tape_db: -90.0,
                    pink_db: -105.0,
                    ..NoiseParams::default()
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::SolidStateClassASingleEnded => {
            let poly = harmonics_to_poly(
                db_to_amp(-42.0),
                db_to_amp(-70.0),
                db_to_amp(-85.0),
                db_to_amp(-94.0),
            );
            AteCustomParams {
                poly_a: poly,
                state_params: StateParams {
                    env_alpha: 0.0006,
                    thermal_alpha: 0.0006,
                    ..StateParams::default()
                },
                noise_params: NoiseParams {
                    thermal_db: -126.0,
                    pink_db: -116.0,
                    ..NoiseParams::default()
                },
                channel_mismatch: ChannelMismatch {
                    gain_db_l: 0.01,
                    gain_db_r: -0.01,
                    phase_deg: 0.12,
                    thd_variance: 0.01,
                    crosstalk_db: -96.0,
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::SolidStateClassAPushPull => {
            let poly = harmonics_to_poly(
                db_to_amp(-56.0),
                db_to_amp(-72.0),
                db_to_amp(-96.0),
                db_to_amp(-102.0),
            );
            AteCustomParams {
                poly_a: poly,
                state_params: StateParams {
                    thermal_alpha: 0.0008,
                    ..StateParams::default()
                },
                noise_params: NoiseParams {
                    thermal_db: -132.0,
                    pink_db: -126.0,
                    ..NoiseParams::default()
                },
                channel_mismatch: ChannelMismatch {
                    gain_db_l: 0.005,
                    gain_db_r: -0.005,
                    phase_deg: 0.06,
                    thd_variance: 0.004,
                    crosstalk_db: -102.0,
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::SolidStateClassAb => {
            let poly = harmonics_to_poly(
                db_to_amp(-80.0),
                db_to_amp(-52.0),
                db_to_amp(-96.0),
                db_to_amp(-54.0),
            );
            AteCustomParams {
                poly_a: poly,
                crossover: CrossoverParams {
                    enabled: true,
                    inner_gain: 0.985,
                    theta: 0.006,
                    negative_inner_gain: 0.985,
                    negative_theta: 0.006,
                },
                state_params: StateParams {
                    thermal_alpha: 0.001,
                    ..StateParams::default()
                },
                noise_params: NoiseParams {
                    thermal_db: -128.0,
                    pink_db: -122.0,
                    ..NoiseParams::default()
                },
                channel_mismatch: ChannelMismatch {
                    gain_db_l: 0.008,
                    gain_db_r: -0.008,
                    phase_deg: 0.1,
                    thd_variance: 0.01,
                    crosstalk_db: -98.0,
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::SolidStateClassD => AteCustomParams {
            poly_a: harmonics_to_poly(
                db_to_amp(-110.0),
                db_to_amp(-115.0),
                db_to_amp(-125.0),
                db_to_amp(-130.0),
            ),
            noise_params: NoiseParams {
                thermal_db: -138.0,
                pink_db: -138.0,
                ..NoiseParams::default()
            },
            channel_mismatch: ChannelMismatch {
                gain_db_l: 0.002,
                gain_db_r: -0.002,
                phase_deg: 0.02,
                thd_variance: 0.002,
                crosstalk_db: -110.0,
            },
            ..AteCustomParams::default()
        },
        AtePreset::TubePushPull => {
            let poly = harmonics_to_poly(
                db_to_amp(-72.0),
                db_to_amp(-25.0),
                db_to_amp(-86.0),
                db_to_amp(-46.0),
            );
            AteCustomParams {
                poly_a: poly,
                crossover: CrossoverParams {
                    enabled: true,
                    inner_gain: 0.06,
                    theta: 0.01,
                    negative_inner_gain: 0.06,
                    negative_theta: 0.012,
                },
                state_params: StateParams {
                    flux_alpha: 0.008,
                    flux_beta: 0.0022,
                    bias_alpha: 0.0004,
                    ..StateParams::default()
                },
                noise_params: NoiseParams {
                    thermal_db: -122.0,
                    pink_db: -112.0,
                    ..NoiseParams::default()
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::FerriteTape => {
            let poly = harmonics_to_poly(
                db_to_amp(-36.0),
                db_to_amp(-46.0),
                db_to_amp(-68.0),
                db_to_amp(-78.0),
            );
            AteCustomParams {
                poly_a: poly,
                state_params: StateParams {
                    flux_alpha: 0.026,
                    flux_beta: 0.005,
                    recovery_alpha: 0.003,
                    recovery_decay: 0.997,
                    ..StateParams::default()
                },
                noise_params: NoiseParams {
                    thermal_db: -125.0,
                    pink_db: -98.0,
                    tape_db: -82.0,
                    ..NoiseParams::default()
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::PhonoStage => {
            let poly = harmonics_to_poly(
                db_to_amp(-48.0),
                db_to_amp(-60.0),
                db_to_amp(-80.0),
                db_to_amp(-85.0),
            );
            AteCustomParams {
                poly_a: poly,
                state_params: StateParams {
                    flux_alpha: 0.004,
                    bias_alpha: 0.0002,
                    ..StateParams::default()
                },
                noise_params: NoiseParams {
                    thermal_db: -128.0,
                    pink_db: -105.0,
                    ..NoiseParams::default()
                },
                channel_mismatch: ChannelMismatch {
                    gain_db_l: 0.004,
                    gain_db_r: -0.004,
                    phase_deg: 0.08,
                    thd_variance: 0.003,
                    crosstalk_db: -104.0,
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::Hybrid => {
            let tube = preset_params(AtePreset::Tube);
            let tape = preset_params(AtePreset::Tape);
            let vinyl = preset_params(AtePreset::Vinyl);
            AteCustomParams {
                poly_a: [
                    (tube.poly_a[0] + tape.poly_a[0]) * 0.5,
                    1.0,
                    (tube.poly_a[2] + tape.poly_a[2]) * 0.5,
                    (tube.poly_a[3] + tape.poly_a[3]) * 0.5,
                    tube.poly_a[4] * 0.5,
                    tube.poly_a[5] * 0.5,
                ],
                state_params: tube.state_params,
                noise_params: NoiseParams {
                    pink_db: (tube.noise_params.pink_db + vinyl.noise_params.pink_db) * 0.5,
                    hum_db: tube.noise_params.hum_db,
                    tape_db: tape.noise_params.tape_db,
                    ..NoiseParams::default()
                },
                channel_mismatch: ChannelMismatch {
                    gain_db_l: 0.01,
                    gain_db_r: -0.01,
                    phase_deg: 0.2,
                    thd_variance: 0.02,
                    crosstalk_db: -85.0,
                },
                ..AteCustomParams::default()
            }
        }
        AtePreset::Custom => AteCustomParams::default(),
    }
}

pub fn make_config(
    preset: AtePreset,
    oversampling: OversamplingMode,
    intensity: f32,
    stereo_variance_seed: u64,
    enable_analyzer: bool,
) -> AteConfig {
    AteConfig {
        enable: true,
        preset,
        intensity: intensity.clamp(0.0, 1.0),
        oversampling,
        stereo_variance_seed,
        enable_analyzer,
        custom_params: None,
    }
}
