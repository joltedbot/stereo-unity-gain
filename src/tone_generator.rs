use crate::devices::{CurrentDevice, DeviceList, get_channel_indexes_from_channel_names};
use crate::errors::{EXIT_CODE_ERROR, LocalError};
use crate::ui::EventType;
use cpal::traits::*;
use cpal::{Device, Host, Stream, default_host};
use crossbeam_channel::Receiver;
use sine::Sine;
use std::error::Error;
use std::process::exit;

mod sine;

const ERROR_MESSAGE_OUTPUT_STREAM_ERROR: &str = "Output Stream Error!";
const DEFAULT_REFERENCE_FREQUENCY: f32 = 1000.0;
const DEFAULT_REFERENCE_LEVEL: i32 = -18;

#[derive(Debug, Clone)]
pub struct ToneParameters {
    pub frequency: f32,
    pub level: i32,
}

pub struct ToneGenerator {
    output_device: Device,
    output_stream: Stream,
    current_output_device: CurrentDevice,
    output_device_list: DeviceList,
    reference_tone: ToneParameters,
    ui_command_receiver: Receiver<EventType>,
}

impl ToneGenerator {
    pub fn new(ui_command_receiver: Receiver<EventType>) -> Result<Self, Box<dyn Error>> {
        let host = default_host();

        let output_device_list = get_output_device_list_from_host(&host)?;

        let output_device = host
            .default_output_device()
            .ok_or(LocalError::NoDefaultOutputDevice)?;

        let current_output_device = get_default_device_data_from_output_device(
            &output_device,
            &output_device_list.devices,
        )?;

        let (left_output_channel_index, right_output_channel_index) =
            get_channel_indexes_from_channel_names(
                &current_output_device.left_channel,
                &current_output_device.right_channel,
            )?;

        let reference_frequency = DEFAULT_REFERENCE_FREQUENCY;
        let reference_level = DEFAULT_REFERENCE_LEVEL;

        let reference_tone = ToneParameters {
            frequency: reference_frequency,
            level: reference_level,
        };

        let output_stream = create_output_steam(
            &output_device,
            left_output_channel_index,
            right_output_channel_index,
            ui_command_receiver.clone(),
            &reference_tone,
        )?;

        output_stream.pause()?;

        Ok(Self {
            output_device,
            output_stream,
            current_output_device,
            output_device_list,
            reference_tone,
            ui_command_receiver,
        })
    }

    pub fn start(&mut self) -> Result<(), LocalError> {
        self.output_stream
            .play()
            .map_err(|err| LocalError::ToneGeneratorStart(err.to_string()))?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), LocalError> {
        self.output_stream
            .pause()
            .map_err(|err| LocalError::ToneGeneratorStop(err.to_string()))?;
        Ok(())
    }

    pub fn get_reference_tone_parameters(&self) -> ToneParameters {
        self.reference_tone.clone()
    }

    pub fn get_output_device_list(&self) -> DeviceList {
        self.output_device_list.clone()
    }

    pub fn get_current_output_device(&self) -> CurrentDevice {
        self.current_output_device.clone()
    }

    pub fn get_current_output_device_channels(&self) -> Vec<String> {
        self.output_device_list.channels[self.current_output_device.index as usize].clone()
    }

    pub fn set_reference_tone_on_ui_callback(&mut self, reference_tone: &ToneParameters) {
        self.reference_tone = reference_tone.clone();
    }

