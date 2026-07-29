use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
#[cfg(feature = "dbus")]
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::scsi::protocol::{apply_task, dir_task, mode_task, rgb_task, save_task, speed_task};
use crate::scsi::sg::Task;
use crate::scsi::Error;

#[cfg_attr(feature = "dbus", derive(Type, Value, OwnedValue))]
#[derive(Debug, Clone, PartialEq, Eq, Copy, Deserialize, Serialize)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Default for Colour {
    fn default() -> Self {
        Colour { r: 166, g: 0, b: 0 }
    }
}

impl FromStr for Colour {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 6 || !s.chars().take(6).all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::ParseColour);
        }
        let r = u8::from_str_radix(&s[0..2], 16).or(Err(Error::ParseColour))?;
        let g = u8::from_str_radix(&s[2..4], 16).or(Err(Error::ParseColour))?;
        let b = u8::from_str_radix(&s[4..6], 16).or(Err(Error::ParseColour))?;
        Ok(Colour { r, g, b })
    }
}

impl From<&[u8; 3]> for Colour {
    fn from(c: &[u8; 3]) -> Self {
        Self {
            r: c[0],
            g: c[1],
            b: c[2],
        }
    }
}

impl From<Colour> for [u8; 3] {
    fn from(c: Colour) -> Self {
        [
            c.r, c.b, c.g,
        ]
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "u")
)]
pub enum Direction {
    #[default]
    Forward = 0,
    Reverse = 1,
}

impl FromStr for Direction {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "forward" => Ok(Direction::Forward),
            "reverse" => Ok(Direction::Reverse),
            _ => Err(Error::ParseSpeed),
        }
    }
}

impl From<u8> for Direction {
    fn from(dir: u8) -> Self {
        match dir {
            1 => Direction::Reverse,
            _ => Direction::Forward,
        }
    }
}

impl From<Direction> for u8 {
    fn from(d: Direction) -> Self {
        d as u8
    }
}

#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "s")
)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Speed {
    Slowest = 4,
    Slow = 3,
    #[default]
    Med = 2,
    Fast = 1,
    Fastest = 0,
}

impl FromStr for Speed {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "slowest" => Ok(Speed::Slowest),
            "slow" => Ok(Speed::Slow),
            "med" => Ok(Speed::Med),
            "fast" => Ok(Speed::Fast),
            "fastest" => Ok(Speed::Fastest),
            _ => Err(Error::ParseSpeed),
        }
    }
}

impl From<Speed> for u8 {
    fn from(s: Speed) -> u8 {
        match s {
            Speed::Slowest => 4,
            Speed::Slow => 3,
            Speed::Med => 2,
            Speed::Fast => 1,
            Speed::Fastest => 0,
        }
    }
}

impl From<u8> for Speed {
    fn from(value: u8) -> Self {
        match value {
            4 => Self::Slowest,
            3 => Self::Slow,
            1 => Self::Fast,
            0 => Self::Fastest,
            _ => Self::Med,
        }
    }
}

/// Enum of modes that convert to the actual number required by a USB HID packet
#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "u")
)]
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Deserialize, Serialize,
)]
pub enum AuraMode {
    Off = 0,
    #[default]
    Static = 1,
    Breathe = 2,
    Flashing = 3,
    RainbowCycle = 4,
    RainbowWave = 5,
    RainbowCycleBreathe = 6,
    ChaseFade = 7,
    RainbowCycleChaseFade = 8,
    Chase = 9,
    RainbowCycleChase = 10,
    RainbowCycleWave = 11,
    RainbowPulseChase = 12,
    RandomFlicker = 13,
    DoubleFade = 14,
}

#[cfg(feature = "dbus")]
impl zbus::zvariant::Basic for AuraMode {
    const SIGNATURE_CHAR: char = 'u';
    const SIGNATURE_STR: &'static str = "u";
}

impl AuraMode {
    pub fn list() -> [String; 15] {
        [
            AuraMode::Off.to_string(),
            AuraMode::Static.to_string(),
            AuraMode::Breathe.to_string(),
            AuraMode::Flashing.to_string(),
            AuraMode::RainbowCycle.to_string(),
            AuraMode::RainbowWave.to_string(),
            AuraMode::RainbowCycleBreathe.to_string(),
            AuraMode::ChaseFade.to_string(),
            AuraMode::RainbowCycleChaseFade.to_string(),
            AuraMode::Chase.to_string(),
            AuraMode::RainbowCycleChase.to_string(),
            AuraMode::RainbowCycleWave.to_string(),
            AuraMode::RainbowPulseChase.to_string(),
            AuraMode::RandomFlicker.to_string(),
            AuraMode::DoubleFade.to_string(),
        ]
    }
}

impl Display for AuraMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AuraMode::Off => "Off",
            AuraMode::Static => "Static",
            AuraMode::Breathe => "Breathe",
            AuraMode::Flashing => "Flashing",
            AuraMode::RainbowCycle => "RainbowCycle",
            AuraMode::RainbowWave => "RainbowWave",
            AuraMode::RainbowCycleBreathe => "RainbowCycleBreathe",
            AuraMode::ChaseFade => "ChaseFade",
            AuraMode::RainbowCycleChaseFade => "RainbowCycleChaseFade",
            AuraMode::Chase => "Chase",
            AuraMode::RainbowCycleChase => "RainbowCycleChase",
            AuraMode::RainbowCycleWave => "RainbowCycleWave",
            AuraMode::RainbowPulseChase => "RainbowPulseChase",
            AuraMode::RandomFlicker => "RandomFlicker",
            AuraMode::DoubleFade => "DoubleFade",
        };
        write!(f, "{s}")
    }
}

