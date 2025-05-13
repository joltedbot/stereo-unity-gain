mod devices;
mod errors;
mod level_meter;
mod tone_generator;

use crate::devices::{get_model_from_string_slice, Devices, DisplayData};
use crate::errors::{handle_localerror, LocalError};
use crate::level_meter::LevelMeter;
use crate::tone_generator::ToneGenerator;
use crossbeam_channel::Receiver;
use slint::{SharedString, Weak};
use std::process::exit;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

slint::include_modules!();

const EXIT_CODE_ERROR: i32 = 1;
const TARGET_OUTPUT_LEVEL: f32 = -12.0;
const RMS_BUFFER_LENGTH: usize = 1440;

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

    // Set up the level meter reading thread to update the UI
    let ui_weak = ui.as_weak();
    let level_meter_clone = level_meter_arc_mutex.clone();

    // Get the meter reader from level meter
    let meter_reader = {
        let mut level_meter = level_meter_clone.lock().unwrap();
        level_meter.get_meter_reader()
    };

    // Spawn a thread to monitor the levels and update the UI
    update_level_meter_values_in_the_ui(ui_weak, meter_reader);

    // Start the UI and enter the main program loop
    ui.run().unwrap();
}

fn update_level_meter_values_in_the_ui(
    ui_weak: Weak<AppWindow>,
    meter_reader: Receiver<(f32, f32)>,
) {
    let mut left_sample_buffer: Vec<f32> = Vec::new();
    let mut right_sample_buffer: Vec<f32> = Vec::new();
    let mut last_left_rms = 0.0;
    let mut last_right_rms = 0.0;

    thread::spawn(move || {
        while let Ok((left_level, right_level)) = meter_reader.recv() {
            if left_sample_buffer.len() >= RMS_BUFFER_LENGTH {
                let left = calculate_rms(&mut left_sample_buffer);
                let right = calculate_rms(&mut right_sample_buffer);
                if last_left_rms != left || last_right_rms != right {
                    last_left_rms = left;
                    last_right_rms = right;

                    let left_delta = left - TARGET_OUTPUT_LEVEL;
                    let right_delta = right - TARGET_OUTPUT_LEVEL;

                    // Format the values for display
                    let left_formatted = {
                        if left_delta > 0.1 {
                            format!("+{:.1}", left_delta)
                        } else {
                            format!("{:.1}", left_delta)
                        }
                    };

                    let right_formatted = {
                        if right_delta > 0.1 {
                            format!("+{:.1}", right_delta)
                        } else {
                            format!("{:.1}", right_delta)
                        }
                    };

                    // Update UI safely on the main thread
                    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_left_level_box_value(SharedString::from(left_formatted));
                        ui.set_right_level_box_value(SharedString::from(right_formatted));
                    });
                }
                left_sample_buffer.pop();
                right_sample_buffer.pop();
            }

            left_sample_buffer.insert(0, left_level);
            right_sample_buffer.insert(0, right_level);
        }
        println!("Level Meter Thread Exited");
    });
}

fn calculate_rms(samples: &mut Vec<f32>) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_of_squares: f32 = samples.iter().map(|&sample| sample * sample).sum();
    ((get_dbfs_from_rms((sum_of_squares / samples.len() as f32).sqrt()) * 10.00).floor()) / 10.0
}

fn get_dbfs_from_rms(sample: f32) -> f32 {
    20.0 * (sample.abs().log10())
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
