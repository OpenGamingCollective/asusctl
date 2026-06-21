pub type Result<T> = std::result::Result<T, SlashError>;

#[derive(thiserror::Error, Debug)]
pub enum SlashError {
    #[error("{0}")]
    Dbus(String),

    #[error("udev {0}: {1}")]
    Udev(String, #[source] std::io::Error),

    #[error("No Slash device found")]
    NoDevice,

    #[error("Unsupported Slash device found")]
    UnsupportedDevice,

    #[error("The data buffer was incorrect length for generating USB packets")]
    DataBufferLength,

    #[error("Could not parse {0}")]
    ParseError(String),
}

impl From<SlashError> for zbus::fdo::Error {
    fn from(err: SlashError) -> Self {
        zbus::fdo::Error::Failed(format!("{}", err))
    }
}
