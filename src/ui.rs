use crate::device_manager::{CurrentDevice, DeviceList};
use crate::errors::LocalError;
use crossbeam_channel::{Receiver, Sender, unbounded};
use slint::{ModelRc, PlatformError, SharedString, VecModel, Weak};
use std::error::Error;

const FATAL_ERROR_MESSAGE_UI_ERROR: &str =
    "A fatal error has occurred in the UI. The application will now exit.";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

pub const LICENSE: &str = env!("CARGO_PKG_LICENSE");

slint::include_modules!();

#[derive(Debug, Clone)]
pub enum EventType {
    MeterLevelUpdate { left: String, right: String },
    MeterModeUpdate(bool),
    MeterDeviceUpdate { index: i32, name: String },
    MeterChannelUpdate { left: String, right: Option<String> },
    ToneFrequencyUpdate(f32),
    ToneLevelUpdate(f32),
    ToneDeviceUpdate { index: i32, name: String },
    ToneChannelUpdate { left: String, right: Option<String> },
    ToneModeUpdate(bool),
    FatalError(LocalError),
    Start,
    Stop,
    Exit,
}

pub struct UI {
    pub ui: AppWindow,
    level_meter_receiver: Receiver<EventType>,
    level_meter_sender: Sender<EventType>,
    tone_generator_receiver: Receiver<EventType>,
    tone_generator_sender: Sender<EventType>,
    input_device_list: DeviceList,
    output_device_list: DeviceList,
    current_input_device: CurrentDevice,
    current_output_device: CurrentDevice,
}

impl UI {
    pub fn new() -> Result<Self, PlatformError> {
        let (meter_sender, meter_receiver) = unbounded();
        let level_meter_receiver = meter_receiver.clone();
        let level_meter_sender = meter_sender;

        let (tone_sender, tone_receiver) = unbounded();
        let tone_generator_receiver = tone_receiver.clone();
        let tone_generator_sender = tone_sender;

        let ui = Self {
            ui: AppWindow::new()?,
            level_meter_receiver,
            level_meter_sender,
            tone_generator_receiver,
            tone_generator_sender,
            input_device_list: DeviceList::default(),
            output_device_list: DeviceList::default(),
            current_input_device: CurrentDevice::default(),
            current_output_device: CurrentDevice::default(),
        };

        Ok(ui)
    }

    pub fn run(&mut self) -> Result<(), PlatformError> {
        self.ui.run()
    }

    pub fn get_level_meter_receiver(&self) -> Receiver<EventType> {
        self.level_meter_receiver.clone()
    }

    pub fn get_tone_generator_receiver(&self) -> Receiver<EventType> {
        self.tone_generator_receiver.clone()
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

        self.ui
            .set_version_number(SharedString::from(VERSION.to_string()));

        self.ui
            .set_description(SharedString::from(DESCRIPTION.to_string()));

        self.ui.set_license(SharedString::from(LICENSE.to_string()));

        self.ui
            .set_input_device_list(get_model_from_string_slice(&self.input_device_list.devices));
        self.ui.set_output_device_list(get_model_from_string_slice(
            &self.output_device_list.devices,
        ));

        self.ui.set_input_channel_list(get_model_from_string_slice(
            &self.input_device_list.channels[self.current_input_device.index as usize].clone(),
        ));

        self.ui.set_output_channel_list(get_model_from_string_slice(
            &self.output_device_list.channels[self.current_output_device.index as usize].clone(),
        ));

        self.ui.set_reference_frequency(reference_frequency);
        self.ui.set_reference_level(reference_level);

        self.ui
            .set_current_output_device(SharedString::from(self.current_output_device.name.clone()));

        self.ui.set_left_current_output_channel(SharedString::from(
            self.current_output_device.left_channel.clone(),
        ));

        self.ui
            .set_current_input_device(SharedString::from(self.current_input_device.name.clone()));

        self.ui.set_left_current_input_channel(SharedString::from(
            self.current_input_device.left_channel.clone(),
        ));

        match &self.current_output_device.right_channel {
            None => self.ui.set_right_output_enabled(false),
            Some(channel) => {
                self.ui.set_right_output_enabled(true);
                self.ui
                    .set_right_current_output_channel(SharedString::from(channel));
            }
        }

        match &self.current_input_device.right_channel {
            None => self.ui.set_right_input_enabled(false),
            Some(channel) => {
                self.ui.set_right_input_enabled(true);
                self.ui
                    .set_right_current_input_channel(SharedString::from(channel));
            }
        }

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
        let ui_weak = self.ui.as_weak();
        let level_meter_sender = self.level_meter_sender.clone();
        let tone_generator_sender = self.tone_generator_sender.clone();

        self.ui.on_start_button_pressed(move |is_active| {
            let event_type = match is_active {
                true => EventType::Start,
                false => EventType::Stop,
            };

            if let Err(error) = level_meter_sender.send(event_type.clone()) {
                println!("Error sending event: {}", error);
                handle_ui_error(&ui_weak, &error.to_string());
            };

            if let Err(error) = tone_generator_sender.send(event_type.clone()) {
                println!("Error sending event: {}", error);
                handle_ui_error(&ui_weak, &error.to_string());
            };
        });
    }

