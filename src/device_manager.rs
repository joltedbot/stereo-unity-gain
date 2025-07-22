use crate::errors::LocalError;
use crate::events::EventType;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, default_host};
use crossbeam_channel::Sender;
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

const RUN_LOOP_SLEEP_DURATION_IN_MILLISECONDS: u64 = 300;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct DeviceList {
    pub devices: Vec<String>,
    pub channels: Vec<Vec<String>>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct CurrentDevice {
    pub name: String,
    pub left_channel: String,
    pub right_channel: Option<String>,
}

pub struct DeviceManager {
    user_interface_sender: Sender<EventType>,
    input_device_sender: Sender<EventType>,
    output_device_sender: Sender<EventType>,
    input_devices: DeviceList,
    output_devices: DeviceList,
    initial_input_device: CurrentDevice,
    current_output_device: CurrentDevice,
}

impl DeviceManager {
    pub fn new(
        user_interface_sender: Sender<EventType>,
        input_device_sender: Sender<EventType>,
        output_device_sender: Sender<EventType>,
    ) -> Result<Self, Box<dyn Error>> {
        let input_devices = get_input_device_list_from_host()?;
        let current_input_device = get_default_input_device_data(&input_devices)?;
        let output_devices = get_output_device_list_from_host()?;
        let current_output_device = get_default_output_device_data(&output_devices)?;

        Ok(Self {
            user_interface_sender,
            input_device_sender,
            output_device_sender,
            input_devices,
            output_devices,
            initial_input_device: current_input_device,
            current_output_device,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.input_device_sender
            .send(EventType::MeterDeviceUpdate {
                name: self.initial_input_device.name.clone(),
                left: self.initial_input_device.left_channel.clone(),
                right: self.initial_input_device.right_channel.clone(),
            })?;

        loop {
            let input_devices = get_input_device_list_from_host()?;
            let output_devices = get_output_device_list_from_host()?;

            if input_devices != self.input_devices {
                self.input_devices = input_devices;
                self.user_interface_sender
                    .send(EventType::InputDeviceListUpdate(self.input_devices.clone()))?;
                self.input_device_sender
                    .send(EventType::InputDeviceListUpdate(self.input_devices.clone()))?;
            }

            if output_devices != self.output_devices {
                self.output_devices = output_devices;
                self.user_interface_sender
                    .send(EventType::OutputDeviceListUpdate(
                        self.output_devices.clone(),
                    ))?;
                self.output_device_sender
                    .send(EventType::OutputDeviceListUpdate(
                        self.output_devices.clone(),
                    ))?;
            }

            sleep(Duration::from_millis(
                RUN_LOOP_SLEEP_DURATION_IN_MILLISECONDS,
            ));
        }
    }

    pub fn get_input_devices(&self) -> DeviceList {
        self.input_devices.clone()
    }
    pub fn get_output_devices(&self) -> DeviceList {
        self.output_devices.clone()
    }

    pub fn get_current_input_device(&self) -> CurrentDevice {
        self.initial_input_device.clone()
    }

    pub fn get_current_output_device(&self) -> CurrentDevice {
        self.current_output_device.clone()
    }
}

fn get_default_input_device_data(
    input_devices: &DeviceList,
) -> Result<CurrentDevice, Box<dyn Error>> {
    let host = default_host();

    let name = input_devices.devices[0].clone();

    let device = host
        .input_devices()?
        .find(|device| device.name().iter().any(|device_name| device_name == &name))
        .ok_or(LocalError::NoDefaultInputDevice)?;

    let default_input_channels = get_channel_list_from_input_device(&device);

    let left_channel = default_input_channels[0].clone();

    let right_channel = if default_input_channels.len() > 1 {
        Some(default_input_channels[1].clone())
    } else {
        None
    };

    Ok(CurrentDevice {
        name,
        left_channel,
        right_channel,
    })
}

fn get_default_output_device_data(
    output_devices: &DeviceList,
) -> Result<CurrentDevice, Box<dyn Error>> {
    let host = default_host();

    let name = output_devices.devices[0].clone();

    let device = host
        .output_devices()?
        .find(|device| device.name().iter().any(|device_name| device_name == &name))
        .ok_or(LocalError::NoDefaultOutputDevice)?;

    let default_output_channels = get_channel_list_from_output_device(&device);

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
    let left_index = get_channel_index_from_name(left_channel)?;
    let mut right_index: Option<usize> = None;

    if right_channel.is_some() {
        right_index = Some(get_channel_index_from_name(
            right_channel.as_ref().unwrap(),
        )?);
    }

    Ok((left_index, right_index))
}

fn get_channel_index_from_name(channel: &str) -> Result<usize, LocalError> {
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
        let result = get_channel_index_from_name(test_str).unwrap();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn return_zero_index_from_channel_name_that_produces_a_negative_index() {
        let test_str = "0";
        let expected_result = 0;
        let result = get_channel_index_from_name(test_str).unwrap();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn return_correct_error_from_alpha_channel_name() {
        let test_str = "abc";
        let expected_result = LocalError::ChannelIndex("invalid digit found in string".to_string());
        let result = get_channel_index_from_name(test_str).unwrap_err();
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
