use crate::devices::DeviceList;
use crate::errors::LocalError;
use cpal::traits::*;
use cpal::{default_host, BuildStreamError, Device, Host, Stream, StreamConfig};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::error::Error;
use std::process::exit;

const EXIT_CODE_ERROR: i32 = 1;
const ERROR_MESSAGE_SELECTED_DEVICE_DOES_NOT_EXIST: &str = "The selected device no longer exists!";
const ERROR_MESSAGE_INPUT_STREAM_ERROR: &str = "Input Stream error!";

const ERROR_MESSAGE_FAILED_TO_START_LEVEL_METER: &str =
    "Failed to start the level meter stream. Cannot continue.";
const ERROR_MESSAGE_FAILED_TO_STOP_LEVEL_METER: &str =
    "Failed to stop the level meter stream. Cannot continue.";

#[derive(Clone)]
pub struct CurrentInputDevice {
    pub index: i32,
    pub name: String,
    pub left_channel: String,
    pub right_channel: String,
}

pub struct InputDevices {
    host: Host,
    pub input_device: Device,
    pub input_stream: Stream,
    pub current_input_device: CurrentInputDevice,
    pub input_device_list: DeviceList,
    channel_consumer: Receiver<(Vec<f32>, Vec<f32>)>,
    channel_producer: Sender<(Vec<f32>, Vec<f32>)>,
}

impl InputDevices {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let host = default_host();

        let input_device_list = get_input_device_list_from_host(&host)?;

        let default_input_device = host
            .default_input_device()
            .ok_or(LocalError::NoDefaultInputDevice)?;

        let input_device_index = get_input_device_data_current_index(
            &input_device_list.devices,
            &default_input_device.name()?,
        );

        let current_input_device =
            get_default_device_data_from_input_device(&default_input_device, input_device_index)?;

        let left_input_channel_index: usize = current_input_device
            .left_channel
            .clone()
            .parse()
            .unwrap_or(1usize)
            .saturating_sub(1);

        let right_input_channel_index: usize = current_input_device
            .right_channel
            .clone()
            .parse()
            .unwrap_or(0usize)
            .saturating_sub(1);

        let input_stream_config: &StreamConfig =
            &default_input_device.default_input_config()?.config();

        let (producer, consumer) = unbounded();
        let channel_consumer = consumer;
        let channel_producer = producer.clone();

        let input_stream = get_current_input_steam(
            &default_input_device,
            input_stream_config,
            left_input_channel_index,
            right_input_channel_index,
            producer.clone(),
        )?;

        input_stream.pause()?;

        Ok(Self {
            host,
            input_device: default_input_device,
            input_stream,
            channel_producer,
            channel_consumer,
            current_input_device,
            input_device_list,
        })
    }

    pub fn start(&mut self) {
        self.input_stream.play().unwrap_or_else(|err| {
            eprintln!("{}: {}", ERROR_MESSAGE_FAILED_TO_START_LEVEL_METER, err);
            exit(EXIT_CODE_ERROR)
        })
    }

    pub fn stop(&mut self) {
        self.input_stream.pause().unwrap_or_else(|err| {
            eprintln!("{}: {}", ERROR_MESSAGE_FAILED_TO_STOP_LEVEL_METER, err);
            exit(EXIT_CODE_ERROR)
        })
    }

    pub fn set_current_input_device_on_ui_callback(&mut self, input_device_data: (i32, String)) {
        self.stop();

        self.set_input_device_from_device_name(input_device_data.1.clone());

        let input_device_channels = &self.input_device_list.channels[input_device_data.0 as usize];
        let left_channel = input_device_channels[0].clone();
        let right_channel = if input_device_channels.len() > 1 {
            input_device_channels[1].clone()
        } else {
            "".to_string()
        };

        self.current_input_device = CurrentInputDevice {
            index: input_device_data.0,
            name: input_device_data.1,
            left_channel,
            right_channel,
        };

        let left_input_channel_index: usize = self
            .current_input_device
            .left_channel
            .clone()
            .parse()
            .unwrap_or(1usize)
            .saturating_sub(1);

        let right_input_channel_index: usize = self
            .current_input_device
            .right_channel
            .clone()
            .parse()
            .unwrap_or(0usize)
            .saturating_sub(1);

        let input_stream_config: &StreamConfig = &self
            .input_device
            .default_input_config()
            .unwrap_or_else(|err| {
                eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
                exit(EXIT_CODE_ERROR)
            })
            .config();

        self.input_stream = get_current_input_steam(
            &self.input_device,
            input_stream_config,
            left_input_channel_index,
            right_input_channel_index,
            self.channel_producer.clone(),
        )
        .unwrap_or_else(|err| {
            eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
            exit(EXIT_CODE_ERROR)
        });

        self.input_stream.pause().unwrap_or_else(|err| {
            eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
            exit(EXIT_CODE_ERROR)
        });
    }

    pub fn set_input_channel_on_ui_callback(
        &mut self,
        left_input_channel: String,
        right_input_channel: String,
    ) {
        self.stop();
        self.current_input_device.left_channel = left_input_channel;
        self.current_input_device.right_channel = right_input_channel;

        let left_input_channel_index: usize = self
            .current_input_device
            .left_channel
            .clone()
            .parse()
            .unwrap_or(1usize)
            .saturating_sub(1);

        let right_input_channel_index: usize = self
            .current_input_device
            .right_channel
            .clone()
            .parse()
            .unwrap_or(0usize)
            .saturating_sub(1);

        let input_stream_config: &StreamConfig = &self
            .input_device
            .default_input_config()
            .unwrap_or_else(|err| {
                eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
                exit(EXIT_CODE_ERROR)
            })
            .config();

        self.input_stream = get_current_input_steam(
            &self.input_device,
            input_stream_config,
            left_input_channel_index,
            right_input_channel_index,
            self.channel_producer.clone(),
        )
        .unwrap_or_else(|err| {
            eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
            exit(EXIT_CODE_ERROR)
        });

        self.input_stream.pause().unwrap_or_else(|err| {
            eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
            exit(EXIT_CODE_ERROR)
        });
    }

    fn set_input_device_from_device_name(&mut self, device_name: String) {
        if let Ok(mut input_devices) = self.host.input_devices() {
            match input_devices.find(|device| {
                device.name().is_ok()
                    && device.name().unwrap_or_else(|err| {
                        eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
                        exit(EXIT_CODE_ERROR)
                    }) == device_name
            }) {
                Some(device) => self.input_device = device,
                None => {
                    eprintln!("{}", ERROR_MESSAGE_SELECTED_DEVICE_DOES_NOT_EXIST);
                    exit(EXIT_CODE_ERROR)
                }
            }
        } else {
            eprintln!("{}", ERROR_MESSAGE_SELECTED_DEVICE_DOES_NOT_EXIST);
            exit(EXIT_CODE_ERROR);
        }
    }

    pub fn get_meter_reader(&mut self) -> Receiver<(Vec<f32>, Vec<f32>)> {
        self.channel_consumer.clone()
    }
}

