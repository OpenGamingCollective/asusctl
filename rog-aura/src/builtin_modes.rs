use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
#[cfg(feature = "dbus")]
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::AURA_LAPTOP_LED_MSG_LEN;
use crate::error::Error;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "u")
)]
pub enum LedBrightness {
    Off = 0,
    Low = 1,
    #[default]
    Med = 2,
    High = 3,
}

impl LedBrightness {
    pub const fn next(&self) -> Self {
        match self {
            Self::Off => Self::Low,
            Self::Low => Self::Med,
            Self::Med => Self::High,
            Self::High => Self::Off,
        }
    }

    pub const fn prev(&self) -> Self {
        match self {
            Self::Off => Self::High,
            Self::Low => Self::Off,
            Self::Med => Self::Low,
            Self::High => Self::Med,
        }
    }

    /// Map the 4-step UI level onto a sysfs brightness value.
    ///
    /// When `max_brightness <= 3` the legacy 0..=3 mapping is kept. Otherwise
    /// levels are scaled across `0..=max_brightness` (Off=0, High=max).
    pub const fn to_scaled(self, max_brightness: u8) -> u8 {
        if max_brightness <= 3 {
            return self as u8;
        }
        match self {
            Self::Off => 0,
            Self::Low => max_brightness / 3,
            Self::Med => ((2u16 * max_brightness as u16) / 3) as u8,
            Self::High => max_brightness,
        }
    }

    /// Inverse of [`Self::to_scaled`].
    pub const fn from_scaled(value: u8, max_brightness: u8) -> Self {
        if max_brightness <= 3 {
            return match value {
                0 => Self::Off,
                1 => Self::Low,
                3 => Self::High,
                _ => Self::Med,
            };
        }
        if value == 0 {
            return Self::Off;
        }
        let low = max_brightness / 3;
        let med = ((2u16 * max_brightness as u16) / 3) as u8;
        if value <= low {
            Self::Low
        } else if value <= med {
            Self::Med
        } else {
            Self::High
        }
    }
}

impl From<u8> for LedBrightness {
    fn from(bright: u8) -> Self {
        match bright {
            0 => LedBrightness::Off,
            1 => LedBrightness::Low,
            3 => LedBrightness::High,
            _ => LedBrightness::Med,
        }
    }
}

impl From<LedBrightness> for u8 {
    fn from(l: LedBrightness) -> Self {
        l as u8
    }
}

impl From<LedBrightness> for i32 {
    fn from(l: LedBrightness) -> Self {
        l as i32
    }
}

impl From<i32> for LedBrightness {
    fn from(l: i32) -> Self {
        match l {
            0 => LedBrightness::Off,
            1 => LedBrightness::Low,
            2 => LedBrightness::Med,
            3 => LedBrightness::High,
            _ => LedBrightness::Med,
        }
    }
}

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
        let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| Error::ParseColour)?;
        let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| Error::ParseColour)?;
        let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| Error::ParseColour)?;
        Ok(Colour { r, g, b })
    }
}

impl From<&[f32; 3]> for Colour {
    fn from(c: &[f32; 3]) -> Self {
        Self {
            r: (255.0 * c[0]) as u8,
            g: (255.0 * c[1]) as u8,
            b: (255.0 * c[2]) as u8,
        }
    }
}

impl From<Colour> for [f32; 3] {
    fn from(c: Colour) -> Self {
        [
            c.r as f32 / 255.0,
            c.g as f32 / 255.0,
            c.b as f32 / 255.0,
        ]
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
            c.r, c.g, c.b,
        ]
    }
}

#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "s")
)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Speed {
    Low = 0xe1,
    #[default]
    Med = 0xeb,
    High = 0xf5,
}

impl FromStr for Speed {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "low" => Ok(Speed::Low),
            "med" => Ok(Speed::Med),
            "high" => Ok(Speed::High),
            _ => Err(Error::ParseSpeed),
        }
    }
}

impl From<i32> for Speed {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Low,
            2 => Self::High,
            _ => Self::Med,
        }
    }
}

