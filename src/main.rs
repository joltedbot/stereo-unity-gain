mod devices;
mod errors;
mod ui;

use crate::devices::Devices;
use crate::errors::{handle_local_error, LocalError};
use crate::ui::UI;
use std::process::exit;
use std::sync::{Arc, Mutex};

const EXIT_CODE_ERROR: i32 = 1;

fn main() -> Result<(), slint::PlatformError> {
    // Initialize the audio devices lists
    let devices = match Devices::new() {
        Ok(device) => device,
        Err(error) => {
            handle_local_error(LocalError::DeviceInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    let devices_arc_mutex = Arc::new(Mutex::new(devices));

    let mut ui = UI::new()?;

    match ui.initialize_ui_with_device_data(devices_arc_mutex.clone()) {
        Ok(_) => {}
        Err(error) => {
            handle_local_error(LocalError::UIInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    match ui.update_level_meter_values_in_the_ui(devices_arc_mutex.clone()) {
        Ok(_) => {}
        Err(error) => {
            handle_local_error(LocalError::MeterReaderUIUpdater, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    // Start the UI and enter the main program loop
    ui.run()
}
