use crate::errors::LocalError;
use cpal::traits::*;
use cpal::{default_host, BuildStreamError, Device, Host, Stream, StreamConfig};
use crossbeam_channel::{unbounded, Receiver, Sender};
use sine::Sine;
use std::error::Error;

mod sine;

const PANIC_MESSAGE_WHEN_DEVICE_PASSED_FROM_THE_UI_DOES_NOT_EXIST: &str =
    "The selected device no longer exists!";

#[derive(Default, Debug, Clone)]
pub struct DisplayData {
    pub input_device_list: (Vec<String>, Vec<Vec<String>>),
    pub output_device_list: (Vec<String>, Vec<Vec<String>>),
}

pub struct Devices {
    host: Host,
    pub input_device: Device,
    pub output_device: Device,
    pub output_stream: Stream,
    pub input_stream: Stream,
    pub active_input_device: (i32, String, String, String),
    pub active_output_device: (i32, String, String, String),
    channel_consumer: Receiver<(Vec<f32>, Vec<f32>)>,
    channel_producer: Sender<(Vec<f32>, Vec<f32>)>,
    display_data: DisplayData,
}

impl Devices {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let host = default_host();

        let input_device = host
            .default_input_device()
            .ok_or(LocalError::NoDefaultInputDevice)?;

        let output_device = host
            .default_output_device()
            .ok_or(LocalError::NoDefaultOutputDevice)?;

        let display_data = get_display_data_from_devices(&host)?;

        let input_device_index = get_input_device_data_current_index(
            &display_data.input_device_list.0,
            &input_device.name()?,
        );

        let output_device_index = get_output_device_data_current_index(
            &display_data.output_device_list.0,
            &output_device.name()?,
        );

        let active_input_device =
            get_default_device_data_from_input_device(&input_device, input_device_index)?;
        let active_output_device =
            get_default_device_data_from_output_device(&output_device, output_device_index)?;

        let left_output_channel_index: usize = active_output_device
            .2
            .clone()
            .parse()
            .unwrap_or(1usize)
            .saturating_sub(1);
        let right_output_channel_index: usize = active_output_device
            .3
            .clone()
            .parse()
            .unwrap_or(0usize)
            .saturating_sub(1);

        let output_stream_config: &StreamConfig = &output_device.default_output_config()?.config();
        let sample_rate = output_stream_config.sample_rate.0 as f32;
        let output_wave = Sine::new(sample_rate);

        let output_stream = get_current_output_steam(
            &output_device,
            output_stream_config,
            left_output_channel_index,
            right_output_channel_index,
            output_wave,
        )?;
        output_stream.pause()?;

        let left_input_channel_index: usize = active_input_device
            .2
            .clone()
            .parse()
            .unwrap_or(1usize)
            .saturating_sub(1);
        let right_input_channel_index: usize = active_input_device
            .3
            .clone()
            .parse()
            .unwrap_or(0usize)
            .saturating_sub(1);
        let input_stream_config: &StreamConfig = &input_device.default_input_config()?.config();

        let (producer, consumer) = unbounded();
        let channel_consumer = consumer;
        let channel_producer = producer.clone();

        let input_stream = get_current_input_steam(
            &input_device,
            input_stream_config,
            left_input_channel_index,
            right_input_channel_index,
            producer.clone(),
        )?;

        input_stream.pause()?;

