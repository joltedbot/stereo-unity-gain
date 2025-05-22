mod devices;
mod errors;
pub mod level_meter;
pub mod tone_generator;
mod ui;

use crate::errors::{handle_local_error, LocalError, EXIT_CODE_ERROR};
use crate::level_meter::LevelMeter;
use crate::tone_generator::ToneGenerator;
use crate::ui::UI;
use slint::ComponentHandle;
use std::process::exit;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), slint::PlatformError> {
    let tone_generator = match ToneGenerator::new() {
        Ok(tone_generator) => tone_generator,
        Err(error) => {
            handle_local_error(LocalError::ToneGeneratorInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    let mut level_meter = match LevelMeter::new() {
        Ok(level_meter) => level_meter,
        Err(error) => {
            handle_local_error(LocalError::LevelMeterInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    let mut ui = UI::new()?;

    if let Err(err) = ui.initialize_ui_with_device_data(
        level_meter.get_input_device_list(),
        level_meter.get_current_input_device(),
        tone_generator.get_output_device_list(),
        tone_generator.get_current_output_device(),
        tone_generator.get_reference_frequency(),
    ) {
        handle_local_error(LocalError::UIInitialization, err.to_string());
        exit(EXIT_CODE_ERROR);
    };

    if let Err(err) = level_meter.start_level_meter(ui.ui.as_weak()) {
        handle_local_error(LocalError::MeterReaderUIUpdater, err.to_string());
        exit(EXIT_CODE_ERROR);
    };

    let level_meter_mutex = Arc::new(Mutex::new(level_meter));
    let tone_generator_mutex = Arc::new(Mutex::new(tone_generator));

    ui.create_ui_callbacks(level_meter_mutex.clone(), tone_generator_mutex.clone());

    // Start the UI and enter the main program loop
    ui.run()
}
