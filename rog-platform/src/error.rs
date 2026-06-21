use zbus::fdo::Error as FdoErr;

pub type Result<T> = std::result::Result<T, PlatformError>;

#[derive(thiserror::Error, Debug)]
pub enum PlatformError {
    #[error("Parse gfx vendor error")]
    ParseVendor,

    #[error("Parse number error")]
    ParseNum,

    #[error("udev {0}: {1}")]
    Udev(String, #[source] std::io::Error),

    #[error("usb {0}")]
    USB(#[source] rusb::Error),

    #[error("Path {0}: {1}")]
    Path(String, #[source] std::io::Error),

    #[error("Read {0}: {1}")]
    Read(String, #[source] std::io::Error),

    #[error("Write {0}: {1}")]
    Write(String, #[source] std::io::Error),

    #[error("Not supported")]
    NotSupported,

    #[error("Attribute not found: {0}")]
    AttrNotFound(String),

    #[error("Missing functionality: {0}")]
    MissingFunction(String),

    #[error(
        "Led node at {0} is missing, please check you have the required patch or dkms module \
         installed: {1}"
    )]
    MissingLedBrightNode(String, #[source] std::io::Error),

    #[error("{0} {1}")]
    IoPath(String, #[source] std::io::Error),

    #[error("std::io error: {0}")]
    Io(#[source] std::io::Error),

    #[error("The input value did not match the attribute value type")]
    InvalidValue,

    #[error("No supported Aura keyboard")]
    NoAuraKeyboard,

    #[error("No Aura keyboard node found")]
    NoAuraNode,

    #[error("CPU control: {0}")]
    CPU(String),
}

impl From<rusb::Error> for PlatformError {
    fn from(err: rusb::Error) -> Self {
        PlatformError::USB(err)
    }
}

impl From<std::io::Error> for PlatformError {
    fn from(err: std::io::Error) -> Self {
        PlatformError::Io(err)
    }
}

impl From<PlatformError> for FdoErr {
    fn from(error: PlatformError) -> Self {
        match error {
            PlatformError::NotSupported => FdoErr::NotSupported("".to_owned()),
            _ => FdoErr::Failed(error.to_string()),
        }
    }
}
