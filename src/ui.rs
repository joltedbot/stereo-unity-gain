use crate::devices::Devices;
use crate::errors::LocalError;
use slint::{ModelRc, PlatformError, SharedString, VecModel, Weak};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;

slint::include_modules!();

const NUMBER_OF_INPUT_BUFFERS_TO_USE_FOR_PEAK_CALCULATION: usize = 20;
const TARGET_OUTPUT_LEVEL: f32 = -12.0;
const DEFAULT_DELTA_MODE: bool = true;
const FATAL_ERROR_MESSAGE_UI_ERROR: &str =
    "A fatal error has occurred in the UI. The application will now exit.";

pub struct UI {
    pub ui: AppWindow,
}

impl UI {
    pub fn new() -> Result<Self, PlatformError> {
        let ui = Self {
            ui: AppWindow::new()?,
        };
        ui.setup_error_handling();
        Ok(ui)
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
            Err(err) => return Err(Box::new(LocalError::UIDeviceData(err.to_string()))),
        };

        let input_device_list = devices.get_input_device_list();
        let output_device_list = devices.get_output_device_list();
        let current_input_device = devices.get_current_input_device();
        let current_output_device = devices.get_current_output_device();

        self.ui
            .set_input_device_list(get_model_from_string_slice(&input_device_list.devices));
        self.ui
            .set_output_device_list(get_model_from_string_slice(&output_device_list.devices));

        self.ui.set_input_channel_list(get_model_from_string_slice(
            &input_device_list.channels[current_input_device.index as usize].clone(),
        ));

        self.ui.set_output_channel_list(get_model_from_string_slice(
            &devices.output_devices.output_device_list.channels
                [current_output_device.index as usize]
                .clone(),
        ));

        self.ui
            .set_current_output_device(SharedString::from(current_output_device.name.clone()));

        self.ui.set_left_current_output_channel(SharedString::from(
            current_output_device.left_channel.clone(),
        ));

        self.ui
            .set_current_input_device(SharedString::from(current_input_device.name.clone()));

        self.ui.set_left_current_input_channel(SharedString::from(
            current_input_device.left_channel.clone(),
        ));

        match current_output_device.right_channel {
            None => self.ui.set_right_output_enabled(false),
            Some(channel) => {
                self.ui.set_right_output_enabled(true);
                self.ui
                    .set_right_current_output_channel(SharedString::from(channel));
            }
        }

        match current_input_device.right_channel {
            None => self.ui.set_right_input_enabled(false),
            Some(channel) => {
                self.ui.set_right_input_enabled(true);
                self.ui
                    .set_right_current_input_channel(SharedString::from(channel));
            }
        }

