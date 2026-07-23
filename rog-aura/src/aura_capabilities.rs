//! Firmware-reported Aura capability data.
//!
//! Newer ASUS laptop keyboards expose the effects supported by their
//! firmware through a HID feature report.  The transport is implemented in
//! `rog-platform`; this module only validates and decodes the report so it is
//! usable from tests and from non-HID callers without touching a device.

use crate::AuraModeNum;

/// HID report id used by the ASUS Aura laptop protocol.
pub const AURA_REPORT_ID: u8 = 0x5d;
/// Feature command used by Armoury Crate for the notebook firmware query.
pub const AURA_CAPABILITY_COMMAND: u8 = 0x9e;
/// The two selectors observed in Armoury Crate's notebook query path.
pub const AURA_CAPABILITY_SELECTORS: [u8; 2] = [
    0x20, 0x15,
];
/// Common notebook report size used by parser fixtures (including report id).
pub const AURA_FEATURE_REPORT_LEN: usize = 64;

/// HID command used by Armoury Crate's `GetRGBKBStatus` probe.
pub const AURA_STATUS_COMMAND: u8 = 0x05;
/// The status response starts with `5d 05 20 31 00` and contains its common
/// fields through byte 14.  Firmware from 2023 onward also uses byte 17.
pub const AURA_STATUS_MIN_REPORT_LEN: usize = 15;

// Byte 13: physical LED regions reported by the keyboard firmware.
pub const STATUS_REGION_LOGO: u8 = 0x01;
pub const STATUS_REGION_LIGHTBAR: u8 = 0x02;
pub const STATUS_REGION_VCUT: u8 = 0x10;
pub const STATUS_REGION_AERO: u8 = 0x20;
pub const STATUS_REGION_BUMP: u8 = 0x40;
pub const STATUS_REGION_REARGLOW: u8 = 0x80;

// Byte 14: controller/format features reported by the keyboard firmware.
pub const STATUS_FEATURE_DEFAULT_COLOR: u8 = 0x04;
pub const STATUS_FEATURE_RGB_WHEEL: u8 = 0x08;
pub const STATUS_FEATURE_ONE_ZONE_RED_EFFECT: u8 = 0x10;
pub const STATUS_FEATURE_KEY_POSITION: u8 = 0x40;

const STATUS_OFFSET: usize = 4;
const LOW_MODE_MASK_OFFSET: usize = 20;
const HIGH_MODE_MASK_OFFSET: usize = 21;

/// Decoded `GetRGBKBStatus` response.  The bit fields are intentionally kept
/// raw: Armoury uses them to select layouts and power-state controls, and
/// their meaning is more stable than any particular UI policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuraHardwareStatus {
    pub keyboard_type: u8,
    pub keyboard_year: u8,
    pub layout: u8,
    pub region_bits: u8,
    pub feature_bits: u8,
    pub model_family: u8,
}

impl AuraHardwareStatus {
    pub fn has_logo(&self) -> bool {
        self.region_bits & STATUS_REGION_LOGO != 0
    }

    pub fn has_lightbar(&self) -> bool {
        self.region_bits & STATUS_REGION_LIGHTBAR != 0
    }

    pub fn has_vcut(&self) -> bool {
        self.region_bits & STATUS_REGION_VCUT != 0
    }

    pub fn has_aero(&self) -> bool {
        self.region_bits & STATUS_REGION_AERO != 0
    }

    pub fn has_bump(&self) -> bool {
        self.region_bits & STATUS_REGION_BUMP != 0
    }

    pub fn has_rearglow(&self) -> bool {
        self.region_bits & STATUS_REGION_REARGLOW != 0
    }

    /// Whether the firmware advertises the key-position map format.
    ///
    /// This is a packet-format capability, not a claim that the keyboard is
    /// per-key addressable.  For example, the tested GV601VV reports this bit
    /// while its backlight type is still single-zone.
    pub fn has_key_position_map(&self) -> bool {
        self.feature_bits & STATUS_FEATURE_KEY_POSITION != 0
    }

