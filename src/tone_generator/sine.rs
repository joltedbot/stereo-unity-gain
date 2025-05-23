const RADS_PER_CYCLE: f32 = 2.0 * std::f32::consts::PI;

pub struct Sine {
    pub phase: f32,
    pub phase_increment: f32,
}

impl Sine {
    pub fn new(sample_rate: f32) -> Self {
        let phase: f32 = 0.0;
        let seconds_per_sample = 1.0 / sample_rate;
        let phase_increment = RADS_PER_CYCLE * seconds_per_sample;

        Self {
            phase,
            phase_increment,
        }
    }

    pub fn generate_tone_sample(
        &mut self,
        reference_frequency: f32,
        dbfs_adjustment_factor: f32,
    ) -> f32 {
        self.phase += self.phase_increment * reference_frequency;
        if self.phase >= RADS_PER_CYCLE {
            self.phase = 0.0;
        }
        self.phase.sin() * dbfs_adjustment_factor
    }
}
