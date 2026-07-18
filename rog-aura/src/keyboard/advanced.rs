use log::warn;
use serde::{Deserialize, Serialize};
#[cfg(feature = "dbus")]
use zbus::zvariant::Type;

// TODO: GZ301Z
// 5dbcd0010300000a00ff0000ff0000ff0000ff0000ff0000ff0000ff0000ff0000ff0000ff0000

/// The `LedCode` used in setting up keyboard layouts is important because it
/// determines the idexing for an RGB value in the final USB packets (for
/// per-key addressable keyboards).
#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub enum LedCode {
    VolUp,
    VolDown,
    MicMute,
    #[default]
    RogApp,
    RogFan,
    Esc,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Del,
    Tilde,
    N1,
    N2,
    N3,
    N4,
    N5,
    N6,
    N7,
    N8,
    N9,
    N0,
    Hyphen,
    Equals,
    Backspace,
    /// For keyboards where the backspace button has 3 LED
    Backspace3_1,
    Backspace3_2,
    Backspace3_3,
    Home,
    Tab,
    Q,
    W,
    E,
    R,
    T,
    Y,
    U,
    I,
    O,
    P,
    LBracket,
    RBracket,
    BackSlash,
    PgUp,
    Caps,
    A,
    S,
    D,
    F,
    G,
    H,
    J,
    K,
    L,
    SemiColon,
    Quote,
    Return,
    /// For keyboards where the return button has 3 LED
    Return3_1,
    Return3_2,
    Return3_3,
    PgDn,
    LShift,
    /// For keyboards where the left shift button has 3 LED
    LShift3_1,
    LShift3_2,
    LShift3_3,
    Z,
    X,
    C,
    V,
    B,
    N,
    M,
    Comma,
    Period,
    FwdSlash,
    Star,
    NumPadDel,
    NumPadPlus,
    NumPadEnter,
    NumPadPause,
    NumPadPrtSc,
    NumPadHome,
    NumLock,
    Rshift,
    Rshift3_1,
    Rshift3_2,
    Rshift3_3,
    End,
    LCtrl,
    LFn,
    Meta,
    LAlt,
    Spacebar,
    /// For keyboards where the spacebar button has 5 LED
    Spacebar5_1,
    Spacebar5_2,
    Spacebar5_3,
    Spacebar5_4,
    Spacebar5_5,
    Pause,
    RAlt,
    PrtSc,
    RCtrl,
    Up,
    Down,
    Left,
    Right,
    RFn,
    MediaPlay,
    MediaStop,
    MediaNext,
    MediaPrev,
    LidLogo,
    LidLeft,
    LidRight,
    /// Used by per-key and multizoned
    LightbarRight,
    /// Used by per-key and multizoned
    LightbarRightCorner,
    /// Used by per-key and multizoned
    LightbarRightBottom,
    /// Used by per-key and multizoned
    LightbarLeftBottom,
    /// Used by per-key and multizoned
    LightbarLeftCorner,
    /// Used by per-key and multizoned
    LightbarLeft,
    /// Use if the keyboard supports only a single zone. This zone uses the same
    /// packet data as the `Zoned*` below
    SingleZone,
    /// Use if the keyboard supports 4 zones, this is the left zone
    ZonedKbLeft,
    /// Use if the keyboard supports 4 zones, this is the left-center zone
    ZonedKbLeftMid,
    /// Use if the keyboard supports 4 zones, this is the right-center zone
    ZonedKbRightMid,
    /// Use if the keyboard supports 4 zones, this is the right zone
    ZonedKbRight,
    /// To be ignored by effects
    Spacing,
    /// To be ignored by effects
    Blocking,
}

impl LedCode {
    pub fn is_placeholder(&self) -> bool {
        matches!(self, Self::Spacing | Self::Blocking)
    }

    pub fn is_keyboard_zone(&self) -> bool {
        matches!(
            self,
            Self::ZonedKbLeft | Self::ZonedKbLeftMid | Self::ZonedKbRightMid | Self::ZonedKbRight
        )
    }

    pub fn is_lightbar_zone(&self) -> bool {
        matches!(
            self,
            Self::LightbarLeft
                | Self::LightbarLeftCorner
                | Self::LightbarLeftBottom
                | Self::LightbarRightBottom
                | Self::LightbarRightCorner
                | Self::LightbarRight
        )
    }
}

