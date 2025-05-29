use crate::device_manager::{CurrentDevice, DeviceList, get_channel_indexes_from_channel_names};
use crate::errors::{EXIT_CODE_ERROR, LocalError};
use crate::ui::EventType;
use cpal::traits::*;
use cpal::{Device, Stream, StreamError, default_host};
use crossbeam_channel::Receiver;
use sine::Sine;
use std::error::Error;
use std::process::exit;
use std::sync::{Arc, Mutex};

mod sine;

const ERROR_MESSAGE_OUTPUT_STREAM_ERROR: &str = "Output Stream Error!";

pub struct ToneGenerator {
    output_device: Device,
    output_stream: Stream,
    current_output_device: CurrentDevice,
    output_device_list: DeviceList,
    reference_frequency: Arc<Mutex<f32>>,
    reference_level: Arc<Mutex<f32>>,
    ui_command_receiver: Receiver<EventType>,
}

impl ToneGenerator {
    pub fn new(
        output_device_list: DeviceList,
        current_output_device: CurrentDevice,
        reference_frequency: f32,
        reference_level: f32,
        ui_command_receiver: Receiver<EventType>,
    ) -> Result<Self, Box<dyn Error>> {
        let host = default_host();

        let output_device = host
            .default_output_device()
            .ok_or(LocalError::NoDefaultOutputDevice)?;

        let (left_output_channel_index, right_output_channel_index) =
            get_channel_indexes_from_channel_names(
                &current_output_device.left_channel,
                &current_output_device.right_channel,
            )?;

        let reference_frequency_arc = Arc::new(Mutex::new(reference_frequency));
        let reference_level_arc = Arc::new(Mutex::new(reference_level));

        let output_stream = create_output_steam(
            &output_device,
            left_output_channel_index,
            right_output_channel_index,
            reference_frequency_arc.clone(),
            reference_level_arc.clone(),
        )?;

        output_stream.pause()?;

        Ok(Self {
            output_device,
            current_output_device,
            reference_frequency: reference_frequency_arc,
            reference_level: reference_level_arc,
            output_stream,
            output_device_list,
            ui_command_receiver,
        })
    }

    pub fn run(&mut self) {
        let ui_command_receiver = self.ui_command_receiver.clone();
        loop {
            if let Ok(event) = ui_command_receiver.try_recv() {
                match event {
                    EventType::Start => self.start().expect("Could Not Start Tone Generator"),
                    EventType::Stop => self.stop().expect("Could Not Stop Tone Generator"),
                    EventType::ToneDeviceUpdate { index, name } => {
                        self.set_output_device_on_ui_callback((index, name))
                            .expect("");
                    }
                    EventType::ToneChannelUpdate { left, right } => {
                        self.set_output_channel_on_ui_callback(left, right)
                            .expect("");
                    }
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
                    _ => (),
                }
            };
        }
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
            self.reference_frequency.clone(),
            self.reference_level.clone(),
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
            self.reference_frequency.clone(),
            self.reference_level.clone(),
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
}

fn create_output_steam(
    device: &Device,
    left_channel_index: usize,
    right_channel_index: Option<usize>,
    reference_frequency: Arc<Mutex<f32>>,
    reference_level: Arc<Mutex<f32>>,
) -> Result<Stream, LocalError> {
    let config_result = device
        .default_output_config()
        .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

    let stream_config = config_result.config();
    let number_of_channels = stream_config.channels;
    let sample_rate = stream_config.sample_rate.0 as f32;
    let mut wave = Sine::new(sample_rate);

    let initial_frequency = match reference_frequency.lock() {
        Ok(frequency) => frequency.to_owned(),
        Err(_) => return Err(LocalError::ToneGeneratorInitialization),
    };

    let initial_level = match reference_level.lock() {
        Ok(level) => level.to_owned(),
        Err(_) => return Err(LocalError::ToneGeneratorInitialization),
    };

    let mut dbfs_adjustment_factor = get_dbfs_adjustment_factor_from_level(initial_level);

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

        let current_dbfs_factor = get_dbfs_adjustment_factor_from_level(current_level);
        if (current_dbfs_factor - dbfs_adjustment_factor).abs() > 0.001 {
            dbfs_adjustment_factor = current_dbfs_factor;
        }

        for channels in data.chunks_mut(number_of_channels as usize) {
            let tone_sample = wave.generate_tone_sample(current_frequency, dbfs_adjustment_factor);
            channels[left_channel_index] = tone_sample;
            if let Some(index) = right_channel_index {
                channels[index] = tone_sample;
            }
        }
    };

    device
        .build_output_stream(&stream_config, callback, stream_error_callback, None)
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

fn stream_error_callback(err: StreamError) {
    eprintln!("{}: {}", ERROR_MESSAGE_OUTPUT_STREAM_ERROR, err);
    exit(EXIT_CODE_ERROR)
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

fn get_dbfs_adjustment_factor_from_level(level: f32) -> f32 {
    10.0_f32.powf(level / 20.0)
}
