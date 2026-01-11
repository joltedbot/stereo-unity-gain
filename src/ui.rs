use super::AppWindow;
use crate::device_manager::{CurrentDevice, DeviceList};
use crate::errors::{EXIT_CODE_ERROR, LocalError};
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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct State {
    meter_delta_mode_active: bool,
    reference_level: i32,
}

pub struct UI {
    pub ui: Weak<AppWindow>,
    level_meter_sender: Sender<EventType>,
    tone_generator_sender: Sender<EventType>,
    user_interface_sender: Sender<EventType>,
    input_device_list: DeviceList,
    output_device_list: DeviceList,
    current_input_device: CurrentDevice,
    current_output_device: CurrentDevice,
    state: Arc<Mutex<State>>,
}

impl UI {
    pub fn new(
        ui_mutex: &Arc<Mutex<Weak<AppWindow>>>,
        tone_generator_sender: Sender<EventType>,
        level_meter_sender: Sender<EventType>,
        user_interface_sender: Sender<EventType>,
    ) -> Self {
        let ui_weak_mutex = ui_mutex
            .lock()
            .unwrap_or_else(|poisoned| {
                poisoned.into_inner()
            });

        Self {
            ui: ui_weak_mutex.clone(),
            tone_generator_sender,
            level_meter_sender,
            user_interface_sender,
            input_device_list: DeviceList::default(),
            output_device_list: DeviceList::default(),
            current_input_device: CurrentDevice::default(),
            current_output_device: CurrentDevice::default(),
            state: Arc::new(Mutex::new(State::default())),
        }
    }

    pub fn run(
        &mut self,
        level_meter_display_receiver: &Receiver<EventType>,
    ) -> Result<(), Box<dyn Error>> {
        let ui_weak = self.ui.clone();

        let state_arc = self.state.clone();

        loop {
            if let Ok(event) = level_meter_display_receiver.recv() {
                match event {
                    EventType::MeterLevelUpdate {
                        mut left,
                        mut right,
                    } => {
                        let state = state_arc
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);

                        if state.meter_delta_mode_active {
                            left -= state.reference_level as f32;
                            right -= state.reference_level as f32;
                        }

                        let left_formatted = format_peak_delta_values_for_display(left);
                        let right_formatted = format_peak_delta_values_for_display(right);

                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_left_level_box_value(SharedString::from(left_formatted));
                            ui.set_right_level_box_value(SharedString::from(right_formatted));
                        });
                    }
                    EventType::RecoverableError(error) => {
                        handle_error_in_ui(&ui_weak, error.as_str());
                    }
                    EventType::FatalError(error) => {
                        handle_fatal_error_in_ui(&ui_weak, error.as_str());
                    }
                    EventType::InputDeviceUpdate(device_name) => {
                        self.update_current_input_device(device_name.clone())?;
                        let level_meter_sender = self.level_meter_sender.clone();

                        if let Err(error) = level_meter_sender.send(EventType::MeterDeviceUpdate {
                            name: device_name,
                            left: self.current_input_device.left_channel.clone(),
                            right: self.current_input_device.right_channel.clone(),
                        }) {
                            handle_error_in_ui(&ui_weak, &error.to_string());
                        }
                    }
                    EventType::OutputDeviceUpdate(device_name) => {
                        self.update_current_output_device(device_name.clone())?;
                        let tone_generator_sender = self.tone_generator_sender.clone();

                        if let Err(error) =
                            tone_generator_sender.send(EventType::ToneDeviceUpdate {
                                name: device_name,
                                left: self.current_output_device.left_channel.clone(),
                                right: self.current_output_device.right_channel.clone(),
                            })
                        {
                            handle_error_in_ui(&ui_weak, &error.to_string());
                        }
                    }
                    EventType::InputChannelUpdate { left, right } => {
                        self.current_input_device.left_channel = left.clone();
                        self.current_input_device.right_channel = right.clone();

                        if let Err(error) =
                            self.level_meter_sender.send(EventType::MeterDeviceUpdate {
                                name: self.current_input_device.name.clone(),
                                left,
                                right,
                            })
                        {
                            handle_error_in_ui(&ui_weak, &error.to_string());
                        }
                    }
                    EventType::OutputChannelUpdate { left, right } => {
                        self.current_output_device.left_channel = left.clone();
                        self.current_output_device.right_channel = right.clone();

                        if let Err(error) =
                            self.tone_generator_sender
                                .send(EventType::ToneDeviceUpdate {
                                    name: self.current_output_device.name.clone(),
                                    left: left.clone(),
                                    right: right.clone(),
                                })
                        {
                            handle_error_in_ui(&ui_weak, &error.to_string());
                        }
                    }
                    EventType::InputDeviceListUpdate(input_device_list) => {
                        self.input_device_list = input_device_list.clone();

                        if !self
                            .input_device_list
                            .devices
                            .contains(&self.current_input_device.name)
                        {
                            self.send_stop_all();

                            let device_name = self.input_device_list.devices[0].clone();
                            self.update_current_input_device(device_name.clone())?;

                            let level_meter_sender = self.level_meter_sender.clone();

                            if let Err(error) =
                                level_meter_sender.send(EventType::MeterDeviceUpdate {
                                    name: device_name,
                                    left: self.current_input_device.left_channel.clone(),
                                    right: self.current_input_device.right_channel.clone(),
                                })
                            {
                                handle_error_in_ui(&ui_weak, &error.to_string());
                            }
                        }

                        self.initialize_displayed_input_device_data()?;
                    }
                    EventType::OutputDeviceListUpdate(output_device_list) => {
                        self.output_device_list = output_device_list.clone();

                        if !self
                            .output_device_list
                            .devices
                            .contains(&self.current_output_device.name)
                        {
                            self.send_stop_all();

                            let device_name = self.output_device_list.devices[0].clone();
                            self.update_current_output_device(device_name.clone())?;

                            let tone_generator_sender = self.tone_generator_sender.clone();
                            if let Err(error) =
                                tone_generator_sender.send(EventType::ToneDeviceUpdate {
                                    name: device_name,
                                    left: self.current_output_device.left_channel.clone(),
                                    right: self.current_output_device.right_channel.clone(),
                                })
                            {
                                handle_error_in_ui(&ui_weak, &error.to_string());
                            }
                        }

                        self.initialize_displayed_output_device_data()?;
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
        current_input_device: CurrentDevice,
        current_output_device: CurrentDevice,
        reference_frequency: f32,
        reference_level: i32,
        delta_mode_active: bool,
    ){
        self.current_input_device = current_input_device;
        self.current_output_device = current_output_device;

        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| {
                    poisoned.into_inner()
                });
            state.reference_level = reference_level;
            state.meter_delta_mode_active = delta_mode_active;
        }

