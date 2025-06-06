const RADS_PER_CYCLE: f32 = 2.0 * std::f32::consts::PI;

pub struct Sine {
    pub phase: f32,
    phase_increment: f32,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_correct_tone_samples_from_valid_frequency_and_adjustment() {
        let sample_rate = 48000.0;
        let mut sine = Sine::new(sample_rate);
        let samples = [
            sine.generate_tone_sample(440.0, 1.0),
            sine.generate_tone_sample(440.0, 1.0),
            sine.generate_tone_sample(440.0, 1.0),
        ];
        let expected_samples = [0.057564028, 0.11493716, 0.1719291];

        assert_eq!(samples, expected_samples);
    }

    #[test]
    fn sine_phase_resets_to_zero_when_it_exceeds_radians_per_wave_cycle() {
        let sample_rate = 4.0;
        let mut sine = Sine::new(sample_rate);
        let freq = 1.0;
        let dbfs = 1.0;

        for _ in 0..4 {
            sine.generate_tone_sample(freq, dbfs);
            println!("{}", sine.phase);
        }
        assert_eq!(sine.phase, 0.0);
    }

    #[test]
    fn return_zero_value_samples_from_zero_reference_frequency() {
        let mut sine = Sine::new(48000.0);
        let sample1 = sine.generate_tone_sample(0.0, 1.0);
        let sample2 = sine.generate_tone_sample(0.0, 1.0);
        assert_eq!(sample1, 0.0);
        assert_eq!(sample2, 0.0);
    }

    #[test]
    fn return_zero_value_samples_from_zero_dbfs_adjustment() {
        let mut sine = Sine::new(48000.0);
        let sample = sine.generate_tone_sample(440.0, 0.0);
        assert_eq!(sample, 0.0);
    }

    #[test]
    fn return_valid_sample_value_from_negative_frequency() {
        let mut sine = Sine::new(48000.0);
        let sample = sine.generate_tone_sample(-440.0, 1.0);
        let expected_negagtive_value = -0.057564028;
        assert_eq!(sample, expected_negagtive_value);
    }
}
