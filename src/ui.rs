use crate::device_manager::{CurrentDevice, DeviceList};
use crate::errors::LocalError;
use crate::events::EventType;
use crossbeam_channel::{Receiver, Sender};
use slint::{ModelRc, SharedString, VecModel, Weak};
use std::error::Error;
use std::process::exit;
use std::sync::{Arc, Mutex};

const FATAL_ERROR_MESSAGE_UI_ERROR: &str =
    "A fatal error has occurred in the UI. The application will now exit.";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const LICENSE: &str = env!("CARGO_PKG_LICENSE");

slint::include_modules!();

pub struct UI {
    pub ui: Weak<AppWindow>,
    level_meter_sender: Sender<EventType>,
    tone_generator_sender: Sender<EventType>,
    input_device_list: DeviceList,
    output_device_list: DeviceList,
    current_input_device: CurrentDevice,
    current_output_device: CurrentDevice,
}

impl UI {
    pub fn new(
        ui_mutex: Arc<Mutex<Weak<AppWindow>>>,
        tone_generator_sender: Sender<EventType>,
        level_meter_sender: Sender<EventType>,
    ) -> Result<Self, Box<dyn Error>> {
        let ui_weak = match ui_mutex.lock() {
            Ok(ui_guard) => ui_guard.clone(),
            Err(_) => {
                return Err(Box::new(LocalError::UIInitialization));
            }
        };

        let ui = Self {
            ui: ui_weak,
            tone_generator_sender,
            level_meter_sender,
            input_device_list: DeviceList::default(),
            output_device_list: DeviceList::default(),
            current_input_device: CurrentDevice::default(),
            current_output_device: CurrentDevice::default(),
        };

        Ok(ui)
    }