impl From<Speed> for i32 {
    fn from(value: Speed) -> Self {
        match value {
            Speed::Low => 0,
            Speed::Med => 1,
            Speed::High => 2,
        }
    }
}

impl From<Speed> for u8 {
    fn from(s: Speed) -> u8 {
        match s {
            Speed::Low => 0,
            Speed::Med => 1,
            Speed::High => 2,
        }
    }
}

impl Speed {
    pub const fn to_dynamic_speed(&self) -> u32 {
        match self {
            Self::Low => 0,
            Self::Med => 1,
            Self::High => 2,
        }
    }

    pub const fn from_dynamic_speed(val: u32) -> Self {
        match val {
            0 => Self::Low,
            2 => Self::High,
            _ => Self::Med,
        }
    }
}
/// Used for Rainbow mode.
///
/// Enum corresponds to the required integer value
#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "s")
)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Direction {
    #[default]
    Right = 0,
    Left = 1,
    Up = 2,
    Down = 3,
}

impl FromStr for Direction {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "right" => Ok(Direction::Right),
            "up" => Ok(Direction::Up),
            "down" => Ok(Direction::Down),
            "left" => Ok(Direction::Left),
            _ => Err(Error::ParseDirection),
        }
    }
}

impl From<i32> for Direction {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Left,
            2 => Self::Up,
            3 => Self::Down,
            _ => Self::Right,
        }
    }
}

impl From<Direction> for i32 {
    fn from(value: Direction) -> Self {
        value as i32
    }
}

impl Direction {
    pub const fn to_dynamic_direction_str(&self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Left => "left",
            Self::Up => "up",
            Self::Down => "down",
        }
    }

    pub fn from_dynamic_direction_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "left" => Self::Left,
            "up" => Self::Up,
            "down" => Self::Down,
            _ => Self::Right,
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
pub enum AuraModeNum {
    #[default]
    Static = 0,
    Breathe = 1,
    RainbowCycle = 2,
    RainbowWave = 3,
    Star = 4,
    Rain = 5,
    Highlight = 6,
    Laser = 7,
    Ripple = 8,
    Pulse = 10,
    Comet = 11,
    Flash = 12,
}

impl Display for AuraModeNum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", <&str>::from(self))
    }
}

impl From<AuraModeNum> for String {
    fn from(mode: AuraModeNum) -> Self {
        <&str>::from(&mode).to_owned()
    }
}

impl From<&AuraModeNum> for &str {
    fn from(mode: &AuraModeNum) -> Self {
        match mode {
            AuraModeNum::Static => "Static",
            AuraModeNum::Breathe => "Breathe",
            AuraModeNum::RainbowCycle => "RainbowCycle",
            AuraModeNum::RainbowWave => "RainbowWave",
            AuraModeNum::Star => "Stars",
            AuraModeNum::Rain => "Rain",
            AuraModeNum::Highlight => "Highlight",
            AuraModeNum::Laser => "Laser",
            AuraModeNum::Ripple => "Ripple",
            AuraModeNum::Pulse => "Pulse",
            AuraModeNum::Comet => "Comet",
            AuraModeNum::Flash => "Flash",
        }
    }
}
impl From<&str> for AuraModeNum {
    fn from(mode: &str) -> Self {
        match mode {
            "Breathe" => AuraModeNum::Breathe,
            "RainbowCycle" => AuraModeNum::RainbowCycle,
            "RainbowWave" => AuraModeNum::RainbowWave,
            "Stars" => AuraModeNum::Star,
            "Rain" => AuraModeNum::Rain,
            "Highlight" => AuraModeNum::Highlight,
            "Laser" => AuraModeNum::Laser,
            "Ripple" => AuraModeNum::Ripple,
            "Pulse" => AuraModeNum::Pulse,
            "Comet" => AuraModeNum::Comet,
            "Flash" => AuraModeNum::Flash,
            _ => AuraModeNum::Static,
        }
    }
}

