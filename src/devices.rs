use crate::errors::LocalError;
use cpal::traits::*;
use cpal::{default_host, Device, DeviceNameError};
use std::error::Error;

pub struct Devices {
    pub output_device_list: Vec<String>,
    pub input_device_list: Vec<String>,
    pub input_device: Device,
    pub output_device: Device,
    pub input_channel_list: Vec<String>,
    pub output_channel_list: Vec<String>,
    pub left_input_channel: String,
    pub right_input_channel: String,
    pub left_output_channel: String,
    pub right_output_channel: String,
}

impl Devices {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let host = default_host();

        let input_device_list: Vec<String> = host
            .input_devices()?
            .filter_map(|device| device.name().ok())
            .collect();

        let output_device_list: Vec<String> = host
            .output_devices()?
            .filter_map(|device| device.name().ok())
            .collect();

        let input_device = host
            .default_input_device()
            .ok_or(LocalError::NoDefaultInputDevice)?;

        let output_device = host
            .default_output_device()
            .ok_or(LocalError::NoDefaultOutputDevice)?;

        let number_of_input_channels = input_device
            .supported_input_configs()?
            .last()
            .ok_or(LocalError::NoDefaultInputChannels)?
            .channels();

        let number_of_output_channels = output_device
            .supported_output_configs()?
            .last()
            .ok_or(LocalError::NoDefaultOutputChannels)?
            .channels();

        let input_channel_list: Vec<String> = (1..=number_of_input_channels)
            .map(|i| i.to_string())
            .collect();

        let output_channel_list: Vec<String> = (1..=number_of_output_channels)
            .map(|i| i.to_string())
            .collect();

        let left_input_channel = input_channel_list[0].clone();
        let left_output_channel = output_channel_list[0].clone();

        let right_input_channel = if number_of_input_channels > 1 {
            input_channel_list[1].clone()
        } else {
            "".to_string()
        };

        let right_output_channel = if number_of_output_channels > 1 {
            output_channel_list[1].clone()
        } else {
            "".to_string()
        };

        Ok(Self {
            input_device_list,
            output_device_list,
            input_device,
            output_device,
            input_channel_list,
            output_channel_list,
            left_input_channel,
            right_input_channel,
            left_output_channel,
            right_output_channel,
        })
    }

    pub fn get_current_input_device_name(&self) -> Result<String, DeviceNameError> {
        self.input_device.name()
    }

    pub fn get_current_output_device_name(&self) -> Result<String, DeviceNameError> {
        self.output_device.name()
    }

    pub fn set_current_input_device(&mut self, device_name: String) {
        let host = default_host();
        self.input_device = host
            .input_devices()
            .unwrap()
            .find(|d| d.name().unwrap().contains(device_name.as_str()))
            .unwrap();
    }

    pub fn set_current_output_device(&mut self, device_name: String) {
        let host = default_host();
        self.output_device = host
            .output_devices()
            .unwrap()
            .find(|d| d.name().unwrap().contains(device_name.as_str()))
            .unwrap();
    }

    pub fn set_current_left_input_channel(&mut self, channel_name: String) {
        self.left_input_channel = channel_name;
    }

    pub fn set_current_right_input_channel(&mut self, channel_name: String) {
        self.right_input_channel = channel_name;
    }

    pub fn set_current_left_output_channel(&mut self, channel_name: String) {
        self.left_output_channel = channel_name;
    }

    pub fn set_current_right_output_channel(&mut self, channel_name: String) {
        self.right_output_channel = channel_name;
    }
}
