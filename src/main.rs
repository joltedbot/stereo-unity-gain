mod device_manager;
mod errors;
pub mod level_meter;
pub mod tone_generator;
mod ui;

use crate::device_manager::DeviceManager;
use crate::errors::{EXIT_CODE_ERROR, LocalError, handle_local_error};
use crate::level_meter::LevelMeter;
use crate::tone_generator::ToneGenerator;
use crate::ui::{AppWindow, EventType, UI};
use crossbeam_channel::{Receiver, Sender, unbounded};
use slint::ComponentHandle;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread;

const DEFAULT_REFERENCE_FREQUENCY: f32 = 1000.0;
const DEFAULT_REFERENCE_LEVEL: i32 = -18;

fn main() -> Result<(), slint::PlatformError> {
    let application = AppWindow::new()?;

    let mut ui = match UI::new(Arc::new(Mutex::new(application.as_weak()))) {
        Ok(ui) => ui,
        Err(error) => {
            handle_local_error(LocalError::UIInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    };

    let mut device_manager = match DeviceManager::new() {
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

    ui.create_ui_callbacks();

    let tone_generator_receiver = ui.get_tone_generator_receiver();
    let output_device_list = device_manager.get_output_devices();
    let current_output_device = device_manager.get_current_output_device();

    thread::spawn(move || {
        let mut tone_generator = match ToneGenerator::new(
            output_device_list,
            current_output_device,
            DEFAULT_REFERENCE_FREQUENCY,
            DEFAULT_REFERENCE_LEVEL as f32,
            tone_generator_receiver,
        ) {
            Ok(tone_generator) => tone_generator,
            Err(error) => {
                handle_local_error(LocalError::ToneGeneratorInitialization, error.to_string());
                exit(EXIT_CODE_ERROR);
            }
        };

        tone_generator.run();
    });

    let (display_sender, display_receiver) = unbounded();

    let level_meter_display_sender: Sender<EventType> = display_sender.clone();
    let level_meter_receiver = ui.get_level_meter_receiver();

    thread::spawn(move || {
        let mut level_meter =
            match LevelMeter::new(level_meter_receiver, DEFAULT_REFERENCE_LEVEL as f32) {
                Ok(level_meter) => level_meter,
                Err(error) => {
                    handle_local_error(LocalError::LevelMeterInitialization, error.to_string());
                    exit(EXIT_CODE_ERROR);
                }
            };

        if let Err(error) = level_meter.run(level_meter_display_sender) {
            handle_local_error(LocalError::LevelMeterInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    });

    let level_meter_display_receiver: Receiver<EventType> = display_receiver.clone();

    thread::spawn(move || {
        if let Err(error) = ui.run(level_meter_display_receiver) {
            handle_local_error(LocalError::UIInitialization, error.to_string());
            exit(EXIT_CODE_ERROR);
        }
    });

    // Start the UI and enter the main program loop
    application.run()
}
