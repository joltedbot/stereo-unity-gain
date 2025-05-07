mod devices;
mod errors;
mod ui;

use crate::devices::Devices;
use crate::errors::{handle_localerror, LocalError};
use crate::ui::UI;
use std::process::exit;
use std::sync::{Arc, Mutex, MutexGuard};

const EXIT_CODE_ERROR: i32 = 1;
const EXIT_CODE_SUCCESS: i32 = 0;

fn main() {
    // Initialize the UI
    let mut ui = match UI::new() {
        Ok(ui) => ui,
        Err(error) => {
            handle_localerror(LocalError::UIInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    // Initialize the audio devices lists
    let devices_arc_mutex = match Devices::new() {
        Ok(device) => Arc::new(Mutex::new(device)),
        Err(error) => {
            handle_localerror(LocalError::DeviceInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    // Limit the scope of the mutex lock to allow the call back functions to not get a deadlock with this device
    {
        let devices: MutexGuard<Devices> = match devices_arc_mutex.lock() {
            Ok(devices) => devices,
            Err(error) => {
                handle_localerror(LocalError::DeviceInitialization, error.to_string());
                exit(EXIT_CODE_ERROR);
            }
        };

        // Populate the device dropdown in the ui
        let input_devices = devices.input_device_list.clone();
        let output_devices = devices.output_device_list.clone();
        ui.set_device_lists(input_devices, output_devices);

        let input_channels = devices.input_channel_list.clone();
        let output_channels = devices.output_channel_list.clone();
        ui.set_channel_lists(input_channels, output_channels);

        // Set the default device as selected in the ui
        let default_input_device = match devices.get_current_input_device_name() {
            Ok(device) => device,
            Err(error) => {
                handle_localerror(LocalError::DeviceInitialization, error.to_string());
                exit(EXIT_CODE_ERROR);
            }
        };

        let default_output_device = match devices.get_current_output_device_name() {
            Ok(device) => device,
            Err(error) => {
                handle_localerror(LocalError::DeviceInitialization, error.to_string());
                exit(EXIT_CODE_ERROR);
            }
        };

        ui.set_default_devices(default_input_device, default_output_device);
        ui.set_default_channels(
            devices.left_input_channel.clone(),
            devices.right_input_channel.clone(),
            devices.left_output_channel.clone(),
            devices.right_output_channel.clone(),
        );
    }

    // Set up the callback functions for the device dropdown menus
    let input_devices_clone = devices_arc_mutex.clone();
    ui.on_selected_input_device(move |device| {
        if let Ok(mut devices) = input_devices_clone.lock() {
            devices.set_current_input_device(device);
        }
    });

    let output_devices_clone = devices_arc_mutex.clone();
    ui.on_selected_output_device(move |device| {
        if let Ok(mut devices) = output_devices_clone.lock() {
            devices.set_current_output_device(device);
        }
    });

    // Set up the callback functions for the channel dropdown menus
    let left_input_channel_clone = devices_arc_mutex.clone();
    ui.on_left_selected_input_channel(move |channel| {
        if let Ok(mut devices) = left_input_channel_clone.lock() {
            devices.set_current_left_input_channel(channel);
        }
    });

    let right_input_channel_clone = devices_arc_mutex.clone();
    ui.on_right_selected_input_channel(move |channel| {
        if let Ok(mut devices) = right_input_channel_clone.lock() {
            devices.set_current_right_input_channel(channel);
        }
    });

    let left_output_channel_clone = devices_arc_mutex.clone();
    ui.on_left_selected_output_channel(move |channel| {
        if let Ok(mut devices) = left_output_channel_clone.lock() {
            devices.set_current_left_output_channel(channel);
        }
    });

    let right_output_channel_clone = devices_arc_mutex.clone();
    ui.on_right_selected_output_channel(move |channel| {
        if let Ok(mut devices) = right_output_channel_clone.lock() {
            devices.set_current_right_output_channel(channel);
        }
    });

    // Start the UI and enter the main program loop
    match ui.run() {
        Ok(_) => exit(EXIT_CODE_SUCCESS),
        Err(error) => {
            handle_localerror(LocalError::UIRun, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    }
}
