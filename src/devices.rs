mod input;
mod output;

use crate::devices::input::{CurrentInputDevice, InputDevices};
use crate::devices::output::{CurrentOutputDevice, OutputDevices};
use crossbeam_channel::Receiver;
use std::error::Error;

#[derive(Clone)]
pub struct DeviceList {
    pub devices: Vec<String>,
    pub channels: Vec<Vec<String>>,
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

    pub fn start(&mut self) {
        self.input_devices.start();
        self.output_devices.start();
    }

    pub fn stop(&mut self) {
        self.input_devices.stop();
        self.input_devices.stop();
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

    pub fn get_current_input_device(&self) -> CurrentInputDevice {
        self.input_devices.current_input_device.clone()
    }

    pub fn get_current_output_device(&self) -> CurrentOutputDevice {
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

    pub fn set_current_input_device_on_ui_callback(&mut self, input_device_data: (i32, String)) {
        self.input_devices
            .set_current_input_device_on_ui_callback(input_device_data)
    }

    pub fn set_current_output_device_on_ui_callback(&mut self, output_device_data: (i32, String)) {
        self.output_devices
            .set_current_output_device_on_ui_callback(output_device_data)
    }

    pub fn set_input_channel_on_ui_callback(
        &mut self,
        left_input_channel: String,
        right_input_channel: String,
    ) {
        self.input_devices
            .set_input_channel_on_ui_callback(left_input_channel, right_input_channel)
    }

    pub fn set_output_channel_on_ui_callback(
        &mut self,
        left_output_channel: String,
        right_output_channel: String,
    ) {
        self.output_devices
            .set_output_channel_on_ui_callback(left_output_channel, right_output_channel)
    }
}
