use thiserror::Error;

use rog_anime::error::AnimeError;

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

    #[error("Anime error: {0}")]
    Anime(#[source] AnimeError),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<AnimeError> for Error {
    fn from(err: AnimeError) -> Self {
        Error::Anime(err)
    }
}

impl From<Error> for zbus::fdo::Error {
    fn from(err: Error) -> Self {
        zbus::fdo::Error::Failed(err.to_string())
    }
}