impl From<u8> for AuraModeNum {
    fn from(mode: u8) -> Self {
        match mode {
            1 => AuraModeNum::Breathe,
            2 => AuraModeNum::RainbowCycle,
            3 => AuraModeNum::RainbowWave,
            4 => AuraModeNum::Star,
            5 => AuraModeNum::Rain,
            6 => AuraModeNum::Highlight,
            7 => AuraModeNum::Laser,
            8 => AuraModeNum::Ripple,
            10 => AuraModeNum::Pulse,
            11 => AuraModeNum::Comet,
            12 => AuraModeNum::Flash,
            _ => AuraModeNum::Static,
        }
    }
}

impl From<i32> for AuraModeNum {
    fn from(mode: i32) -> Self {
        (mode as u8).into()
    }
}

impl From<AuraModeNum> for i32 {
    fn from(value: AuraModeNum) -> Self {
        value as i32
    }
}

impl From<AuraEffect> for AuraModeNum {
    fn from(value: AuraEffect) -> Self {
        value.mode
    }
}

impl AuraModeNum {
    /// Return the corresponding Dynamic Lighting effect name, if available.
    pub const fn to_dynamic_effect_str(&self) -> Option<&'static str> {
        match self {
            Self::Static => Some("static"),
            Self::Breathe => Some("breathing"),
            Self::RainbowCycle => Some("spectrum_cycle"),
            Self::RainbowWave => Some("rainbow"),
            Self::Pulse | Self::Flash => Some("strobe"),
            _ => None,
        }
    }

    /// Parse a Dynamic Lighting effect name into an `AuraModeNum`.
    pub fn from_dynamic_effect_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "static" => Some(Self::Static),
            "breathing" => Some(Self::Breathe),
            "spectrum_cycle" => Some(Self::RainbowCycle),
            "rainbow" => Some(Self::RainbowWave),
            "strobe" => Some(Self::Pulse),
            _ => None,
        }
    }
}

#[cfg(feature = "dbus")]
impl zbus::zvariant::Basic for AuraModeNum {
    const SIGNATURE_CHAR: char = 'u';
    const SIGNATURE_STR: &'static str = "u";
}

/// Base effects have no zoning, while multizone is 1-4
#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "u")
)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum AuraZone {
    /// Used if keyboard has no zones, or if setting all
    #[default]
    None = 0,
    /// Leftmost zone
    Key1 = 1,
    /// Zone after leftmost
    Key2 = 2,
    /// Zone second from right
    Key3 = 3,
    /// Rightmost zone
    Key4 = 4,
    /// Logo on the lid (or elsewhere?)
    Logo = 5,
    /// The left part of a lightbar (typically on the front of laptop)
    BarLeft = 6,
    /// The right part of a lightbar
    BarRight = 7,
}

impl FromStr for AuraZone {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.to_ascii_lowercase().as_str() {
            "0" | "none" => Ok(AuraZone::None),
            "1" | "one" => Ok(AuraZone::Key1),
            "2" | "two" => Ok(AuraZone::Key2),
            "3" | "three" => Ok(AuraZone::Key3),
            "4" | "four" => Ok(AuraZone::Key4),
            "5" | "logo" => Ok(AuraZone::Logo),
            "6" | "lightbar-left" | "lightbar" | "bar" => Ok(AuraZone::BarLeft),
            "7" | "lightbar-right" => Ok(AuraZone::BarRight),
            _ => Err(Error::ParseSpeed),
        }
    }
}

impl From<i32> for AuraZone {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Key1,
            2 => Self::Key2,
            3 => Self::Key3,
            4 => Self::Key4,
            5 => Self::Logo,
            6 => Self::BarLeft,
            7 => Self::BarRight,
            _ => Self::default(),
        }
    }
}

impl From<AuraZone> for i32 {
    fn from(value: AuraZone) -> Self {
        value as i32
    }
}