        let ui_weak = self.ui.clone();

        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_version_number(SharedString::from(VERSION.to_string()));
            ui.set_description(SharedString::from(DESCRIPTION.to_string()));
            ui.set_license(SharedString::from(LICENSE.to_string()));
            ui.set_reference_frequency(reference_frequency);
            ui.set_reference_level(reference_level);
        });
        
    }

    fn initialize_displayed_input_device_data(&mut self) -> Result<(), Box<dyn Error>> {
        let current_input_device = self.current_input_device.clone();
        let input_device_list = self.input_device_list.clone();

        let ui_weak = self.ui.clone();

        ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_input_device_list(get_model_from_string_slice(&input_device_list.devices));
            ui.set_current_input_device(SharedString::from(current_input_device.name.clone()));
        })?;

        self.update_input_device_display_data(&self.current_input_device.name.clone())?;

        Ok(())
    }

    fn initialize_displayed_output_device_data(&mut self) -> Result<(), Box<dyn Error>> {
        let ui_weak = self.ui.clone();

        let left_output_channel = self.current_output_device.left_channel.clone();
        let right_output_channel = self.current_output_device.right_channel.clone();
        let output_device_list = self.output_device_list.clone();
        let current_output_device = self.current_output_device.clone();
        let output_device_index = get_current_device_index_from_device_list(
            &output_device_list,
            &current_output_device.name,
        )?;

        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_output_device_list(get_model_from_string_slice(&output_device_list.devices));

            ui.set_output_channel_list(get_model_from_string_slice(
                &output_device_list.channels[output_device_index as usize].clone(),
            ));

            ui.set_current_output_device(SharedString::from(current_output_device.name.clone()));

            ui.set_left_current_output_channel(SharedString::from(left_output_channel));

            match &right_output_channel {
                None => ui.set_right_output_enabled(false),
                Some(channel) => {
                    ui.set_right_output_enabled(true);
                    ui.set_right_current_output_channel(SharedString::from(channel));
                }
            }
        });

        Ok(())
    }

    fn update_current_input_device(&mut self, device_name: String) -> Result<(), Box<dyn Error>> {
        self.current_input_device.name = device_name.clone();

        let device_index = self
            .input_device_list
            .devices
            .iter()
            .position(|i| *i == device_name)
            .unwrap_or(0);

        let input_channels = self.input_device_list.channels[device_index].clone();

        self.current_input_device.left_channel = input_channels[0].clone();

        self.current_input_device.right_channel = if input_channels.len() > 1 {
            Some(input_channels[1].clone())
        } else {
            None
        };

        self.update_input_device_display_data(&device_name)?;

        Ok(())
    }

    fn update_current_output_device(&mut self, device_name: String) -> Result<(), Box<dyn Error>> {
        self.current_output_device.name = device_name.clone();

        let device_index = self
            .output_device_list
            .devices
            .iter()
            .position(|i| *i == device_name)
            .unwrap_or(0);
        let output_channels = self.output_device_list.channels[device_index].clone();

        self.current_output_device.left_channel = output_channels[0].clone();

        self.current_output_device.right_channel = if output_channels.len() > 1 {
            Some(output_channels[1].clone())
        } else {
            None
        };

        self.update_output_device_display_data(&device_name)?;

        Ok(())
    }

    fn update_input_device_display_data(
        &mut self,
        device_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let left_input_channel = self.current_input_device.left_channel.clone();
        let right_input_channel = self.current_input_device.right_channel.clone();
        let input_device_list = self.input_device_list.clone();
        let ui_weak = self.ui.clone();

        let input_device_index =
            get_current_device_index_from_device_list(&input_device_list, device_name)?;

        ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_input_channel_list(get_model_from_string_slice(
                &input_device_list.channels[input_device_index as usize].clone(),
            ));

            ui.set_left_current_input_channel(SharedString::from(left_input_channel));

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
        })?;

        Ok(())
    }
    fn update_output_device_display_data(
        &mut self,
        device_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let left_output_channel = self.current_output_device.left_channel.clone();
        let right_output_channel = self.current_output_device.right_channel.clone();
        let output_device_list = self.output_device_list.clone();
        let output_device_index =
            get_current_device_index_from_device_list(&output_device_list, device_name)?;

        let ui_weak = self.ui.clone();

        ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_output_channel_list(get_model_from_string_slice(
                &output_device_list.channels[output_device_index as usize].clone(),
            ));

            ui.set_left_current_output_channel(SharedString::from(left_output_channel));

            match &right_output_channel {
                None => {
                    ui.set_right_output_enabled(false);
                    ui.set_right_level_box_enabled(false);
                }
                Some(channel) => {
                    ui.set_right_output_enabled(true);
                    ui.set_right_level_box_enabled(true);
                    ui.set_right_current_output_channel(SharedString::from(channel));
                }
            }
        })?;

        Ok(())
    }

    pub fn create_ui_callbacks(&self) {
        self.on_close_error_dialog();
        self.on_close_fatal_error_dialog();

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

    fn on_start_button_pressed_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = if let Some(ui) = ui_weak.upgrade() {
            ui
        } else {
            eprintln!("Start Button Callback: {}", FATAL_ERROR_MESSAGE_UI_ERROR);
            exit(1);
        };

        let level_meter_sender = self.level_meter_sender.clone();
        let tone_generator_sender = self.tone_generator_sender.clone();

        ui.on_start_button_pressed(move |is_active| {
            let event_type = if is_active {
                EventType::Start
            } else {
                EventType::Stop
            };

            if let Err(error) = level_meter_sender.send(event_type.clone()) {
                eprintln!("Error sending event: {error}");
                handle_error_in_ui(&ui_weak, &error.to_string());
            }

            if let Err(error) = tone_generator_sender.send(event_type.clone()) {
                eprintln!("Error sending event: {error}");
                handle_error_in_ui(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_select_new_input_device_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = if let Some(ui) = ui_weak.upgrade() {
            ui
        } else {
            eprintln!(
                "New Input Device Callback: {}",
                FATAL_ERROR_MESSAGE_UI_ERROR
            );
            exit(1);
        };

        let user_interface_sender = self.user_interface_sender.clone();

        ui.on_selected_input_device(move |device| {
            let device_name = device.to_string();

            if let Err(error) =
                user_interface_sender.send(EventType::InputDeviceUpdate(device_name.clone()))
            {
                handle_error_in_ui(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_select_new_output_device_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = if let Some(ui) = ui_weak.upgrade() {
            ui
        } else {
            eprintln!("New Output Callback: {}", FATAL_ERROR_MESSAGE_UI_ERROR);
            exit(1);
        };

        let user_interface_sender = self.user_interface_sender.clone();

        ui.on_selected_output_device(move |device| {
            if let Err(error) =
                user_interface_sender.send(EventType::OutputDeviceUpdate(device.to_string()))
            {
                handle_error_in_ui(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_select_new_input_channel_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = if let Some(ui) = ui_weak.upgrade() {
            ui
        } else {
            eprintln!(
                "New Input Channel Callback: {}",
                FATAL_ERROR_MESSAGE_UI_ERROR
            );
            exit(1);
        };

        let user_interface_sender = self.user_interface_sender.clone();

        ui.on_selected_input_channel(move |left_channel, right_channel| {
            let left_input_channel = left_channel.to_string();
            let right_input_channel = if right_channel.is_empty() {
                None
            } else {
                Some(right_channel.to_string())
            };

            if let Err(error) = user_interface_sender.send(EventType::InputChannelUpdate {
                left: left_input_channel,
                right: right_input_channel,
            }) {
                handle_error_in_ui(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_select_new_output_channel_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = if let Some(ui) = ui_weak.upgrade() {
            ui
        } else {
            eprintln!(
                "New Output Channel Callback: {}",
                FATAL_ERROR_MESSAGE_UI_ERROR
            );
            exit(1);
        };

        let user_interface_sender = self.user_interface_sender.clone();

        ui.on_selected_output_channel(move |left_channel, right_channel| {
            let left_output_channel = left_channel.to_string();
            let right_output_channel = if right_channel.is_empty() {
                None
            } else {
                Some(right_channel.to_string())
            };

            if let Err(error) = user_interface_sender.send(EventType::OutputChannelUpdate {
                left: left_output_channel,
                right: right_output_channel,
            }) {
                handle_error_in_ui(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_reference_tone_frequency_changed_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = if let Some(ui) = ui_weak.upgrade() {
            ui
        } else {
            eprintln!(
                "Tone Frequency Change Callback: {}",
                FATAL_ERROR_MESSAGE_UI_ERROR
            );
            exit(1);
        };

        let reference_tone_sender = self.tone_generator_sender.clone();

        ui.on_tone_frequency_changed(move |frequency| {
            if let Err(error) =
                reference_tone_sender.send(EventType::ToneFrequencyUpdate(frequency))
            {
                handle_error_in_ui(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_tone_mode_updated_callback(&self) {
        let ui_weak = self.ui.clone();

        let ui = if let Some(ui) = ui_weak.upgrade() {
            ui
        } else {
            eprintln!(
                "Tone Mode Updated Callback: {}",
                FATAL_ERROR_MESSAGE_UI_ERROR
            );
            exit(1);
        };

        let tone_generator_sender = self.tone_generator_sender.clone();

        ui.on_tone_mode_checked(move |sine_mode_enabled| {
            if let Err(error) =
                tone_generator_sender.send(EventType::ToneModeUpdate(sine_mode_enabled))
            {
                handle_error_in_ui(&ui_weak, &error.to_string());
            }
        });
    }

    fn on_reference_tone_level_changed_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = if let Some(ui) = ui_weak.upgrade() {
            ui
        } else {
            eprintln!(
                "Tone Level Change Callback: {}",
                FATAL_ERROR_MESSAGE_UI_ERROR
            );
            exit(1);
        };

        let level_meter_sender = self.level_meter_sender.clone();
        let tone_generator_sender = self.tone_generator_sender.clone();
        let state_arc = self.state.clone();

        ui.on_tone_level_changed(move |level| {
            let mut state = state_arc
                .lock()
                .unwrap_or_else(|poisoned| {
                    poisoned.into_inner()
                });

            state.reference_level = level;

            if let Err(error) = level_meter_sender.send(EventType::ToneLevelUpdate(level as f32)) {
                handle_error_in_ui(&ui_weak, &error.to_string());
            }
            if let Err(error) = tone_generator_sender.send(EventType::ToneLevelUpdate(level as f32))
            {
                handle_error_in_ui(&ui_weak, &error.to_string());
            }
        });
    }

    pub fn on_delta_mode_switch_toggled_callback(&self) {
        let ui_weak = self.ui.clone();
        let ui = if let Some(ui) = ui_weak.upgrade() {
            ui
        } else {
            eprintln!(
                "Delta Mode Toggled Callback: {}",
                FATAL_ERROR_MESSAGE_UI_ERROR
            );
            exit(1);
        };

        let state_arc = self.state.clone();

        ui.on_delta_mode_checked(move |delta_mode_active| {
            let mut state = state_arc
                .lock()
                .unwrap_or_else(|poisoned| {
                    poisoned.into_inner()
                });
            state.meter_delta_mode_active = delta_mode_active;
        });
    }

    fn on_close_error_dialog(&self) {
        let ui_weak = self.ui.clone();
        let ui = get_ui_from_ui_weak_reference(&ui_weak);

        ui.on_close_error_dialog(move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.set_error_dialog_visible(false);
            });
        });
    }

    fn on_close_fatal_error_dialog(&self) {
        let ui_weak = self.ui.clone();
        let ui = get_ui_from_ui_weak_reference(&ui_weak);

        ui.on_close_error_dialog(|| {
            exit(EXIT_CODE_ERROR);
        });
    }

    fn send_stop_all(&self) {
        let ui_weak = self.ui.clone();

        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            ui.set_start_button_active(false);
        });

        if let Err(error) = self.level_meter_sender.send(EventType::Stop) {
            eprintln!("Error sending event: {error}");
            handle_error_in_ui(&ui_weak, &error.to_string());
        }

        if let Err(error) = self.tone_generator_sender.send(EventType::Stop) {
            eprintln!("Error sending event: {error}");
            handle_error_in_ui(&ui_weak, &error.to_string());
        }
    }
}

fn get_ui_from_ui_weak_reference(ui_weak: &Weak<AppWindow>) -> AppWindow {
    if let Some(ui) = ui_weak.upgrade() {
        ui
    } else {
        eprintln!("Close Dialog Callback: {}", FATAL_ERROR_MESSAGE_UI_ERROR);
        exit(1);
    }
}

fn get_current_device_index_from_device_list(
    device_list: &DeviceList,
    device_name: &str,
) -> Result<i32, LocalError> {
    let index = match device_list
        .devices
        .iter()
        .position(|name| name == device_name)
    {
        Some(index) => index as i32,
        None => return Err(LocalError::DeviceNameNotPresent(String::from(device_name))),
    };
    Ok(index)
}

fn get_model_from_string_slice(devices: &[String]) -> ModelRc<SharedString> {
    let name_list: Vec<SharedString> = devices.iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from_slice(name_list.as_slice()))
}

fn handle_error_in_ui(ui_weak: &Weak<AppWindow>, error_message: &str) {
    let error = error_message.to_string();
    let _ = ui_weak.upgrade_in_event_loop(|ui| {
        ui.set_error_message(SharedString::from(error));
        ui.set_error_dialog_visible(true);
    });
}

fn handle_fatal_error_in_ui(ui_weak: &Weak<AppWindow>, error_message: &str) {
    let error = error_message.to_string();
    let _ = ui_weak.upgrade_in_event_loop(|ui| {
        ui.set_fatal_error_message(SharedString::from(error));
        ui.set_fatal_error_dialog_visible(true);
    });
}

fn format_peak_delta_values_for_display(peak_delta_value: f32) -> String {
    if peak_delta_value.is_infinite() || peak_delta_value.is_nan() {
        "-".to_string()
    } else if (peak_delta_value < 0.0) & (peak_delta_value > -0.1) {
        "0.0".to_string()
    } else if peak_delta_value > 0.1 {
        format!("+{:.1}", peak_delta_value)
    } else {
        format!("{:.1}", peak_delta_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_dash_delta_value_for_display_if_infinity_nan_or_negative_infinity() {
        let dash_delta_values = [f32::NEG_INFINITY, f32::INFINITY, f32::NAN];
        let expected_result = "-";

        for value in dash_delta_values {
            let result = format_peak_delta_values_for_display(value);
            assert_eq!(result, expected_result);
        }
    }
}