impl FromStr for AuraMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "off" => Ok(AuraMode::Off),
            "static" => Ok(AuraMode::Static),
            "breathe" => Ok(AuraMode::Breathe),
            "flashing" => Ok(AuraMode::Flashing),
            "rainbowcycle" => Ok(AuraMode::RainbowCycle),
            "rainbowwave" => Ok(AuraMode::RainbowWave),
            "rainbowcyclebreathe" => Ok(AuraMode::RainbowCycleBreathe),
            "chasefade" => Ok(AuraMode::ChaseFade),
            "rainbowcyclechasefade" => Ok(AuraMode::RainbowCycleChaseFade),
            "chase" => Ok(AuraMode::Chase),
            "rainbowcyclechase" => Ok(AuraMode::RainbowCycleChase),
            "rainbowcyclewave" => Ok(AuraMode::RainbowCycleWave),
            "rainbowpulsechase" => Ok(AuraMode::RainbowPulseChase),
            "randomflicker" => Ok(AuraMode::RandomFlicker),
            "doublefade" => Ok(AuraMode::DoubleFade),
            _ => Err(Error::ParseMode),
        }
    }
}

impl From<u8> for AuraMode {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Off,
            1 => Self::Static,
            2 => Self::Breathe,
            3 => Self::Flashing,
            4 => Self::RainbowCycle,
            5 => Self::RainbowWave,
            6 => Self::RainbowCycleBreathe,
            7 => Self::ChaseFade,
            8 => Self::RainbowCycleChaseFade,
            9 => Self::Chase,
            10 => Self::RainbowCycleChase,
            11 => Self::RainbowCycleWave,
            12 => Self::RainbowPulseChase,
            13 => Self::RandomFlicker,
            14 => Self::DoubleFade,
            _ => Self::Static,
        }
    }
}

impl From<AuraMode> for u8 {
    fn from(value: AuraMode) -> Self {
        value as u8
    }
}

#[cfg_attr(feature = "dbus", derive(Type, Value, OwnedValue))]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ModeData {
    pub mode: AuraMode,
    pub zone: u32,
    pub colour1: Colour,
    pub colour2: Colour,
    pub colour3: Colour,
    pub colour4: Colour,
    pub speed: Speed,
    pub direction: Direction,
}

impl Display for ModeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Mode: {}, Zone: {}, Colour1: {:?}, Colour2: {:?}, Speed: {:?}, Direction: {:?}",
            self.mode, self.zone, self.colour1, self.colour2, self.speed, self.direction
        )
    }
}

impl ModeData {
    pub fn default_with_mode(mode: AuraMode) -> Self {
        Self {
            mode,
            zone: 0,
            colour1: Colour::default(),
            colour2: Colour::default(),
            colour3: Colour::default(),
            colour4: Colour::default(),
            speed: Speed::default(),
            direction: Direction::default(),
        }
    }

    pub fn mode(&self) -> &AuraMode {
        &self.mode
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mode: AuraMode,
        zone: u32,
        colour1: Colour,
        colour2: Colour,
        colour3: Colour,
        colour4: Colour,
        speed: Speed,
        direction: Direction,
    ) -> Self {
        ModeData {
            mode,
            zone,
            colour1,
            colour2,
            colour3,
            colour4,
            speed,
            direction,
        }
    }

    pub fn to_tasks(&self) -> Vec<Task> {
        let mut tasks = Vec::new();
        match self.mode {
            AuraMode::Off | AuraMode::Static => {
                tasks.push(rgb_task(
                    self.zone,
                    &[
                        self.colour1.r, self.colour1.g, self.colour1.b,
                    ],
                ));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::Breathe => {
                tasks.push(rgb_task(
                    self.zone,
                    &[
                        self.colour1.r, self.colour1.g, self.colour1.b,
                    ],
                ));
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::Flashing => {
                tasks.push(rgb_task(
                    self.zone,
                    &[
                        self.colour1.r, self.colour1.g, self.colour1.b,
                    ],
                ));
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::RainbowCycle => {
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::RainbowWave => {
                tasks.push(dir_task(self.direction as u8));
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::RainbowCycleBreathe => {
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::ChaseFade => {
                tasks.push(rgb_task(
                    self.zone,
                    &[
                        self.colour1.r, self.colour1.g, self.colour1.b,
                    ],
                ));
                tasks.push(dir_task(self.direction as u8));
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::RainbowCycleChaseFade => {
                tasks.push(dir_task(self.direction as u8));
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::Chase => {
                tasks.push(rgb_task(
                    self.zone,
                    &[
                        self.colour1.r, self.colour1.g, self.colour1.b,
                    ],
                ));
                tasks.push(dir_task(self.direction as u8));
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::RainbowCycleChase => {
                tasks.push(dir_task(self.direction as u8));
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::RainbowCycleWave => {
                tasks.push(dir_task(self.direction as u8));
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::RainbowPulseChase => {
                tasks.push(dir_task(self.direction as u8));
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::RandomFlicker => {
                tasks.push(rgb_task(
                    self.zone,
                    &[
                        self.colour1.r, self.colour1.g, self.colour1.b,
                    ],
                ));
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
            AuraMode::DoubleFade => {
                tasks.push(rgb_task(
                    self.zone,
                    &[
                        self.colour1.r, self.colour1.g, self.colour1.b,
                    ],
                ));
                tasks.push(dir_task(self.direction as u8));
                tasks.push(speed_task(self.speed as u8));
                tasks.push(mode_task(self.mode as u8));
            }
        }
        tasks.push(apply_task());
        tasks.push(save_task());
        tasks
    }
}

pub type AuraEffect = ModeData;

impl From<&ModeData> for Vec<Task> {
    fn from(effect: &ModeData) -> Self {
        effect.to_tasks()
    }
}
