use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{ChannelCount, Device, Stream, StreamConfig, StreamError};
use std::error::Error;

const TONE_FREQUENCY: f32 = 1000.0;
const RADS_PER_CYCLE: f32 = 2.0 * std::f32::consts::PI;
const DBFS_SAMPLE_ADJUSTMENT_FACTOR: f32 = 0.25118864; // I need to reduce it by -12 or 10.0_f32.powf(-12.0 / 20.0)

#[derive(Clone)]
struct SineWave {
    pub phase: f32,
    pub sample_rate: u32,
    pub seconds_per_sample: f32,
    pub phase_increment: f32,
}

pub struct ToneGenerator {
    device: Device,
    number_of_channels: ChannelCount,
    left_channel_index: usize,
    right_channel_index: usize,
    stream: Stream,
    stream_config: StreamConfig,
    wave: SineWave,
}

impl ToneGenerator {
    pub fn new(
        device: Device,
        left_channel: u8,
        right_channel: u8,
    ) -> Result<Self, Box<dyn Error>> {
        let left_channel_index = (left_channel - 1) as usize;
        let mut right_channel_index = right_channel as usize;
        right_channel_index = right_channel_index.saturating_sub(1);

        let phase: f32 = 0.0;
        let stream_config: StreamConfig = device.default_output_config()?.into();
        let sample_rate = stream_config.sample_rate.0;
        let seconds_per_sample = 1.0 / sample_rate as f32;
        let number_of_channels = stream_config.channels;
        let phase_increment = RADS_PER_CYCLE * TONE_FREQUENCY * seconds_per_sample;

        let wave = SineWave {
            phase,
            sample_rate,
            seconds_per_sample,
            phase_increment,
        };

        let mut closure_wave = wave.clone();

        let stream = device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                craete_output_stream(
                    data,
                    number_of_channels,
                    left_channel_index,
                    right_channel_index,
                    &mut closure_wave,
                )
            },
            stream_error_callback,
            None,
        )?;

        stream.pause()?;

        Ok(Self {
            device,
            stream,
            stream_config,
            wave,
            number_of_channels,
            left_channel_index,
            right_channel_index,
        })
    }

    pub fn start(&mut self) {
        self.stream
            .play()
            .expect("Failed to start tone generator stream. Can not continue.");
    }

    pub fn stop(&mut self) {
        self.stream
            .pause()
            .expect("Failed to stop tone generator stream. Can not continue.");
    }

    pub fn change_device(
        &mut self,
        new_device: Device,
        left_channel: u8,
        right_channel: u8,
    ) -> Result<(), Box<dyn Error>> {
        self.stop();

        self.device = new_device.to_owned();
        self.left_channel_index = (left_channel - 1) as usize;
        self.right_channel_index = right_channel.saturating_sub(1) as usize;

        self.stream_config = new_device.default_output_config()?.into();
        self.wave.sample_rate = self.stream_config.sample_rate.0;
        self.number_of_channels = self.stream_config.channels;
        self.wave.seconds_per_sample = 1.0 / self.wave.sample_rate as f32;

        let number_of_channels = self.number_of_channels;
        let mut wave = self.wave.clone();
        let left_channel_index = self.left_channel_index;
        let right_channel_index = self.right_channel_index;

        self.stream = self.device.build_output_stream(
            &self.stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                craete_output_stream(
                    data,
                    number_of_channels,
                    left_channel_index,
                    right_channel_index,
                    &mut wave,
                )
            },
            stream_error_callback,
            None,
        )?;

        self.stop();

        Ok(())
    }

    pub fn change_channel(
        &mut self,
        left_channel: u8,
        right_channel: u8,
    ) -> Result<(), Box<dyn Error>> {
        self.stop();

        self.left_channel_index = (left_channel - 1) as usize;
        self.right_channel_index = right_channel.saturating_sub(1) as usize;

        let number_of_channels = self.number_of_channels;
        let mut wave = self.wave.clone();
        let left_channel_index = self.left_channel_index;
        let right_channel_index = self.right_channel_index;

        self.stream = self.device.build_output_stream(
            &self.stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                craete_output_stream(
                    data,
                    number_of_channels,
                    left_channel_index,
                    right_channel_index,
                    &mut wave,
                )
            },
            stream_error_callback,
            None,
        )?;

        self.stop();

        Ok(())
    }
}

fn craete_output_stream(
    data: &mut [f32],
    number_of_channels: ChannelCount,
    left_channel_index: usize,
    right_channel_index: usize,
    wave: &mut SineWave,
) {
    for channels in data.chunks_mut(number_of_channels as usize) {
        channels[left_channel_index] = wave.phase.sin() * DBFS_SAMPLE_ADJUSTMENT_FACTOR;
        channels[right_channel_index] = wave.phase.sin() * DBFS_SAMPLE_ADJUSTMENT_FACTOR;
        wave.phase += wave.phase_increment;
        if wave.phase >= RADS_PER_CYCLE {
            wave.phase = 0.0;
        }
    }
}

fn stream_error_callback(err: StreamError) {
    panic!("Output Stream error: {}", err);
}