    pub fn set_output_device_on_ui_callback(
        &mut self,
        output_device_data: (i32, String),
    ) -> Result<(), LocalError> {
        self.stop()?;

        self.update_current_output_device(output_device_data)
            .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))
    }

    pub fn update_current_output_device(
        &mut self,
        output_device_data: (i32, String),
    ) -> Result<(), LocalError> {
        self.output_device =
            self.get_output_device_from_device_name(output_device_data.1.clone())?;

        let output_device_channels =
            &self.output_device_list.channels[output_device_data.0 as usize];

        let left_channel = output_device_channels[0].clone();
        let right_channel = if output_device_channels.len() > 1 {
            Some(output_device_channels[1].clone())
        } else {
            None
        };

        self.current_output_device = CurrentDevice {
            index: output_device_data.0,
            name: output_device_data.1,
            left_channel,
            right_channel,
        };

        self.set_output_device(self.current_output_device.clone())?;

        Ok(())
    }

    pub fn set_output_channel_on_ui_callback(
        &mut self,
        left_output_channel: String,
        right_output_channel: Option<String>,
    ) -> Result<(), LocalError> {
        self.stop()?;

        self.current_output_device.left_channel = left_output_channel;
        self.current_output_device.right_channel = right_output_channel;

        self.set_output_device(self.current_output_device.clone())?;

        let (left_output_channel_index, right_output_channel_index) =
            get_channel_indexes_from_channel_names(
                &self.current_output_device.left_channel,
                &self.current_output_device.right_channel,
            )?;

        self.output_stream = create_output_steam(
            &self.output_device,
            left_output_channel_index,
            right_output_channel_index,
            self.ui_command_receiver.clone(),
            &self.reference_tone,
        )
        .map_err(|err| LocalError::OutputStream(err.to_string()))?;

        self.output_stream
            .pause()
            .map_err(|err| LocalError::OutputStream(err.to_string()))?;

        Ok(())
    }

    pub fn set_output_device(&mut self, device: CurrentDevice) -> Result<(), LocalError> {
        self.output_device = self.get_output_device_from_device_name(device.name.clone())?;
        self.current_output_device = device;

        let (left_output_channel_index, right_output_channel_index) =
            get_channel_indexes_from_channel_names(
                &self.current_output_device.left_channel,
                &self.current_output_device.right_channel,
            )?;

        self.output_stream = create_output_steam(
            &self.output_device,
            left_output_channel_index,
            right_output_channel_index,
            self.ui_command_receiver.clone(),
            &self.reference_tone,
        )
        .map_err(|err| LocalError::OutputStream(err.to_string()))?;

        self.output_stream
            .pause()
            .map_err(|err| LocalError::OutputStream(err.to_string()))?;

        Ok(())
    }

    fn get_output_device_from_device_name(
        &mut self,
        device_name: String,
    ) -> Result<Device, LocalError> {
        let host = default_host();

        let mut output_devices = host
            .output_devices()
            .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

        match output_devices
            .find(|device| device.name().is_ok() && device.name().unwrap() == device_name)
        {
            Some(device) => Ok(device),
            None => Err(LocalError::DeviceNotFound(device_name)),
        }
    }

    pub fn reset_to_default_output_device(&mut self) -> Result<CurrentDevice, LocalError> {
        self.stop()?;

        let host = default_host();
        let default_output_device = host
            .default_output_device()
            .ok_or(LocalError::NoDefaultOutputDevice)?;

        let output_device_list = &self.output_device_list.devices;

        let current_output_device =
            get_default_device_data_from_output_device(&default_output_device, output_device_list)
                .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

        self.set_output_device(current_output_device.clone())?;

        Ok(current_output_device)
    }
}

fn create_output_steam(
    device: &Device,
    left_channel_index: usize,
    right_channel_index: Option<usize>,
    ui_command_receiver: Receiver<EventType>,
    reference_tone: &ToneParameters,
) -> Result<Stream, LocalError> {
    let config_result = device
        .default_output_config()
        .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

    let stream_config = config_result.config();
    let number_of_channels = stream_config.channels;
    let sample_rate = stream_config.sample_rate.0 as f32;
    let mut wave = Sine::new(sample_rate);
    let reference_level = reference_tone.level;
    let mut reference_frequency = reference_tone.frequency;
    let mut dbfs_adjustment_factor = get_dbfs_adjustment_factor_from_level(reference_level);

    device
        .build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                while let Ok(command) = ui_command_receiver.try_recv() {
                    match command {
                        EventType::ToneFrequencyUpdate(frequency) => {
                            reference_frequency = frequency
                        }
                        EventType::ToneLevelUpdate(level) => {
                            dbfs_adjustment_factor = get_dbfs_adjustment_factor_from_level(level)
                        }
                        _ => (),
                    }
                }

                for channels in data.chunks_mut(number_of_channels as usize) {
                    let tone_sample =
                        wave.generate_tone_sample(reference_frequency, dbfs_adjustment_factor);
                    channels[left_channel_index] = tone_sample;
                    if let Some(index) = right_channel_index {
                        channels[index] = tone_sample;
                    }
                }
            },
            |err| {
                eprintln!("{}: {}", ERROR_MESSAGE_OUTPUT_STREAM_ERROR, err);
                exit(EXIT_CODE_ERROR)
            },
            None,
        )
        .map_err(|err| LocalError::OutputStream(err.to_string()))
}

pub fn get_default_device_data_from_output_device(
    device: &Device,
    device_list: &[String],
) -> Result<CurrentDevice, Box<dyn Error>> {
    let name = device.name()?;

    let index = device_list
        .iter()
        .position(|device_name| device_name == &name)
        .map(|pos| pos as i32)
        .unwrap_or(0);

    let default_output_channels = get_channel_list_from_output_device(device);

    let left_channel = default_output_channels[0].clone();
    let right_channel = if default_output_channels.len() > 1 {
        Some(default_output_channels[1].clone())
    } else {
        None
    };

    Ok(CurrentDevice {
        index,
        name,
        left_channel,
        right_channel,
    })
}

fn get_output_device_list_from_host(host: &Host) -> Result<DeviceList, Box<dyn Error>> {
    let mut output_devices: Vec<String> = Vec::new();
    let mut output_channels: Vec<Vec<String>> = Vec::new();

    host.output_devices()?.for_each(|device| {
        if let Ok(name) = device.name() {
            output_devices.push(name);
            output_channels.push(get_channel_list_from_output_device(&device));
        }
    });

    Ok(DeviceList {
        devices: output_devices,
        channels: output_channels,
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

fn get_dbfs_adjustment_factor_from_level(level: i32) -> f32 {
    10.0_f32.powf(level as f32 / 20.0)
}
