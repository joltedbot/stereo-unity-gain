use crate::devices::{CurrentDevice, DeviceList};
use crate::level_meter::LevelMeter;
use crate::tone_generator::ToneGenerator;
use slint::{ModelRc, PlatformError, SharedString, VecModel, Weak};
use std::error::Error;
use std::sync::{Arc, Mutex};

slint::include_modules!();

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
        input_device_list: DeviceList,
        current_input_device: CurrentDevice,
        output_device_list: DeviceList,
        current_output_device: CurrentDevice,
    ) -> Result<(), Box<dyn Error>> {
        self.ui
            .set_input_device_list(get_model_from_string_slice(&input_device_list.devices));
        self.ui
            .set_output_device_list(get_model_from_string_slice(&output_device_list.devices));

        self.ui.set_input_channel_list(get_model_from_string_slice(
            &input_device_list.channels[current_input_device.index as usize].clone(),
        ));

        self.ui.set_output_channel_list(get_model_from_string_slice(
            &output_device_list.channels[current_output_device.index as usize].clone(),
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

    pub fn create_ui_callbacks(
        &self,
        input_device_mutex: Arc<Mutex<LevelMeter>>,
        output_device_mutex: Arc<Mutex<ToneGenerator>>,
    ) {
        self.on_select_new_input_device_callback(input_device_mutex.clone());
        self.on_select_new_input_channel_callback(input_device_mutex.clone());

        self.on_select_new_output_device_callback(output_device_mutex.clone());
        self.on_select_new_output_channel_callback(output_device_mutex.clone());

        self.on_start_button_pressed_callback(
            input_device_mutex.clone(),
            output_device_mutex.clone(),
        );
        self.on_delta_mode_switch_toggled_callback(input_device_mutex.clone());
    }

    pub fn on_start_button_pressed_callback(
        &self,
        input_device_mutex: Arc<Mutex<LevelMeter>>,
        output_device_mutex: Arc<Mutex<ToneGenerator>>,
    ) {
        let ui_weak = self.ui.as_weak();

        self.ui.on_start_button_pressed(move |is_active| {
            match input_device_mutex.lock() {
                Ok(mut input_device) => match is_active {
                    true => {
                        if let Err(error) = input_device.start() {
                            handle_ui_error(&ui_weak, &error.to_string())
                        }
                    }
                    false => {
                        if let Err(error) = input_device.stop() {
                            handle_ui_error(&ui_weak, &error.to_string())
                        }
                    }
                },
                Err(error) => handle_ui_error(&ui_weak, &error.to_string()),
            }

            match output_device_mutex.lock() {
                Ok(mut output_device) => match is_active {
                    true => {
                        if let Err(error) = output_device.start() {
                            handle_ui_error(&ui_weak, &error.to_string())
                        }
                    }
                    false => {
                        if let Err(error) = output_device.stop() {
                            handle_ui_error(&ui_weak, &error.to_string())
                        }
                    }
                },
                Err(error) => handle_ui_error(&ui_weak, &error.to_string()),
            }
        });
    }

    pub fn on_select_new_input_device_callback(&self, input_device_mutex: Arc<Mutex<LevelMeter>>) {
        let ui_weak = self.ui.as_weak();

        self.ui.on_selected_input_device(move |index, device| {
            if let Ok(mut input_device) = input_device_mutex.lock() {
                let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);

                match input_device
                    .set_current_input_device_on_ui_callback((index, device.to_string()))
                {
                    Ok(_) => {
                        let current_input_device = input_device.get_current_input_device();
                        let input_device_list = get_model_from_string_slice(
                            input_device.get_current_input_device_channels().as_slice(),
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
                        if let Err(err) = input_device.reset_to_default_input_device() {
                            ui.set_error_message(SharedString::from(err.to_string()));
                            ui.set_error_dialog_visible(true);
                        }
                    }
                }
            }
        });
    }

    pub fn on_select_new_output_device_callback(
        &self,
        output_devices_mutex: Arc<Mutex<ToneGenerator>>,
    ) {
        let ui_weak = self.ui.as_weak();

        self.ui.on_selected_output_device(move |index, device| {
            if let Ok(mut output_device) = output_devices_mutex.lock() {
                let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);

                match output_device
                    .set_current_output_device_on_ui_callback((index, device.to_string()))
                {
                    Ok(_) => {
                        let current_output_device = output_device.get_current_output_device();
                        let output_device_list = get_model_from_string_slice(
                            output_device
                                .get_current_output_device_channels()
                                .as_slice(),
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
                        if let Err(err) = output_device.reset_to_default_output_device() {
                            ui.set_error_message(SharedString::from(err.to_string()));
                            ui.set_error_dialog_visible(true);
                        }
                    }
                }
            }
        });
    }

    pub fn on_select_new_input_channel_callback(&self, input_device_mutex: Arc<Mutex<LevelMeter>>) {
        let ui_weak = self.ui.as_weak();
        self.ui
            .on_selected_input_channel(move |left_channel, right_channel| {
                if let Ok(mut device) = input_device_mutex.lock() {
                    let left_input_channel = left_channel.to_string();

                    let right_input_channel = if right_channel.is_empty() {
                        None
                    } else {
                        Some(right_channel.to_string())
                    };

                    if device
                        .set_input_channel_on_ui_callback(left_input_channel, right_input_channel)
                        .is_err()
                    {
                        if let Err(err) = device.reset_to_default_input_device() {
                            let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);
                            ui.set_error_message(SharedString::from(err.to_string()));
                            ui.set_error_dialog_visible(true);
                        }
                    }
                }
            });
    }

    pub fn on_select_new_output_channel_callback(
        &self,
        output_devices_mutex: Arc<Mutex<ToneGenerator>>,
    ) {
        let ui_weak = self.ui.as_weak();
        self.ui
            .on_selected_output_channel(move |left_channel, right_channel| {
                if let Ok(mut device) = output_devices_mutex.lock() {
                    let left_output_channel = left_channel.to_string();
                    let right_output_channel = if right_channel.is_empty() {
                        None
                    } else {
                        Some(right_channel.to_string())
                    };

                    if device
                        .set_output_channel_on_ui_callback(
                            left_output_channel,
                            right_output_channel,
                        )
                        .is_err()
                    {
                        if let Err(err) = device.reset_to_default_output_device() {
                            let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);
                            ui.set_error_message(SharedString::from(err.to_string()));
                            ui.set_error_dialog_visible(true);
                        }
                    }
                }
            });
    }

    pub fn on_delta_mode_switch_toggled_callback(
        &self,
        input_devices_mutex: Arc<Mutex<LevelMeter>>,
    ) {
        let ui_weak = self.ui.as_weak();

        self.ui
            .on_delta_mode_checked(move |delta_mode_enabled| match input_devices_mutex.lock() {
                Ok(mut device) => {
                    let mode_sender = device.get_meter_mode_sender();
                    if let Err(error) = mode_sender.send(delta_mode_enabled) {
                        handle_ui_error(&ui_weak, &error.to_string());
                    }
                }
                Err(error) => {
                    handle_ui_error(&ui_weak, &error.to_string());
                }
            });
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

pub fn handle_ui_error(ui_weak: &Weak<AppWindow>, error_message: &str) {
    let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);
    ui.set_error_message(SharedString::from(error_message.to_string()));
    ui.set_error_dialog_visible(true);
}
