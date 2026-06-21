use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to open: {0}")]
    Io(#[source] std::io::Error),

    #[error("Failed to load user config")]
    ConfigLoadFail,

    #[error("Failed to lock user config")]
    ConfigLockFail,

    #[error("XDG environment vars appear unset")]
    XdgVars,

    #[error("Zbus error: {0}")]
    Zbus(#[source] zbus::Error),

    #[error("Zbus FDO error: {0}")]
    ZbusFdo(#[source] zbus::fdo::Error),

    #[error("Notification error: {0}")]
    Notification(#[source] notify_rust::error::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<zbus::Error> for Error {
    fn from(err: zbus::Error) -> Self {
        Error::Zbus(err)
    }
}

impl From<zbus::fdo::Error> for Error {
    fn from(err: zbus::fdo::Error) -> Self {
        Error::ZbusFdo(err)
    }
}

impl From<notify_rust::error::Error> for Error {
    fn from(err: notify_rust::error::Error) -> Self {
        Error::Notification(err)
    }
}
