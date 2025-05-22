use crate::devices::{get_channel_indexes_from_channel_names, CurrentDevice, DeviceList};
use crate::errors::{LocalError, EXIT_CODE_ERROR};
use crate::ui::AppWindow;
use cpal::traits::*;
use cpal::{default_host, Device, Host, Stream};
use crossbeam_channel::{unbounded, Receiver, Sender};
use slint::{SharedString, Weak};
use std::error::Error;
use std::process::exit;
use std::thread;

const ERROR_MESSAGE_INPUT_STREAM_ERROR: &str = "Input Stream error!";
const NUMBER_OF_INPUT_BUFFERS_TO_USE_FOR_PEAK_CALCULATION: usize = 20;
const TARGET_OUTPUT_LEVEL: f32 = -12.0;
const DEFAULT_DELTA_MODE: bool = true;

pub type ReaderState = bool;

pub struct LevelMeter {
    input_device: Device,
    input_stream: Stream,
    current_input_device: CurrentDevice,
    input_device_list: DeviceList,
    sample_buffer_receiver: Receiver<(Vec<f32>, Vec<f32>)>,
    sample_buffer_sender: Sender<(Vec<f32>, Vec<f32>)>,
    meter_mode_receiver: Receiver<ReaderState>,
    meter_mode_sender: Sender<ReaderState>,
}

impl LevelMeter {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let host = default_host();

        let input_device_list = get_input_device_list_from_host(&host)?;

        let input_device = host
            .default_input_device()
            .ok_or(LocalError::NoDefaultInputDevice)?;

        let current_input_device =
            get_default_device_data_from_input_device(&input_device, &input_device_list.devices)?;

        let (left_input_channel_index, right_input_channel_index) =
            get_channel_indexes_from_channel_names(
                &current_input_device.left_channel,
                &current_input_device.right_channel,
            )?;

        let (sample_sender, sample_receiver) = unbounded();
        let sample_buffer_receiver = sample_receiver;
        let sample_buffer_sender = sample_sender.clone();

        let (mode_sender, mode_receiver) = unbounded();
        let meter_mode_receiver = mode_receiver;
        let meter_mode_sender = mode_sender;

        let input_stream = create_input_stream(
            &input_device,
            left_input_channel_index,
            right_input_channel_index,
            sample_sender,
        )?;

        input_stream.pause()?;

