use crate::device_manager::DeviceList;
use crossbeam_channel::{Receiver, Sender, unbounded};

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    MeterLevelUpdate {
        left: String,
        right: String,
    },
    MeterModeUpdate(bool),
    MeterDeviceUpdate {
        name: String,
        left: String,
        right: Option<String>,
    },
    ToneFrequencyUpdate(f32),
    ToneLevelUpdate(f32),
    ToneDeviceUpdate {
        name: String,
        left: String,
        right: Option<String>,
    },
    ToneModeUpdate(bool),
    InputDeviceListUpdate(DeviceList),
    OutputDeviceListUpdate(DeviceList),
    InputDeviceUpdate(String),
    OutputDeviceUpdate(String),
    InputChannelUpdate {
        left: String,
        right: Option<String>,
    },
    OutputChannelUpdate {
        left: String,
        right: Option<String>,
    },
    RecoverableError(String),
    FatalError(String),
    Start,
    Stop,
    Exit,
}

pub struct Events {
    tone_generator_sender: Sender<EventType>,
    tone_generator_receiver: Receiver<EventType>,
    level_meter_sender: Sender<EventType>,
    level_meter_receiver: Receiver<EventType>,
    user_interface_sender: Sender<EventType>,
    user_interface_receiver: Receiver<EventType>,
}

impl Events {
    pub fn new() -> Self {
        let (tone_generator_sender, tone_generator_receiver) = unbounded();
        let (level_meter_sender, level_meter_receiver) = unbounded();
        let (user_interface_sender, user_interface_receiver) = unbounded();

        Events {
            tone_generator_sender,
            tone_generator_receiver,
            level_meter_sender,
            level_meter_receiver,
            user_interface_sender,
            user_interface_receiver,
        }
    }

    pub fn get_tone_generator_sender(&self) -> Sender<EventType> {
        self.tone_generator_sender.clone()
    }

    pub fn get_tone_generator_receiver(&self) -> Receiver<EventType> {
        self.tone_generator_receiver.clone()
    }

    pub fn get_level_meter_sender(&self) -> Sender<EventType> {
        self.level_meter_sender.clone()
    }

    pub fn get_level_meter_receiver(&self) -> Receiver<EventType> {
        self.level_meter_receiver.clone()
    }

    pub fn get_user_interface_sender(&self) -> Sender<EventType> {
        self.user_interface_sender.clone()
    }

    pub fn get_user_interface_receiver(&self) -> Receiver<EventType> {
        self.user_interface_receiver.clone()
    }
}