fn get_current_input_steam(
    device: &Device,
    stream_config: &StreamConfig,
    left_channel_index: usize,
    right_channel_index: usize,
    producer: Sender<(Vec<f32>, Vec<f32>)>,
) -> Result<Stream, BuildStreamError> {
    let number_of_channels = stream_config.channels;

    device.build_input_stream(
        stream_config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let number_of_frames = data.len() / number_of_channels as usize;
            let mut left_channel_samples = Vec::with_capacity(number_of_frames);
            let mut right_channel_samples = Vec::with_capacity(number_of_frames);

            data.chunks_exact(number_of_channels as usize)
                .for_each(|frame| {
                    left_channel_samples.push(frame[left_channel_index]);
                    right_channel_samples.push(frame[right_channel_index]);
                });

            match producer.send((left_channel_samples, right_channel_samples)) {
                Ok(_) => {}
                Err(error) => {
                    println!("Error sending data to channel consumer: {}", error);
                }
            }
        },
        |err| {
            eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
            exit(EXIT_CODE_ERROR);
        },
        None,
    )
}

fn get_default_device_data_from_input_device(
    input_device: &Device,
    input_device_index: i32,
) -> Result<CurrentInputDevice, Box<dyn Error>> {
    let current_input_device = input_device.name()?;
    let default_input_channels = get_channel_list_from_input_device(input_device);
    let left_channel = default_input_channels[0].clone();
    let right_channel = if default_input_channels.len() > 1 {
        default_input_channels[1].clone()
    } else {
        "".to_string()
    };

    Ok(CurrentInputDevice {
        index: input_device_index,
        name: current_input_device,
        left_channel,
        right_channel,
    })
}

fn get_channel_list_from_input_device(input_device: &Device) -> Vec<String> {
    let supported_input_configs = input_device.supported_input_configs();

    if let Ok(configs) = supported_input_configs {
        match configs.last() {
            None => (),
            Some(config) => {
                let number_of_input_channels = config.channels();
                return (1..=number_of_input_channels)
                    .map(|i| i.to_string())
                    .collect();
            }
        }
    }

    Vec::new()
}

fn get_input_device_data_current_index(
    input_device_list: &[String],
    current_input_device: &String,
) -> i32 {
    input_device_list
        .iter()
        .position(|i| i == current_input_device)
        .unwrap_or(0) as i32
}

fn get_input_device_list_from_host(host: &Host) -> Result<DeviceList, Box<dyn Error>> {
    let mut input_devices: Vec<String> = Vec::new();
    let mut input_channels: Vec<Vec<String>> = Vec::new();

    host.input_devices()?.for_each(|device| {
        if let Ok(name) = device.name() {
            input_devices.push(name);
            input_channels.push(get_channel_list_from_input_device(&device));
        }
    });

    Ok(DeviceList {
        devices: input_devices,
        channels: input_channels,
    })
}
