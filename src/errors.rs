use thiserror::Error;

pub const EXIT_CODE_ERROR: i32 = 1;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LocalError {
    #[error("Error Initializing Tone Generator")]
    ToneGeneratorInitialization,

    #[error("Error Initializing Level Meter")]
    LevelMeterInitialization,

    #[error("Error Initializing User Interface")]
    UIInitialization,

    #[error("Error Populating User Interface")]
    UIInitialPopulation,

    #[error("Error Initializing Audio Devices")]
    DeviceManagerInitialization,

    #[error("Could not initialize the meter reader UI process")]
    MeterReaderUIUpdater,

    #[error("No Default Input Audio Devices")]
    NoDefaultInputDevice,

    #[error("No Default Output Audio Devices")]
    NoDefaultOutputDevice,

    #[error("Error Retrieving the Audio Devices Data for the UI: {0}")]
    UIDeviceData(String),

    #[error("Error starting the tone generator: {0}")]
    ToneGeneratorStart(String),

    #[error("Error stoping the tone generator: {0}")]
    ToneGeneratorStop(String),

    #[error("Error starting the level meter: {0}")]
    LevelMeterStart(String),

    #[error("Error stoping the level meter: {0}")]
    LevelMeterStop(String),

    #[error("Device {0} no longer exists")]
    DeviceNotFound(String),

    #[error("Failed to configure an output stream: {0}")]
    OutputStream(String),

    #[error("Failed to configure an input stream: {0}")]
    InputStream(String),

    #[error("Failed to access device configuration: {0}")]
    DeviceConfiguration(String),

    #[error("Failed to generate channel index: {0}")]
    ChannelIndex(String),

    #[error("A device named {0} no longer exists on this host.")]
    DeviceNameNotPresent(String),
}

pub fn handle_local_error(local_error: LocalError, specific_error: String) {
    eprintln!("\n{}: {}", local_error, specific_error);
}