/// Represents the per-key raw USB packets
pub type AuraLaptopUsbPackets = Vec<Vec<u8>>;

/// A `UsbPackets` contains all data to change the full set of keyboard
/// key colours individually.
///
/// Each row of the internal array is a full HID packet that can be sent
/// to the keyboard EC. One row controls one group of keys, these keys are not
/// necessarily all on the same row of the keyboard, with some splitting between
/// two rows.
#[cfg_attr(feature = "dbus", derive(Type))]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LedUsbPackets {
    /// The packet data used to send data to the USB keyboard
    usb_packets: AuraLaptopUsbPackets,
    /// Wether or not this packet collection is zoned. The determines which
    /// starting bytes are used and what the indexing is for lightbar RGB
    /// colours
    zoned: bool,
}

impl Default for LedUsbPackets {
    fn default() -> Self {
        Self::new_per_key()
    }
}

impl LedUsbPackets {
    /// Set up a series of per-key packets. This includes setting all the
    /// required starting bytes per packet, but does not set any colours.
    ///
    /// These packets will not work with per-zone keyboards
    pub fn new_per_key() -> Self {
        let mut set = vec![vec![0u8; 64]; 11];
        // set[0].copy_from_slice(&KeyColourArray::get_init_msg());
        for (count, row) in set.iter_mut().enumerate() {
            row[0] = 0x5d; // Report ID
            row[1] = 0xbc; // Mode = custom??, 0xb3 is builtin
            row[2] = 0x00;
            row[3] = 0x01; // ??
            row[4] = 0x01; // ??, 4,5,6 are normally RGB for builtin mode colours
            row[5] = 0x01; // ??
            row[6] = (count as u8) << 4; // Key group
            if count == 10 {
                row[7] = 0x08; // 0b00001000
            } else {
                row[7] = 0x10; // 0b00010000 addressing? flips for group a0
            }
            row[8] = 0x00;
        }
        Self {
            usb_packets: set,
            zoned: false,
        }
    }

    /// Create new zoned packets. Although the result is a nested `Vec` only the
    /// first vector is available. The final packet is slightly different
    /// for single-zoned compared to multizoned.
    ///
    /// This packet will not work with per-key keyboards
    ///
    /// Wireshark captures show:
    /// ```ignore
    /// 5d,bc,01,01,00,00,00,00,00,ff,00,00, RED, single zone
    /// 5d,bc,01,01,04,00,00,00,00,ff,00,00, RED, multizone
    /// ```
    pub fn new_zoned(multizoned: bool) -> Self {
        let mut pkt = vec![0u8; 64];
        pkt[0] = 0x5d; // Report ID
        pkt[1] = 0xbc; // Mode = custom??, 0xb3 is builtin
        pkt[2] = 0x01;
        pkt[3] = 0x01; // ??
        if !multizoned {
            pkt[4] = 0x00; // This doesn't actually seem to matter on this
                           // keyboard?
        } else {
            pkt[4] = 0x04; // ??, 4,5,6 are normally RGB for builtin mode
                           // colours
        }
        Self {
            usb_packets: vec![pkt],
            zoned: true,
        }
    }

    /// Initialise and clear the keyboard for custom effects, this must be done
    /// for every time mode switches from builtin to custom
    #[inline]
    pub const fn get_init_msg() -> [u8; 64] {
        let mut init = [0u8; 64];
        init[0] = 0x5d; // Report ID
        init[1] = 0xbc; // Mode = custom??, 0xb3 is builtin
        init
    }

    /// Set the RGB colour of an `LedCode`
    #[inline]
    pub fn set(&mut self, key: LedCode, r: u8, g: u8, b: u8) {
        if let Some(c) = self.rgb_for_led_code(key) {
            c[0] = r;
            c[1] = g;
            c[2] = b;
        }
    }

