mod devices;
mod errors;
mod level_meter;
mod tone_generator;

use crate::devices::{get_model_from_string_slice, Devices, DisplayData};
use crate::errors::{handle_localerror, LocalError};
use crate::level_meter::LevelMeter;
use crate::tone_generator::ToneGenerator;
use slint::SharedString;
use std::process::exit;
use std::rc::Rc;
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

    // Set up the Tone Generator

    let left_channel: u8 = devices.active_output_device.2.clone().parse().unwrap_or(1);
    let right_channel: u8 = devices.active_output_device.3.clone().parse().unwrap_or(0);

    let tone_generator =
        match ToneGenerator::new(devices.output_device.clone(), left_channel, right_channel) {
            Ok(tone_generator) => tone_generator,
            Err(error) => {
                handle_localerror(LocalError::DeviceInitialization, error.to_string());
                exit(EXIT_CODE_ERROR);
            }
        };

    // Set up the level meter

    let input_left_channel: u8 = devices.active_input_device.2.clone().parse().unwrap_or(1);
    let input_right_channel: u8 = devices.active_input_device.3.clone().parse().unwrap_or(0);

    let level_meter = match LevelMeter::new(
        devices.input_device.clone(),
        input_left_channel,
        input_right_channel,
    ) {
        Ok(meter) => meter,
        Err(error) => {
            handle_localerror(LocalError::MeterInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    // Set up the callback functions for the device dropdown menus
    let devices_arc_mutex = Arc::new(Mutex::new(devices));
    let tone_generator_arc_mutex = Rc::new(Mutex::new(tone_generator));
    let level_meter_arc_mutex = Rc::new(Mutex::new(level_meter));

    // INPUT DEVICE CHANGE CALLBACK
    let input_devices_clone = devices_arc_mutex.clone();
    let tone_generator_clone = tone_generator_arc_mutex.clone();
    let level_meter_clone = level_meter_arc_mutex.clone();
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

            if let Ok(mut tone_generator) = tone_generator_clone.lock() {
                tone_generator.stop();
            }

            if let Ok(mut level_meter) = level_meter_clone.lock() {
                let left_channel = devices.active_input_device.2.clone().parse().unwrap_or(1);
                let right_channel = devices.active_input_device.3.clone().parse().unwrap_or(0);

                level_meter
                    .change_device(devices.input_device.clone(), left_channel, right_channel)
                    .expect("Could Not Input Change Devices");
            }
        }
    });

    // OUTPUT DEVICE CHANGE CALLBACK
    let output_devices_clone = devices_arc_mutex.clone();
    let tone_generator_clone = tone_generator_arc_mutex.clone();
    let level_meter_clone = level_meter_arc_mutex.clone();
    let output_ui_weak = ui.as_weak();
    ui.on_selected_output_device(move |index, device| {
        if let Ok(mut devices) = output_devices_clone.lock() {
            devices.set_current_output_device_on_ui_callback((index, device.to_string()));

            let ui = output_ui_weak.upgrade().unwrap();
            ui.set_output_channel_list(devices.get_current_output_device_channels());
            ui.set_left_current_output_channel(SharedString::from(
                devices.active_output_device.2.clone(),
            ));

            if devices.active_output_device.3.is_empty() {
                ui.set_right_output_enabled(false);
            } else {
                ui.set_right_output_enabled(true);
                ui.set_right_current_output_channel(SharedString::from(
                    devices.active_output_device.3.clone(),
                ));
            }

            if let Ok(mut level_meter) = level_meter_clone.lock() {
                level_meter.stop();
            }

            if let Ok(mut tone_generator) = tone_generator_clone.lock() {
                let left_channel = devices.active_output_device.2.clone().parse().unwrap_or(1);
                let right_channel = devices.active_output_device.3.clone().parse().unwrap_or(0);

                tone_generator
                    .change_device(devices.output_device.clone(), left_channel, right_channel)
                    .expect("Could Not Change Output Devices");
            }
        }
    });

    // OUTPUT CHANNEL CHANGE CALLBACK
    let output_devices_clone = devices_arc_mutex.clone();
    let tone_generator_clone = tone_generator_arc_mutex.clone();
    let level_meter_clone = level_meter_arc_mutex.clone();
    ui.on_selected_output_channel(move |left_channel, right_channel| {
        if let Ok(mut devices) = output_devices_clone.lock() {
            devices.set_output_channel_on_ui_callback(
                left_channel.to_string(),
                right_channel.to_string(),
            );

            if let Ok(mut level_meter) = level_meter_clone.lock() {
                level_meter.stop();
            }

            if let Ok(mut tone_generator) = tone_generator_clone.lock() {
                let left_channel = devices.active_output_device.2.clone().parse().unwrap_or(1);
                let right_channel = devices.active_output_device.3.clone().parse().unwrap_or(0);

                tone_generator
                    .change_channel(left_channel, right_channel)
                    .expect("Could Not Change Output Channel");
            }
        }
    });

    // INPUT CHANNEL CHANGE CALLBACK
    let input_devices_clone = devices_arc_mutex.clone();
    let level_meter_clone = level_meter_arc_mutex.clone();
    let tone_generator_clone = tone_generator_arc_mutex.clone();

    ui.on_selected_input_channel(move |left_channel, right_channel| {
        if let Ok(mut devices) = input_devices_clone.lock() {
            devices.set_input_channel_on_ui_callback(
                left_channel.to_string(),
                right_channel.to_string(),
            );

            if let Ok(mut tone_generator) = tone_generator_clone.lock() {
                tone_generator.stop();
            }

            if let Ok(mut level_meter) = level_meter_clone.lock() {
                let left_channel = devices.active_input_device.2.clone().parse().unwrap_or(1);
                let right_channel = devices.active_input_device.3.clone().parse().unwrap_or(0);

                level_meter
                    .change_channel(left_channel, right_channel)
                    .expect("Could Not Change Input Channel");
            }
        }
    });

    // Set up the callback for the start stop button
    let tone_generator_clone = tone_generator_arc_mutex.clone();
    let level_meter_clone = level_meter_arc_mutex.clone();
    ui.on_start_button_pressed(move |is_active| {
        if let Ok(mut tone_generator) = tone_generator_clone.lock() {
            match is_active {
                true => tone_generator.start(),
                false => tone_generator.stop(),
            }
        };

        if let Ok(mut level_meter) = level_meter_clone.lock() {
            match is_active {
                true => level_meter.start(),
                false => level_meter.stop(),
            }
        };
    });

    // Start the UI and enter the main program loop

    ui.run().unwrap();
}

fn set_ui_device_data(ui: &AppWindow, devices: &Devices, display_data: &DisplayData) {
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
