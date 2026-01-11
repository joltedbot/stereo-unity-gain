use crate::device_manager::{CurrentDevice, get_channel_indexes_from_channel_names};
use crate::errors::{EXIT_CODE_ERROR, LocalError};
use crate::events::EventType;
use cpal::traits::{StreamTrait, DeviceTrait, HostTrait};
use cpal::{Device, Stream, default_host};
use crossbeam_channel::{Receiver, Sender};
use sine::Sine;
use square::Square;
use std::error::Error;
use std::process::exit;
use std::sync::{Arc, Mutex};

mod sine;
mod square;

const ERROR_MESSAGE_OUTPUT_STREAM_ERROR: &str = "Output Stream Error!";
const MINIMUM_DBFS_FACTOR_THRESHOLD: f32 = 0.001;

pub trait WaveShape {
    fn new(sample_rate: f32) -> Self;
    fn generate_tone_sample(&mut self, _reference_frequency: f32, target_level: f32) -> f32;
}

pub struct ToneGenerator {
    output_stream: Option<Stream>,
    sine_mode_enabled: Arc<Mutex<bool>>,
    reference_frequency: Arc<Mutex<f32>>,
    reference_level: Arc<Mutex<f32>>,
    ui_command_receiver: Receiver<EventType>,
    user_interface_sender: Sender<EventType>,
}