    pub fn has_default_color(&self) -> bool {
        self.feature_bits & STATUS_FEATURE_DEFAULT_COLOR != 0
    }

    pub fn has_rgb_wheel(&self) -> bool {
        self.feature_bits & STATUS_FEATURE_RGB_WHEEL != 0
    }

    pub fn has_one_zone_red_effect(&self) -> bool {
        self.feature_bits & STATUS_FEATURE_ONE_ZONE_RED_EFFECT != 0
    }
}

/// Decode Armoury Crate's `GetRGBKBStatus` feature response.
pub fn parse_hardware_status(report: &[u8]) -> Option<AuraHardwareStatus> {
    if report.len() < AURA_STATUS_MIN_REPORT_LEN
        || report[0] != AURA_REPORT_ID
        || report[1] != AURA_STATUS_COMMAND
        || report[2] != 0x20
        || report[3] != 0x31
        || report[4] != 0
    {
        return None;
    }

    // Armoury uses byte 5 as the exclusive populated-end offset.  Reject
    // truncated or nonsensical lengths so a stale buffer cannot turn random
    // bits into hardware capabilities.
    let payload_len = report[5] as usize;
    if !(AURA_STATUS_MIN_REPORT_LEN..=report.len()).contains(&payload_len) {
        return None;
    }

    let keyboard_year = report[10];
    let model_family = if keyboard_year >= 0x23 {
        if payload_len <= 17 {
            return None;
        }
        report[17]
    } else {
        0
    };

    Some(AuraHardwareStatus {
        keyboard_type: report[9],
        keyboard_year,
        layout: report[12],
        region_bits: report[13],
        feature_bits: report[14],
        model_family,
    })
}

/// AC/DC awake-state readback returned in bytes 26..29 of `0x9e`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuraAwakePowerReadback {
    pub raw: [u8; 4],
    pub keyboard_ac: bool,
    pub keyboard_dc: bool,
    pub logo_ac: bool,
    pub logo_dc: bool,
    pub lightbar_ac: bool,
    pub lightbar_dc: bool,
    pub aero_ac: bool,
    pub aero_dc: bool,
    pub vcut_ac: bool,
    pub vcut_dc: bool,
    pub bump_ac: bool,
    pub bump_dc: bool,
    pub rear_glow_ac: bool,
    pub rear_glow_dc: bool,
    pub wheel_ac: bool,
    pub wheel_dc: bool,
}

impl AuraAwakePowerReadback {
    fn from_raw(raw: [u8; 4]) -> Self {
        Self {
            raw,
            keyboard_ac: raw[0] & 0x02 != 0,
            keyboard_dc: raw[2] & 0x02 != 0,
            logo_ac: raw[0] & 0x04 != 0,
            logo_dc: raw[2] & 0x04 != 0,
            lightbar_ac: raw[0] & 0x08 != 0,
            lightbar_dc: raw[2] & 0x08 != 0,
            aero_ac: raw[0] & 0x10 != 0,
            aero_dc: raw[2] & 0x10 != 0,
            vcut_ac: raw[0] & 0x40 != 0,
            vcut_dc: raw[2] & 0x40 != 0,
            bump_ac: raw[1] & 0x02 != 0,
            bump_dc: raw[3] & 0x02 != 0,
            rear_glow_ac: raw[0] & 0x80 != 0,
            rear_glow_dc: raw[2] & 0x80 != 0,
            wheel_ac: raw[1] & 0x01 != 0,
            wheel_dc: raw[3] & 0x01 != 0,
        }
    }
}

/// Four-state power readback masks returned in bytes 22..25 of `0x9e`.
///
/// Each bit is the current boot, awake, sleep, or shutdown enable setting.
/// These values must not be interpreted as proof that a zone is supported;
/// physical-zone support comes from `GetRGBKBStatus`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuraPowerStateReadback {
    pub keyboard: u8,
    pub logo: u8,
    pub lightbar: u8,
    pub aero: u8,
    pub vcut: u8,
    pub bump: u8,
    pub rear_glow: u8,
    /// Present when byte 26 bit 0 advertises the extended AC/DC readback.
    pub awake_ac_dc: Option<AuraAwakePowerReadback>,
}

