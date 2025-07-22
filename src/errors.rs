use thiserror::Error;

pub const EXIT_CODE_ERROR: i32 = 1;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LocalError {
    #[error("Error Initializing Level Meter: {0}")]
    LevelMeterInitialization(String),

    #[error("Error Reading Input Samples from Buffer: {0}")]
    LevelMeterReadRingBuffer(String),

    #[error("Error sending level meter value updates to the UI")]
    LevelMeterUISender,

    #[error("Error retrieving shared Level Meter receiver. Recovering Data.")]
    LevelMeterReceiver,

    #[error("Error retrieving shared Level Meter reference level. Recovering Data.")]
    LevelMeterReferenceLevel,

    #[error("Error retrieving shared Level Meter delta mode state. Recovering Data.")]
    LevelMeterDeltaMode,

    #[error("Could not initialize the meter reader UI process")]
    LevelMeterReaderUIUpdater,

    #[error("Level Reader Exited Abnormally: {0}")]
    LevelMeterLoopExitError(String),

    #[error("The level meter input steam failed.")]
    LevelMeterInputStreamFailure,

    #[error("Error starting the level meter: {0}")]
    LevelMeterStart(String),

    #[error("Error stoping the level meter: {0}")]
    LevelMeterStop(String),

    #[error("Failed to configure an input stream: {0}")]
    LevelMeterConfigureInputStream(String),

    #[error("Error Initializing Tone Generator")]
    ToneGeneratorInitialization,

    #[error("Error starting the tone generator: {0}")]
    ToneGeneratorStart(String),

    #[error("Error stoping the tone generator: {0}")]
    ToneGeneratorStop(String),

    #[error("Failed to configure an output stream: {0}")]
    ToneGeneratorOutputStream(String),

    #[error("Error Initializing User Interface")]
    UIInitialization,

    #[error("Error Populating User Interface")]
    UIInitialPopulation,

    #[error("Error Retrieving the Audio Devices Data for the UI: {0}")]
    UIDeviceData(String),

    #[error("Error Initializing Audio Devices")]
    DeviceManagerInitialization,

    #[error("No Default Input Audio Devices")]
    NoDefaultInputDevice,

    #[error("No Default Output Audio Devices")]
    NoDefaultOutputDevice,

    #[error("Device {0} no longer exists")]
    DeviceNotFound(String),

    #[error("A device named {0} no longer exists on this host.")]
    DeviceNameNotPresent(String),

    #[error("Failed to access device configuration: {0}")]
    DeviceConfiguration(String),

    #[error("Failed to generate channel index: {0}")]
    ChannelIndex(String),

    #[error("A fatal error has occured and the application is not exiting.")]
    FatalError,
}

pub fn handle_local_error(local_error: LocalError, specific_error: String) {
    eprintln!("\n{}: {}", local_error, specific_error);
}
