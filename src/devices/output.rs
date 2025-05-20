use crate::devices::{get_channel_indexes_from_channel_names, CurrentDevice, DeviceList};
use crate::errors::{LocalError, EXIT_CODE_ERROR};
use cpal::traits::*;
use cpal::{default_host, Device, Host, Stream};
use sine::Sine;
use std::error::Error;
use std::process::exit;

mod sine;

const ERROR_MESSAGE_OUTPUT_STREAM_ERROR: &str = "Output Stream error!";

pub struct OutputDevices {
    host: Host,
    pub output_device: Device,
    pub output_stream: Stream,
    pub current_output_device: CurrentDevice,
    pub output_device_list: DeviceList,
}

impl OutputDevices {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let host = default_host();

        let output_device_list = get_output_device_list_from_host(&host)?;

        let default_output_device = host
            .default_output_device()
            .ok_or(LocalError::NoDefaultOutputDevice)?;

        let current_output_device = get_default_device_data_from_output_device(
            &default_output_device,
            &output_device_list.devices,
        )?;

        let (left_output_channel_index, right_output_channel_index) =
            get_channel_indexes_from_channel_names(
                &current_output_device.left_channel,
                &current_output_device.right_channel,
            )?;

        let output_stream = create_output_steam(
            &default_output_device,
            left_output_channel_index,
            right_output_channel_index,
        )?;

        output_stream.pause()?;

        Ok(Self {
            host,
            output_device: default_output_device,
            output_stream,
            current_output_device,
            output_device_list,
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

    pub fn set_output_device_on_ui_callback(
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
        let mut output_devices = self
            .host
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
) -> Result<Stream, LocalError> {
    let config_result = device
        .default_output_config()
        .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

    let stream_config = config_result.config();
    let number_of_channels = stream_config.channels;
    let sample_rate = stream_config.sample_rate.0 as f32;
    let mut wave = Sine::new(sample_rate);

    device
        .build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for channels in data.chunks_mut(number_of_channels as usize) {
                    let tone_sample = wave.generate_tone_sample();
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

pub fn get_output_device_data_current_index(device_list: &[String], device_name: &str) -> i32 {
    device_list
        .iter()
        .position(|name| name == device_name)
        .map(|pos| pos as i32)
        .unwrap_or(0)
}

pub fn get_default_device_data_from_output_device(
    device: &Device,
    device_list: &[String],
) -> Result<CurrentDevice, Box<dyn Error>> {
    let name = device.name()?;
    let index = get_output_device_data_current_index(device_list, &name);
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