/// Decode the power-state portion of an `NBFWRunmodeReadback` response.
pub fn parse_power_state_readback(report: &[u8]) -> Option<AuraPowerStateReadback> {
    if report.len() <= 25
        || report[0] != AURA_REPORT_ID
        || report[1] != AURA_CAPABILITY_COMMAND
        || report[2] != 1
        || !AURA_CAPABILITY_SELECTORS.contains(&report[3])
        || report[4] != 1
    {
        return None;
    }

    Some(AuraPowerStateReadback {
        keyboard: report[22] & 0xaa,
        logo: report[22] & 0x55,
        lightbar: report[23] & 0x1e,
        aero: report[23] & 0x01,
        vcut: report[24] & 0x0f,
        bump: report[24] & 0xf0,
        rear_glow: report[25] & 0x0f,
        awake_ac_dc: (report.len() > 29 && report[26] & 0x01 != 0).then(|| {
            AuraAwakePowerReadback::from_raw([
                report[26], report[27], report[28], report[29],
            ])
        }),
    })
}

/// The capability bits returned by the keyboard firmware.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuraFirmwareCapabilities {
    pub mode_mask_low: u8,
    pub mode_mask_high: u8,
    pub modes: Vec<AuraModeNum>,
}

/// Decode an Armoury Crate `NBFWRunmodeReadback` response.
///
/// Armoury's internal mode numbers are not identical to `AuraModeNum`: for
/// example, firmware bit 2 is UWP ColorCycle, while asusd sends that effect
/// as `AuraModeNum::RainbowCycle` (packet value 2).  Keeping the translation
/// here prevents callers from accidentally exposing the Windows indexes.
pub fn parse_firmware_capabilities(report: &[u8]) -> Option<AuraFirmwareCapabilities> {
    if report.len() < HIGH_MODE_MASK_OFFSET + 1
        || report[0] != AURA_REPORT_ID
        || report[1] != AURA_CAPABILITY_COMMAND
        || report[2] != 1
        || !AURA_CAPABILITY_SELECTORS.contains(&report[3])
        || report[STATUS_OFFSET] != 1
    {
        return None;
    }

    let mode_mask_low = report[LOW_MODE_MASK_OFFSET];
    let mode_mask_high = report[HIGH_MODE_MASK_OFFSET];
    let mut modes = Vec::with_capacity(12);

    // Byte 20: Static, Breath, ColorCycle, Rainbow, Star, Rain, Reactive,
    // Laser.  These are the firmware bit positions, not the Linux enum
    // values (which intentionally omit a value 9).
    let low_modes = [
        (0x01, AuraModeNum::Static),
        (0x02, AuraModeNum::Breathe),
        (0x04, AuraModeNum::RainbowCycle),
        (0x08, AuraModeNum::RainbowWave),
        (0x10, AuraModeNum::Star),
        (0x20, AuraModeNum::Rain),
        (0x40, AuraModeNum::Highlight),
        (0x80, AuraModeNum::Laser),
    ];
    for (bit, mode) in low_modes {
        if mode_mask_low & bit != 0 {
            modes.push(mode);
        }
    }

    // Byte 21: Ripple, Strobing, (reserved), Comet, FlashDash.
    let high_modes = [
        (0x01, AuraModeNum::Ripple),
        (0x02, AuraModeNum::Pulse),
        (0x08, AuraModeNum::Comet),
        (0x10, AuraModeNum::Flash),
    ];
    for (bit, mode) in high_modes {
        if mode_mask_high & bit != 0 {
            modes.push(mode);
        }
    }

    // A valid response with no known mode bits is not useful as an override;
    // leave the static support database in charge in that case.  Static is
    // required because every supported Aura keyboard needs a base effect.
    if modes.is_empty() || !modes.contains(&AuraModeNum::Static) {
        return None;
    }

    Some(AuraFirmwareCapabilities {
        mode_mask_low,
        mode_mask_high,
        modes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(low: u8, high: u8) -> [u8; AURA_FEATURE_REPORT_LEN] {
        let mut report = [0; AURA_FEATURE_REPORT_LEN];
        report[0] = AURA_REPORT_ID;
        report[1] = AURA_CAPABILITY_COMMAND;
        report[2] = 1;
        report[3] = AURA_CAPABILITY_SELECTORS[0];
        report[STATUS_OFFSET] = 1;
        report[LOW_MODE_MASK_OFFSET] = low;
        report[HIGH_MODE_MASK_OFFSET] = high;
        report
    }

    #[test]
    fn decodes_all_known_mode_bits() {
        let parsed = parse_firmware_capabilities(&report(0xff, 0x1b)).unwrap();
        assert_eq!(parsed.mode_mask_low, 0xff);
        assert_eq!(parsed.mode_mask_high, 0x1b);
        assert_eq!(
            parsed.modes,
            vec![
                AuraModeNum::Static,
                AuraModeNum::Breathe,
                AuraModeNum::RainbowCycle,
                AuraModeNum::RainbowWave,
                AuraModeNum::Star,
                AuraModeNum::Rain,
                AuraModeNum::Highlight,
                AuraModeNum::Laser,
                AuraModeNum::Ripple,
                AuraModeNum::Pulse,
                AuraModeNum::Comet,
                AuraModeNum::Flash,
            ]
        );
    }

    #[test]
    fn accepts_both_armoury_selectors() {
        let mut response = report(0x05, 0);
        response[3] = AURA_CAPABILITY_SELECTORS[1];
        assert_eq!(
            parse_firmware_capabilities(&response).unwrap().modes,
            vec![
                AuraModeNum::Static,
                AuraModeNum::RainbowCycle
            ]
        );
    }

    #[test]
    fn rejects_malformed_or_unsupported_reports() {
        assert!(parse_firmware_capabilities(&[0; 22]).is_none());
        assert!(parse_firmware_capabilities(&report(0, 0)).is_none());

        let mut response = report(0x01, 0);
        response[STATUS_OFFSET] = 0;
        assert!(parse_firmware_capabilities(&response).is_none());

        let mut response = report(0x01, 0);
        response[1] = 0;
        assert!(parse_firmware_capabilities(&response).is_none());
    }

    #[test]
    fn decodes_hardware_status_fields() {
        let mut response = [0u8; AURA_FEATURE_REPORT_LEN];
        response[..6].copy_from_slice(&[
            AURA_REPORT_ID, AURA_STATUS_COMMAND, 0x20, 0x31, 0, 18,
        ]);
        response[9] = 4;
        response[10] = 0x24;
        response[12] = 3;
        response[13] = STATUS_REGION_LOGO | STATUS_REGION_LIGHTBAR | STATUS_REGION_REARGLOW;
        response[14] = STATUS_FEATURE_RGB_WHEEL | STATUS_FEATURE_KEY_POSITION;
        response[17] = 2;

        let parsed = parse_hardware_status(&response).unwrap();
        assert_eq!(parsed.keyboard_type, 4);
        assert_eq!(parsed.keyboard_year, 0x24);
        assert_eq!(parsed.layout, 3);
        assert!(parsed.has_logo());
        assert!(parsed.has_lightbar());
        assert!(parsed.has_rearglow());
        assert!(parsed.has_key_position_map());
        assert!(parsed.has_rgb_wheel());
        assert!(!parsed.has_default_color());
        assert!(!parsed.has_one_zone_red_effect());
        assert_eq!(parsed.model_family, 2);
    }

    #[test]
    fn accepts_shorter_pre_2023_hardware_status() {
        let mut response = [0u8; AURA_FEATURE_REPORT_LEN];
        response[..6].copy_from_slice(&[
            AURA_REPORT_ID,
            AURA_STATUS_COMMAND,
            0x20,
            0x31,
            0,
            AURA_STATUS_MIN_REPORT_LEN as u8,
        ]);
        response[10] = 0x22;
        response[14] = STATUS_FEATURE_DEFAULT_COLOR | STATUS_FEATURE_ONE_ZONE_RED_EFFECT;

        let parsed = parse_hardware_status(&response).unwrap();
        assert_eq!(parsed.model_family, 0);
        assert!(parsed.has_default_color());
        assert!(parsed.has_one_zone_red_effect());
    }

    #[test]
    fn rejects_truncated_hardware_status() {
        let mut response = [0u8; AURA_FEATURE_REPORT_LEN];
        response[..6].copy_from_slice(&[
            AURA_REPORT_ID, AURA_STATUS_COMMAND, 0x20, 0x31, 0, 14,
        ]);
        assert!(parse_hardware_status(&response).is_none());

        response[5] = 17;
        response[10] = 0x23;
        assert!(parse_hardware_status(&response).is_none());

        response[5] = 65;
        assert!(parse_hardware_status(&response).is_none());
    }

    #[test]
    fn decodes_power_state_and_ac_dc_readback() {
        let mut response = report(0x01, 0);
        response[22] = 0xaa | 0x55;
        response[23] = 0x1f;
        response[24] = 0xff;
        response[25] = 0x0f;
        response[26..30].copy_from_slice(&[
            0x01, 0x02, 0x04, 0x08,
        ]);

        let parsed = parse_power_state_readback(&response).unwrap();
        assert_eq!(parsed.keyboard, 0xaa);
        assert_eq!(parsed.logo, 0x55);
        assert_eq!(parsed.lightbar, 0x1e);
        assert_eq!(parsed.aero, 1);
        assert_eq!(parsed.vcut, 0x0f);
        assert_eq!(parsed.bump, 0xf0);
        assert_eq!(parsed.rear_glow, 0x0f);
        let awake = parsed.awake_ac_dc.unwrap();
        assert_eq!(awake.raw, [1, 2, 4, 8]);
        assert!(awake.bump_ac);
        assert!(awake.logo_dc);
        assert!(!awake.keyboard_ac);
        assert!(!awake.wheel_dc);
    }

    #[test]
    fn decodes_live_gv601vv_firmware_reports() {
        // Captured read-only from a GV601VV (BIOS 314, USB 0b05:19b6).
        // Only populated/decoded offsets are retained in this fixture.
        let mut status = [0u8; AURA_FEATURE_REPORT_LEN];
        status[..18].copy_from_slice(&[
            0x5d, 0x05, 0x20, 0x31, 0x00, 0x1a, 0x01, 0x40, 0x00, 0x04, 0x23, 0x04, 0x01, 0x00,
            0x46, 0x03, 0x11, 0x02,
        ]);
        let status = parse_hardware_status(&status).unwrap();
        assert_eq!(status.keyboard_type, 0x04);
        assert_eq!(status.keyboard_year, 0x23);
        assert_eq!(status.layout, 0x01);
        assert_eq!(status.region_bits, 0x00);
        assert_eq!(status.feature_bits, 0x46);
        assert_eq!(status.model_family, 0x02);

        let mut capabilities = report(0x07, 0x02);
        capabilities[6] = 0x02;
        capabilities[10] = 0xeb;
        capabilities[22..30].copy_from_slice(&[
            0xaa, 0x00, 0x00, 0x00, 0xff, 0x03, 0x01, 0x00,
        ]);

        let parsed = parse_firmware_capabilities(&capabilities).unwrap();
        assert_eq!(parsed.mode_mask_low, 0x07);
        assert_eq!(parsed.mode_mask_high, 0x02);
        assert_eq!(
            parsed.modes,
            vec![
                AuraModeNum::Static,
                AuraModeNum::Breathe,
                AuraModeNum::RainbowCycle,
                AuraModeNum::Pulse,
            ]
        );

        let power = parse_power_state_readback(&capabilities).unwrap();
        assert_eq!(power.keyboard, 0xaa);
        assert_eq!(power.awake_ac_dc.unwrap().raw, [0xff, 0x03, 0x01, 0x00]);
    }
}