    /// Indexes in to `UsbPackets` at the correct row and column
    /// to set a series of three bytes to the chosen R,G,B values
    ///
    /// Indexing is different for `zoned` and assumes that only one packet is
    /// generated for all the zones
    fn rgb_for_led_code(&mut self, led_code: LedCode) -> Option<&mut [u8]> {
        let zoned = self.zoned;
        // Tuples are indexes in to array
        #[allow(clippy::match_same_arms)]
        let (row, col) = match led_code {
            LedCode::VolDown => (0, 15),
            LedCode::VolUp => (0, 18),
            LedCode::MicMute => (0, 21),
            LedCode::RogApp => (0, 24),
            //
            LedCode::Esc => (1, 24),
            LedCode::F1 => (1, 30),
            LedCode::F2 => (1, 33),
            LedCode::F3 => (1, 36),
            LedCode::F4 => (1, 39),
            LedCode::F5 => (1, 45),
            LedCode::F6 => (1, 48),
            LedCode::F7 => (1, 51),
            LedCode::F8 => (1, 54),
            //
            LedCode::F9 => (2, 12),
            LedCode::F10 => (2, 15),
            LedCode::F11 => (2, 18),
            LedCode::F12 => (2, 21),
            LedCode::Del => (2, 24),
            LedCode::Tilde => (2, 39),
            LedCode::N1 => (2, 42),
            LedCode::N2 => (2, 45),
            LedCode::N3 => (2, 48),
            LedCode::N4 => (2, 51),
            LedCode::N5 => (2, 54),
            //
            LedCode::N6 => (3, 9),
            LedCode::N7 => (3, 12),
            LedCode::N8 => (3, 15),
            LedCode::N9 => (3, 18),
            LedCode::N0 => (3, 21),
            LedCode::Hyphen => (3, 24),
            LedCode::Equals => (3, 27),
            LedCode::Backspace3_1 => (3, 30),
            LedCode::Backspace3_2 => (3, 33),
            LedCode::Backspace3_3 => (3, 36),
            LedCode::Home => (3, 39),
            LedCode::Tab => (3, 54),
            //
            LedCode::Q => (4, 9),
            LedCode::W => (4, 12),
            LedCode::E => (4, 15),
            LedCode::R => (4, 18),
            LedCode::T => (4, 21),
            LedCode::Y => (4, 24),
            LedCode::U => (4, 27),
            LedCode::I => (4, 30),
            LedCode::O => (4, 33),
            LedCode::P => (4, 36),
            LedCode::LBracket => (4, 39),
            LedCode::RBracket => (4, 42),
            LedCode::BackSlash => (4, 45),
            LedCode::PgUp => (4, 54),
            //
            LedCode::Caps => (5, 21),
            LedCode::A => (5, 24),
            LedCode::S => (5, 27),
            LedCode::D => (5, 30),
            LedCode::F => (5, 33),
            LedCode::G => (5, 36),
            LedCode::H => (5, 39),
            LedCode::J => (5, 42),
            LedCode::K => (5, 45),
            LedCode::L => (5, 48),
            LedCode::SemiColon => (5, 51),
            LedCode::Quote => (5, 54),
            //
            LedCode::Return => (6, 9),
            LedCode::Return3_1 => (6, 12),
            LedCode::Return3_2 => (6, 15),
            LedCode::Return3_3 => (6, 18),
            LedCode::PgDn => (6, 21),
            LedCode::LShift => (6, 36),
            LedCode::LShift3_1 => (6, 39),
            LedCode::LShift3_2 => (6, 36),
            LedCode::LShift3_3 => (6, 36),
            LedCode::Z => (6, 42),
            LedCode::X => (6, 45),
            LedCode::C => (6, 48),
            LedCode::V => (6, 51),
            LedCode::B => (6, 54),
            //
            LedCode::N => (7, 9),
            LedCode::M => (7, 12),
            LedCode::Comma => (7, 15),
            LedCode::Period => (7, 18),
            LedCode::FwdSlash => (7, 21),
            LedCode::Rshift => (7, 24),
            LedCode::Rshift3_1 => (7, 27),
            LedCode::Rshift3_2 => (7, 30),
            LedCode::Rshift3_3 => (7, 33),
            LedCode::End => (7, 36),
            LedCode::LCtrl => (7, 51),
            LedCode::LFn => (7, 54),
            //
            LedCode::Meta => (8, 9),
            LedCode::LAlt => (8, 12),
            LedCode::Spacebar5_1 => (8, 15),
            LedCode::Spacebar5_2 => (8, 18),
            LedCode::Spacebar5_3 => (8, 21),
            LedCode::Spacebar5_4 => (8, 24),
            LedCode::Spacebar5_5 => (8, 27),
            LedCode::RAlt => (8, 30),
            LedCode::PrtSc => (8, 33),
            LedCode::RCtrl => (8, 36),
            LedCode::Up => (8, 42),
            LedCode::RFn => (8, 51),
            //
            LedCode::Left => (9, 54),
            //
            LedCode::Down => (10, 9),
            LedCode::Right => (10, 12),
            LedCode::LidLogo => (11, 9),
            LedCode::LidLeft => (11, 36),
            LedCode::LidRight => (11, 39),
            //
            LedCode::SingleZone | LedCode::ZonedKbLeft => (0, 9),
            LedCode::ZonedKbLeftMid => (0, 12),
            LedCode::ZonedKbRightMid => (0, 15),
            LedCode::ZonedKbRight => (0, 18),
            LedCode::LightbarRight => if zoned {(0, 27)}  else { (11, 15)},
            LedCode::LightbarRightCorner => if zoned {(0, 30)}  else {(11, 18)},
            LedCode::LightbarRightBottom => if zoned {(0, 33)}  else{(11, 21)},
            LedCode::LightbarLeftBottom => if zoned {(0, 36)}  else{(11, 24)},
            LedCode::LightbarLeftCorner => if zoned {(0, 39)}  else{(11, 27)},
            LedCode::LightbarLeft => if zoned {(0, 42)}  else{(11, 30)},
            //
            LedCode::Spacing
            | LedCode::Blocking
            // TODO: the addressing of the following
            | LedCode::MediaPlay
            | LedCode::MediaStop
            | LedCode::MediaPrev
            | LedCode::MediaNext
            | LedCode::Pause
            | LedCode::NumLock
            | LedCode::Star
            | LedCode::NumPadDel
            | LedCode::NumPadPlus
            | LedCode::NumPadEnter
            | LedCode::NumPadPause
            | LedCode::NumPadPrtSc
            | LedCode::NumPadHome
            | LedCode::RogFan
            | LedCode::Spacebar
            | LedCode::Backspace => return None,
        };

        if self.zoned && row > 0 {
            warn!(
                "LedCode {led_code:?} for zoned is not correct or out of Zone range. Setting to 0",
            );
            return None;
        }

        Some(&mut self.usb_packets[row][col..=col + 2])
    }

