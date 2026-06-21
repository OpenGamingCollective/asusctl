#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not parse colour")]
    ParseColour,

    #[error("Could not parse speed")]
    ParseSpeed,

    #[error("Could not parse direction")]
    ParseDirection,

    #[error("Could not parse brightness")]
    ParseBrightness,

    #[error("IO Error: {0}: {1}")]
    IoPath(String, #[source] std::io::Error),

    #[error("RON Parse Error: {0}")]
    Ron(#[source] ron::Error),

    #[error("RON Parse Error: {0}")]
    RonParse(#[source] ron::error::SpannedError),
}

impl From<ron::Error> for Error {
    fn from(e: ron::Error) -> Self {
        Self::Ron(e)
    }
}

impl From<ron::error::SpannedError> for Error {
    fn from(e: ron::error::SpannedError) -> Self {
        Self::RonParse(e)
    }
}