/// Default factory modes structure. This easily converts to an USB HID packet
/// with:
/// ```rust
/// // let bytes: [u8; LED_MSG_LEN] = mode.into();
/// ```
#[cfg_attr(feature = "dbus", derive(Type, Value, OwnedValue))]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AuraEffect {
    /// The effect type
    pub mode: AuraModeNum,
    /// `AuraZone::None` for no zone or zoneless keyboards
    pub zone: AuraZone,
    /// Primary colour for all modes
    pub colour1: Colour,
    /// Secondary colour in some modes like Breathing or Stars
    pub colour2: Colour,
    /// One of three speeds for modes that support speed (most that animate)
    pub speed: Speed,
    /// Up, down, left, right. Only Rainbow mode seems to use this
    pub direction: Direction,
}

impl AuraEffect {
    pub fn mode(&self) -> &AuraModeNum {
        &self.mode
    }

    pub fn default_with_mode(mode: AuraModeNum) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }

    pub fn zone(&self) -> AuraZone {
        self.zone
    }

    /// Convert the effect colours to an array of RGB tuples for Dynamic Lighting palette.
    pub fn to_dynamic_palette(&self) -> Vec<(u8, u8, u8)> {
        let mut p = Vec::with_capacity(2);
        p.push((self.colour1.r, self.colour1.g, self.colour1.b));
        if self.colour2.r != 0 || self.colour2.g != 0 || self.colour2.b != 0 {
            p.push((self.colour2.r, self.colour2.g, self.colour2.b));
        }
        p
    }
}

impl Default for AuraEffect {
    fn default() -> Self {
        Self {
            mode: AuraModeNum::Static,
            zone: AuraZone::None,
            colour1: Colour { r: 166, g: 0, b: 0 },
            colour2: Colour { r: 0, g: 0, b: 0 },
            speed: Speed::Med,
            direction: Direction::Right,
        }
    }
}

impl Display for AuraEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Parses `AuraEffect` in to packet data for writing to the USB interface
///
/// Byte structure where colour is RGB, one byte per R, G, B:
/// ```ignore
/// | 0 | 1 | 2   | 3   | 4, 5, 6 | 7    | 8        | 9 | 10, 11, 12|
/// |---|---|-----|-----|---------|------|----------|---|-----------|
/// |5d |b3 |Zone |Mode |Colour 1 |Speed |Direction |00 |Colour 2   |
/// ```
impl From<&AuraEffect> for [u8; AURA_LAPTOP_LED_MSG_LEN] {
    fn from(aura: &AuraEffect) -> Self {
        let mut msg = [0u8; AURA_LAPTOP_LED_MSG_LEN];
        msg[0] = 0x5d;
        msg[1] = 0xb3;
        msg[2] = aura.zone as u8;
        msg[3] = aura.mode as u8;
        msg[4] = aura.colour1.r;
        msg[5] = aura.colour1.g;
        msg[6] = aura.colour1.b;
        msg[7] = aura.speed as u8;
        msg[8] = aura.direction as u8;
        msg[10] = aura.colour2.r;
        msg[11] = aura.colour2.g;
        msg[12] = aura.colour2.b;
        msg
    }
}

impl From<&AuraEffect> for Vec<u8> {
    fn from(aura: &AuraEffect) -> Self {
        let mut msg = vec![0u8; AURA_LAPTOP_LED_MSG_LEN];
        msg[0] = 0x5d;
        msg[1] = 0xb3;
        msg[2] = aura.zone as u8;
        msg[3] = aura.mode as u8;
        msg[4] = aura.colour1.r;
        msg[5] = aura.colour1.g;
        msg[6] = aura.colour1.b;
        msg[7] = aura.speed as u8;
        msg[8] = aura.direction as u8;
        msg[10] = aura.colour2.r;
        msg[11] = aura.colour2.g;
        msg[12] = aura.colour2.b;
        msg
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AURA_LAPTOP_LED_MSG_LEN, AuraEffect, AuraModeNum, AuraZone, Colour, Direction,
        LedBrightness, Speed,
    };