        Ok(())
    }

    pub fn create_ui_callbacks(&self, devices_mutex: Arc<Mutex<Devices>>) {
        self.on_select_new_input_device_callback(devices_mutex.clone());

        self.on_select_new_output_device_callback(devices_mutex.clone());

        self.on_select_new_output_channel_callback(devices_mutex.clone());

        self.on_select_new_input_channel_callback(devices_mutex.clone());

        self.on_start_button_pressed_callback(devices_mutex.clone());

        self.on_delta_mode_switch_toggled_callback(devices_mutex.clone());
    }

    pub fn on_start_button_pressed_callback(&self, devices_mutex: Arc<Mutex<Devices>>) {
        let ui = self
            .ui
            .as_weak()
            .upgrade()
            .expect(FATAL_ERROR_MESSAGE_UI_ERROR);
        self.ui.on_start_button_pressed(move |is_active| {
            if let Ok(mut devices) = devices_mutex.lock() {
                let result = match is_active {
                    true => devices.start(),
                    false => devices.stop(),
                };

                if let Err(err) = result {
                    ui.set_error_message(SharedString::from(err.to_string()));
                    ui.set_error_dialog_visible(true);
                };
            };
        });
    }

    pub fn on_select_new_input_device_callback(&self, devices_mutex: Arc<Mutex<Devices>>) {
        let ui_weak = self.ui.as_weak();

        self.ui.on_selected_input_device(move |index, device| {
            if let Ok(mut devices) = devices_mutex.lock() {
                let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);

                match devices.set_current_input_device_on_ui_callback((index, device.to_string())) {
                    Ok(_) => {
                        let current_input_device = devices.get_current_input_device();
                        let input_device_list = get_model_from_string_slice(
                            devices.get_current_input_device_channels().as_slice(),
                        );

                        ui.set_input_channel_list(input_device_list);
                        ui.set_left_current_input_channel(SharedString::from(
                            current_input_device.left_channel.clone(),
                        ));

                        match current_input_device.right_channel {
                            None => ui.set_right_input_enabled(false),
                            Some(channel) => {
                                ui.set_right_input_enabled(true);
                                ui.set_right_current_input_channel(SharedString::from(channel));
                            }
                        }
                    }
                    Err(_) => {
                        if let Err(err) = devices.input_devices.reset_to_default_input_device() {
                            ui.set_error_message(SharedString::from(err.to_string()));
                            ui.set_error_dialog_visible(true);
                        }
                    }
                }
            }
        });
    }

    pub fn on_select_new_output_device_callback(&self, devices_mutex: Arc<Mutex<Devices>>) {
        let ui_weak = self.ui.as_weak();

        self.ui.on_selected_output_device(move |index, device| {
            if let Ok(mut devices) = devices_mutex.lock() {
                let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);

                match devices.set_current_output_device_on_ui_callback((index, device.to_string()))
                {
                    Ok(_) => {
                        let current_output_device = devices.get_current_output_device();
                        let output_device_list = get_model_from_string_slice(
                            devices.get_current_output_device_channels().as_slice(),
                        );
                        ui.set_output_channel_list(output_device_list);
                        ui.set_left_current_output_channel(SharedString::from(
                            current_output_device.left_channel.clone(),
                        ));

                        match current_output_device.right_channel {
                            None => ui.set_right_output_enabled(false),
                            Some(channel) => {
                                ui.set_right_output_enabled(true);
                                ui.set_right_current_output_channel(SharedString::from(channel));
                            }
                        }
                    }
                    Err(_) => {
                        if let Err(err) = devices.output_devices.reset_to_default_output_device() {
                            ui.set_error_message(SharedString::from(err.to_string()));
                            ui.set_error_dialog_visible(true);
                        }
                    }
                }
            }
        });
    }

    pub fn on_select_new_input_channel_callback(&self, devices_mutex: Arc<Mutex<Devices>>) {
        let ui_weak = self.ui.as_weak();
        self.ui
            .on_selected_input_channel(move |left_channel, right_channel| {
                if let Ok(mut devices) = devices_mutex.lock() {
                    let left_input_channel = left_channel.to_string();

                    let right_input_channel = if right_channel.is_empty() {
                        None
                    } else {
                        Some(right_channel.to_string())
                    };

                    if devices
                        .set_input_channel_on_ui_callback(left_input_channel, right_input_channel)
                        .is_err()
                    {
                        if let Err(err) = devices.input_devices.reset_to_default_input_device() {
                            let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);
                            ui.set_error_message(SharedString::from(err.to_string()));
                            ui.set_error_dialog_visible(true);
                        }
                    }
                }
            });
    }

    pub fn on_select_new_output_channel_callback(&self, devices_mutex: Arc<Mutex<Devices>>) {
        let ui_weak = self.ui.as_weak();
        self.ui
            .on_selected_output_channel(move |left_channel, right_channel| {
                if let Ok(mut devices) = devices_mutex.lock() {
                    let left_output_channel = left_channel.to_string();
                    let right_output_channel = if right_channel.is_empty() {
                        None
                    } else {
                        Some(right_channel.to_string())
                    };

                    if devices
                        .set_output_channel_on_ui_callback(
                            left_output_channel,
                            right_output_channel,
                        )
                        .is_err()
                    {
                        if let Err(err) = devices.output_devices.reset_to_default_output_device() {
                            let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);
                            ui.set_error_message(SharedString::from(err.to_string()));
                            ui.set_error_dialog_visible(true);
                        }
                    }
                }
            });
    }

    pub fn on_delta_mode_switch_toggled_callback(&self, devices_mutex: Arc<Mutex<Devices>>) {
        let ui_weak = self.ui.as_weak();

        self.ui
            .on_delta_mode_checked(move |delta_mode_enabled| match devices_mutex.lock() {
                Ok(mut devices) => {
                    let mode_sender = devices.get_meter_mode_sender();
                    match mode_sender.send(delta_mode_enabled) {
                        Ok(_) => devices.delta_mode_enabled = delta_mode_enabled,
                        Err(error) => handle_ui_error(&ui_weak, &error.to_string()),
                    }
                }
                Err(error) => {
                    handle_ui_error(&ui_weak, &error.to_string());
                }
            });
    }

    pub fn start_level_meter(
        &self,
        devices_mutex: Arc<Mutex<Devices>>,
    ) -> Result<(), Box<dyn Error>> {
        let ui_weak = self.ui.as_weak();
        let mut left_input_buffer_collection: Vec<Vec<f32>> = Vec::new();
        let mut right_input_buffer_collection: Vec<Vec<f32>> = Vec::new();
        let mut last_left_peak = 0.0;
        let mut last_right_peak = 0.0;

        let mut devices = devices_mutex
            .lock()
            .map_err(|error| LocalError::UIDeviceData(error.to_string()))?;

        let sample_receiver = devices.get_sample_buffer_receiver();
        let mode_receiver = devices.get_meter_mode_receiver();

        let mut delta_mode = DEFAULT_DELTA_MODE;

        thread::spawn(move || {
            while let Ok((left_samples, right_samples)) = sample_receiver.recv() {
                if let Ok(delta_mode_enabled) = mode_receiver.try_recv() {
                    delta_mode = delta_mode_enabled;
                };

                if left_input_buffer_collection.len()
                    > NUMBER_OF_INPUT_BUFFERS_TO_USE_FOR_PEAK_CALCULATION
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

                    let mut left = get_peak_of_sine_wave_samples(&mut left_samples_buffer);
                    let mut right = get_peak_of_sine_wave_samples(&mut right_samples_buffer);

                    if last_left_peak != left || last_right_peak != right {
                        last_left_peak = left;
                        last_right_peak = right;

                        if delta_mode {
                            left -= TARGET_OUTPUT_LEVEL;
                            right -= TARGET_OUTPUT_LEVEL;
                        }

                        let left_formatted = format_peak_delta_values_for_display(left);
                        let right_formatted = format_peak_delta_values_for_display(right);

                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_left_level_box_value(SharedString::from(left_formatted));
                            ui.set_right_level_box_value(SharedString::from(right_formatted));
                        });
                    }
                }

                left_input_buffer_collection.insert(0, left_samples);
                right_input_buffer_collection.insert(0, right_samples);
            }
        });

        Ok(())
    }

    fn setup_error_handling(&self) {
        let ui_weak = self.ui.as_weak();

        self.ui.on_close_error_dialog(move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.set_error_dialog_visible(false);
            });
        });
    }
}

pub fn get_model_from_string_slice(devices: &[String]) -> ModelRc<SharedString> {
    let name_list: Vec<SharedString> = devices.iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from_slice(name_list.as_slice()))
}

fn get_peak_of_sine_wave_samples(samples: &mut [f32]) -> f32 {
    let peak = samples.iter().fold(0.0f32, |acc, &x| x.abs().max(acc));
    get_dbfs_from_peak_sample(peak)
}

fn get_dbfs_from_peak_sample(sample: f32) -> f32 {
    20.0 * (sample.abs().log10())
}

fn format_peak_delta_values_for_display(peak_delta_value: f32) -> String {
    if peak_delta_value > 0.1 {
        format!("+{:.1}", peak_delta_value)
    } else if (peak_delta_value < 0.0) & (peak_delta_value > -0.1) {
        "0.0".to_string()
    } else if peak_delta_value == f32::NEG_INFINITY {
        "-".to_string()
    } else {
        format!("{:.1}", peak_delta_value)
    }
}

pub fn handle_ui_error(ui_weak: &Weak<AppWindow>, error_message: &str) {
    let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);
    ui.set_error_message(SharedString::from(error_message.to_string()));
    ui.set_error_dialog_visible(true);
}
