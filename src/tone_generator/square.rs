use crate::tone_generator::WaveShape;

const PI: f32 = std::f32::consts::PI;

const ADJUSTMENT_FACTOR_TO_MATCH_SINE_REFERENCE_LEVEL: f32 = 2.3;

pub struct Square {
    x_coord: f32,
    x_increment: f32,
    sample_rate: f32,
}

impl WaveShape for Square {
    fn new(sample_rate: f32) -> Self {
        let x_coord = 0.0;
        let x_increment = 1.0;

        Self {
            x_coord,
            x_increment,
            sample_rate,
        }
    }

    fn generate_tone_sample(&mut self, reference_frequency: f32, target_level: f32) -> f32 {
        let mut y_coord: f32 =
            (reference_frequency * (2.0 * PI) * (self.x_coord / self.sample_rate)).sin();

        if y_coord >= 0.0 {
            y_coord = 1.0;
        } else {
            y_coord = -1.0;
        }

        self.x_coord += self.x_increment;
        y_coord * get_dbfs_adjustment_factor_from_target_level(target_level)
    }
}

fn get_dbfs_adjustment_factor_from_target_level(level: f32) -> f32 {
    10.0_f32.powf((level - ADJUSTMENT_FACTOR_TO_MATCH_SINE_REFERENCE_LEVEL) / 20.0)
}