    #[test]
    fn led_brightness_scales_for_dynamic_lighting() {
        assert_eq!(LedBrightness::Off.to_scaled(255), 0);
        assert_eq!(LedBrightness::Low.to_scaled(255), 85);
        assert_eq!(LedBrightness::Med.to_scaled(255), 170);
        assert_eq!(LedBrightness::High.to_scaled(255), 255);

        assert_eq!(LedBrightness::from_scaled(0, 255), LedBrightness::Off);
        assert_eq!(LedBrightness::from_scaled(85, 255), LedBrightness::Low);
        assert_eq!(LedBrightness::from_scaled(170, 255), LedBrightness::Med);
        assert_eq!(LedBrightness::from_scaled(255, 255), LedBrightness::High);

        // Legacy 0..=3 path when max_brightness is small.
        assert_eq!(LedBrightness::Med.to_scaled(3), 2);
        assert_eq!(LedBrightness::from_scaled(2, 3), LedBrightness::Med);
        assert_eq!(LedBrightness::High.to_scaled(3), 3);
    }

    #[test]
    fn check_led_static_packet() {
        let st = AuraEffect {
            mode: AuraModeNum::Static,
            zone: AuraZone::None,
            colour1: Colour {
                r: 0xff,
                g: 0x11,
                b: 0xdd,
            },
            colour2: Colour::default(),
            speed: Speed::Med,
            direction: Direction::Right,
        };
        let ar = <[u8; AURA_LAPTOP_LED_MSG_LEN]>::from(&st);

        println!("{:02x?}", ar);
        let check = [
            0x5d, 0xb3, 0x0, 0x0, 0xff, 0x11, 0xdd, 0xeb, 0x0, 0x0, 0xa6, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0,
        ];
        assert_eq!(ar, check);
    }

