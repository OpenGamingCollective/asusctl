//! Utils for writing to the `AniMe` USB device
//!
//! Use of the device requires a few steps:
//! 1. Initialise the device by writing the two packets from
//! `get_init_packets()` 2. Write data from `AnimePacketType`
//! 3. Write the packet from `get_flush_packet()`, which tells the device to
//! display the data from step 2
//!
//! Step 1 need to applied only on fresh system boot.

use std::str::FromStr;

use dmi_id::DMIID;
use serde::{Deserialize, Serialize};
#[cfg(feature = "dbus")]
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::AnimeType;
use crate::error::AnimeError;

const PACKET_SIZE: usize = 640;
const DEV_PAGE: u8 = 0x5e;
pub const VENDOR_ID: u16 = 0x0b05;
pub const PROD_ID: u16 = 0x193b;

#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "u")
)]
#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Deserialize, Serialize)]
/// Base LED brightness of the display
pub enum Brightness {
    Off = 0,
    Low = 1,
    #[default]
    Med = 2,
    High = 3,
}

impl FromStr for Brightness {
    type Err = AnimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Off" | "off" => Brightness::Off,
            "Low" | "low" => Brightness::Low,
            "Med" | "med" => Brightness::Med,
            "High" | "high" => Brightness::High,
            _ => Brightness::Med,
        })
    }
}

impl From<u8> for Brightness {
    fn from(v: u8) -> Brightness {
        match v {
            0 => Brightness::Off,
            1 => Brightness::Low,
            3 => Brightness::High,
            _ => Brightness::Med,
        }
    }
}

impl From<i32> for Brightness {
    fn from(v: i32) -> Brightness {
        (v as u8).into()
    }
}

impl From<Brightness> for i32 {
    fn from(v: Brightness) -> i32 {
        v as i32
    }
}

#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "s")
)]
#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Deserialize, Serialize)]
pub enum AnimBooting {
    #[default]
    GlitchConstruction = 0,
    StaticEmergence = 1,
}

impl FromStr for AnimBooting {
    type Err = AnimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GlitchConstruction" => Ok(Self::GlitchConstruction),
            "StaticEmergence" => Ok(Self::StaticEmergence),
            _ => Err(AnimeError::ParseError(s.to_owned())),
        }
    }
}

impl From<i32> for AnimBooting {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::GlitchConstruction,
            1 => Self::StaticEmergence,
            _ => Self::default(),
        }
    }
}

impl From<AnimBooting> for i32 {
    fn from(value: AnimBooting) -> Self {
        value as i32
    }
}

#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "s")
)]
#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Deserialize, Serialize)]
pub enum AnimAwake {
    #[default]
    BinaryBannerScroll = 0,
    RogLogoGlitch = 1,
}

impl FromStr for AnimAwake {
    type Err = AnimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BinaryBannerScroll" => Ok(Self::BinaryBannerScroll),
            "RogLogoGlitch" => Ok(Self::RogLogoGlitch),
            _ => Err(AnimeError::ParseError(s.to_owned())),
        }
    }
}

impl From<i32> for AnimAwake {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::BinaryBannerScroll,
            1 => Self::RogLogoGlitch,
            _ => Self::default(),
        }
    }
}

impl From<AnimAwake> for i32 {
    fn from(value: AnimAwake) -> Self {
        value as i32
    }
}

#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "s")
)]
#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Deserialize, Serialize)]
pub enum AnimSleeping {
    #[default]
    BannerSwipe = 0,
    Starfield = 1,
}

impl FromStr for AnimSleeping {
    type Err = AnimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BannerSwipe" => Ok(Self::BannerSwipe),
            "Starfield" => Ok(Self::Starfield),
            _ => Err(AnimeError::ParseError(s.to_owned())),
        }
    }
}

impl From<i32> for AnimSleeping {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::BannerSwipe,
            1 => Self::Starfield,
            _ => Self::default(),
        }
    }
}

impl From<AnimSleeping> for i32 {
    fn from(value: AnimSleeping) -> Self {
        value as i32
    }
}

#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "s")
)]
#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Deserialize, Serialize)]
pub enum AnimShutdown {
    #[default]
    GlitchOut = 0,
    SeeYa = 1,
}

impl FromStr for AnimShutdown {
    type Err = AnimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GlitchOut" => Ok(Self::GlitchOut),
            "SeeYa" => Ok(Self::SeeYa),
            _ => Err(AnimeError::ParseError(s.to_owned())),
        }
    }
}

