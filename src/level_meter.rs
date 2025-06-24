use crate::device_manager::{CurrentDevice, DeviceList, get_channel_indexes_from_channel_names};
use crate::errors::{EXIT_CODE_ERROR, LocalError, handle_local_error};
use crate::events::EventType;
use cpal::traits::*;
use cpal::{Device, Stream, default_host};
use crossbeam_channel::{Receiver, Sender};
use rtrb::{Consumer, Producer, RingBuffer};
use std::error::Error;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread;

const ERROR_MESSAGE_INPUT_STREAM_ERROR: &str = "Input Stream error!";
const ERROR_MESSAGE_RUN_LOOP: &str =
    "Could not reference shared data withing the Level Meter Run Loop!";
const INPUT_BUFFERS_FOR_PEAK_CALCULATION: usize = 20;
const DEFAULT_DELTA_MODE: bool = true;
const RING_BUFFER_SIZE: usize = 1024;

struct SampleFrameBuffer {
    is_alive: bool,
    error_message: String,
    left: Vec<f32>,
    right: Vec<f32>,
}

pub struct LevelMeter {
    input_stream: Stream,
    current_input_device: CurrentDevice,
    input_device_list: DeviceList,
    sample_consumer: Arc<Mutex<Consumer<SampleFrameBuffer>>>,
    sample_producer: Arc<Mutex<Producer<SampleFrameBuffer>>>,
    delta_mode_enabled: Arc<Mutex<bool>>,
    reference_level: Arc<Mutex<f32>>,
    ui_command_receiver: Receiver<EventType>,
}

impl LevelMeter {
    pub fn new(
        input_device_list: DeviceList,
        current_input_device: CurrentDevice,
        ui_command_receiver: Receiver<EventType>,
        default_reference_level: f32,
    ) -> Result<Self, Box<dyn Error>> {
        let input_device = get_input_device_from_device_name(&current_input_device.name)?;

        let (left_input_channel_index, right_input_channel_index) =
            get_channel_indexes_from_channel_names(
                &current_input_device.left_channel,
                &current_input_device.right_channel,
            )?;

        let (sample_producer, sample_consumer) = RingBuffer::new(RING_BUFFER_SIZE);
        let sample_producer_mutex = Arc::new(Mutex::new(sample_producer));

        let input_stream = create_input_stream(
            &input_device,
            left_input_channel_index,
            right_input_channel_index,
            sample_producer_mutex.clone(),
        )?;

        input_stream.pause()?;

        let default_delta_mode_enabled = DEFAULT_DELTA_MODE;
        let delta_mode_enabled = Arc::new(Mutex::new(default_delta_mode_enabled));
        let reference_level = Arc::new(Mutex::new(default_reference_level));

        Ok(Self {
            input_stream,
            sample_consumer: Arc::new(Mutex::new(sample_consumer)),
            sample_producer: sample_producer_mutex,
            ui_command_receiver,
            current_input_device,
            input_device_list,
            delta_mode_enabled,
            reference_level,
        })
    }

    pub fn run(
        &mut self,
        level_meter_display_sender: Sender<EventType>,
    ) -> Result<(), Box<dyn Error>> {
        self.start_level_meter(level_meter_display_sender)?;

        let event_consumer = self.ui_command_receiver.clone();

        loop {
            if let Ok(event) = event_consumer.try_recv() {
                match event {
                    EventType::MeterModeUpdate(new_delta_mode) => {
                        match self.delta_mode_enabled.lock() {
                            Ok(mut delta_mode) => *delta_mode = new_delta_mode,
                            Err(_) => {
                                handle_local_error(
                                    LocalError::LevelMeterInitialization,
                                    ERROR_MESSAGE_RUN_LOOP.to_string(),
                                );
                                exit(EXIT_CODE_ERROR);
                            }
                        };
                    }
                    EventType::ToneLevelUpdate(new_level) => {
                        match self.reference_level.lock() {
                            Ok(mut level) => *level = new_level,
                            Err(_) => {
                                handle_local_error(
                                    LocalError::LevelMeterInitialization,
                                    ERROR_MESSAGE_RUN_LOOP.to_string(),
                                );
                                exit(EXIT_CODE_ERROR);
                            }
                        };
                    }
                    EventType::MeterDeviceUpdate { name, left, right } => {
                        self.current_input_device = CurrentDevice {
                            name,
                            left_channel: left,
                            right_channel: right,
                        };
                        self.set_input_device_on_ui_callback()?;
                    }
                    EventType::InputDeviceListUpdate(device_list) => {
                        self.input_device_list = device_list;
                    }
                    EventType::Start => self.start()?,
                    EventType::Stop => self.stop()?,
                    _ => (),
                }
            };
        }
    }