    #[test]
    fn check_led_static_zone_packet() {
        let mut st = AuraEffect {
            mode: AuraModeNum::Static,
            zone: AuraZone::Key1,
            colour1: Colour {
                r: 0xff,
                g: 0,
                b: 0,
            },
            colour2: Colour { r: 0, g: 0, b: 0 },
            speed: Speed::Low,
            direction: Direction::Left,
        };
        let capture = [
            0x5d, 0xb3, 0x01, 0x00, 0xff, 0x00, 0x00, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(
            <[u8; AURA_LAPTOP_LED_MSG_LEN]>::from(&st)[..9],
            capture[..9]
        );

        st.zone = AuraZone::Key2;
        st.colour1 = Colour {
            r: 0xff,
            g: 0xff,
            b: 0,
        };
        let capture = [
            0x5d, 0xb3, 0x02, 0x00, 0xff, 0xff, 0x00, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(
            <[u8; AURA_LAPTOP_LED_MSG_LEN]>::from(&st)[..9],
            capture[..9]
        );

        st.zone = AuraZone::Key3;
        st.colour1 = Colour {
            r: 0,
            g: 0xff,
            b: 0xff,
        };
        let capture = [
            0x5d, 0xb3, 0x03, 0x00, 0x00, 0xff, 0xff, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(
            <[u8; AURA_LAPTOP_LED_MSG_LEN]>::from(&st)[..9],
            capture[..9]
        );

        st.zone = AuraZone::Key4;
        st.colour1 = Colour {
            r: 0xff,
            g: 0x00,
            b: 0xff,
        };
        let capture = [
            0x5d, 0xb3, 0x04, 0x00, 0xff, 0x00, 0xff, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(
            <[u8; AURA_LAPTOP_LED_MSG_LEN]>::from(&st)[..9],
            capture[..9]
        );

        st.zone = AuraZone::Logo;
        st.colour1 = Colour {
            r: 0x2c,
            g: 0xff,
            b: 0x00,
        };
        let capture = [
            0x5d, 0xb3, 0x05, 0x00, 0x2c, 0xff, 0x00, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(
            <[u8; AURA_LAPTOP_LED_MSG_LEN]>::from(&st)[..9],
            capture[..9]
        );

        st.zone = AuraZone::BarLeft;
        st.colour1 = Colour {
            r: 0xff,
            g: 0x00,
            b: 0x00,
        };
        let capture = [
            0x5d, 0xb3, 0x06, 0x00, 0xff, 0x00, 0x00, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(
            <[u8; AURA_LAPTOP_LED_MSG_LEN]>::from(&st)[..9],
            capture[..9]
        );

        st.zone = AuraZone::BarRight;
        st.colour1 = Colour {
            r: 0xff,
            g: 0x00,
            b: 0xcd,
        };
        let capture = [
            0x5d, 0xb3, 0x07, 0x00, 0xff, 0x00, 0xcd, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(
            <[u8; AURA_LAPTOP_LED_MSG_LEN]>::from(&st)[..9],
            capture[..9]
        );

        st.mode = AuraModeNum::RainbowWave;
        let capture = [
            0x5d, 0xb3, 0x07, 0x03, 0xff, 0x00, 0xcd, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(
            <[u8; AURA_LAPTOP_LED_MSG_LEN]>::from(&st)[..9],
            capture[..9]
        );
    }

    #[test]
    fn test_dynamic_lighting_conversions() {
        assert_eq!(AuraModeNum::Static.to_dynamic_effect_str(), Some("static"));
        assert_eq!(
            AuraModeNum::Breathe.to_dynamic_effect_str(),
            Some("breathing")
        );
        assert_eq!(
            AuraModeNum::RainbowCycle.to_dynamic_effect_str(),
            Some("spectrum_cycle")
        );
        assert_eq!(
            AuraModeNum::RainbowWave.to_dynamic_effect_str(),
            Some("rainbow")
        );
        assert_eq!(AuraModeNum::Pulse.to_dynamic_effect_str(), Some("strobe"));
        assert_eq!(AuraModeNum::Flash.to_dynamic_effect_str(), Some("strobe"));
        assert_eq!(AuraModeNum::Star.to_dynamic_effect_str(), None);

        assert_eq!(
            AuraModeNum::from_dynamic_effect_str("static"),
            Some(AuraModeNum::Static)
        );
        assert_eq!(
            AuraModeNum::from_dynamic_effect_str("breathing"),
            Some(AuraModeNum::Breathe)
        );
        assert_eq!(
            AuraModeNum::from_dynamic_effect_str("spectrum_cycle"),
            Some(AuraModeNum::RainbowCycle)
        );
        assert_eq!(
            AuraModeNum::from_dynamic_effect_str("rainbow"),
            Some(AuraModeNum::RainbowWave)
        );
        assert_eq!(
            AuraModeNum::from_dynamic_effect_str("strobe"),
            Some(AuraModeNum::Pulse)
        );
        assert_eq!(AuraModeNum::from_dynamic_effect_str("unknown"), None);

        assert_eq!(Speed::Low.to_dynamic_speed(), 0);
        assert_eq!(Speed::Med.to_dynamic_speed(), 1);
        assert_eq!(Speed::High.to_dynamic_speed(), 2);
        assert_eq!(Speed::from_dynamic_speed(0), Speed::Low);
        assert_eq!(Speed::from_dynamic_speed(1), Speed::Med);
        assert_eq!(Speed::from_dynamic_speed(2), Speed::High);

        assert_eq!(Direction::Right.to_dynamic_direction_str(), "right");
        assert_eq!(Direction::Left.to_dynamic_direction_str(), "left");
        assert_eq!(Direction::Up.to_dynamic_direction_str(), "up");
        assert_eq!(Direction::Down.to_dynamic_direction_str(), "down");
        assert_eq!(
            Direction::from_dynamic_direction_str("left"),
            Direction::Left
        );
        assert_eq!(
            Direction::from_dynamic_direction_str("right"),
            Direction::Right
        );

        let effect = AuraEffect {
            colour1: Colour {
                r: 0xff,
                g: 0x10,
                b: 0x20,
            },
            colour2: Colour {
                r: 0x00,
                g: 0x30,
                b: 0x40,
            },
            ..Default::default()
        };
        let palette = effect.to_dynamic_palette();
        assert_eq!(
            palette,
            vec![
                (0xff, 0x10, 0x20),
                (0x00, 0x30, 0x40)
            ]
        );
    }
}
