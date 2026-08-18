use image::ImageError;

pub type Result<T> = std::result::Result<T, AnimeError>;

#[derive(thiserror::Error, Debug)]
pub enum AnimeError {
    #[error("No frames in image")]
    NoFrames,

    #[error("Could not open: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image error: {0}")]
    Image(#[from] ImageError),

    #[error("Image file format error")]
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

    #[error("Image width cannot be zero")]
    ZeroWidth,

    #[error("The data buffer was incorrect length for generating USB packets")]
    DataBufferLength,

    #[error("The gif used for pixel-perfect gif is wider than {0}")]
    PixelGifWidth(usize),

    #[error("The gif used for pixel-perfect gif is taller than {0}")]
    PixelGifHeight(usize),

    #[error("Could not parse {0}")]
    ParseError(String),
}

impl From<AnimeError> for zbus::fdo::Error {
    fn from(err: AnimeError) -> Self {
        zbus::fdo::Error::Failed(format!("{}", err))
    }
}
