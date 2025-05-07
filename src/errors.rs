use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LocalError {
    #[error("Error Initializing User Interface")]
    UIInitialization,

    #[error("Error Creating User Interface")]
    UIRun,

    #[error("Error Initializing Audio Devices")]
    DeviceInitialization,

    #[error("No Default Input Audio Devices")]
    NoDefaultInputDevice,

    #[error("No Default Output Audio Devices")]
    NoDefaultOutputDevice,

    #[error("No Default Input Audio Device Channels")]
    NoDefaultInputChannels,

    #[error("No Default Output Audio Device Channels")]
    NoDefaultOutputChannels,
}

pub fn handle_localerror(local_error: LocalError, specific_error: String) {
    eprintln!("\n{}: {}", local_error, specific_error);
}
