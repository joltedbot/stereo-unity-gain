use crate::errors::{EXIT_CODE_ERROR, LocalError, handle_local_error};
use crate::ui::UI;
use crate::{DEFAULT_REFERENCE_FREQUENCY, DEFAULT_REFERENCE_LEVEL};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, default_host};
use std::error::Error;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct DeviceList {
    pub devices: Vec<String>,
    pub channels: Vec<Vec<String>>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct CurrentDevice {
    pub index: i32,
    pub name: String,
    pub left_channel: String,
    pub right_channel: Option<String>,
}

pub struct DeviceManager {
    input_devices: DeviceList,
    output_devices: DeviceList,
    current_input_device: CurrentDevice,
    current_output_device: CurrentDevice,
}

impl DeviceManager {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let input_devices = get_input_device_list_from_host()?;
        let current_input_device = get_default_input_device_data(&input_devices)?;
        let output_devices = get_output_device_list_from_host()?;
        let current_output_device = get_default_output_device_data(&output_devices)?;

        Ok(Self {
            input_devices,
            output_devices,
            current_input_device,
            current_output_device,
        })
    }

    pub fn update_device_lists(&mut self) -> Result<(), Box<dyn Error>> {
        self.input_devices = get_input_device_list_from_host()?;
        self.output_devices = get_output_device_list_from_host()?;

        Ok(())
    }

    pub fn run(&mut self, ui_mutex: Arc<Mutex<UI>>) -> Result<(), Box<dyn Error>> {
        loop {
            let input_devices = get_input_device_list_from_host()?;
            let output_devices = get_output_device_list_from_host()?;

            if input_devices != self.input_devices || output_devices != self.output_devices {
                println!("Updating device lists...");
                self.update_device_lists()?;
                self.current_input_device = get_default_input_device_data(&input_devices)?;
                self.current_output_device = get_default_output_device_data(&output_devices)?;

                let mut ui = match ui_mutex.lock() {
                    Ok(ui) => ui,
                    Err(err) => {
                        eprintln!("Device Manager Run: {}", err);
                        continue;
                    }
                };

                if let Err(err) = ui.initialize_ui_with_device_data(
                    self.get_input_devices(),
                    self.get_current_input_device(),
                    self.get_output_devices(),
                    self.get_current_output_device(),
                    DEFAULT_REFERENCE_FREQUENCY,
                    DEFAULT_REFERENCE_LEVEL,
                ) {
                    handle_local_error(LocalError::UIInitialization, err.to_string());
                    exit(EXIT_CODE_ERROR);
                }
            }

            sleep(Duration::from_millis(100));
        }
    }

    pub fn get_input_devices(&self) -> DeviceList {
        self.input_devices.clone()
    }
    pub fn get_output_devices(&self) -> DeviceList {
        self.output_devices.clone()
    }

    pub fn get_current_input_device(&self) -> CurrentDevice {
        self.current_input_device.clone()
    }

    pub fn get_current_output_device(&self) -> CurrentDevice {
        self.current_output_device.clone()
    }
}

fn get_default_input_device_data(
    input_devices: &DeviceList,
) -> Result<CurrentDevice, Box<dyn Error>> {
    let host = default_host();

    let device = host
        .default_input_device()
        .ok_or(LocalError::NoDefaultInputDevice)?;

    let name = device.name()?;
    let index = input_devices
        .devices
        .iter()
        .position(|i| i == &name)
        .unwrap_or(0) as i32;

    let default_input_channels = get_channel_list_from_input_device(&device);

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

fn get_default_output_device_data(
    output_devices: &DeviceList,
) -> Result<CurrentDevice, Box<dyn Error>> {
    let host = default_host();

    let device = host
        .default_output_device()
        .ok_or(LocalError::NoDefaultOutputDevice)?;

    let name = device.name()?;
    let index = output_devices
        .devices
        .iter()
        .position(|i| i == &name)
        .unwrap_or(0) as i32;

    let default_output_channels = get_channel_list_from_output_device(&device);

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

fn get_input_device_list_from_host() -> Result<DeviceList, Box<dyn Error>> {
    let mut input_devices: Vec<String> = Vec::new();
    let mut input_channels: Vec<Vec<String>> = Vec::new();

    let host = default_host();

    host.input_devices()?.for_each(|device| {
        if let Ok(name) = device.name() {
            input_devices.push(name);
            let input_channel = get_channel_list_from_input_device(&device);
            input_channels.push(input_channel);
        }
    });

    Ok(DeviceList {
        devices: input_devices,
        channels: input_channels,
    })
}

fn get_output_device_list_from_host() -> Result<DeviceList, Box<dyn Error>> {
    let mut output_devices: Vec<String> = Vec::new();
    let mut output_channels: Vec<Vec<String>> = Vec::new();

    let host = default_host();

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

fn get_channel_list_from_input_device(input_device: &Device) -> Vec<String> {
    if let Ok(config) = input_device.default_input_config() {
        let number_of_input_channels = config.channels();
        let channels = (1..=number_of_input_channels)
            .map(|i| i.to_string())
            .collect();

        return channels;
    };

    Vec::new()
}

fn get_channel_list_from_output_device(output_device: &Device) -> Vec<String> {
    if let Ok(config) = output_device.default_output_config() {
        let number_of_output_channels = config.channels();
        let channels = (1..=number_of_output_channels)
            .map(|i| i.to_string())
            .collect();

        return channels;
    };

    Vec::new()
}

pub fn get_channel_indexes_from_channel_names(
    left_channel: &str,
    right_channel: &Option<String>,
) -> Result<(usize, Option<usize>), LocalError> {
    let left_index = get_index_from_name(left_channel)?;
    let mut right_index: Option<usize> = None;

    if right_channel.is_some() {
        right_index = Some(get_index_from_name(right_channel.as_ref().unwrap())?);
    }

    Ok((left_index, right_index))
}

fn get_index_from_name(channel: &str) -> Result<usize, LocalError> {
    let channel_number = channel
        .parse::<usize>()
        .map_err(|err| LocalError::ChannelIndex(err.to_string()))?;

    Ok(channel_number.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_correct_index_from_valid_channel_name() {
        let test_str = "3";
        let expected_result = 2;
        let result = get_index_from_name(test_str).unwrap();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn return_zero_index_from_channel_name_that_produces_a_negative_index() {
        let test_str = "0";
        let expected_result = 0;
        let result = get_index_from_name(test_str).unwrap();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn return_correct_error_from_alpha_channel_name() {
        let test_str = "abc";
        let expected_result = LocalError::ChannelIndex("invalid digit found in string".to_string());
        let result = get_index_from_name(test_str).unwrap_err();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn return_correct_channel_indexes_from_valid_channel_names() {
        let test_left = "2";
        let test_right = Some(3.to_string());
        let (left, right) = get_channel_indexes_from_channel_names(test_left, &test_right).unwrap();
        assert_eq!(left, 1);
        assert_eq!(right, Some(2));
    }

    #[test]
    fn return_correct_channel_indexes_from_only_left_channel_name() {
        let test_left = "2";
        let test_right = None;
        let (left, right) = get_channel_indexes_from_channel_names(test_left, &test_right).unwrap();
        assert_eq!(left, 1);
        assert_eq!(right, None);
    }
}
