use crate::devices::Devices;
use crate::errors::LocalError;
use slint::{ModelRc, PlatformError, SharedString, VecModel};
use std::error::Error;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread;

slint::include_modules!();

const NUMBER_OF_INPUT_BUFFERS_TO_USE_FOR_RMS_CALCULATION: usize = 19;
const TARGET_OUTPUT_LEVEL: f32 = -12.0;

const FATAL_ERROR_MESSAGE_UI_ERROR: &str =
    "A fatal error has occurred in the UI. The application will now exit.";

pub struct UI {
    ui: AppWindow,
}

impl UI {
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self {
            ui: AppWindow::new()?,
        })
    }

    pub fn run(&mut self) -> Result<(), PlatformError> {
        self.ui.run()
    }

    pub fn initialize_ui_with_device_data(
        &mut self,
        devices_mutex: Arc<Mutex<Devices>>,
    ) -> Result<(), Box<dyn Error>> {
        let devices = match devices_mutex.lock() {
            Ok(devices) => devices,
            Err(_) => return Err(Box::new(LocalError::UIDeviceData)),
        };

        let display_data = devices.get_display_data();

        let input_device_model = get_model_from_string_slice(&display_data.input_device_list.0);
        self.ui.set_input_device_list(input_device_model);

        let output_device_model = get_model_from_string_slice(&display_data.output_device_list.0);
        self.ui.set_output_device_list(output_device_model);

        self.ui.set_input_channel_list(get_model_from_string_slice(
            &display_data.input_device_list.1[devices.active_input_device.0 as usize].clone(),
        ));

        self.ui.set_output_channel_list(get_model_from_string_slice(
            &display_data.output_device_list.1[devices.active_output_device.0 as usize].clone(),
        ));

        self.ui
            .set_current_output_device(SharedString::from(devices.active_output_device.1.clone()));
        self.ui.set_left_current_output_channel(SharedString::from(
            devices.active_output_device.2.clone(),
        ));
        self.ui
            .set_current_input_device(SharedString::from(devices.active_input_device.1.clone()));
        self.ui.set_left_current_input_channel(SharedString::from(
            devices.active_input_device.2.clone(),
        ));

        if devices.active_output_device.3.is_empty() {
            self.ui.set_right_output_enabled(false);
        } else {
            self.ui.set_right_output_enabled(true);
            self.ui.set_right_current_output_channel(SharedString::from(
                devices.active_output_device.3.clone(),
            ));
        }

        if devices.active_input_device.3.is_empty() {
            self.ui.set_right_input_enabled(false);
        } else {
            self.ui.set_right_input_enabled(true);
            self.ui.set_right_current_input_channel(SharedString::from(
                devices.active_input_device.3.clone(),
            ));
        }

        // INPUT DEVICE CHANGE CALLBACK
        self.on_select_new_input_device_callback(devices_mutex.clone());

        // OUTPUT DEVICE CHANGE CALLBACK
        self.on_select_new_output_device_callback(devices_mutex.clone());

        // OUTPUT CHANNEL CHANGE CALLBACK
        self.on_select_new_output_channel_callback(devices_mutex.clone());

        // INPUT CHANNEL CHANGE CALLBACK
        self.on_select_new_input_channel_callback(devices_mutex.clone());

        // Set up the callback for the start stop button
        self.on_start_button_pressed_callback(devices_mutex.clone());

        // Set up the callback for the fatal error dialog box
        self.ui.on_fatal_error_clicked(|| {
            exit(1);
        });

        Ok(())
    }

    pub fn call_fatal_error(&self, message: &str) {
        self.ui.set_fatal_error_message(SharedString::from(message));
        self.ui.set_fatal_error_visable(true);
    }

    pub fn on_start_button_pressed_callback(&self, devices_mutex: Arc<Mutex<Devices>>) {
        self.ui.on_start_button_pressed(move |is_active| {
            if let Ok(mut devices) = devices_mutex.lock() {
                match is_active {
                    true => devices.start(),
                    false => devices.stop(),
                }
            };
        });
    }

    pub fn on_select_new_input_device_callback(&self, devices_mutex: Arc<Mutex<Devices>>) {
        let ui_weak = self.ui.as_weak();

        self.ui.on_selected_input_device(move |index, device| {
            if let Ok(mut devices) = devices_mutex.lock() {
                devices.set_current_input_device_on_ui_callback((index, device.to_string()));

                let app_weak = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);

                let input_device_list = get_model_from_string_slice(
                    devices.get_current_input_device_channels().as_slice(),
                );

                app_weak.set_input_channel_list(input_device_list);
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
    }

    pub fn on_select_new_output_device_callback(&self, devices_mutex: Arc<Mutex<Devices>>) {
        let ui_weak = self.ui.as_weak();

        self.ui.on_selected_output_device(move |index, device| {
            if let Ok(mut devices) = devices_mutex.lock() {
                let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);

                devices.set_current_output_device_on_ui_callback((index, device.to_string()));

                let output_device_list = get_model_from_string_slice(
                    devices.get_current_output_device_channels().as_slice(),
                );
                ui.set_output_channel_list(output_device_list);
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
            }
        });
    }

    pub fn on_select_new_input_channel_callback(&self, devices_mutex: Arc<Mutex<Devices>>) {
        self.ui
            .on_selected_input_channel(move |left_channel, right_channel| {
                if let Ok(mut devices) = devices_mutex.lock() {
                    devices.set_input_channel_on_ui_callback(
                        left_channel.to_string(),
                        right_channel.to_string(),
                    );
                }
            });
    }

    pub fn on_select_new_output_channel_callback(&self, devices_mutex: Arc<Mutex<Devices>>) {
        self.ui
            .on_selected_output_channel(move |left_channel, right_channel| {
                if let Ok(mut devices) = devices_mutex.lock() {
                    devices.set_output_channel_on_ui_callback(
                        left_channel.to_string(),
                        right_channel.to_string(),
                    );
                }
            });
    }

    pub fn update_level_meter_values_in_the_ui(
        &self,
        devices_mutex: Arc<Mutex<Devices>>,
    ) -> Result<(), Box<dyn Error>> {
        let meter_reader = match devices_mutex.lock() {
            Ok(mut reader) => reader.get_meter_reader(),
            Err(_) => return Err(Box::new(LocalError::UIDeviceData)),
        };

        let ui_weak = self.ui.as_weak();
        let mut left_input_buffer_collection: Vec<Vec<f32>> = Vec::new();
        let mut right_input_buffer_collection: Vec<Vec<f32>> = Vec::new();
        let mut last_left_rms = 0.0;
        let mut last_right_rms = 0.0;

        thread::spawn(move || {
            while let Ok((left_samples, right_samples)) = meter_reader.recv() {
                left_input_buffer_collection.insert(0, left_samples);
                right_input_buffer_collection.insert(0, right_samples);

                if left_input_buffer_collection.len()
                    > NUMBER_OF_INPUT_BUFFERS_TO_USE_FOR_RMS_CALCULATION
                {
                    let mut left_samples_buffer: Vec<f32> = left_input_buffer_collection
                        .iter()
                        .flatten()
                        .copied()
                        .collect();

                    left_input_buffer_collection.truncate(0);

                    let mut right_samples_buffer: Vec<f32> = right_input_buffer_collection
                        .iter()
                        .flatten()
                        .copied()
                        .collect();

                    right_input_buffer_collection.truncate(0);

                    let left = get_rms_of_sine_wave_samples(&mut left_samples_buffer);
                    let right = get_rms_of_sine_wave_samples(&mut right_samples_buffer);

                    if last_left_rms != left || last_right_rms != right {
                        last_left_rms = left;
                        last_right_rms = right;

                        let left_delta = left - TARGET_OUTPUT_LEVEL;
                        let right_delta = right - TARGET_OUTPUT_LEVEL;

                        // Format the values for display
                        let left_formatted = format_rms_delta_values_for_display(left_delta);
                        let right_formatted = format_rms_delta_values_for_display(right_delta);

                        // Update UI safely on the main thread
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_left_level_box_value(SharedString::from(left_formatted));
                            ui.set_right_level_box_value(SharedString::from(right_formatted));
                        });
                    }
                }
            }
        });

        Ok(())
    }
}

pub fn get_model_from_string_slice(devices: &[String]) -> ModelRc<SharedString> {
    let name_list: Vec<SharedString> = devices.iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from_slice(name_list.as_slice()))
}

fn get_rms_of_sine_wave_samples(samples: &mut [f32]) -> f32 {
    let peak = samples
        .iter()
        .fold(0.0, |acc, &x| if x.abs() > acc { x } else { acc });
    get_dbfs_from_rms(peak)
}

fn get_dbfs_from_rms(sample: f32) -> f32 {
    20.0 * (sample.abs().log10())
}

fn format_rms_delta_values_for_display(rms_delta_value: f32) -> String {
    if rms_delta_value > 0.1 {
        format!("+{:.1}", rms_delta_value)
    } else if (rms_delta_value < 0.0) & (rms_delta_value > -0.1) {
        "0.0".to_string()
    } else {
        format!("{:.1}", rms_delta_value)
    }
}
