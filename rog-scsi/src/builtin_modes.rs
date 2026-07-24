use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
#[cfg(feature = "dbus")]
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::error::Error;
use crate::scsi::{apply_task, dir_task, mode_task, rgb_task, save_task, speed_task};

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
        write!(f, "{}", <&str>::from(self))
    }
}

impl From<AuraMode> for String {
    fn from(mode: AuraMode) -> Self {
        <&str>::from(&mode).to_owned()
    }
}

impl From<&AuraMode> for &str {
    fn from(mode: &AuraMode) -> Self {
        match mode {
            AuraMode::Off => "Off",
            AuraMode::Static => "Static",
            AuraMode::Breathe => "Breathe",
            AuraMode::RainbowCycle => "RainbowCycle",
            AuraMode::RainbowWave => "RainbowWave",
            AuraMode::Flashing => "Flashing",
            AuraMode::RainbowCycleBreathe => "RainbowCycleBreathe",
            AuraMode::ChaseFade => "ChaseFade",
            AuraMode::RainbowCycleChaseFade => "RainbowCycleChaseFade",
            AuraMode::Chase => "Chase",
            AuraMode::RainbowCycleChase => "RainbowCycleChase",
            AuraMode::RainbowCycleWave => "RainbowCycleWave",
            AuraMode::RainbowPulseChase => "RainbowPulseChase",
            AuraMode::RandomFlicker => "RandomFlicker",
            AuraMode::DoubleFade => "DoubleFade",
        }
    }
}

impl FromStr for AuraMode {
    type Err = Error;

    fn from_str(mode: &str) -> Result<Self, Self::Err> {
        match mode {
            "Off" => Ok(Self::Off),
            "Static" => Ok(Self::Static),
            "Breathe" => Ok(Self::Breathe),
            "RainbowCycle" => Ok(Self::RainbowCycle),
            "RainbowWave" => Ok(Self::RainbowWave),
            "Flashing" => Ok(Self::Flashing),
            "RainbowCycleBreathe" => Ok(Self::RainbowCycleBreathe),
            "ChaseFade" => Ok(Self::ChaseFade),
            "RainbowCycleChaseFade" => Ok(Self::RainbowCycleChaseFade),
            "Chase" => Ok(Self::Chase),
            "RainbowCycleChase" => Ok(Self::RainbowCycleChase),
            "RainbowCycleWave" => Ok(Self::RainbowCycleWave),
            "RainbowPulseChase" => Ok(Self::RainbowPulseChase),
            "RandomFlicker" => Ok(Self::RandomFlicker),
            "DoubleFade" => Ok(Self::DoubleFade),
            _ => Err(Error::ParseMode),
        }
    }
}

impl From<&str> for AuraMode {
    fn from(mode: &str) -> Self {
        AuraMode::from_str(mode).unwrap_or_default()
    }
}

impl From<u8> for AuraMode {
    fn from(mode: u8) -> Self {
        match mode {
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

impl From<AuraEffect> for AuraMode {
    fn from(value: AuraEffect) -> Self {
        value.mode
    }
}

/// Default factory modes structure.
#[cfg_attr(feature = "dbus", derive(Type, Value, OwnedValue))]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AuraEffect {
    /// The effect type
    pub mode: AuraMode,
    /// One of three speeds for modes that support speed (most that animate)
    pub speed: Speed,
    /// Up, down, left, right. Only Rainbow mode seems to use this
    pub direction: Direction,
    /// Primary colour for all modes
    pub colour1: Colour,
    /// Secondary colour in some modes like Breathing or Stars
    pub colour2: Colour,
    pub colour3: Colour,
    pub colour4: Colour,
}

impl AuraEffect {
    pub fn mode(&self) -> &AuraMode {
        &self.mode
    }

    pub fn mode_name(&self) -> &str {
        <&str>::from(&self.mode)
    }

    pub fn mode_num(&self) -> u8 {
        self.mode as u8
    }

    pub fn default_with_mode(mode: AuraMode) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }
}

impl Default for AuraEffect {
    fn default() -> Self {
        Self {
            mode: AuraMode::Static,
            colour1: Colour { r: 166, g: 0, b: 0 },
            colour2: Colour { r: 0, g: 0, b: 0 },
            colour3: Colour { r: 166, g: 0, b: 0 },
            colour4: Colour { r: 0, g: 0, b: 0 },
            speed: Speed::Med,
            direction: Direction::Forward,
        }
    }
}

impl Display for AuraEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "AuraEffect {{")?;
        writeln!(f, "  mode: {}", self.mode())?;
        writeln!(f, "  speed: {:?}", self.speed)?;
        writeln!(f, "  direction: {:?}", self.direction)?;
        writeln!(f, "  colour1: {:?}", self.colour1)?;
        writeln!(f, "  colour2: {:?}", self.colour2)?;
        writeln!(f, "  colour3: {:?}", self.colour3)?;
        writeln!(f, "  colour4: {:?}", self.colour4)?;
        writeln!(f, "}}")
    }
}

impl From<&AuraEffect> for Vec<sg::Task> {
    fn from(effect: &AuraEffect) -> Self {
        let mut tasks = Vec::new();

        tasks.append(&mut vec![
            mode_task(effect.mode as u8),
            rgb_task(0, &effect.colour1.into()),
            rgb_task(1, &effect.colour2.into()),
            rgb_task(2, &effect.colour3.into()),
            rgb_task(3, &effect.colour4.into()),
        ]);

        if !matches!(effect.mode, AuraMode::Static | AuraMode::Off) {
            tasks.push(speed_task(effect.speed as u8));
        }
        if matches!(
            effect.mode,
            AuraMode::RainbowWave
                | AuraMode::ChaseFade
                | AuraMode::RainbowCycleChaseFade
                | AuraMode::Chase
                | AuraMode::RainbowCycleChase
                | AuraMode::RainbowCycleWave
                | AuraMode::RainbowPulseChase
        ) {
            tasks.push(dir_task(effect.direction as u8));
        }

        tasks.append(&mut vec![
            apply_task(),
            save_task(),
        ]);
        tasks
    }
}