    pub fn run(
        &mut self,
        level_meter_display_receiver: Receiver<EventType>,
    ) -> Result<(), Box<dyn Error>> {
        let ui_weak = self.ui.clone();

        loop {
            if let Ok(event) = level_meter_display_receiver.recv() {
                match event {
                    EventType::MeterLevelUpdate { left, right } => {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_left_level_box_value(SharedString::from(left));
                            ui.set_right_level_box_value(SharedString::from(right));
                        });
                    }
                    EventType::FatalError(error) => {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            if !ui.get_error_dialog_visible() {
                                ui.set_error_message(SharedString::from(error.to_string()));
                                ui.set_error_dialog_visible(true);
                            }
                        });
                    }
                    EventType::Exit => {
                        break;
                    }
                    _ => (),
                }
            }
        }

        Ok(())
    }

    pub fn initialize_ui_with_device_data(
        &mut self,
        input_device_list: DeviceList,
        current_input_device: CurrentDevice,
        output_device_list: DeviceList,
        current_output_device: CurrentDevice,
        reference_frequency: f32,
        reference_level: i32,
    ) -> Result<(), Box<dyn Error>> {
        self.input_device_list = input_device_list;
        self.output_device_list = output_device_list;
        self.current_input_device = current_input_device;
        self.current_output_device = current_output_device;

        let ui_weak = self.ui.clone();

        let left_input_channel = self.current_input_device.left_channel.clone();
        let right_input_channel = self.current_input_device.right_channel.clone();
        let left_output_channel = self.current_output_device.left_channel.clone();
        let right_output_channel = self.current_output_device.right_channel.clone();
        let input_device_list = self.input_device_list.clone();
        let output_device_list = self.output_device_list.clone();
        let current_input_device = self.current_input_device.clone();
        let current_output_device = self.current_output_device.clone();

        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_version_number(SharedString::from(VERSION.to_string()));

            ui.set_description(SharedString::from(DESCRIPTION.to_string()));

            ui.set_license(SharedString::from(LICENSE.to_string()));

            ui.set_input_device_list(get_model_from_string_slice(&input_device_list.devices));
            ui.set_output_device_list(get_model_from_string_slice(&output_device_list.devices));

            ui.set_input_channel_list(get_model_from_string_slice(
                &input_device_list.channels[current_input_device.index as usize].clone(),
            ));

            ui.set_output_channel_list(get_model_from_string_slice(
                &output_device_list.channels[current_output_device.index as usize].clone(),
            ));

            ui.set_reference_frequency(reference_frequency);
            ui.set_reference_level(reference_level);

            ui.set_current_output_device(SharedString::from(current_output_device.name.clone()));

            ui.set_left_current_output_channel(SharedString::from(left_output_channel));

            ui.set_current_input_device(SharedString::from(current_input_device.name.clone()));

            ui.set_left_current_input_channel(SharedString::from(left_input_channel));

            match &right_output_channel {
                None => ui.set_right_output_enabled(false),
                Some(channel) => {
                    ui.set_right_output_enabled(true);
                    ui.set_right_current_output_channel(SharedString::from(channel));
                }
            }

            match &right_input_channel {
                None => {
                    ui.set_right_input_enabled(false);
                    ui.set_right_level_box_enabled(false);
                }
                Some(channel) => {
                    ui.set_right_input_enabled(true);
                    ui.set_right_level_box_enabled(true);
                    ui.set_right_current_input_channel(SharedString::from(channel));
                }
            }
        });

        Ok(())
    }

    pub fn create_ui_callbacks(&self) {
        self.on_close_error_dialog();

        self.on_select_new_input_device_callback();
        self.on_select_new_input_channel_callback();

        self.on_select_new_output_device_callback();
        self.on_select_new_output_channel_callback();

        self.on_start_button_pressed_callback();
        self.on_delta_mode_switch_toggled_callback();

        self.on_reference_tone_frequency_changed_callback();

        self.on_reference_tone_level_changed_callback();

        self.on_tone_mode_updated_callback();
    }

    pub fn on_start_button_pressed_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => {
                eprintln!("Start Button Callback: {}", FATAL_ERROR_MESSAGE_UI_ERROR);
                exit(1);
            }
        };

        let level_meter_sender = self.level_meter_sender.clone();
        let tone_generator_sender = self.tone_generator_sender.clone();

        ui.on_start_button_pressed(move |is_active| {
            let event_type = match is_active {
                true => EventType::Start,
                false => EventType::Stop,
            };

            if let Err(error) = level_meter_sender.send(event_type.clone()) {
                eprintln!("Error sending event: {}", error);
                handle_ui_error(&ui_weak, &error.to_string());
            };

            if let Err(error) = tone_generator_sender.send(event_type.clone()) {
                eprintln!("Error sending event: {}", error);
                handle_ui_error(&ui_weak, &error.to_string());
            };
        });
    }

    pub fn on_select_new_input_device_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => {
                eprintln!(
                    "New Input Device Callback: {}",
                    FATAL_ERROR_MESSAGE_UI_ERROR
                );
                exit(1);
            }
        };

        let level_meter_sender = self.level_meter_sender.clone();
        let input_device_list = self.input_device_list.clone();

        ui.on_selected_input_device(move |index, device| {
            let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);
            if let Err(error) = level_meter_sender.send(EventType::MeterDeviceUpdate {
                index,
                name: device.to_string(),
            }) {
                handle_ui_error(&ui_weak, &error.to_string());
            };

            let input_channels = &input_device_list.channels[index as usize];
            let left_channel = input_channels[0].clone();

            if input_channels.len() > 1 {
                ui.set_right_input_enabled(true);
                ui.set_right_level_box_enabled(true);
                ui.set_right_current_input_channel(SharedString::from(input_channels[1].clone()));
            } else {
                ui.set_right_input_enabled(false);
                ui.set_right_level_box_enabled(false);
            }

            let input_channel_model = get_model_from_string_slice(input_channels);
            ui.set_input_channel_list(input_channel_model);
            ui.set_left_current_input_channel(SharedString::from(left_channel));
        });
    }

    pub fn on_select_new_output_device_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => {
                eprintln!("New Output Callback: {}", FATAL_ERROR_MESSAGE_UI_ERROR);
                exit(1);
            }
        };

        let tone_generator_sender = self.tone_generator_sender.clone();
        let output_device_list = self.output_device_list.clone();

        ui.on_selected_output_device(move |index, device| {
            let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);
            if let Err(error) = tone_generator_sender.send(EventType::ToneDeviceUpdate {
                index,
                name: device.to_string(),
            }) {
                handle_ui_error(&ui_weak, &error.to_string());
            };

            let output_channels = &output_device_list.channels[index as usize];
            let left_channel = output_channels[0].clone();

            if output_channels.len() > 1 {
                ui.set_right_output_enabled(true);
                ui.set_right_level_box_enabled(true);
                ui.set_right_current_output_channel(SharedString::from(output_channels[1].clone()));
            } else {
                ui.set_right_output_enabled(false);
                ui.set_right_level_box_enabled(false);
            }

            let output_channel_model = get_model_from_string_slice(output_channels);
            ui.set_output_channel_list(output_channel_model);
            ui.set_left_current_output_channel(SharedString::from(left_channel));
        });
    }

    pub fn on_select_new_input_channel_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => {
                eprintln!(
                    "New Input Channel Callback: {}",
                    FATAL_ERROR_MESSAGE_UI_ERROR
                );
                exit(1);
            }
        };

        let input_channel_sender = self.level_meter_sender.clone();

        ui.on_selected_input_channel(move |left_channel, right_channel| {
            let left_input_channel = left_channel.to_string();
            let right_input_channel = if right_channel.is_empty() {
                None
            } else {
                Some(right_channel.to_string())
            };

            if let Err(error) = input_channel_sender.send(EventType::MeterChannelUpdate {
                left: left_input_channel,
                right: right_input_channel,
            }) {
                handle_ui_error(&ui_weak, &error.to_string());
            };
        });
    }

    pub fn on_select_new_output_channel_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => {
                eprintln!(
                    "New Output Channel Callback: {}",
                    FATAL_ERROR_MESSAGE_UI_ERROR
                );
                exit(1);
            }
        };

        let output_channel_sender = self.tone_generator_sender.clone();

        ui.on_selected_output_channel(move |left_channel, right_channel| {
            let left_output_channel = left_channel.to_string();
            let right_output_channel = if right_channel.is_empty() {
                None
            } else {
                Some(right_channel.to_string())
            };

            if let Err(error) = output_channel_sender.send(EventType::ToneChannelUpdate {
                left: left_output_channel,
                right: right_output_channel,
            }) {
                handle_ui_error(&ui_weak, &error.to_string());
            };
        });
    }

    fn on_reference_tone_frequency_changed_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => {
                eprintln!(
                    "Tone Frequency Change Callback: {}",
                    FATAL_ERROR_MESSAGE_UI_ERROR
                );
                exit(1);
            }
        };

        let reference_tone_sender = self.tone_generator_sender.clone();

        ui.on_tone_frequency_changed(move |frequency| {
            if let Err(error) =
                reference_tone_sender.send(EventType::ToneFrequencyUpdate(frequency))
            {
                handle_ui_error(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_tone_mode_updated_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => {
                eprintln!(
                    "Tone Mode Updated Callback: {}",
                    FATAL_ERROR_MESSAGE_UI_ERROR
                );
                exit(1);
            }
        };

        let tone_generator_sender = self.tone_generator_sender.clone();

        ui.on_tone_mode_checked(move |sine_mode_enabled| {
            if let Err(error) =
                tone_generator_sender.send(EventType::ToneModeUpdate(sine_mode_enabled))
            {
                handle_ui_error(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_reference_tone_level_changed_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => {
                eprintln!(
                    "Tone Level Change Callback: {}",
                    FATAL_ERROR_MESSAGE_UI_ERROR
                );
                exit(1);
            }
        };

        let level_meter_sender = self.level_meter_sender.clone();
        let tone_generator_sender = self.tone_generator_sender.clone();

        ui.on_tone_level_changed(move |level| {
            if let Err(error) = level_meter_sender.send(EventType::ToneLevelUpdate(level as f32)) {
                handle_ui_error(&ui_weak, &error.to_string());
            }
            if let Err(error) = tone_generator_sender.send(EventType::ToneLevelUpdate(level as f32))
            {
                handle_ui_error(&ui_weak, &error.to_string());
            }
        });
    }

    pub fn on_delta_mode_switch_toggled_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => {
                eprintln!(
                    "Delta Mode Toggled Callback: {}",
                    FATAL_ERROR_MESSAGE_UI_ERROR
                );
                exit(1);
            }
        };

        let mode_sender = self.level_meter_sender.clone();

        ui.on_delta_mode_checked(move |delta_mode_enabled| {
            if let Err(error) = mode_sender.send(EventType::MeterModeUpdate(delta_mode_enabled)) {
                handle_ui_error(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_close_error_dialog(&self) {
        let ui_weak = self.ui.clone();
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => {
                eprintln!("Close Dialog Callback: {}", FATAL_ERROR_MESSAGE_UI_ERROR);
                exit(1);
            }
        };

        ui.on_close_error_dialog(move || {
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
