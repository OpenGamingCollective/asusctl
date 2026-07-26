mod builtin_modes;
mod error;
mod scsi;
pub mod sg;

pub use builtin_modes::*;
pub use error::*;
use serde::{Deserialize, Serialize};
pub use sg::{Device, Task};

pub const PROD_SCSI_ARION: &str = "1932";

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ScsiType {
    Arion,
    #[default]
    Unsupported,
}

impl ScsiType {
    pub const fn prod_id_str(&self) -> &str {
        match self {
            ScsiType::Arion => PROD_SCSI_ARION,
            ScsiType::Unsupported => "",
        }
    }
}

impl From<&str> for ScsiType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            PROD_SCSI_ARION | "0x1932" => Self::Arion,
            _ => Self::Unsupported,
        }
    }
}

impl From<ScsiType> for &str {
    fn from(s: ScsiType) -> Self {
        match s {
            ScsiType::Arion => PROD_SCSI_ARION,
            ScsiType::Unsupported => "Unsupported",
        }
    }
}

pub fn open_device(path: &str) -> Result<Device, std::io::Error> {
    Device::open(path)
}