        Ok(Self {
            host,
            input_device,
            output_device,
            output_stream,
            input_stream,
            channel_producer,
            channel_consumer,
            active_input_device,
            active_output_device,
            display_data,
        })
    }

    pub fn start(&mut self) {
        self.input_stream
            .play()
            .expect("Failed to start the level meter stream. Cannot continue.");

        self.output_stream
            .play()
            .expect("Failed to start the tone generator stream. Cannot continue.");
    }

    pub fn stop(&mut self) {
        self.input_stream
            .pause()
            .expect("Failed to stop the level meter stream. Cannot continue.");

        self.output_stream
            .pause()
            .expect("Failed to stop the tone generator stream. Cannot continue.");
    }

    pub fn get_display_data(&self) -> DisplayData {
        self.display_data.clone()
    }

    pub fn get_current_input_device_channels(&self) -> Vec<String> {
        self.display_data.input_device_list.1[self.active_input_device.0 as usize].clone()
    }

    pub fn get_current_output_device_channels(&self) -> Vec<String> {
        self.display_data.output_device_list.1[self.active_output_device.0 as usize].clone()
    }

    pub fn set_current_input_device_on_ui_callback(&mut self, input_device_data: (i32, String)) {
        self.stop();

        self.set_input_device_from_device_name(input_device_data.1.clone());

        let input_device_channels =
            &self.display_data.input_device_list.1[input_device_data.0 as usize];
        let input_left_channel = input_device_channels[0].clone();
        let input_right_channel = if input_device_channels.len() > 1 {
            input_device_channels[1].clone()
        } else {
            "".to_string()
        };

        self.active_input_device = (
            input_device_data.0,
            input_device_data.1,
            input_left_channel,
            input_right_channel,
        );

        let left_input_channel_index: usize = self
            .active_input_device
            .2
            .clone()
            .parse()
            .unwrap_or(1usize)
            .saturating_sub(1);
        let right_input_channel_index: usize = self
            .active_input_device
            .3
            .clone()
            .parse()
            .unwrap_or(0usize)
            .saturating_sub(1);

        let input_stream_config: &StreamConfig = &self
            .input_device
            .default_input_config()
            .unwrap_or_else(|err| {
                panic!("Output Stream error: {}", err);
            })
            .config();

        self.input_stream = get_current_input_steam(
            &self.input_device,
            input_stream_config,
            left_input_channel_index,
            right_input_channel_index,
            self.channel_producer.clone(),
        )
        .unwrap_or_else(|err| panic!("Output Stream error: {}", err));

        self.input_stream
            .pause()
            .unwrap_or_else(|err| panic!("Input Stream error: {}", err));
    }

    pub fn set_current_output_device_on_ui_callback(&mut self, output_device_data: (i32, String)) {
        self.stop();
        self.set_output_device_from_device_name(output_device_data.1.clone());

        let output_device_channels =
            &self.display_data.output_device_list.1[output_device_data.0 as usize];
        let output_left_channel = output_device_channels[0].clone();
        let output_right_channel = if output_device_channels.len() > 1 {
            output_device_channels[1].clone()
        } else {
            "".to_string()
        };

        self.active_output_device = (
            output_device_data.0,
            output_device_data.1,
            output_left_channel,
            output_right_channel,
        );

        let output_stream_config: &StreamConfig = &self
            .output_device
            .default_output_config()
            .unwrap_or_else(|err| {
                panic!("Output Stream error: {}", err);
            })
            .config();

        let sample_rate = output_stream_config.sample_rate.0 as f32;
        let output_wave = Sine::new(sample_rate);
        let left_output_channel_index: usize = self
            .active_output_device
            .2
            .clone()
            .parse()
            .unwrap_or(1usize)
            .saturating_sub(1);

        let right_output_channel_index: usize = self
            .active_output_device
            .3
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
        .unwrap_or_else(|err| panic!("Output Stream error: {}", err));

        self.output_stream
            .pause()
            .unwrap_or_else(|err| panic!("Output Stream error: {}", err));
    }

    pub fn set_input_channel_on_ui_callback(
        &mut self,
        left_input_channel: String,
        right_input_channel: String,
    ) {
        self.stop();
        self.active_input_device.2 = left_input_channel;
        self.active_input_device.3 = right_input_channel;

        let left_input_channel_index: usize = self
            .active_input_device
            .2
            .clone()
            .parse()
            .unwrap_or(1usize)
            .saturating_sub(1);

        let right_input_channel_index: usize = self
            .active_input_device
            .3
            .clone()
            .parse()
            .unwrap_or(0usize)
            .saturating_sub(1);

        let input_stream_config: &StreamConfig = &self
            .input_device
            .default_input_config()
            .unwrap_or_else(|err| {
                panic!("Input Stream error: {}", err);
            })
            .config();

        self.input_stream = get_current_input_steam(
            &self.input_device,
            input_stream_config,
            left_input_channel_index,
            right_input_channel_index,
            self.channel_producer.clone(),
        )
        .unwrap_or_else(|err| panic!("Input Stream error: {}", err));

        self.input_stream
            .pause()
            .unwrap_or_else(|err| panic!("Input Stream error: {}", err));
    }

    pub fn set_output_channel_on_ui_callback(
        &mut self,
        left_output_channel: String,
        right_output_channel: String,
    ) {
        self.stop();
        self.active_output_device.2 = left_output_channel;
        self.active_output_device.3 = right_output_channel;

        let output_stream_config: &StreamConfig = &self
            .output_device
            .default_output_config()
            .unwrap_or_else(|err| {
                panic!("Output Stream error: {}", err);
            })
            .config();

        let sample_rate = output_stream_config.sample_rate.0 as f32;
        let output_wave = Sine::new(sample_rate);
        let left_output_channel_index: usize =
            self.active_output_device.2.clone().parse().unwrap_or(1) - 1;
        let right_output_channel_index: usize =
            self.active_output_device.3.clone().parse().unwrap_or(0) - 1;

        self.output_stream = get_current_output_steam(
            &self.output_device,
            output_stream_config,
            left_output_channel_index,
            right_output_channel_index,
            output_wave,
        )
        .unwrap_or_else(|err| panic!("Output Stream error: {}", err));

        self.output_stream
            .pause()
            .unwrap_or_else(|err| panic!("Output Stream error: {}", err));
    }

    fn set_input_device_from_device_name(&mut self, device_name: String) {
        if let Ok(mut input_devices) = self.host.input_devices() {
            match input_devices.find(|device| {
                device.name().is_ok()
                    && device
                        .name()
                        .unwrap_or_else(|err| panic!("Output Stream error: {}", err))
                        == device_name
            }) {
                Some(device) => self.input_device = device,
                None => panic!(
                    "{}",
                    PANIC_MESSAGE_WHEN_DEVICE_PASSED_FROM_THE_UI_DOES_NOT_EXIST
                ),
            }
        } else {
            // Because this is called from a UI callback, there isn't a way to simply gracefully recover
            panic!(
                "{}",
                PANIC_MESSAGE_WHEN_DEVICE_PASSED_FROM_THE_UI_DOES_NOT_EXIST
            );
        }
    }

    fn set_output_device_from_device_name(&mut self, device_name: String) {
        if let Ok(mut output_devices) = self.host.output_devices() {
            match output_devices.find(|device| {
                device.name().is_ok() && device.name().unwrap_or_default() == device_name
            }) {
                Some(device) => self.output_device = device,
                None => panic!(
                    "{}",
                    PANIC_MESSAGE_WHEN_DEVICE_PASSED_FROM_THE_UI_DOES_NOT_EXIST
                ),
            }
        } else {
            // Because this is called from a UI callback, there isn't a way to simply gracefully recover
            panic!(
                "{}",
                PANIC_MESSAGE_WHEN_DEVICE_PASSED_FROM_THE_UI_DOES_NOT_EXIST
            );
        }
    }

    pub fn get_meter_reader(&mut self) -> Receiver<(Vec<f32>, Vec<f32>)> {
        self.channel_consumer.clone()
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
        |err| panic!("Output Stream error: {}", err),
        None,
    )
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
        |err| panic!("Output Stream error: {}", err),
        None,
    )
}

