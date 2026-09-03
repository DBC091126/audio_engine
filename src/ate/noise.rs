use std::sync::OnceLock;

use super::config::NoiseParams;

#[derive(Debug, Clone)]
pub struct Lcg {
    state: u64,
}

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 32) as u32
    }

    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn gaussian(&mut self) -> f32 {
        static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
        let table = TABLE.get_or_init(|| {
            let mut rng = Lcg::new(0x5DEECE66D);
            let mut values = Vec::with_capacity(8192);
            for _ in 0..8192 {
                let u1 = (rng.next_f32() + 1.0e-9).min(0.999_999);
                let u2 = rng.next_f32();
                values.push(
                    (-2.0 * u1.ln()).sqrt()
                        * (2.0 * std::f32::consts::PI * u2).cos(),
                );
            }
            values
        });
        let index = self.next_u32() as usize % table.len();
        table[index]
    }
}

#[derive(Debug, Clone)]
pub struct NoiseSimulator {
    rng: Lcg,
    pink_state: f32,
    tape_state: f32,
}

impl NoiseSimulator {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Lcg::new(seed),
            pink_state: 0.0,
            tape_state: 0.0,
        }
    }

    pub fn add_noise(&mut self, samples: &mut [f32], params: &NoiseParams, sample_rate: f32) {
        const ACTIVE_FLOOR: f32 = -140.0;
        let thermal = db_to_amp(params.thermal_db);
        let pink = db_to_amp(params.pink_db);
        let hum = db_to_amp(params.hum_db);
        let vinyl = db_to_amp(params.vinyl_db);
        let crackle = db_to_amp(params.crackle_db);
        let tape = db_to_amp(params.tape_db);
        let do_thermal = params.thermal_db > ACTIVE_FLOOR;
        let do_pink = params.pink_db > ACTIVE_FLOOR;
        let do_hum = params.hum_db > ACTIVE_FLOOR;
        let do_vinyl = params.vinyl_db > ACTIVE_FLOOR;
        let do_crackle = params.crackle_db > ACTIVE_FLOOR;
        let do_tape = params.tape_db > ACTIVE_FLOOR;
        if !(do_thermal || do_pink || do_hum || do_vinyl || do_crackle || do_tape) {
            return;
        }

        for (i, sample) in samples.iter_mut().enumerate() {
            let white = if do_thermal || do_pink || do_tape || do_crackle {
                self.rng.gaussian()
            } else {
                0.0
            };

            if do_pink {
                self.pink_state += 0.02 * white;
                self.pink_state *= 0.98;
            }
            if do_tape {
                self.tape_state += 0.08 * white;
                self.tape_state *= 0.96;
            }

            let mut noise = 0.0f32;
            if do_thermal {
                noise += white * thermal;
            }
            if do_pink {
                noise += self.pink_state * pink;
            }
            if do_tape {
                noise += (white - self.tape_state) * tape;
            }
            if do_hum {
                let t = i as f32 / sample_rate;
                let phase = 2.0 * std::f32::consts::PI * params.hum_hz * t;
                noise += (phase.sin()
                    + 0.5 * (2.0 * phase).sin()
                    + 0.25 * (3.0 * phase).sin())
                    * hum;
            }
            if do_vinyl {
                let t = i as f32 / sample_rate;
                noise += (self.pink_state * 0.5
                    + 0.4 * (2.0 * std::f32::consts::PI * 28.0 * t).sin())
                    * vinyl;
            }
            if do_crackle && self.rng.next_f32() < 0.000_02 {
                noise += self.rng.gaussian().signum() * 0.8 * crackle;
            }
            *sample += noise;
        }
    }
}

pub fn db_to_amp(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}
