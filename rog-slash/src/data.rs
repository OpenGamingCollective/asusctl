use std::fmt::Display;
use std::str::FromStr;

use dmi_id::DMIID;
use serde::{Deserialize, Serialize};
#[cfg(feature = "dbus")]
use zbus::zvariant::Type;
use zbus::zvariant::{OwnedValue, Value};

use crate::error::SlashError;
use crate::usb::{PROD_ID1, PROD_ID1_STR, PROD_ID2, PROD_ID2_STR};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum SlashType {
    GA403_2024,
    GA403_2025,
    GA605_2024,
    GA605_2025,
    GU605_2024,
    GU605_2025,
    G614_2025,
    #[default]
    Unsupported,
}

impl SlashType {
    pub const fn prod_id(&self) -> u16 {
        match self {
            SlashType::GA403_2025 => PROD_ID2,
            SlashType::GA403_2024 => PROD_ID1,
            SlashType::GA605_2025 => PROD_ID2,
            SlashType::GA605_2024 => PROD_ID2,
            SlashType::GU605_2025 => PROD_ID2,
            SlashType::GU605_2024 => PROD_ID1,
            SlashType::G614_2025 => PROD_ID2,
            SlashType::Unsupported => 0,
        }
    }

    pub const fn prod_id_str(&self) -> &str {
        match self {
            SlashType::GA403_2025 => PROD_ID2_STR,
            SlashType::GA403_2024 => PROD_ID1_STR,
            SlashType::GA605_2025 => PROD_ID2_STR,
            SlashType::GA605_2024 => PROD_ID2_STR,
            SlashType::GU605_2025 => PROD_ID2_STR,
            SlashType::GU605_2024 => PROD_ID1_STR,
            SlashType::G614_2025 => PROD_ID2_STR,
            SlashType::Unsupported => "",
        }
    }

    pub fn from_dmi() -> Self {
        let board_name = DMIID::new().unwrap_or_default().board_name.to_uppercase();
        if board_name.contains("G614F") {
            SlashType::G614_2025
        } else if [
            "GA403W", "GA403UH", "GA403UM", "GA403UP", "GA403GM",
        ]
        .iter()
        .any(|s| board_name.contains(s))
        {
            SlashType::GA403_2025
        } else if board_name.contains("GA403") {
            SlashType::GA403_2024
        } else if board_name.contains("GA605K") {
            SlashType::GA605_2025
        } else if board_name.contains("GA605") {
            SlashType::GA605_2024
        } else if board_name.contains("GU605C") {
            SlashType::GU605_2025
        } else if board_name.contains("GU605") {
            SlashType::GU605_2024
        } else {
            SlashType::Unsupported
        }
    }
}

impl FromStr for SlashType {
    type Err = SlashError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "GA403_2025" => Self::GA403_2025,
            "GA403_2024" => Self::GA403_2024,
            "GA605_2025" => Self::GA605_2025,
            "GA605_2024" => Self::GA605_2024,
            "GU605_2025" => Self::GU605_2025,
            "GU605_2024" => Self::GU605_2024,
            "G614_2025" => Self::G614_2025,
            _ => Self::Unsupported,
        })
    }
}

#[cfg_attr(feature = "dbus", derive(Type, Value, OwnedValue))]
#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Deserialize, Serialize)]
pub enum SlashMode {
    Static = 0x06,
    Bounce = 0x10,
    Slash = 0x12,
    Loading = 0x13,
    BitStream = 0x1d,
    Transmission = 0x1a,
    #[default]
    Flow = 0x19,
    Flux = 0x25,
    Phantom = 0x24,
    Spectrum = 0x26,
    Hazard = 0x32,
    Interfacing = 0x33,
    Ramp = 0x34,
    GameOver = 0x42,
    Start = 0x43,
    Buzzer = 0x44,
}

impl FromStr for SlashMode {
    type Err = SlashError;

    fn from_str(s: &str) -> Result<Self, SlashError> {
        match s {
            "Static" => Ok(SlashMode::Static),
            "Bounce" => Ok(SlashMode::Bounce),
            "Slash" => Ok(SlashMode::Slash),
            "Loading" => Ok(SlashMode::Loading),
            "BitStream" => Ok(SlashMode::BitStream),
            "Transmission" => Ok(SlashMode::Transmission),
            "Flow" => Ok(SlashMode::Flow),
            "Flux" => Ok(SlashMode::Flux),
            "Phantom" => Ok(SlashMode::Phantom),
            "Spectrum" => Ok(SlashMode::Spectrum),
            "Hazard" => Ok(SlashMode::Hazard),
            "Interfacing" => Ok(SlashMode::Interfacing),
            "Ramp" => Ok(SlashMode::Ramp),
            "GameOver" => Ok(SlashMode::GameOver),
            "Start" => Ok(SlashMode::Start),
            "Buzzer" => Ok(SlashMode::Buzzer),
            _ => Ok(SlashMode::Bounce),
        }
    }
}

impl Display for SlashMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match &self {
            SlashMode::Static => String::from("Static"),
            SlashMode::Bounce => String::from("Bounce"),
            SlashMode::Slash => String::from("Slash"),
            SlashMode::Loading => String::from("Loading"),
            SlashMode::BitStream => String::from("BitStream"),
            SlashMode::Transmission => String::from("Transmission"),
            SlashMode::Flow => String::from("Flow"),
            SlashMode::Flux => String::from("Flux"),
            SlashMode::Phantom => String::from("Phantom"),
            SlashMode::Spectrum => String::from("Spectrum"),
            SlashMode::Hazard => String::from("Hazard"),
            SlashMode::Interfacing => String::from("Interfacing"),
            SlashMode::Ramp => String::from("Ramp"),
            SlashMode::GameOver => String::from("GameOver"),
            SlashMode::Start => String::from("Start"),
            SlashMode::Buzzer => String::from("Buzzer"),
        };
        write!(f, "{str}")
    }
}

impl SlashMode {
    pub fn list() -> [String; 16] {
        [
            SlashMode::Static.to_string(),
            SlashMode::Bounce.to_string(),
            SlashMode::Slash.to_string(),
            SlashMode::Loading.to_string(),
            SlashMode::BitStream.to_string(),
            SlashMode::Transmission.to_string(),
            SlashMode::Flow.to_string(),
            SlashMode::Flux.to_string(),
            SlashMode::Phantom.to_string(),
            SlashMode::Spectrum.to_string(),
            SlashMode::Hazard.to_string(),
            SlashMode::Interfacing.to_string(),
            SlashMode::Ramp.to_string(),
            SlashMode::GameOver.to_string(),
            SlashMode::Start.to_string(),
            SlashMode::Buzzer.to_string(),
        ]
    }
}

#[cfg_attr(feature = "dbus", derive(Type))]
#[derive(Debug, PartialEq, Eq, Copy, Clone, Deserialize, Serialize)]
pub struct DeviceState {
    pub slash_enabled: bool,
    pub slash_brightness: u8,
    pub slash_interval: u8,
    pub slash_mode: SlashMode,
}
