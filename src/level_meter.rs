use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig, StreamError};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::error::Error;

#[derive(PartialEq)]
enum StreamState {
    Playing,
    Stopped,
}

pub struct LevelMeter {
    device: Device,
    left_channel_index: usize,
    right_channel_index: usize,
    stream: Stream,
    stream_state: StreamState,
    channel_consumer: Receiver<(Vec<f32>, Vec<f32>)>,
    channel_producer: Sender<(Vec<f32>, Vec<f32>)>,
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

        let (producer, consumer) = unbounded();

        let stream_producer = producer.clone();
        let channel_consumer = consumer;
        let channel_producer = producer;

        let stream = get_input_stream_for_current_device(
            &device,
            left_channel_index,
            right_channel_index,
            stream_producer,
        )?;

        stream.pause()?;

        Ok(Self {
            device,
            stream,
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
        self.stream_state = StreamState::Stopped;
    }

    pub fn get_meter_reader(&mut self) -> Receiver<(Vec<f32>, Vec<f32>)> {
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

        self.stream = get_input_stream_for_current_device(
            &self.device,
            self.left_channel_index,
            self.right_channel_index,
            self.channel_producer.clone(),
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

        self.stream = get_input_stream_for_current_device(
            &self.device,
            self.left_channel_index,
            self.right_channel_index,
            self.channel_producer.clone(),
        )?;

        self.stream.pause()?;

        Ok(())
    }
}

fn get_input_stream_for_current_device(
    device: &Device,
    left_channel_index: usize,
    right_channel_index: usize,
    producer: Sender<(Vec<f32>, Vec<f32>)>,
) -> Result<Stream, Box<dyn Error>> {
    let stream_config: StreamConfig = device.default_input_config()?.into();
    let number_of_channels = stream_config.channels;

    let new_stream = device.build_input_stream(
        &stream_config,
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
        stream_error_callback,
        None,
    )?;

    Ok(new_stream)
}

fn stream_error_callback(err: StreamError) {
    panic!("Input Stream error: {}", err);
}
