use super::config::StateParams;

#[derive(Debug, Clone)]
pub struct AteState {
    pub envelope: f32,
    pub bias: f32,
    pub flux: f32,
    pub thermal: f32,
    pub recovery: f32,
}

impl Default for AteState {
    fn default() -> Self {
        Self::new()
    }
}

impl AteState {
    pub fn new() -> Self {
        Self {
            envelope: 0.0,
            bias: 0.0,
            flux: 0.0,
            thermal: 0.0,
            recovery: 0.0,
        }
    }

    pub fn update(&mut self, x: f32, params: &StateParams) {
        self.envelope += params.env_alpha * (x.abs() - self.envelope);
        self.bias += params.bias_alpha * (x - self.bias);
        self.flux += params.flux_alpha * x - params.flux_beta * self.flux;
        let power = x * x;
        self.thermal += params.thermal_alpha * (power - self.thermal);
        self.recovery += params.recovery_alpha * (x.abs() - self.recovery);
        self.recovery *= params.recovery_decay;
    }
}