    #[inline]
    pub fn get(&self) -> AuraLaptopUsbPackets {
        self.usb_packets.clone()
    }

    #[inline]
    pub fn get_ref(&self) -> &AuraLaptopUsbPackets {
        &self.usb_packets
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut AuraLaptopUsbPackets {
        &mut self.usb_packets
    }
}

impl From<LedUsbPackets> for AuraLaptopUsbPackets {
    fn from(k: LedUsbPackets) -> Self {
        k.usb_packets
    }
}

impl From<LedCode> for &str {
    fn from(k: LedCode) -> Self {
        (&k).into()
    }
}

impl From<&LedCode> for &str {
    fn from(k: &LedCode) -> Self {
        #[allow(clippy::match_same_arms)]
        match k {
            LedCode::VolUp => "Volume Up",
            LedCode::VolDown => "Volume Down",
            LedCode::MicMute => "Mute Mic",
            LedCode::RogApp => "ROG",
            LedCode::RogFan => "Fan Control",
            LedCode::Esc => "Escape",
            LedCode::F1 => "F1",
            LedCode::F2 => "F2",
            LedCode::F3 => "F3",
            LedCode::F4 => "F4",
            LedCode::F5 => "F5",
            LedCode::F6 => "F6",
            LedCode::F7 => "F7",
            LedCode::F8 => "F8",
            LedCode::F9 => "F9",
            LedCode::F10 => "F10",
            LedCode::F11 => "F11",
            LedCode::F12 => "F12",
            LedCode::Del => "Delete",
            LedCode::Tilde => "Tilde",
            LedCode::N1 => "1",
            LedCode::N2 => "2",
            LedCode::N3 => "3",
            LedCode::N4 => "4",
            LedCode::N5 => "5",
            LedCode::N6 => "6",
            LedCode::N7 => "7",
            LedCode::N8 => "8",
            LedCode::N9 => "9",
            LedCode::N0 => "0",
            LedCode::Hyphen => "-",
            LedCode::Equals => "=",
            LedCode::Backspace => "Backspace",
            LedCode::Backspace3_1 => "Backspace LED 1",
            LedCode::Backspace3_2 => "Backspace LED 2",
            LedCode::Backspace3_3 => "Backspace LED 3",
            LedCode::Home => "Home",
            LedCode::Tab => "Tab",
            LedCode::Q => "Q",
            LedCode::W => "W",
            LedCode::E => "E",
            LedCode::R => "R",
            LedCode::T => "T",
            LedCode::Y => "Y",
            LedCode::U => "U",
            LedCode::I => "I",
            LedCode::O => "O",
            LedCode::P => "P",
            LedCode::LBracket => "[",
            LedCode::RBracket => "]",
            LedCode::BackSlash => "\\",
            LedCode::PgUp => "Page Up",
            LedCode::Caps => "Caps Lock",
            LedCode::A => "A",
            LedCode::S => "S",
            LedCode::D => "D",
            LedCode::F => "F",
            LedCode::G => "G",
            LedCode::H => "H",
            LedCode::J => "J",
            LedCode::K => "K",
            LedCode::L => "L",
            LedCode::SemiColon => ";",
            LedCode::Quote => "'",
            LedCode::Return => "Return",
            LedCode::Return3_1 => "Return LED 1",
            LedCode::Return3_2 => "Return LED 2",
            LedCode::Return3_3 => "Return LED 3",
            LedCode::PgDn => "Page Down",
            LedCode::LShift => "Left Shift",
            LedCode::LShift3_1 => "Left Shift LED 1",
            LedCode::LShift3_2 => "Left Shift LED 2",
            LedCode::LShift3_3 => "Left Shift LED 3",
            LedCode::Z => "Z",
            LedCode::X => "X",
            LedCode::C => "C",
            LedCode::V => "V",
            LedCode::B => "B",
            LedCode::N => "N",
            LedCode::M => "M",
            LedCode::Comma => ",",
            LedCode::Period => ".",
            LedCode::Star => "*",
            LedCode::NumPadDel => "Delete",
            LedCode::NumPadPlus => "+",
            LedCode::NumPadEnter => "Enter",
            LedCode::NumPadPause => "Pause",
            LedCode::NumPadPrtSc => "Print Screen",
            LedCode::NumPadHome => "Home",
            LedCode::NumLock => "Num-Lock",
            LedCode::FwdSlash => "/",
            LedCode::Rshift => "Right Shift",
            LedCode::Rshift3_1 => "Right Shift LED 1",
            LedCode::Rshift3_2 => "Right Shift LED 2",
            LedCode::Rshift3_3 => "Right Shift LED 3",
            LedCode::End => "End",
            LedCode::LCtrl => "Left Control",
            LedCode::LFn => "Left Fn",
            LedCode::Meta => "Meta",
            LedCode::LAlt => "Left Alt",
            LedCode::Spacebar => "Space",
            LedCode::Spacebar5_1 => "Space LED 1",
            LedCode::Spacebar5_2 => "Space LED 2",
            LedCode::Spacebar5_3 => "Space LED 3",
            LedCode::Spacebar5_4 => "Space LED 4",
            LedCode::Spacebar5_5 => "Space LED 5",
            LedCode::RAlt => "Right Alt",
            LedCode::PrtSc => "Print Screen",
            LedCode::RCtrl => "Right Control",
            LedCode::Pause => "Pause",
            LedCode::Up => "Up",
            LedCode::Down => "Down",
            LedCode::Left => "Left",
            LedCode::Right => "Right",
            LedCode::RFn => "Right Fn",
            LedCode::MediaPlay => "Media Play",
            LedCode::MediaStop => "Media Stop",
            LedCode::MediaNext => "Media Next",
            LedCode::MediaPrev => "Media Previous",
            LedCode::LidLogo => "Lid Logo",
            LedCode::LidLeft => "Lid Left",
            LedCode::LidRight => "Lid Right",
            LedCode::LightbarRight => "Lightbar Right",
            LedCode::LightbarRightCorner => "Lightbar Right Corner",
            LedCode::LightbarRightBottom => "Lightbar Right Bottom",
            LedCode::LightbarLeftBottom => "Lightbar Left Bottom",
            LedCode::LightbarLeftCorner => "Lightbar Left Corner",
            LedCode::LightbarLeft => "Lightbar Left",
            LedCode::Spacing | LedCode::Blocking => "",
            LedCode::SingleZone => "Single Zoned Keyboard",
            LedCode::ZonedKbLeft => "Left Zone (zone 1)",
            LedCode::ZonedKbLeftMid => "Center-left Zone (zone 2)",
            LedCode::ZonedKbRightMid => "Center-right Zone (zone 3)",
            LedCode::ZonedKbRight => "Right Zone (zone 4)",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::keyboard::{AuraLaptopUsbPackets, LedCode, LedUsbPackets};

    macro_rules! colour_check_zoned {
        ($zone:expr, $pkt_idx_start:expr) => {
            let mut zone = LedUsbPackets::new_zoned(true);
            let c = zone.rgb_for_led_code($zone).unwrap();
            c[0] = 255;
            c[1] = 255;
            c[2] = 255;

            let pkt: AuraLaptopUsbPackets = zone.into();
            assert_eq!(pkt[0][$pkt_idx_start], 0xff);
            assert_eq!(pkt[0][$pkt_idx_start + 1], 0xff);
            assert_eq!(pkt[0][$pkt_idx_start + 2], 0xff);
        };
    }

    #[test]
    fn zone_to_packet_check() {
        let zone = LedUsbPackets::new_zoned(true);
        let pkt: AuraLaptopUsbPackets = zone.into();
        assert_eq!(pkt[0][0], 0x5d);
        assert_eq!(pkt[0][1], 0xbc);
        assert_eq!(pkt[0][2], 0x01);
        assert_eq!(pkt[0][3], 0x01);
        assert_eq!(pkt[0][4], 0x04);

        colour_check_zoned!(LedCode::ZonedKbLeft, 9);
        colour_check_zoned!(LedCode::ZonedKbLeftMid, 12);
        colour_check_zoned!(LedCode::ZonedKbRightMid, 15);
        colour_check_zoned!(LedCode::ZonedKbRight, 18);

        colour_check_zoned!(LedCode::LightbarRight, 27);
        colour_check_zoned!(LedCode::LightbarRightCorner, 30);
        colour_check_zoned!(LedCode::LightbarRightBottom, 33);
        colour_check_zoned!(LedCode::LightbarLeftBottom, 36);
        colour_check_zoned!(LedCode::LightbarLeftCorner, 39);
        colour_check_zoned!(LedCode::LightbarLeft, 42);
    }

    #[test]
    fn perkey_to_packet_check() {
        let per_key = LedUsbPackets::new_per_key();
        let pkt: AuraLaptopUsbPackets = per_key.into();
        assert_eq!(pkt[0][0], 0x5d);
        assert_eq!(pkt[0][1], 0xbc);
        assert_eq!(pkt[0][2], 0x00);
        assert_eq!(pkt[0][3], 0x01);
        assert_eq!(pkt[0][4], 0x01);
        assert_eq!(pkt[0][5], 0x01);

        let mut per_key = LedUsbPackets::new_per_key();
        let c = per_key.rgb_for_led_code(LedCode::D).unwrap();
        c[0] = 255;
        c[1] = 255;
        c[2] = 255;
        let c = per_key.rgb_for_led_code(LedCode::O).unwrap();
        c[0] = 255;
        c[1] = 255;
        c[2] = 255;
        let c = per_key.rgb_for_led_code(LedCode::N0).unwrap();
        c[0] = 255;
        c[1] = 255;
        c[2] = 255;
        let c = per_key.rgb_for_led_code(LedCode::M).unwrap();
        c[0] = 255;
        c[1] = 255;
        c[2] = 255;

        let pkt: AuraLaptopUsbPackets = per_key.into();
        assert_eq!(pkt[5][30], 0xff); // D, red
        assert_eq!(pkt[5][31], 0xff); // D
        assert_eq!(pkt[5][32], 0xff); // D
        assert_eq!(pkt[5][33], 0x00); // D

        assert_eq!(pkt[4][33], 0xff); // O, red
        assert_eq!(pkt[4][34], 0xff); // O
        assert_eq!(pkt[4][35], 0xff); // O
        assert_eq!(pkt[4][36], 0x00); // O

        assert_eq!(pkt[7][12], 0xff); // M, red
        assert_eq!(pkt[7][13], 0xff); // M
        assert_eq!(pkt[7][14], 0xff); // M
        assert_eq!(pkt[7][15], 0x00); // M
    }
}
