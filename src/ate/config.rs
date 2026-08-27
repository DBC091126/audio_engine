/// Device family preset used by the ATE engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AtePreset {
    #[default]
    CleanAnalog,
    Tube,
    VintageSolidState,
    VintageDac,
    Vinyl,
    Tape,
    Hybrid,
    Custom,
}

/// Internal oversampling multiplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OversamplingMode {
    None,
    X2,
    #[default]
    X4,
    X8,
}

impl OversamplingMode {
    pub fn factor(self) -> usize {
        match self {
            OversamplingMode::None => 1,
            OversamplingMode::X2 => 2,
            OversamplingMode::X4 => 4,
            OversamplingMode::X8 => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JitterType {
    White,
    Periodic,
    Correlated,
    #[default]
    Mixed,
}

#[derive(Debug, Clone)]
pub struct AteConfig {
    pub enable: bool,
    pub preset: AtePreset,
    pub intensity: f32,
    pub oversampling: OversamplingMode,
    pub stereo_variance_seed: u64,
    pub enable_analyzer: bool,
    pub custom_params: Option<AteCustomParams>,
}

impl Default for AteConfig {
    fn default() -> Self {
        Self {
            enable: false,
            preset: AtePreset::CleanAnalog,
            intensity: 0.5,
            oversampling: OversamplingMode::X4,
            stereo_variance_seed: 0x4154_455f_5644,
            enable_analyzer: false,
            custom_params: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AteCustomParams {
    pub poly_a: [f32; 6],
    pub mem_poly: Vec<f32>,
    pub crossover: CrossoverParams,
    pub state_params: StateParams,
    pub noise_params: NoiseParams,
    pub jitter_params: JitterParams,
    pub channel_mismatch: ChannelMismatch,
}

impl Default for AteCustomParams {
    fn default() -> Self {
        Self {
            poly_a: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            mem_poly: Vec::new(),
            crossover: CrossoverParams::default(),
            state_params: StateParams::default(),
            noise_params: NoiseParams::default(),
            jitter_params: JitterParams::default(),
            channel_mismatch: ChannelMismatch::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CrossoverParams {
    pub enabled: bool,
    pub inner_gain: f32,
    pub theta: f32,
    pub negative_inner_gain: f32,
    pub negative_theta: f32,
}

impl Default for CrossoverParams {
    fn default() -> Self {
        Self {
            enabled: false,
            inner_gain: 0.02,
            theta: 0.05,
            negative_inner_gain: 0.01,
            negative_theta: 0.06,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StateParams {
    pub env_alpha: f32,
    pub bias_alpha: f32,
    pub flux_alpha: f32,
    pub flux_beta: f32,
    pub thermal_alpha: f32,
    pub recovery_alpha: f32,
    pub recovery_decay: f32,
}

impl Default for StateParams {
    fn default() -> Self {
        Self {
            env_alpha: 0.0005,
            bias_alpha: 0.0001,
            flux_alpha: 0.002,
            flux_beta: 0.001,
            thermal_alpha: 0.0002,
            recovery_alpha: 0.0004,
            recovery_decay: 0.9999,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoiseParams {
    pub thermal_db: f32,
    pub pink_db: f32,
    pub hum_db: f32,
    pub hum_hz: f32,
    pub vinyl_db: f32,
    pub crackle_db: f32,
    pub tape_db: f32,
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            thermal_db: -120.0,
            pink_db: -100.0,
            hum_db: -90.0,
            hum_hz: 60.0,
            vinyl_db: -80.0,
            crackle_db: -60.0,
            tape_db: -90.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct JitterParams {
    pub enabled: bool,
    pub rms_ps: f32,
    pub periodic_hz: f32,
    pub periodic_db: f32,
    pub correlated_db: f32,
    pub jitter_type: JitterType,
}

impl Default for JitterParams {
    fn default() -> Self {
        Self {
            enabled: false,
            rms_ps: 30.0,
            periodic_hz: 1000.0,
            periodic_db: -90.0,
            correlated_db: -100.0,
            jitter_type: JitterType::Mixed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelMismatch {
    pub gain_db_l: f32,
    pub gain_db_r: f32,
    pub phase_deg: f32,
    pub thd_variance: f32,
    pub crosstalk_db: f32,
}

impl Default for ChannelMismatch {
    fn default() -> Self {
        Self {
            gain_db_l: 0.0,
            gain_db_r: 0.0,
            phase_deg: 0.0,
            thd_variance: 0.0,
            crosstalk_db: -90.0,
        }
    }
}