    pub fn on_select_new_input_device_callback(&self) {
        let ui_weak = self.ui.as_weak();
        let input_device_sender = self.level_meter_sender.clone();
        let input_device_list = self.input_device_list.clone();

        self.ui.on_selected_input_device(move |index, device| {
            let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);
            if let Err(error) = input_device_sender.send(EventType::MeterDeviceUpdate {
                index,
                name: device.to_string(),
            }) {
                handle_ui_error(&ui_weak, &error.to_string());
            };

            let input_channels = &input_device_list.channels[index as usize];
            let left_channel = input_channels[0].clone();

            if input_channels.len() > 1 {
                ui.set_right_input_enabled(true);
                ui.set_right_current_input_channel(SharedString::from(input_channels[1].clone()));
            } else {
                ui.set_right_input_enabled(false);
            }

            let input_channel_model = get_model_from_string_slice(input_channels);
            ui.set_input_channel_list(input_channel_model);
            ui.set_left_current_input_channel(SharedString::from(left_channel));
        });
    }

    pub fn on_select_new_output_device_callback(&self) {
        let ui_weak = self.ui.as_weak();
        let output_device_sender = self.tone_generator_sender.clone();
        let output_device_list = self.output_device_list.clone();

        self.ui.on_selected_output_device(move |index, device| {
            let ui = ui_weak.upgrade().expect(FATAL_ERROR_MESSAGE_UI_ERROR);
            if let Err(error) = output_device_sender.send(EventType::ToneDeviceUpdate {
                index,
                name: device.to_string(),
            }) {
                handle_ui_error(&ui_weak, &error.to_string());
            };

            let output_channels = &output_device_list.channels[index as usize];
            let left_channel = output_channels[0].clone();

            if output_channels.len() > 1 {
                ui.set_right_output_enabled(true);
                ui.set_right_current_output_channel(SharedString::from(output_channels[1].clone()));
            } else {
                ui.set_right_output_enabled(false);
            }

            let output_channel_model = get_model_from_string_slice(output_channels);
            ui.set_output_channel_list(output_channel_model);
            ui.set_left_current_output_channel(SharedString::from(left_channel));
        });
    }

    pub fn on_select_new_input_channel_callback(&self) {
        let ui_weak = self.ui.as_weak();
        let input_channel_sender = self.level_meter_sender.clone();

        self.ui
            .on_selected_input_channel(move |left_channel, right_channel| {
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
        let ui_weak = self.ui.as_weak();
        let output_channel_sender = self.tone_generator_sender.clone();

        self.ui
            .on_selected_output_channel(move |left_channel, right_channel| {
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
        let ui_weak = self.ui.as_weak();
        let reference_tone_sender = self.tone_generator_sender.clone();

        self.ui.on_tone_frequency_changed(move |frequency| {
            if let Err(error) =
                reference_tone_sender.send(EventType::ToneFrequencyUpdate(frequency))
            {
                handle_ui_error(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_tone_mode_updated_callback(&self) {
        let ui_weak = self.ui.as_weak();
        let tone_generator_sender = self.tone_generator_sender.clone();

        self.ui.on_tone_mode_checked(move |sine_mode_enabled| {
            if let Err(error) =
                tone_generator_sender.send(EventType::ToneModeUpdate(sine_mode_enabled))
            {
                handle_ui_error(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_reference_tone_level_changed_callback(&self) {
        let ui_weak = self.ui.as_weak();
        let level_meter_sender = self.level_meter_sender.clone();
        let tone_generator_sender = self.tone_generator_sender.clone();

        self.ui.on_tone_level_changed(move |level| {
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
        let ui_weak = self.ui.as_weak();
        let mode_sender = self.level_meter_sender.clone();

        self.ui.on_delta_mode_checked(move |delta_mode_enabled| {
            if let Err(error) = mode_sender.send(EventType::MeterModeUpdate(delta_mode_enabled)) {
                handle_ui_error(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_close_error_dialog(&self) {
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