    pub fn start(&mut self) -> Result<(), LocalError> {
        self.input_stream
            .play()
            .map_err(|err| LocalError::LevelMeterStart(err.to_string()))?;

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), LocalError> {
        self.input_stream
            .pause()
            .map_err(|err| LocalError::LevelMeterStop(err.to_string()))?;
        Ok(())
    }

    pub fn set_input_device_on_ui_callback(&mut self) -> Result<(), LocalError> {
        self.stop()?;

        let input_device = get_input_device_from_device_name(&self.current_input_device.name)?;

        let (left_input_channel_index, right_input_channel_index) =
            get_channel_indexes_from_channel_names(
                &self.current_input_device.left_channel,
                &self.current_input_device.right_channel,
            )?;

        self.input_stream = create_input_stream(
            &input_device,
            left_input_channel_index,
            right_input_channel_index,
            self.sample_producer.clone(),
        )
        .map_err(|err| LocalError::InputStream(err.to_string()))?;

        self.input_stream
            .pause()
            .map_err(|err| LocalError::InputStream(err.to_string()))?;

        Ok(())
    }

    pub fn start_level_meter(
        &mut self,
        level_meter_display_sender: Sender<EventType>,
    ) -> Result<(), Box<dyn Error>> {
        let mut left_input_buffer_collection: Vec<Vec<f32>> = Vec::new();
        let mut right_input_buffer_collection: Vec<Vec<f32>> = Vec::new();
        let mut last_left_peak = 0.0;
        let mut last_right_peak = 0.0;
        let sample_receiver_mutex = self.sample_consumer.clone();

        let refence_level_arc = self.reference_level.clone();
        let delta_mode_enabled_arc = self.delta_mode_enabled.clone();

        thread::spawn(move || {
            let mut sample_receiver = match sample_receiver_mutex.lock() {
                Ok(sample_receiver) => sample_receiver,
                Err(err) => {
                    eprintln!(
                        "Level Meter Run: {}: {}",
                        ERROR_MESSAGE_INPUT_STREAM_ERROR, err
                    );
                    exit(EXIT_CODE_ERROR);
                }
            };

            loop {
                if let Ok(sample_buffers) = sample_receiver.pop() {
                    if !sample_buffers.is_alive {
                        if let Err(error) = level_meter_display_sender
                            .send(EventType::FatalError(sample_buffers.error_message))
                        {
                            eprintln!("Level Meter Display Error: {}", error);
                        }
                    }

                    if left_input_buffer_collection.len() > INPUT_BUFFERS_FOR_PEAK_CALCULATION {
                        let mut left_samples_buffer: Vec<f32> = left_input_buffer_collection
                            .iter()
                            .flatten()
                            .copied()
                            .collect();

                        left_input_buffer_collection.truncate(0);

                        let mut right_samples_buffer: Vec<f32> = right_input_buffer_collection
                            .iter()
                            .flatten()
                            .copied()
                            .collect();

                        right_input_buffer_collection.truncate(0);

                        let mut left = get_peak_of_sine_wave_samples(&mut left_samples_buffer);
                        let mut right = get_peak_of_sine_wave_samples(&mut right_samples_buffer);

                        if last_left_peak != left || last_right_peak != right {
                            last_left_peak = left;
                            last_right_peak = right;

                            let reference_level = match refence_level_arc.lock() {
                                Ok(level) => level.to_owned(),
                                Err(_) => {
                                    handle_local_error(
                                        LocalError::LevelMeterInitialization,
                                        ERROR_MESSAGE_RUN_LOOP.to_string(),
                                    );
                                    exit(EXIT_CODE_ERROR);
                                }
                            };

                            let delta_mode_enabled = match delta_mode_enabled_arc.lock() {
                                Ok(enabled) => enabled.to_owned(),
                                Err(_) => {
                                    handle_local_error(
                                        LocalError::LevelMeterInitialization,
                                        ERROR_MESSAGE_RUN_LOOP.to_string(),
                                    );
                                    exit(EXIT_CODE_ERROR);
                                }
                            };

                            if delta_mode_enabled {
                                left -= reference_level;
                                right -= reference_level;
                            }

                            let left_formatted = format_peak_delta_values_for_display(left);
                            let right_formatted = format_peak_delta_values_for_display(right);

                            if let Err(error) =
                                level_meter_display_sender.send(EventType::MeterLevelUpdate {
                                    left: left_formatted,
                                    right: right_formatted,
                                })
                            {
                                eprintln!("Error sending event: {}", error);
                            };
                        }
                    }

                    left_input_buffer_collection.insert(0, sample_buffers.left);
                    right_input_buffer_collection.insert(0, sample_buffers.right);
                }
            }
        });

        Ok(())
    }
}

