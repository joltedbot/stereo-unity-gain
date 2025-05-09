mod devices;
mod errors;

use crate::devices::{get_model_from_string_slice, Devices, DisplayData};
use crate::errors::{handle_localerror, LocalError};
use slint::SharedString;
use std::process::exit;
use std::sync::{Arc, Mutex};

slint::include_modules!();

const EXIT_CODE_ERROR: i32 = 1;

fn main() {
    // Initialize the UI
    let ui = AppWindow::new().unwrap();

    // Initialize the audio devices lists
    let devices = match Devices::new() {
        Ok(device) => device,
        Err(error) => {
            handle_localerror(LocalError::DeviceInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    let device_display_data = devices.get_display_data();
    set_ui_device_data(&ui, &devices, &device_display_data);

    // Set up the callback functions for the device dropdown menus
    let devices_arc_mutex = Arc::new(Mutex::new(devices));

    let input_devices_clone = devices_arc_mutex.clone();
    let input_ui_weak = ui.as_weak();
    ui.on_selected_input_device(move |index, device| {
        if let Ok(mut devices) = input_devices_clone.lock() {
            devices.set_current_input_device_on_ui_callback((index, device.to_string()));

            let app_weak = input_ui_weak.upgrade().unwrap();
            app_weak.set_input_channel_list(devices.get_current_input_device_channels());
            app_weak.set_left_current_input_channel(SharedString::from(
                devices.active_input_device.2.clone(),
            ));

            if devices.active_input_device.3.is_empty() {
                app_weak.set_right_input_enabled(false);
            } else {
                app_weak.set_right_input_enabled(true);
                app_weak.set_right_current_input_channel(SharedString::from(
                    devices.active_input_device.3.clone(),
                ));
            }
        }
    });

    let output_devices_clone = devices_arc_mutex.clone();
    let output_ui_weak = ui.as_weak();
    ui.on_selected_output_device(move |index, device| {
        if let Ok(mut devices) = output_devices_clone.lock() {
            devices.set_current_output_device_on_ui_callback((index, device.to_string()));

            let app_weak = output_ui_weak.upgrade().unwrap();
            app_weak.set_output_channel_list(devices.get_current_output_device_channels());
            app_weak.set_left_current_output_channel(SharedString::from(
                devices.active_output_device.2.clone(),
            ));
            if devices.active_output_device.3.is_empty() {
                app_weak.set_right_output_enabled(false);
            } else {
                app_weak.set_right_output_enabled(true);
                app_weak.set_right_current_output_channel(SharedString::from(
                    devices.active_output_device.3.clone(),
                ));
            }
        }
    });

    // Set up the callback functions for the channel dropdown menus
    let left_output_devices_clone = devices_arc_mutex.clone();
    ui.on_selected_left_output_channel(move |channel| {
        if let Ok(mut devices) = left_output_devices_clone.lock() {
            devices.set_left_output_channel_on_ui_callback(channel.to_string());
        }
    });

    let right_output_devices_clone = devices_arc_mutex.clone();
    ui.on_selected_right_output_channel(move |channel| {
        if let Ok(mut devices) = right_output_devices_clone.lock() {
            devices.set_right_output_channel_on_ui_callback(channel.to_string());
        }
    });

    let left_input_devices_clone = devices_arc_mutex.clone();
    ui.on_selected_left_input_channel(move |channel| {
        if let Ok(mut devices) = left_input_devices_clone.lock() {
            devices.set_left_input_channel_on_ui_callback(channel.to_string());
        }
    });

    let right_input_devices_clone = devices_arc_mutex.clone();
    ui.on_selected_right_input_channel(move |channel| {
        if let Ok(mut devices) = right_input_devices_clone.lock() {
            devices.set_right_input_channel_on_ui_callback(channel.to_string());
        }
    });

    // Start the UI and enter the main program loop

    ui.run().unwrap();
}

pub fn set_ui_device_data(ui: &AppWindow, devices: &Devices, display_data: &DisplayData) {
    let input_device_model = get_model_from_string_slice(&display_data.input_device_list.0);
    ui.set_input_device_list(input_device_model);

    let output_device_model = get_model_from_string_slice(&display_data.output_device_list.0);
    ui.set_output_device_list(output_device_model);

    ui.set_input_channel_list(get_model_from_string_slice(
        &display_data.input_device_list.1[devices.active_input_device.0 as usize].clone(),
    ));

    ui.set_output_channel_list(get_model_from_string_slice(
        &display_data.output_device_list.1[devices.active_output_device.0 as usize].clone(),
    ));

    ui.set_current_output_device(SharedString::from(devices.active_output_device.1.clone()));
    ui.set_left_current_output_channel(SharedString::from(devices.active_output_device.2.clone()));
    ui.set_current_input_device(SharedString::from(devices.active_input_device.1.clone()));
    ui.set_left_current_input_channel(SharedString::from(devices.active_input_device.2.clone()));

    if devices.active_output_device.3.is_empty() {
        ui.set_right_output_enabled(false);
    } else {
        ui.set_right_output_enabled(true);
        ui.set_right_current_output_channel(SharedString::from(
            devices.active_output_device.3.clone(),
        ));
    }

    if devices.active_input_device.3.is_empty() {
        ui.set_right_input_enabled(false);
    } else {
        ui.set_right_input_enabled(true);
        ui.set_right_current_input_channel(SharedString::from(
            devices.active_input_device.3.clone(),
        ));
    }
}
