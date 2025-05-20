mod devices;
mod errors;
mod ui;

use crate::devices::Devices;
use crate::errors::{EXIT_CODE_ERROR, LocalError, handle_local_error};
use crate::ui::UI;
use std::process::exit;
use std::rc::Rc;
use std::sync::Mutex;

fn main() -> Result<(), slint::PlatformError> {
    let devices = match Devices::new() {
        Ok(device) => device,
        Err(error) => {
            handle_local_error(LocalError::DeviceInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    let devices_mutex = Rc::new(Mutex::new(devices));

    let mut ui = UI::new()?;

    if let Err(err) = ui.initialize_ui_with_device_data(devices_mutex.clone()) {
        handle_local_error(LocalError::UIInitialization, err.to_string());
        exit(EXIT_CODE_ERROR);
    };

    if let Err(err) = ui.start_level_meter(devices_mutex.clone()) {
        handle_local_error(LocalError::MeterReaderUIUpdater, err.to_string());
        exit(EXIT_CODE_ERROR);
    };

    ui.create_ui_callbacks(devices_mutex.clone());

    // Start the UI and enter the main program loop
    ui.run()
}