        Ok(Self {
            input_device,
            input_stream,
            sample_buffer_sender,
            sample_buffer_receiver,
            meter_mode_sender,
            meter_mode_receiver,
            current_input_device,
            input_device_list,
        })
    }

    pub fn start(&mut self) -> Result<(), LocalError> {
        self.input_stream
            .play()
            .map_err(|err| LocalError::LevelMeterStart(err.to_string()))?;

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), LocalError> {
        self.input_stream
            .pause()
            .map_err(|err| LocalError::LevelMeterStop(err.to_string()))?;
        Ok(())
    }

    pub fn get_current_input_device(&self) -> CurrentDevice {
        self.current_input_device.clone()
    }

    pub fn get_input_device_list(&self) -> DeviceList {
        self.input_device_list.clone()
    }

    pub fn get_current_input_device_channels(&self) -> Vec<String> {
        self.input_device_list.channels[self.current_input_device.index as usize].clone()
    }

    pub fn set_current_input_device_on_ui_callback(
        &mut self,
        input_device_data: (i32, String),
    ) -> Result<(), LocalError> {
        self.stop()?;
        self.set_input_device_on_ui_callback(input_device_data)
            .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))
    }

    pub fn set_input_device_on_ui_callback(
        &mut self,
        input_device_data: (i32, String),
    ) -> Result<(), LocalError> {
        self.input_device = self.get_input_device_from_device_name(input_device_data.1.clone())?;

        let input_device_channels = &self.input_device_list.channels[input_device_data.0 as usize];

        let left_channel = input_device_channels[0].clone();
        let right_channel = if input_device_channels.len() > 1 {
            Some(input_device_channels[1].clone())
        } else {
            None
        };

        self.current_input_device = CurrentDevice {
            index: input_device_data.0,
            name: input_device_data.1,
            left_channel,
            right_channel,
        };

        self.set_input_device(self.current_input_device.clone())?;

        Ok(())
    }

    pub fn set_input_channel_on_ui_callback(
        &mut self,
        left_input_channel: String,
        right_input_channel: Option<String>,
    ) -> Result<(), LocalError> {
        self.current_input_device.left_channel = left_input_channel;
        self.current_input_device.right_channel = right_input_channel;

        let (left_input_channel_index, right_input_channel_index) =
            get_channel_indexes_from_channel_names(
                &self.current_input_device.left_channel,
                &self.current_input_device.right_channel,
            )?;

        self.input_stream = create_input_stream(
            &self.input_device,
            left_input_channel_index,
            right_input_channel_index,
            self.sample_buffer_sender.clone(),
        )
        .map_err(|err| LocalError::InputStream(err.to_string()))?;

        self.input_stream
            .pause()
            .map_err(|err| LocalError::InputStream(err.to_string()))?;

        Ok(())
    }

    fn set_input_device(&mut self, device: CurrentDevice) -> Result<(), LocalError> {
        self.input_device = self.get_input_device_from_device_name(device.name.clone())?;
        self.current_input_device = device;

        let (left_input_channel_index, right_input_channel_index) =
            get_channel_indexes_from_channel_names(
                &self.current_input_device.left_channel,
                &self.current_input_device.right_channel,
            )?;

        self.input_stream = create_input_stream(
            &self.input_device,
            left_input_channel_index,
            right_input_channel_index,
            self.sample_buffer_sender.clone(),
        )
        .map_err(|err| LocalError::InputStream(err.to_string()))?;

        self.input_stream
            .pause()
            .map_err(|err| LocalError::InputStream(err.to_string()))?;

        Ok(())
    }

    fn get_input_device_from_device_name(
        &mut self,
        device_name: String,
    ) -> Result<Device, LocalError> {
        let host = default_host();

        let mut input_devices = host
            .input_devices()
            .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

        match input_devices
            .find(|device| device.name().is_ok() && device.name().unwrap() == device_name)
        {
            Some(device) => Ok(device),
            None => Err(LocalError::DeviceNotFound(device_name)),
        }
    }

    pub fn get_meter_mode_receiver(&mut self) -> Receiver<ReaderState> {
        self.meter_mode_receiver.clone()
    }

    pub fn get_meter_mode_sender(&mut self) -> Sender<ReaderState> {
        self.meter_mode_sender.clone()
    }

    pub fn get_sample_buffer_receiver(&mut self) -> Receiver<(Vec<f32>, Vec<f32>)> {
        self.sample_buffer_receiver.clone()
    }

    pub fn reset_to_default_input_device(&mut self) -> Result<CurrentDevice, LocalError> {
        self.stop()?;

        let host = default_host();
        let default_input_device = host
            .default_input_device()
            .ok_or(LocalError::NoDefaultInputDevice)?;

        let input_device_list = &self.input_device_list.devices;

        let current_input_device =
            get_default_device_data_from_input_device(&default_input_device, input_device_list)
                .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

        self.set_input_device(self.current_input_device.clone())?;

        Ok(current_input_device)
    }

    pub fn start_level_meter(&mut self, ui: Weak<AppWindow>) -> Result<(), Box<dyn Error>> {
        let mut left_input_buffer_collection: Vec<Vec<f32>> = Vec::new();
        let mut right_input_buffer_collection: Vec<Vec<f32>> = Vec::new();
        let mut last_left_peak = 0.0;
        let mut last_right_peak = 0.0;
        let mut delta_mode = DEFAULT_DELTA_MODE;
        let sample_receiver = self.sample_buffer_receiver.clone();
        let mode_receiver = self.meter_mode_receiver.clone();

        let ui_weak = ui;

        thread::spawn(move || {
            while let Ok((left_samples, right_samples)) = sample_receiver.recv() {
                if let Ok(delta_mode_enabled) = mode_receiver.try_recv() {
                    delta_mode = delta_mode_enabled;
                };

                if left_input_buffer_collection.len()
                    > NUMBER_OF_INPUT_BUFFERS_TO_USE_FOR_PEAK_CALCULATION
                {
                    let mut left_samples_buffer: Vec<f32> = left_input_buffer_collection
                        .iter()
                        .flatten()
                        .copied()
                        .collect();

                    left_input_buffer_collection.truncate(0);

                    let mut right_samples_buffer: Vec<f32> = right_input_buffer_collection
                        .iter()
                        .flatten()
                        .copied()
                        .collect();

                    right_input_buffer_collection.truncate(0);

                    let mut left = get_peak_of_sine_wave_samples(&mut left_samples_buffer);
                    let mut right = get_peak_of_sine_wave_samples(&mut right_samples_buffer);

                    if last_left_peak != left || last_right_peak != right {
                        last_left_peak = left;
                        last_right_peak = right;

                        if delta_mode {
                            left -= TARGET_OUTPUT_LEVEL;
                            right -= TARGET_OUTPUT_LEVEL;
                        }

                        let left_formatted = format_peak_delta_values_for_display(left);
                        let right_formatted = format_peak_delta_values_for_display(right);

                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_left_level_box_value(SharedString::from(left_formatted));
                            ui.set_right_level_box_value(SharedString::from(right_formatted));
                        });
                    }
                }

                left_input_buffer_collection.insert(0, left_samples);
                right_input_buffer_collection.insert(0, right_samples);
            }
        });

        Ok(())
    }
}

