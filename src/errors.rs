use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LocalError {
    #[error("Error Initializing Audio Devices")]
    DeviceInitialization,

    #[error("Error Initializing User Interface")]
    UIInitialization,

    #[error("Could not initialize the meter reader UI process")]
    MeterReaderUIUpdater,

    #[error("No Default Input Audio Devices")]
    NoDefaultInputDevice,

    #[error("No Default Output Audio Devices")]
    NoDefaultOutputDevice,

    #[error("Error Retrieving the Audio Devices Data for the UI")]
    UIDeviceData,
}

pub fn handle_local_error(local_error: LocalError, specific_error: String) {
    eprintln!("\n{}: {}", local_error, specific_error);
}
