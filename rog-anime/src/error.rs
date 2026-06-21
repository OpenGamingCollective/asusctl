use gif::DecodingError;
use png_pong::decode::Error as PngError;

pub type Result<T> = std::result::Result<T, AnimeError>;

#[derive(thiserror::Error, Debug)]
pub enum AnimeError {
    #[error("No frames in PNG")]
    NoFrames,

    #[error("Could not open: {0}")]
    Io(#[source] std::io::Error),

    #[error("PNG error: {0}")]
    Png(#[source] PngError),

    #[error("GIF error: {0}")]
    Gif(#[source] DecodingError),

    #[error("PNG file is not 8bit greyscale")]
    Format,

    #[error("The input image size is incorrect, expected {0}x{1}")]
    IncorrectSize(u32, u32),

    #[error("{0}")]
    Dbus(String),

    #[error("udev {0}: {1}")]
    Udev(String, #[source] std::io::Error),

    #[error("No AniMe Matrix device found")]
    NoDevice,

    #[error("Unsupported AniMe Matrix device found")]
    UnsupportedDevice,

    #[error("Image brightness must be between 0.0 and 1.0 (inclusive), was {0}")]
    InvalidBrightness(f32),

    #[error("The data buffer was incorrect length for generating USB packets")]
    DataBufferLength,

    #[error("The gif used for pixel-perfect gif is wider than {0}")]
    PixelGifWidth(usize),

    #[error("The gif used for pixel-perfect gif is taller than {0}")]
    PixelGifHeight(usize),

    #[error("Could not parse {0}")]
    ParseError(String),
}

impl From<std::io::Error> for AnimeError {
    fn from(err: std::io::Error) -> Self {
        AnimeError::Io(err)
    }
}

impl From<PngError> for AnimeError {
    fn from(err: PngError) -> Self {
        AnimeError::Png(err)
    }
}

impl From<DecodingError> for AnimeError {
    fn from(err: DecodingError) -> Self {
        AnimeError::Gif(err)
    }
}

impl From<AnimeError> for zbus::fdo::Error {
    fn from(err: AnimeError) -> Self {
        zbus::fdo::Error::Failed(format!("{}", err))
    }
}