impl ToneGenerator {
    pub fn new(
        reference_frequency: f32,
        reference_level: f32,
        ui_command_receiver: Receiver<EventType>,
        user_interface_sender: Sender<EventType>,
    ) -> Result<Self, Box<dyn Error>> {
        let reference_frequency_arc = Arc::new(Mutex::new(reference_frequency));
        let reference_level_arc = Arc::new(Mutex::new(reference_level));
        let sine_mode_arc = Arc::new(Mutex::new(true));

        Ok(Self {
            sine_mode_enabled: sine_mode_arc,
            reference_frequency: reference_frequency_arc,
            reference_level: reference_level_arc,
            output_stream: None,
            ui_command_receiver,
            user_interface_sender,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let ui_command_receiver = self.ui_command_receiver.clone();
        loop {
            if let Ok(event) = ui_command_receiver.try_recv() {
                match event {
                    EventType::Start => self.start().expect("Could Not Start Tone Generator"),
                    EventType::Stop => self.stop().expect("Could Not Stop Tone Generator"),
                    EventType::ToneFrequencyUpdate(new_frequency) => {
                        if let Ok(mut freq) = self.reference_frequency.lock() {
                            *freq = new_frequency;
                        }
                    }
                    EventType::ToneLevelUpdate(new_level) => {
                        if let Ok(mut level) = self.reference_level.lock() {
                            *level = new_level;
                        }
                    }
                    EventType::ToneModeUpdate(sine_enabled) => {
                        if let Ok(mut sine_mode_enabled) = self.sine_mode_enabled.lock() {
                            *sine_mode_enabled = sine_enabled;
                        }
                    }
                    EventType::ToneDeviceUpdate { name, left, right } => {
                        self.update_output_stream_on_new_device(&name, &left, right.as_ref())?;
                    }
                    _ => (),
                }
            }
        }
    }

    pub fn start(&mut self) -> Result<(), LocalError> {
        if let Some(ref mut stream) = self.output_stream {
            stream
                .play()
                .map_err(|err| LocalError::ToneGeneratorStart(err.to_string()))?;
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), LocalError> {
        if let Some(ref mut stream) = self.output_stream {
            stream
                .pause()
                .map_err(|err| LocalError::ToneGeneratorStart(err.to_string()))?;
        }
        Ok(())
    }

    pub fn update_output_stream_on_new_device(
        &mut self,name: &str, left_channel: &str, right_channel: Option<&String>,) -> Result<(), LocalError> {
        self.stop()?;

        let output_device = get_output_device_from_device_name(name)?;

        let (left_output_channel_index, right_output_channel_index) =
            get_channel_indexes_from_channel_names(left_channel, right_channel)?;

        let user_interface_sender = self.user_interface_sender.clone();

        let output_stream = create_output_steam(
            &output_device,
            left_output_channel_index,
            right_output_channel_index,
            self.sine_mode_enabled.clone(),
            self.reference_frequency.clone(),
            self.reference_level.clone(),
            user_interface_sender,
        )
        .map_err(|err| LocalError::ToneGeneratorOutputStream(err.to_string()))?;

        output_stream
            .pause()
            .map_err(|err| LocalError::ToneGeneratorOutputStream(err.to_string()))?;

        self.output_stream = Some(output_stream);

        Ok(())
    }
}

fn create_output_steam(
    device: &Device,
    left_channel_index: usize,
    right_channel_index: Option<usize>,
    sine_mode_enabled: Arc<Mutex<bool>>,
    reference_frequency: Arc<Mutex<f32>>,
    reference_level: Arc<Mutex<f32>>,
    user_interface_sender: Sender<EventType>,
) -> Result<Stream, LocalError> {
    let config_result = device
        .default_output_config()
        .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

    let stream_config = config_result.config();
    let number_of_channels = stream_config.channels;
    let sample_rate = stream_config.sample_rate as f32;
    let mut sine_wave = Sine::new(sample_rate);
    let mut square_wave = Square::new(sample_rate);

    let initial_frequency = match reference_frequency.lock() {
        Ok(frequency) => frequency.to_owned(),
        Err(_) => return Err(LocalError::ToneGeneratorInitialization),
    };

    let initial_level = match reference_level.lock() {
        Ok(level) => level.to_owned(),
        Err(_) => return Err(LocalError::ToneGeneratorInitialization),
    };

    let initial_sine_mode = match sine_mode_enabled.lock() {
        Ok(sine_mode_enabled) => sine_mode_enabled.to_owned(),
        Err(_) => return Err(LocalError::ToneGeneratorInitialization),
    };

    let mut dbfs_adjustment_factor = get_dbfs_adjustment_factor_from_target_level(initial_level);

    let callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        let current_frequency = if let Ok(frequency) = reference_frequency.lock() {
            *frequency
        } else {
            initial_frequency
        };

        let current_level = if let Ok(level) = reference_level.lock() {
            *level
        } else {
            initial_level
        };

        let current_sine_mode = if let Ok(sine_mode_enabled) = sine_mode_enabled.lock() {
            *sine_mode_enabled
        } else {
            initial_sine_mode
        };

        let current_dbfs_factor = get_dbfs_adjustment_factor_from_target_level(current_level);
        if (current_dbfs_factor - dbfs_adjustment_factor).abs() > MINIMUM_DBFS_FACTOR_THRESHOLD {
            dbfs_adjustment_factor = current_dbfs_factor;
        }

        for channels in data.chunks_mut(number_of_channels as usize) {
            let tone_sample = if current_sine_mode {
                sine_wave.generate_tone_sample(current_frequency, current_level)
            } else {
                square_wave.generate_tone_sample(current_frequency, current_level)
            };

            channels[left_channel_index] = tone_sample;
            if let Some(index) = right_channel_index {
                channels[index] = tone_sample;
            }
        }
    };

    device
        .build_output_stream(
            &stream_config,
            callback,
            move |error| {
                if let Err(err) =
                    user_interface_sender.send(EventType::FatalError(error.to_string()))
                {
                    eprintln!("{ERROR_MESSAGE_OUTPUT_STREAM_ERROR}: {err}");
                    exit(EXIT_CODE_ERROR);
                }
            },
            None,
        )
        .map_err(|err| LocalError::ToneGeneratorOutputStream(err.to_string()))
}

fn get_output_device_from_device_name(device_name: &str) -> Result<Device, LocalError> {
    let host = default_host();

    let mut output_devices = host
        .output_devices()
        .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

    match output_devices.find(|device| {
        device
            .description()
            .is_ok_and(|device| device.name() == device_name)
    }) {
        Some(device) => Ok(device),
        None => Err(LocalError::DeviceNotFound(device_name.to_string())),
    }
}

pub fn get_default_device_data_from_output_device(
    device: &Device,
) -> Result<CurrentDevice, Box<dyn Error>> {
    let name = device.description()?.name().to_string();

    let default_output_channels = get_channel_list_from_output_device(device);

    let left_channel = default_output_channels[0].clone();
    let right_channel = if default_output_channels.len() > 1 {
        Some(default_output_channels[1].clone())
    } else {
        None
    };

    Ok(CurrentDevice {
        name,
        left_channel,
        right_channel,
    })
}

fn get_channel_list_from_output_device(device: &Device) -> Vec<String> {
    if let Ok(config) = device.default_output_config() {
        (1..=config.channels() as usize)
            .map(|i| i.to_string())
            .collect()
    } else {
        Vec::new()
    }
}

fn get_dbfs_adjustment_factor_from_target_level(level: f32) -> f32 {
    10.0_f32.powf(level / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_correct_dbfs_adjustment_factor_from_valid_level_value() {
        let test_level = -20.0;
        let result = get_dbfs_adjustment_factor_from_target_level(test_level);
        let correct_result = 0.1;
        assert_eq!(result, correct_result);
    }

    #[test]
    fn return_correct_dbfs_adjustment_factor_from_zero_level() {
        let test_level = 0.0;
        let result = get_dbfs_adjustment_factor_from_target_level(test_level);
        let correct_result = 1.0;
        assert_eq!(result, correct_result);
    }
}
