use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{ChannelCount, Device, Stream, StreamConfig, StreamError};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::error::Error;

#[derive(PartialEq)]
enum StreamState {
    Playing,
    Stopped,
}

pub struct LevelMeter {
    device: Device,
    number_of_channels: ChannelCount,
    left_channel_index: usize,
    right_channel_index: usize,
    stream: Stream,
    stream_state: StreamState,
    stream_config: StreamConfig,
    channel_consumer: Receiver<(f32, f32)>,
    channel_producer: Sender<(f32, f32)>,
}

impl LevelMeter {
    pub fn new(
        device: Device,
        left_channel: u8,
        right_channel: u8,
    ) -> Result<Self, Box<dyn Error>> {
        let left_channel_index = (left_channel - 1) as usize;
        let mut right_channel_index = right_channel as usize;
        right_channel_index = right_channel_index.saturating_sub(1);

        let stream_config: StreamConfig = device.default_input_config()?.into();
        let number_of_channels = stream_config.channels;

        let (producer, consumer) = unbounded();

        let thread_producer = producer.clone();
        let channel_consumer = consumer;
        let channel_producer = producer;

        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for channels in data.chunks_exact(number_of_channels as usize) {
                    let left = channels[left_channel_index];
                    let right = channels[right_channel_index];

                    match thread_producer.send((left, right)) {
                        Ok(_) => {}
                        Err(error) => {
                            println!("Error sending data to channel consumer: {}", error);
                        }
                    }
                }
            },
            stream_error_callback,
            None,
        )?;

        stream.pause()?;

        Ok(Self {
            device,
            stream,
            stream_config,
            number_of_channels,
            left_channel_index,
            right_channel_index,
            channel_consumer,
            channel_producer,
            stream_state: StreamState::Stopped,
        })
    }

    pub fn start(&mut self) {
        if self.stream_state == StreamState::Playing {
            return;
        }

        println!("Start Meter");
        self.stream
            .play()
            .expect("Failed to start meter stream. Cannot continue.");
        self.stream_state = StreamState::Playing;
    }

    pub fn stop(&mut self) {
        if self.stream_state == StreamState::Stopped {
            return;
        }

        println!("Stop Meter");
        self.stream
            .pause()
            .expect("Failed to stop meter stream. Cannot continue.");

        let _ = self.channel_producer.send((999.9, 0.0));

        self.stream_state = StreamState::Stopped;
    }

    pub fn get_meter_reader(&mut self) -> Receiver<(f32, f32)> {
        self.channel_consumer.clone()
    }

    pub fn change_device(
        &mut self,
        new_device: Device,
        left_channel: u8,
        right_channel: u8,
    ) -> Result<(), Box<dyn Error>> {
        self.stream.pause()?;

        self.device = new_device.to_owned();
        self.left_channel_index = (left_channel - 1) as usize;
        self.right_channel_index = right_channel.saturating_sub(1) as usize;

        self.stream_config = self.device.default_input_config()?.into();
        let number_of_channels = self.stream_config.channels;
        self.number_of_channels = number_of_channels;

        let left_channel_index = self.left_channel_index;
        let right_channel_index = self.right_channel_index;

        let producer = self.channel_producer.clone();

        self.stream = self.device.build_input_stream(
            &self.stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for channels in data.chunks_exact(number_of_channels as usize) {
                    let left = channels[left_channel_index];
                    let right = channels[right_channel_index];

                    match producer.send((left, right)) {
                        Ok(_) => {}
                        Err(error) => {
                            println!("Error sending data to channel consumer: {}", error);
                        }
                    }
                }
            },
            stream_error_callback,
            None,
        )?;

        self.stream.pause()?;

        Ok(())
    }

    pub fn change_channel(
        &mut self,
        left_channel: u8,
        right_channel: u8,
    ) -> Result<(), Box<dyn Error>> {
        self.stream.pause()?;

        self.left_channel_index = (left_channel - 1) as usize;
        self.right_channel_index = right_channel.saturating_sub(1) as usize;

        let number_of_channels = self.stream_config.channels;
        self.number_of_channels = number_of_channels;

        let left_channel_index = self.left_channel_index;
        let right_channel_index = self.right_channel_index;

        let producer = self.channel_producer.clone();

        self.stream = self.device.build_input_stream(
            &self.stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for channels in data.chunks_exact(number_of_channels as usize) {
                    let left = channels[left_channel_index];
                    let right = channels[right_channel_index];

                    let _skip_bad_samples = producer.send((left, right));
                }
            },
            stream_error_callback,
            None,
        )?;

        self.stream.pause()?;

        Ok(())
    }
}

fn stream_error_callback(err: StreamError) {
    panic!("Input Stream error: {}", err);
}