fn create_input_stream(
    device: &Device,
    left_channel_index: usize,
    right_channel_index: Option<usize>,
    sample_producer_mutex: Arc<Mutex<Producer<SampleFrameBuffer>>>,
) -> Result<Stream, LocalError> {
    let config_result = device
        .default_input_config()
        .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

    let stream_config = config_result.config();

    let number_of_channels = stream_config.channels;

    let mut left_channel_samples = Vec::new();
    let mut right_channel_samples = Vec::new();

    let error_producer_mutex = sample_producer_mutex.clone();

    device
        .build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                data.chunks_exact(number_of_channels as usize)
                    .for_each(|frame| {
                        left_channel_samples.push(frame[left_channel_index]);
                        if let Some(index) = right_channel_index {
                            right_channel_samples.push(frame[index]);
                        }
                    });

                let mut sample_producer = match sample_producer_mutex.lock() {
                    Ok(sample_producer) => sample_producer,
                    Err(err) => {
                        eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
                        exit(EXIT_CODE_ERROR);
                    }
                };

                if let Err(err) = sample_producer.push(SampleFrameBuffer {
                    is_alive: true,
                    error_message: String::new(),
                    left: left_channel_samples.clone(),
                    right: right_channel_samples.clone(),
                }) {
                    eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
                }

                left_channel_samples.clear();
                right_channel_samples.clear();
            },
            move |error| {
                let mut error_producer = match error_producer_mutex.lock() {
                    Ok(error_producer) => error_producer,
                    Err(err) => {
                        eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
                        exit(EXIT_CODE_ERROR);
                    }
                };

                if let Err(err) = error_producer.push(SampleFrameBuffer {
                    is_alive: false,
                    error_message: error.to_string(),
                    left: Vec::new(),
                    right: Vec::new(),
                }) {
                    eprintln!("{}: {}", ERROR_MESSAGE_INPUT_STREAM_ERROR, err);
                    exit(EXIT_CODE_ERROR);
                }
            },
            None,
        )
        .map_err(|err| LocalError::InputStream(err.to_string()))
}

fn get_input_device_from_device_name(device_name: &str) -> Result<Device, LocalError> {
    let host = default_host();

    let mut input_devices = host
        .input_devices()
        .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;

    match input_devices
        .find(|device| device.name().is_ok() && device.name().unwrap() == device_name)
    {
        Some(device) => Ok(device),
        None => Err(LocalError::DeviceNotFound(device_name.to_string())),
    }
}

fn get_peak_of_sine_wave_samples(samples: &mut [f32]) -> f32 {
    let peak = samples.iter().fold(0.0f32, |acc, &x| x.abs().max(acc));
    get_dbfs_from_sample_value(peak)
}

fn get_dbfs_from_sample_value(sample: f32) -> f32 {
    20.0 * (sample.abs().log10())
}

fn format_peak_delta_values_for_display(peak_delta_value: f32) -> String {
    if peak_delta_value.is_infinite() || peak_delta_value.is_nan() {
        "-".to_string()
    } else if (peak_delta_value < 0.0) & (peak_delta_value > -0.1) {
        "0.0".to_string()
    } else if peak_delta_value > 0.1 {
        format!("+{:.1}", peak_delta_value)
    } else {
        format!("{:.1}", peak_delta_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_correct_peak_of_sine_wave_samples() {
        let mut test_samples = [0.1, -0.5, 0.3, 0.7, -0.2];
        let peak_sample = get_peak_of_sine_wave_samples(&mut test_samples);
        // The peak is 0.7, so dbfs should be 20*log10(0.7)
        let expected_result = 20.0 * 0.7_f32.abs().log10();
        assert!((peak_sample - expected_result).abs() < 1e-5);
    }

    #[test]
    fn return_neg_infinity_for_peak_of_sine_wave_samples_when_samples_are_empty() {
        let mut test_samples: [f32; 0] = [];
        let dbfs = get_peak_of_sine_wave_samples(&mut test_samples);
        assert_eq!(dbfs, f32::NEG_INFINITY);
    }

    #[test]
    fn return_correct_dbfs_from_valid_sample() {
        let dbfs = get_dbfs_from_sample_value(-0.5);
        let expected_result = -6.0206003;
        assert_eq!(dbfs, expected_result);
    }

    #[test]
    fn return_negative_infinity_dbfs_when_sample_value_is_zero() {
        let dbfs = get_dbfs_from_sample_value(0.0);
        assert_eq!(dbfs, f32::NEG_INFINITY);
    }

    #[test]
    fn return_dash_delta_value_for_display_if_infinity_nan_or_negative_infinity() {
        let dash_delta_values = [f32::NEG_INFINITY, f32::INFINITY, f32::NAN];
        let expected_result = "-";

        for value in dash_delta_values {
            let result = format_peak_delta_values_for_display(value);
            assert_eq!(result, expected_result);
        }
    }
}
