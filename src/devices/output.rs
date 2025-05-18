use crate::devices::DeviceList;
use crate::errors::LocalError;
use cpal::traits::*;
use cpal::{default_host, BuildStreamError, Device, Host, Stream, StreamConfig};
use sine::Sine;
use std::error::Error;
use std::process::exit;

mod sine;

const EXIT_CODE_ERROR: i32 = 1;
const ERROR_MESSAGE_SELECTED_DEVICE_DOES_NOT_EXIST: &str = "The selected device no longer exists!";
const ERROR_MESSAGE_OUTPUT_STREAM_ERROR: &str = "Output Stream error!";

const ERROR_MESSAGE_FAILED_TO_START_TONE_GENERATOR: &str =
    "Failed to start the tone generator stream. Cannot continue.";
const ERROR_MESSAGE_FAILED_TO_STOP_TONE_GENERATOR: &str =
    "Failed to stop the tone generator stream. Cannot continue.";

#[derive(Clone)]
pub struct CurrentOutputDevice {
    pub index: i32,
    pub name: String,
    pub left_channel: String,
    pub right_channel: String,
}

pub struct OutputDevices {
    host: Host,
    pub output_device: Device,
    pub output_stream: Stream,
    pub current_output_device: CurrentOutputDevice,
    pub output_device_list: DeviceList,
}

impl OutputDevices {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let host = default_host();

        let output_device_list = get_output_device_list_from_host(&host)?;

        let default_output_device = host
            .default_output_device()
            .ok_or(LocalError::NoDefaultOutputDevice)?;

        let output_device_index = get_output_device_data_current_index(
            &output_device_list.devices,
            &default_output_device.name()?,
        );

        let current_output_device = get_default_device_data_from_output_device(
            &default_output_device,
            output_device_index,
        )?;

        let left_output_channel_index: usize = current_output_device
            .left_channel
            .clone()
            .parse()
            .unwrap_or(1usize)
            .saturating_sub(1);

        let right_output_channel_index: usize = current_output_device
            .right_channel
            .clone()
            .parse()
            .unwrap_or(0usize)
            .saturating_sub(1);

        let output_stream_config: &StreamConfig =
            &default_output_device.default_output_config()?.config();
        let sample_rate = output_stream_config.sample_rate.0 as f32;
        let output_wave = Sine::new(sample_rate);