fn get_default_device_data_from_input_device(
    input_device: &Device,
    input_device_index: i32,
) -> Result<(i32, String, String, String), Box<dyn Error>> {
    let active_input_device = input_device.name()?;
    let default_input_channels = get_channel_list_from_input_device(input_device);
    let input_left_channel = default_input_channels[0].clone();
    let input_right_channel = if default_input_channels.len() > 1 {
        default_input_channels[1].clone()
    } else {
        "".to_string()
    };
    Ok((
        input_device_index,
        active_input_device,
        input_left_channel,
        input_right_channel,
    ))
}

fn get_default_device_data_from_output_device(
    output_device: &Device,
    output_device_index: i32,
) -> Result<(i32, String, String, String), Box<dyn Error>> {
    let active_output_device = output_device.name()?;
    let default_output_channels = get_channel_list_from_output_device(output_device);
    let output_left_channel = default_output_channels[0].clone();
    let output_right_channel = if default_output_channels.len() > 1 {
        default_output_channels[1].clone()
    } else {
        "".to_string()
    };
    Ok((
        output_device_index,
        active_output_device,
        output_left_channel,
        output_right_channel,
    ))
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

fn get_input_device_data_current_index(
    input_device_list: &[String],
    active_input_device: &String,
) -> i32 {
    input_device_list
        .iter()
        .position(|i| i == active_input_device)
        .unwrap_or(0) as i32
}

fn get_output_device_data_current_index(
    output_device_list: &[String],
    active_output_device: &String,
) -> i32 {
    output_device_list
        .iter()
        .position(|i| i == active_output_device)
        .unwrap_or(0) as i32
}

fn get_display_data_from_devices(host: &Host) -> Result<DisplayData, Box<dyn Error>> {
    let mut input_devices: Vec<String> = Vec::new();
    let mut input_channels: Vec<Vec<String>> = Vec::new();

    host.input_devices()?.for_each(|device| {
        if let Ok(name) = device.name() {
            input_devices.push(name);
            input_channels.push(get_channel_list_from_input_device(&device));
        }
    });

    let mut output_devices: Vec<String> = Vec::new();
    let mut output_channels: Vec<Vec<String>> = Vec::new();

    host.output_devices()?.for_each(|device| {
        if let Ok(name) = device.name() {
            output_devices.push(name);
            output_channels.push(get_channel_list_from_output_device(&device));
        }
    });

    let input_device_list = (input_devices, input_channels);
    let output_device_list = (output_devices, output_channels);

    Ok(DisplayData {
        input_device_list,
        output_device_list,
    })
}
