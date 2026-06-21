use zbus::fdo::Error as FdoErr;

#[derive(thiserror::Error, Debug)]
pub enum ProfileError {
    #[error("Path {0}: {1}")]
    Path(String, #[source] std::io::Error),

    #[error("Read {0}: {1}")]
    Read(String, #[source] std::io::Error),

    #[error("Write {0}: {1}")]
    Write(String, #[source] std::io::Error),

    #[error("Not supported")]
    NotSupported,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("std::io error: {0}")]
    Io(#[source] std::io::Error),

    #[error("Invalid profile name")]
    ParseProfileName,

    #[error("Could not parse number to 0-255: {0}")]
    ParseFanCurveDigit(#[source] std::num::ParseIntError),

    #[error("Invalid {0}, previous value {1} is higher than next value {2}")]
    ParseFanCurvePrevHigher(&'static str, u8, u8),

    #[error("Invalid percentage, {0} is higher than 100")]
    ParseFanCurvePercentOver100(u8),

    #[error("Less than 8 curve points supplied")]
    NotEnoughPoints,
}

impl From<std::io::Error> for ProfileError {
    fn from(err: std::io::Error) -> Self {
        ProfileError::Io(err)
    }
}

impl From<ProfileError> for FdoErr {
    fn from(error: ProfileError) -> Self {
        match error {
            ProfileError::NotSupported => FdoErr::NotSupported("".to_owned()),
            _ => FdoErr::Failed(error.to_string()),
        }
    }
}