fn create_input_stream(
    device: &Device,
    left_channel_index: usize,
    right_channel_index: Option<usize>,
    producer: Sender<(Vec<f32>, Vec<f32>)>,
) -> Result<Stream, LocalError> {
    let config_result = device
        .default_input_config()
        .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

    let stream_config = config_result.config();

    let number_of_channels = stream_config.channels;

    let mut left_channel_samples = Vec::new();
    let mut right_channel_samples = Vec::new();

    device
        .build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                data.chunks_exact(number_of_channels as usize)
                    .for_each(|frame| {
                        left_channel_samples.push(frame[left_channel_index]);
                        if let Some(index) = right_channel_index {
                            right_channel_samples.push(frame[index]);
                        }
                    });

                if let Err(err) =
                    producer.send((left_channel_samples.clone(), right_channel_samples.clone()))
                {
                    eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
                }

                left_channel_samples.clear();
                right_channel_samples.clear();
            },
            |err| {
                eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
                exit(EXIT_CODE_ERROR);
            },
            None,
        )
        .map_err(|err| LocalError::InputStream(err.to_string()))
}

fn get_default_device_data_from_input_device(
    device: &Device,
    device_list: &[String],
) -> Result<CurrentDevice, Box<dyn Error>> {
    let name = device.name()?;
    let index = device_list.iter().position(|i| i == &name).unwrap_or(0) as i32;

    let default_input_channels = get_channel_list_from_input_device(device);

    let left_channel = default_input_channels[0].clone();

    let right_channel = if default_input_channels.len() > 1 {
        Some(default_input_channels[1].clone())
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

fn get_peak_of_sine_wave_samples(samples: &mut [f32]) -> f32 {
    let peak = samples.iter().fold(0.0f32, |acc, &x| x.abs().max(acc));
    get_dbfs_from_peak_sample(peak)
}

fn get_dbfs_from_peak_sample(sample: f32) -> f32 {
    20.0 * (sample.abs().log10())
}

fn format_peak_delta_values_for_display(peak_delta_value: f32) -> String {
    if peak_delta_value > 0.1 {
        format!("+{:.1}", peak_delta_value)
    } else if (peak_delta_value < 0.0) & (peak_delta_value > -0.1) {
        "0.0".to_string()
    } else if peak_delta_value == f32::NEG_INFINITY {
        "-".to_string()
    } else {
        format!("{:.1}", peak_delta_value)
    }
}
