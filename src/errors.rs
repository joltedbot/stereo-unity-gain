use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LocalError {
    #[error("Error Initializing Audio Devices")]
    DeviceInitialization,

    #[error("No Default Input Audio Devices")]
    NoDefaultInputDevice,

    #[error("No Default Output Audio Devices")]
    NoDefaultOutputDevice,

    #[error("Error Initializing Level")]
    MeterInitialization,
}

pub fn handle_localerror(local_error: LocalError, specific_error: String) {
    eprintln!("\n{}: {}", local_error, specific_error);
}