        let output_stream = get_current_output_steam(
            &default_output_device,
            output_stream_config,
            left_output_channel_index,
            right_output_channel_index,
            output_wave,
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

    pub fn start(&mut self) {
        self.output_stream.play().unwrap_or_else(|err| {
            eprintln!("{}: {}", ERROR_MESSAGE_FAILED_TO_START_TONE_GENERATOR, err);
            exit(EXIT_CODE_ERROR)
        })
    }

    pub fn stop(&mut self) {
        self.output_stream.pause().unwrap_or_else(|err| {
            eprintln!("{}: {}", ERROR_MESSAGE_FAILED_TO_STOP_TONE_GENERATOR, err);
            exit(EXIT_CODE_ERROR)
        })
    }

    pub fn set_current_output_device_on_ui_callback(&mut self, output_device_data: (i32, String)) {
        self.stop();
        self.set_output_device_from_device_name(output_device_data.1.clone());

        let output_device_channels =
            &self.output_device_list.channels[output_device_data.0 as usize];
        let left_channel = output_device_channels[0].clone();
        let right_channel = if output_device_channels.len() > 1 {
            output_device_channels[1].clone()
        } else {
            "".to_string()
        };

        self.current_output_device = CurrentOutputDevice {
            index: output_device_data.0,
            name: output_device_data.1,
            left_channel,
            right_channel,
        };

        let output_stream_config: &StreamConfig = &self
            .output_device
            .default_output_config()
            .unwrap_or_else(|err| {
                eprintln!("{}: {}", ERROR_MESSAGE_OUTPUT_STREAM_ERROR, err);
                exit(EXIT_CODE_ERROR)
            })
            .config();

        let sample_rate = output_stream_config.sample_rate.0 as f32;
        let output_wave = Sine::new(sample_rate);
        let left_output_channel_index: usize = self
            .current_output_device
            .left_channel
            .clone()
            .parse()
            .unwrap_or(1usize)
            .saturating_sub(1);

        let right_output_channel_index: usize = self
            .current_output_device
            .right_channel
            .clone()
            .parse()
            .unwrap_or(0usize)
            .saturating_sub(1);

        self.output_stream = get_current_output_steam(
            &self.output_device,
            output_stream_config,
            left_output_channel_index,
            right_output_channel_index,
            output_wave,
        )
        .unwrap_or_else(|err| {
            eprintln!("{}: {}", ERROR_MESSAGE_OUTPUT_STREAM_ERROR, err);
            exit(EXIT_CODE_ERROR)
        });

        self.output_stream
            .pause()
            .unwrap_or_else(|err| eprintln!("{}: {}", ERROR_MESSAGE_OUTPUT_STREAM_ERROR, err));
    }

    pub fn set_output_channel_on_ui_callback(
        &mut self,
        left_output_channel: String,
        right_output_channel: String,
    ) {
        self.stop();
        self.current_output_device.left_channel = left_output_channel;
        self.current_output_device.right_channel = right_output_channel;

        let output_stream_config: &StreamConfig = &self
            .output_device
            .default_output_config()
            .unwrap_or_else(|err| {
                eprintln!("{}: {}", ERROR_MESSAGE_OUTPUT_STREAM_ERROR, err);
                exit(EXIT_CODE_ERROR)
            })
            .config();

        let sample_rate = output_stream_config.sample_rate.0 as f32;
        let output_wave = Sine::new(sample_rate);
        let left_output_channel_index: usize = self
            .current_output_device
            .left_channel
            .clone()
            .parse()
            .unwrap_or(1)
            - 1;
        let right_output_channel_index: usize = self
            .current_output_device
            .right_channel
            .clone()
            .parse()
            .unwrap_or(0)
            - 1;

        self.output_stream = get_current_output_steam(
            &self.output_device,
            output_stream_config,
            left_output_channel_index,
            right_output_channel_index,
            output_wave,
        )
        .unwrap_or_else(|err| {
            eprintln!("{}: {}", ERROR_MESSAGE_OUTPUT_STREAM_ERROR, err);
            exit(EXIT_CODE_ERROR)
        });

        self.output_stream
            .pause()
            .unwrap_or_else(|err| eprintln!("{}: {}", ERROR_MESSAGE_OUTPUT_STREAM_ERROR, err));
    }

    fn set_output_device_from_device_name(&mut self, device_name: String) {
        if let Ok(mut output_devices) = self.host.output_devices() {
            match output_devices.find(|device| {
                device.name().is_ok() && device.name().unwrap_or_default() == device_name
            }) {
                Some(device) => self.output_device = device,
                None => eprintln!("{}", ERROR_MESSAGE_SELECTED_DEVICE_DOES_NOT_EXIST),
            }
        } else {
            // Because this is called from a UI callback, there isn't a way to simply gracefully recover
            eprintln!("{}", ERROR_MESSAGE_SELECTED_DEVICE_DOES_NOT_EXIST);
        }
    }
}

fn get_current_output_steam(
    device: &Device,
    stream_config: &StreamConfig,
    left_channel_index: usize,
    right_channel_index: usize,
    mut wave: Sine,
) -> Result<Stream, BuildStreamError> {
    let number_of_channels = stream_config.channels;

    device.build_output_stream(
        stream_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for channels in data.chunks_mut(number_of_channels as usize) {
                let tone_sample = wave.generate_tone_sample();
                channels[left_channel_index] = tone_sample;
                channels[right_channel_index] = tone_sample;
            }
        },
        |err| eprintln!("{}: {}", ERROR_MESSAGE_OUTPUT_STREAM_ERROR, err),
        None,
    )
}

fn get_default_device_data_from_output_device(
    output_device: &Device,
    output_device_index: i32,
) -> Result<CurrentOutputDevice, Box<dyn Error>> {
    let current_output_device = output_device.name()?;
    let default_output_channels = get_channel_list_from_output_device(output_device);
    let left_channel = default_output_channels[0].clone();
    let right_channel = if default_output_channels.len() > 1 {
        default_output_channels[1].clone()
    } else {
        "".to_string()
    };

    Ok(CurrentOutputDevice {
        index: output_device_index,
        name: current_output_device,
        left_channel,
        right_channel,
    })
}

fn get_channel_list_from_output_device(output_device: &Device) -> Vec<String> {
    let supported_output_configs = output_device.supported_output_configs();

    if let Ok(configs) = supported_output_configs {
        match configs.last() {
            None => (),
            Some(config) => {
                let number_of_output_channels = config.channels();
                return (1..=number_of_output_channels)
                    .map(|i| i.to_string())
                    .collect();
            }
        }
    }

    Vec::new()
}

fn get_output_device_data_current_index(
    output_device_list: &[String],
    current_output_device: &String,
) -> i32 {
    output_device_list
        .iter()
        .position(|i| i == current_output_device)
        .unwrap_or(0) as i32
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
