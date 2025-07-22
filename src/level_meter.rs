use crate::device_manager::get_channel_indexes_from_channel_names;
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
const INPUT_BUFFERS_FOR_PEAK_CALCULATION: usize = 20;
const RING_BUFFER_SIZE: usize = 1024;

struct SampleFrameBuffer {
    is_alive: bool,
    error_message: String,
    left: Vec<f32>,
    right: Vec<f32>,
}

pub struct LevelMeter {
    input_stream: Option<Stream>,
    sample_consumer: Arc<Mutex<Consumer<SampleFrameBuffer>>>,
    sample_producer: Arc<Mutex<Producer<SampleFrameBuffer>>>,
    ui_command_receiver: Receiver<EventType>,
}

impl LevelMeter {
    pub fn new(ui_command_receiver: Receiver<EventType>) -> Result<Self, Box<dyn Error>> {
        let (sample_producer, sample_consumer) = RingBuffer::new(RING_BUFFER_SIZE);
        let sample_producer_arc = Arc::new(Mutex::new(sample_producer));

        Ok(Self {
            input_stream: None,
            sample_consumer: Arc::new(Mutex::new(sample_consumer)),
            sample_producer: sample_producer_arc,
            ui_command_receiver,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let event_consumer = self.ui_command_receiver.clone();

        loop {
            if let Ok(event) = event_consumer.try_recv() {
                match event {
                    EventType::Start => self.start()?,
                    EventType::Stop => self.stop()?,
                    EventType::MeterDeviceUpdate { name, left, right } => {
                        self.update_input_stream_on_new_device(name, left, right)?
                    }
                    _ => (),
                }
            };
        }
    }

    fn start(&mut self) -> Result<(), LocalError> {
        if let Some(ref mut stream) = self.input_stream {
            stream
                .play()
                .map_err(|err| LocalError::LevelMeterStart(err.to_string()))?;
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<(), LocalError> {
        if let Some(ref mut stream) = self.input_stream {
            stream
                .pause()
                .map_err(|err| LocalError::LevelMeterStop(err.to_string()))?;
        }
        Ok(())
    }

    fn update_input_stream_on_new_device(
        &mut self,
        device_name: String,
        left_channel: String,
        right_channel: Option<String>,
    ) -> Result<(), LocalError> {
        self.stop()?;

        let input_device = get_input_device_from_device_name(&device_name)?;

        let (left_input_channel_index, right_input_channel_index) =
            get_channel_indexes_from_channel_names(&left_channel, &right_channel)?;

        let input_stream = create_input_stream(
            &input_device,
            left_input_channel_index,
            right_input_channel_index,
            self.sample_producer.clone(),
        )
        .map_err(|err| LocalError::LevelMeterConfigureInputStream(err.to_string()))?;

        input_stream
            .pause()
            .map_err(|err| LocalError::LevelMeterConfigureInputStream(err.to_string()))?;

        self.input_stream = Some(input_stream);

        Ok(())
    }

    pub fn run_input_sample_processor(
        &mut self,
        user_interface_sender: Sender<EventType>,
    ) -> Result<(), Box<dyn Error>> {
        let mut left_input_buffer_collector: Vec<Vec<f32>> = Vec::new();
        let mut right_input_buffer_collector: Vec<Vec<f32>> = Vec::new();
        let mut previous_left_peak: f32 = 0.0;
        let mut previous_right_peak: f32 = 0.0;

        let sample_receiver_arc = self.sample_consumer.clone();

        thread::spawn(move || {
            let mut sample_receiver = sample_receiver_arc
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            loop {
                if let Ok(sample_buffers) = sample_receiver.pop() {
                    check_and_exit_if_sample_ring_buffer_is_not_alive(
                        &user_interface_sender,
                        &sample_buffers,
                    );

                    if left_input_buffer_collector.len() > INPUT_BUFFERS_FOR_PEAK_CALCULATION {
                        let mut left_samples_buffer: Vec<f32> =
                            consolidate_sample_buffer_collector_to_sample_buffer(
                                &mut left_input_buffer_collector,
                            );
                        let mut right_samples_buffer: Vec<f32> =
                            consolidate_sample_buffer_collector_to_sample_buffer(
                                &mut right_input_buffer_collector,
                            );

                        let new_left_peak =
                            get_peak_value_of_collected_samples(&mut left_samples_buffer);
                        let new_right_peak =
                            get_peak_value_of_collected_samples(&mut right_samples_buffer);

                        if previous_left_peak != new_left_peak
                            || previous_right_peak != new_right_peak
                        {
                            previous_left_peak = new_left_peak;
                            previous_right_peak = new_right_peak;

                            send_updated_meter_values_to_the_ui(
                                &user_interface_sender,
                                new_left_peak,
                                new_right_peak,
                            );
                        }
                    }

                    left_input_buffer_collector.insert(0, sample_buffers.left);
                    right_input_buffer_collector.insert(0, sample_buffers.right);
                }
            }
        });

        Ok(())
    }
}

fn check_and_exit_if_sample_ring_buffer_is_not_alive(
    user_interface_sender: &Sender<EventType>,
    sample_buffers: &SampleFrameBuffer,
) {
    if !sample_buffers.is_alive {
        if let Err(err) = user_interface_sender.send(EventType::FatalError(
            LocalError::LevelMeterReadRingBuffer(sample_buffers.error_message.clone()).to_string(),
        )) {
            handle_local_error(LocalError::LevelMeterInputStreamFailure, err.to_string());
            exit(EXIT_CODE_ERROR);
        }
    }
}

fn send_updated_meter_values_to_the_ui(
    user_interface_sender: &Sender<EventType>,
    left: f32,
    right: f32,
) {
    if let Err(error) = user_interface_sender.send(EventType::MeterLevelUpdate { left, right }) {
        handle_local_error(LocalError::LevelMeterUISender, error.to_string());
        exit(1);
    };
}

fn consolidate_sample_buffer_collector_to_sample_buffer(
    input_buffer_collector: &mut Vec<Vec<f32>>,
) -> Vec<f32> {
    let input_buffer = input_buffer_collector.iter().flatten().copied().collect();
    input_buffer_collector.clear();
    input_buffer
}

fn create_input_stream(
    device: &Device,
    left_channel_index: usize,
    right_channel_index: Option<usize>,
    sample_producer_arc: Arc<Mutex<Producer<SampleFrameBuffer>>>,
) -> Result<Stream, LocalError> {
    let default_device_configuration = device
        .default_input_config()
        .map_err(|err| LocalError::DeviceConfiguration(err.to_string()))?;
    let stream_config = default_device_configuration.config();
    let number_of_channels = stream_config.channels;
    let mut left_channel_samples = Vec::new();
    let mut right_channel_samples = Vec::new();

    let error_producer_arc = sample_producer_arc.clone();

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

                let mut sample_producer = sample_producer_arc
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());

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
                let mut error_producer = error_producer_arc
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());

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
        .map_err(|err| LocalError::LevelMeterConfigureInputStream(err.to_string()))
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

fn get_peak_value_of_collected_samples(samples: &mut [f32]) -> f32 {
    let peak = samples.iter().fold(0.0f32, |acc, &x| x.abs().max(acc));
    get_dbfs_from_sample_value(peak)
}

fn get_dbfs_from_sample_value(sample: f32) -> f32 {
    20.0 * (sample.abs().log10())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_correct_peak_of_sine_wave_samples() {
        let mut test_samples = [0.1, -0.5, 0.3, 0.7, -0.2];
        let peak_sample = get_peak_value_of_collected_samples(&mut test_samples);
        // The peak is 0.7, so dbfs should be 20*log10(0.7)
        let expected_result = 20.0 * 0.7_f32.abs().log10();
        assert!((peak_sample - expected_result).abs() < 1e-5);
    }

    #[test]
    fn return_neg_infinity_for_peak_of_sine_wave_samples_when_samples_are_empty() {
        let mut test_samples: [f32; 0] = [];
        let dbfs = get_peak_value_of_collected_samples(&mut test_samples);
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
}
