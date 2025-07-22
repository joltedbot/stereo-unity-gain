mod device_manager;
mod errors;
mod events;
pub mod level_meter;
pub mod tone_generator;
mod ui;

use crate::device_manager::DeviceManager;
use crate::errors::{EXIT_CODE_ERROR, LocalError, handle_local_error};
use crate::events::EventType;
use crate::events::Events;
use crate::level_meter::LevelMeter;
use crate::tone_generator::ToneGenerator;
use crate::ui::UI;
use crossbeam_channel::{Receiver, Sender};
use slint::ComponentHandle;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread;

slint::include_modules!();

const DEFAULT_REFERENCE_FREQUENCY: f32 = 1000.0;
const DEFAULT_REFERENCE_LEVEL: i32 = -18;

fn main() -> Result<(), slint::PlatformError> {
    // Initialize Slint Application
    let application = AppWindow::new()?;

    // Initialize Events Module
    let events = Events::new();

    // Initialize UI Module
    let tone_generator_sender = events.get_tone_generator_sender();
    let level_meter_sender = events.get_level_meter_sender();
    let user_interface_sender = events.get_user_interface_sender();

    let mut ui = match UI::new(
        Arc::new(Mutex::new(application.as_weak())),
        tone_generator_sender,
        level_meter_sender,
        user_interface_sender,
    ) {
        Ok(ui) => ui,
        Err(error) => {
            handle_local_error(LocalError::UIInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    ui.create_ui_callbacks();

    // Initialize Device Manager Module
    let device_manager_ui_sender = events.get_user_interface_sender();
    let level_meter_sender = events.get_level_meter_sender();
    let tone_generator_sender = events.get_tone_generator_sender();

    let mut device_manager = match DeviceManager::new(
        device_manager_ui_sender,
        level_meter_sender,
        tone_generator_sender,
    ) {
        Ok(device_manager) => device_manager,
        Err(error) => {
            handle_local_error(LocalError::DeviceManagerInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    if let Err(err) = ui.initialize_ui_with_device_data(
        device_manager.get_input_devices(),
        device_manager.get_current_input_device(),
        device_manager.get_output_devices(),
        device_manager.get_current_output_device(),
        DEFAULT_REFERENCE_FREQUENCY,
        DEFAULT_REFERENCE_LEVEL,
    ) {
        handle_local_error(LocalError::UIInitialization, err.to_string());
        exit(EXIT_CODE_ERROR);
    }

    // Initialize Tone Generator Module
    let tone_generator_receiver = events.get_tone_generator_receiver();
    let output_device_list = device_manager.get_output_devices();
    let current_output_device = device_manager.get_current_output_device();
    let tone_generator_ui_sender = events.get_user_interface_sender();

    thread::spawn(move || {
        let mut tone_generator = match ToneGenerator::new(
            output_device_list,
            current_output_device,
            DEFAULT_REFERENCE_FREQUENCY,
            DEFAULT_REFERENCE_LEVEL as f32,
            tone_generator_receiver,
            tone_generator_ui_sender,
        ) {
            Ok(tone_generator) => tone_generator,
            Err(error) => {
                handle_local_error(LocalError::ToneGeneratorInitialization, error.to_string());
                exit(EXIT_CODE_ERROR);
            }
        };
        if let Err(error) = tone_generator.run() {
            handle_local_error(LocalError::ToneGeneratorInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    });

    // Initialize Level Meter Module
    let level_meter_ui_sender: Sender<EventType> = events.get_user_interface_sender();
    let level_meter_receiver = events.get_level_meter_receiver();

    thread::spawn(move || {
        let mut level_meter = match LevelMeter::new(level_meter_receiver) {
            Ok(level_meter) => level_meter,
            Err(error) => {
                handle_local_error(
                    LocalError::LevelMeterInitialization(error.to_string()),
                    String::new(),
                );
                exit(EXIT_CODE_ERROR);
            }
        };

        if let Err(error) = level_meter.run_input_sample_processor(level_meter_ui_sender) {
            handle_local_error(
                LocalError::LevelMeterInitialization(error.to_string()),
                String::new(),
            );
            exit(EXIT_CODE_ERROR);
        }

        if let Err(error) = level_meter.run() {
            handle_local_error(
                LocalError::LevelMeterInitialization(error.to_string()),
                String::new(),
            );
            exit(EXIT_CODE_ERROR);
        }
    });

    // Spawn the run loop for the Devices Manager module consuming the initialized object
    thread::spawn(move || {
        if let Err(error) = device_manager.run() {
            handle_local_error(LocalError::DeviceManagerInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    });

    // Spawn the run loop for the UI module consuming the initialized object
    let user_interface_receiver: Receiver<EventType> = events.get_user_interface_receiver();
    thread::spawn(move || {
        if let Err(error) = ui.run(user_interface_receiver) {
            handle_local_error(LocalError::UIInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    });

    // Start the UI and enter the main program loop
    application.run()
}
