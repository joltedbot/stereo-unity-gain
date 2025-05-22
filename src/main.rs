mod devices;
mod errors;
pub mod level_meter;
pub mod tone_generator;
mod ui;

use crate::errors::{handle_local_error, LocalError, EXIT_CODE_ERROR};
use crate::level_meter::LevelMeter;
use crate::tone_generator::ToneGenerator;
use crate::ui::UI;
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

    let level_meter = match LevelMeter::new() {
        Ok(level_meter) => level_meter,
        Err(error) => {
            handle_local_error(LocalError::LevelMeterInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    let level_meter_mutex = Arc::new(Mutex::new(level_meter));
    let tone_generator_mutex = Arc::new(Mutex::new(tone_generator));

    let mut ui = UI::new()?;

    if let Err(err) =
        ui.initialize_ui_with_device_data(level_meter_mutex.clone(), tone_generator_mutex.clone())
    {
        handle_local_error(LocalError::UIInitialization, err.to_string());
        exit(EXIT_CODE_ERROR);
    };

    if let Err(err) = ui.start_level_meter(level_meter_mutex.clone()) {
        handle_local_error(LocalError::MeterReaderUIUpdater, err.to_string());
        exit(EXIT_CODE_ERROR);
    };

    ui.create_ui_callbacks(level_meter_mutex.clone(), tone_generator_mutex.clone());

    // Start the UI and enter the main program loop
    ui.run()
}
