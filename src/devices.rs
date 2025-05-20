mod input;
mod output;

use crate::devices::input::InputDevices;
use crate::devices::output::OutputDevices;
use crate::errors::LocalError;
use crossbeam_channel::Receiver;
use std::error::Error;

#[derive(Clone, Default, Debug)]
pub struct DeviceList {
    pub devices: Vec<String>,
    pub channels: Vec<Vec<String>>,
}

#[derive(Clone, Default, Debug)]
pub struct CurrentDevice {
    pub index: i32,
    pub name: String,
    pub left_channel: String,
    pub right_channel: Option<String>,
}

pub struct Devices {
    pub input_devices: InputDevices,
    pub output_devices: OutputDevices,
}

impl Devices {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let input_devices = InputDevices::new()?;
        let output_devices = OutputDevices::new()?;

        Ok(Self {
            input_devices,
            output_devices,
        })
    }

    pub fn start(&mut self) -> Result<(), LocalError> {
        self.input_devices.start()?;
        self.output_devices.start()?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), LocalError> {
        self.input_devices.stop()?;
        self.output_devices.stop()?;
        Ok(())
    }

    pub fn get_current_input_device(&self) -> CurrentDevice {
        self.input_devices.current_input_device.clone()
    }

    pub fn get_input_device_list(&self) -> DeviceList {
        self.input_devices.input_device_list.clone()
    }

    pub fn get_current_input_device_channels(&self) -> Vec<String> {
        self.input_devices.input_device_list.channels
            [self.input_devices.current_input_device.index as usize]
            .clone()
    }

    pub fn get_output_device_list(&self) -> DeviceList {
        self.output_devices.output_device_list.clone()
    }

    pub fn get_current_output_device(&self) -> CurrentDevice {
        self.output_devices.current_output_device.clone()
    }

    pub fn get_current_output_device_channels(&self) -> Vec<String> {
        self.output_devices.output_device_list.channels
            [self.output_devices.current_output_device.index as usize]
            .clone()
    }

    pub fn get_meter_reader(&mut self) -> Receiver<(Vec<f32>, Vec<f32>)> {
        self.input_devices.get_meter_reader()
    }

    pub fn set_current_input_device_on_ui_callback(
        &mut self,
        input_device_data: (i32, String),
    ) -> Result<(), LocalError> {
        self.stop()?;
        self.input_devices
            .set_input_device_on_ui_callback(input_device_data)
            .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))
    }

    pub fn set_current_output_device_on_ui_callback(
        &mut self,
        output_device_data: (i32, String),
    ) -> Result<(), LocalError> {
        self.stop()?;
        self.output_devices
            .set_output_device_on_ui_callback(output_device_data)
            .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))
    }

    pub fn set_input_channel_on_ui_callback(
        &mut self,
        left_input_channel: String,
        right_input_channel: Option<String>,
    ) -> Result<(), LocalError> {
        self.stop()?;
        self.input_devices
            .set_input_channel_on_ui_callback(left_input_channel, right_input_channel)
            .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))
    }

    pub fn set_output_channel_on_ui_callback(
        &mut self,
        left_output_channel: String,
        right_output_channel: Option<String>,
    ) -> Result<(), LocalError> {
        self.stop()?;
        self.output_devices
            .set_output_channel_on_ui_callback(left_output_channel, right_output_channel)
            .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))
    }
}

fn get_channel_index_from_channel_name(channel: &str) -> Result<usize, LocalError> {
    let channel_number = channel
        .parse::<usize>()
        .map_err(|err| LocalError::ChannelIndex(err.to_string()))?;

    Ok(channel_number.saturating_sub(1))
}

pub fn get_channel_indexes_from_channel_names(
    left_channel: &str,
    right_channel: &Option<String>,
) -> Result<(usize, Option<usize>), LocalError> {
    let left_index = get_channel_index_from_channel_name(left_channel)?;
    let mut right_index = None;

    if right_channel.is_some() {
        right_index = Some(get_channel_index_from_channel_name(
            right_channel.as_ref().unwrap(),
        )?);
    }

    Ok((left_index, right_index))
}