impl From<i32> for AnimShutdown {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::GlitchOut,
            1 => Self::SeeYa,
            _ => Self::default(),
        }
    }
}

impl From<AnimShutdown> for i32 {
    fn from(value: AnimShutdown) -> Self {
        value as i32
    }
}

/// `get_maybe_anime_type` is very broad, matching on part of the laptop board
/// name only. For this reason `find_node()` must be used also to verify if the
/// USB device is available.
///
/// The currently known USB device for the AniMe Matrix is `0x193b` (see
/// `PROD_ID` above). `0x19b6` is a different ASUS USB device (the N-KEY
/// keyboard interface) — historical comment was incorrect.
#[inline]
pub fn get_anime_type() -> AnimeType {
    let dmi = DMIID::new().unwrap_or_default();
    let board_name = dmi.board_name.to_uppercase();

    if board_name.contains("GA401I") || board_name.contains("GA401Q") {
        AnimeType::GA401
    } else if board_name.contains("GA402R")
        || board_name.contains("GA402X")
        || board_name.contains("GA402N")
    {
        AnimeType::GA402
    } else if board_name.contains("GU604V") {
        AnimeType::GU604
    } else if board_name.contains("G635L") {
        AnimeType::G635L
    } else if board_name.contains("G835L") {
        AnimeType::G835L
    } else {
        AnimeType::Unsupported
    }
}

/// Get the two device initialization packets. These are required for device
/// start after the laptop boots.
#[inline]
pub const fn pkts_for_init() -> [[u8; PACKET_SIZE]; 2] {
    let mut packets = [[0; PACKET_SIZE]; 2];
    packets[0][0] = DEV_PAGE; // This is the USB page we're using throughout
    let mut count = 0;
    // TODO: memcpy or slice copy
    let bytes = "ASUS Tech.Inc.".as_bytes();
    while count < bytes.len() {
        packets[0][count + 1] = bytes[count];
        count += 1;
    }
    //
    packets[1][0] = DEV_PAGE;
    packets[1][1] = 0xc2;
    packets
}

/// Should be written to the device after writing the two main data packets that
/// make up the display data packet
#[inline]
pub const fn pkt_flush() -> [u8; PACKET_SIZE] {
    let mut pkt = [0; PACKET_SIZE];
    pkt[0] = DEV_PAGE;
    pkt[1] = 0xc0;
    pkt[2] = 0x03;
    pkt
}

/// Packet for setting the brightness (0-3). Requires
/// `pkt_for_apply()` to be written after.
#[inline]
pub const fn pkt_set_brightness(brightness: Brightness) -> [u8; PACKET_SIZE] {
    let mut pkt = [0; PACKET_SIZE];
    pkt[0] = DEV_PAGE;
    pkt[1] = 0xc0;
    pkt[2] = 0x04;
    pkt[3] = brightness as u8;
    pkt
}

/// Enable the display?
#[inline]
pub const fn pkt_set_enable_display(status: bool) -> [u8; PACKET_SIZE] {
    let mut pkt = [0; PACKET_SIZE];
    pkt[0] = DEV_PAGE;
    pkt[1] = 0xc3;
    pkt[2] = 0x01;
    pkt[3] = if status { 0x00 } else { 0x80 };
    pkt
}

/// Enable builtin animations?
#[inline]
pub const fn pkt_set_enable_powersave_anim(status: bool) -> [u8; PACKET_SIZE] {
    let mut pkt = [0; PACKET_SIZE];
    pkt[0] = DEV_PAGE;
    pkt[1] = 0xc4;
    pkt[2] = 0x01;
    pkt[3] = if status { 0x00 } else { 0x80 };
    pkt
}

/// Set which animations are shown for each stage
#[inline]
pub const fn pkt_set_builtin_animations(
    boot: AnimBooting,
    awake: AnimAwake,
    sleep: AnimSleeping,
    shutdown: AnimShutdown,
) -> [u8; PACKET_SIZE] {
    let mut pkt = [0; PACKET_SIZE];
    pkt[0] = DEV_PAGE;
    pkt[1] = 0xc5;
    pkt[2] = (awake as u8)
        | ((sleep as u8) << 0x01)
        | ((shutdown as u8) << 0x02)
        | ((boot as u8) << 0x03);
    pkt
}
