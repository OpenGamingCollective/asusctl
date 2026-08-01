use std::fs;
use std::path::PathBuf;

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use crate::error::{PlatformError, Result};

/// ASUS WMI Device ID for DialPad hardware toggle (`IIA0 == 0x00100063`)
pub const ASUS_WMI_DEVID_DIALPAD: u32 = 0x00100063;

/// Default maximum brightness level (0-255)
pub const DEFAULT_MAX_BRIGHTNESS: u8 = 255;

/// Operating mode for the DialPad controller ("hardware", "virtual", "auto").
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum DialpadMode {
    Hardware = 0,
    VirtualSoftware = 1,
    #[default]
    Auto = 2,
}

impl std::fmt::Display for DialpadMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hardware => write!(f, "hardware"),
            Self::VirtualSoftware => write!(f, "virtual"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

impl std::str::FromStr for DialpadMode {
    type Err = PlatformError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hardware" | "hw" => Ok(Self::Hardware),
            "virtual" | "virtualsoftware" | "sw" => Ok(Self::VirtualSoftware),
            "auto" => Ok(Self::Auto),
            _ => Err(PlatformError::MissingFunction(format!(
                "Invalid DialPad mode: {s}"
            ))),
        }
    }
}

/// The Dialpad device provides access to ASUS DialPad backlight and hardware/software status.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct Dialpad {
    path: Option<PathBuf>,
    wmi_dev_id_path: Option<PathBuf>,
    mode: DialpadMode,
    is_hardware_capable: bool,
    is_virtual_capable: bool,
    cached_brightness: u8,
}

impl Dialpad {
    fn build(
        path: Option<PathBuf>,
        wmi_dev_id_path: Option<PathBuf>,
        is_hardware_capable: bool,
        is_virtual_capable: bool,
    ) -> Self {
        Self {
            path,
            wmi_dev_id_path,
            mode: DialpadMode::Auto,
            is_hardware_capable,
            is_virtual_capable,
            cached_brightness: DEFAULT_MAX_BRIGHTNESS,
        }
    }

    pub fn new() -> Result<Self> {
        let wmi_path = PathBuf::from("/sys/devices/platform/asus-wmi/dev_id");
        let wmi_dev_id_path = if wmi_path.exists() {
            Some(wmi_path)
        } else {
            None
        };

        let is_virtual_capable = Self::has_asus_touchpad().unwrap_or(false);

        // Scan for physical LED device
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("leds").map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for device in enumerator.scan_devices().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("scan_devices failed".into(), err)
        })? {
            let name = device.sysname().to_string_lossy();
            if name == "asus::dialpad" || name == "asus_dialpad" {
                info!(
                    "Found hardware DialPad LED device at {:?}",
                    device.syspath()
                );
                return Ok(Self::build(
                    Some(device.syspath().to_path_buf()),
                    wmi_dev_id_path,
                    true,
                    is_virtual_capable,
                ));
            }
        }

        let fallback_path = PathBuf::from("/sys/class/leds/asus::dialpad");
        if fallback_path.exists() {
            info!(
                "Found hardware DialPad LED at fallback path {:?}",
                fallback_path
            );
            return Ok(Self::build(
                Some(fallback_path),
                wmi_dev_id_path,
                true,
                is_virtual_capable,
            ));
        }

        // If physical LED is missing, but an ASUS touchpad input device exists, initialize VirtualSoftware capability
        if is_virtual_capable {
            info!("Physical DialPad LED not found, but ASUS Touchpad detected. Initializing VirtualSoftware mode.");
            return Ok(Self::build(None, wmi_dev_id_path, false, true));
        }

        Err(PlatformError::MissingFunction(
            "Neither hardware DialPad LED nor ASUS touchpad found".into(),
        ))
    }

    /// Check if the system is manufactured by ASUS.
    pub fn is_asus_system() -> bool {
        if let Ok(vendor) = fs::read_to_string("/sys/class/dmi/id/sys_vendor") {
            let v = vendor.to_lowercase();
            if v.contains("asus") || v.contains("asustek") {
                return true;
            }
        }
        if let Ok(vendor) = fs::read_to_string("/sys/class/dmi/id/board_vendor") {
            let v = vendor.to_lowercase();
            if v.contains("asus") || v.contains("asustek") {
                return true;
            }
        }
        false
    }

    /// Check if an ASUS precision touchpad device exists on an ASUS system.
    pub fn has_asus_touchpad() -> Result<bool> {
        if !Self::is_asus_system() {
            return Ok(false);
        }

        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("input").map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for device in enumerator.scan_devices().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("scan_devices failed".into(), err)
        })? {
            let sysname = device.sysname().to_string_lossy();
            if sysname.starts_with("event") {
                if let Some(parent) = device.parent() {
                    let is_touchpad = device
                        .property_value("ID_INPUT_TOUCHPAD")
                        .map(|v| v == "1")
                        .unwrap_or(false);
                    let name = parent
                        .attribute_value("name")
                        .or_else(|| device.attribute_value("name"))
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_else(|| parent.sysname().to_string_lossy().to_lowercase());

                    let name_matches = name.contains("touchpad")
                        || name.contains("dialpad")
                        || name.contains("elan")
                        || name.contains("synaptics")
                        || name.contains("asue");

                    if is_touchpad || name_matches {
                        info!("Found ASUS touchpad device at {:?}", parent.syspath());
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Calculate radial rotation angle (Δθ in radians) between two touch coordinates
    /// (x1, y1) and (x2, y2) relative to dialpad center (cx, cy).
    pub fn calculate_rotation_angle(x1: f64, y1: f64, x2: f64, y2: f64, cx: f64, cy: f64) -> f64 {
        let theta1 = (y1 - cy).atan2(x1 - cx);
        let theta2 = (y2 - cy).atan2(x2 - cx);
        let mut delta = theta2 - theta1;
        while delta > std::f64::consts::PI {
            delta -= 2.0 * std::f64::consts::PI;
        }
        while delta < -std::f64::consts::PI {
            delta += 2.0 * std::f64::consts::PI;
        }
        delta
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn mode(&self) -> DialpadMode {
        self.mode
    }

    pub fn active_mode(&self) -> Result<DialpadMode> {
        match self.mode {
            DialpadMode::Auto => {
                if self.is_hardware_capable {
                    Ok(DialpadMode::Hardware)
                } else if self.is_virtual_capable {
                    Ok(DialpadMode::VirtualSoftware)
                } else {
                    Err(PlatformError::NotSupported)
                }
            }
            DialpadMode::Hardware => {
                if self.is_hardware_capable {
                    Ok(DialpadMode::Hardware)
                } else {
                    Err(PlatformError::NotSupported)
                }
            }
            DialpadMode::VirtualSoftware => {
                if self.is_virtual_capable {
                    Ok(DialpadMode::VirtualSoftware)
                } else {
                    Err(PlatformError::NotSupported)
                }
            }
        }
    }

    pub fn set_mode(&mut self, mode: DialpadMode) -> Result<()> {
        let supported = match mode {
            DialpadMode::Hardware => self.is_hardware_capable,
            DialpadMode::VirtualSoftware => self.is_virtual_capable,
            DialpadMode::Auto => self.is_hardware_capable || self.is_virtual_capable,
        };

        if !supported {
            return Err(PlatformError::NotSupported);
        }

        self.mode = mode;
        Ok(())
    }

    pub fn is_hardware_capable(&self) -> bool {
        self.is_hardware_capable
    }

    pub fn is_virtual_capable(&self) -> bool {
        self.is_virtual_capable
    }

    pub fn get_brightness(&mut self) -> Result<u8> {
        match self.active_mode()? {
            DialpadMode::VirtualSoftware => Ok(self.cached_brightness),
            DialpadMode::Hardware => {
                if let Some(ref path) = self.path {
                    let file = path.join("brightness");
                    let content = fs::read_to_string(&file)
                        .map_err(|e| PlatformError::IoPath(file.to_string_lossy().into(), e))?;
                    let val = content
                        .trim()
                        .parse::<u32>()
                        .map_err(|_| PlatformError::ParseNum)?;
                    let clamped = val.min(DEFAULT_MAX_BRIGHTNESS as u32) as u8;
                    self.cached_brightness = clamped;
                    Ok(clamped)
                } else {
                    Ok(self.cached_brightness)
                }
            }
            DialpadMode::Auto => Err(PlatformError::NotSupported),
        }
    }

    pub fn get_max_brightness(&self) -> Result<u8> {
        match self.active_mode()? {
            DialpadMode::VirtualSoftware => Ok(DEFAULT_MAX_BRIGHTNESS),
            DialpadMode::Hardware => {
                if let Some(ref path) = self.path {
                    let file = path.join("max_brightness");
                    if file.exists() {
                        let content = fs::read_to_string(&file)
                            .map_err(|e| PlatformError::IoPath(file.to_string_lossy().into(), e))?;
                        let val = content
                            .trim()
                            .parse::<u32>()
                            .map_err(|_| PlatformError::ParseNum)?;
                        Ok(val.min(DEFAULT_MAX_BRIGHTNESS as u32) as u8)
                    } else {
                        Ok(DEFAULT_MAX_BRIGHTNESS)
                    }
                } else {
                    Ok(DEFAULT_MAX_BRIGHTNESS)
                }
            }
            DialpadMode::Auto => Err(PlatformError::NotSupported),
        }
    }

    pub fn set_brightness(&mut self, val: u8) -> Result<()> {
        self.cached_brightness = val;
        match self.active_mode()? {
            DialpadMode::VirtualSoftware => Ok(()),
            DialpadMode::Hardware => {
                if let Some(ref path) = self.path {
                    let file = path.join("brightness");
                    fs::write(&file, val.to_string())
                        .map_err(|e| PlatformError::IoPath(file.to_string_lossy().into(), e))?;
                }
                Ok(())
            }
            DialpadMode::Auto => Err(PlatformError::NotSupported),
        }
    }

    /// Send WMI command `0x00100063` to toggle hardware DialPad ACPI state.
    /// Note: Some ASUS laptop models expose only LED control via sysfs; WMI ACPI toggle may legitimately fail or be unsupported.
    pub fn set_wmi_hardware_state(&self, enabled: bool) -> Result<()> {
        let val = if enabled { 1 } else { 0 };

        for dir in [
            "asus-nb-wmi", "asus-wmi",
        ] {
            let base = PathBuf::from("/sys/kernel/debug").join(dir);
            let dev_id = base.join("dev_id");
            let ctrl_param = base.join("ctrl_param");
            let devs = base.join("devs");

            if dev_id.exists() && ctrl_param.exists() && devs.exists() {
                debug!("Sending WMI command via debugfs DEVS interface at {base:?}");
                fs::write(&dev_id, format!("{ASUS_WMI_DEVID_DIALPAD:#x}"))
                    .map_err(|e| PlatformError::IoPath(dev_id.to_string_lossy().into(), e))?;
                fs::write(&ctrl_param, format!("{val:#x}"))
                    .map_err(|e| PlatformError::IoPath(ctrl_param.to_string_lossy().into(), e))?;
                // Reading `devs` triggers the DEVS WMI call in kernel driver
                fs::read_to_string(&devs)
                    .map_err(|e| PlatformError::IoPath(devs.to_string_lossy().into(), e))?;
                return Ok(());
            }
        }

        if let Some(ref wmi_path) = self.wmi_dev_id_path {
            let cmd = format!("{ASUS_WMI_DEVID_DIALPAD:#x} {val}");
            debug!("Sending WMI command to {wmi_path:?}: {cmd}");
            fs::write(wmi_path, &cmd).map_err(|e| {
                warn!("WMI DialPad toggle write failed: {e}");
                PlatformError::IoPath(wmi_path.to_string_lossy().into(), e)
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_dialpad_mode_parsing() {
        assert_eq!(
            DialpadMode::from_str("hardware").unwrap(),
            DialpadMode::Hardware
        );
        assert_eq!(DialpadMode::from_str("hw").unwrap(), DialpadMode::Hardware);
        assert_eq!(
            DialpadMode::from_str("virtual").unwrap(),
            DialpadMode::VirtualSoftware
        );
        assert_eq!(
            DialpadMode::from_str("sw").unwrap(),
            DialpadMode::VirtualSoftware
        );
        assert_eq!(DialpadMode::from_str("auto").unwrap(), DialpadMode::Auto);
        assert!(DialpadMode::from_str("invalid").is_err());
    }

    #[test]
    fn test_dialpad_mode_display() {
        assert_eq!(DialpadMode::Hardware.to_string(), "hardware");
        assert_eq!(DialpadMode::VirtualSoftware.to_string(), "virtual");
        assert_eq!(DialpadMode::Auto.to_string(), "auto");
    }

    #[test]
    fn test_dialpad_active_mode_resolution() {
        let dialpad = Dialpad {
            path: None,
            wmi_dev_id_path: None,
            mode: DialpadMode::Auto,
            is_hardware_capable: true,
            is_virtual_capable: true,
            cached_brightness: 255,
        };

        assert_eq!(dialpad.mode(), DialpadMode::Auto);
        assert_eq!(dialpad.active_mode().unwrap(), DialpadMode::Hardware);

        let virtual_only = Dialpad {
            is_hardware_capable: false,
            ..dialpad.clone()
        };
        assert_eq!(
            virtual_only.active_mode().unwrap(),
            DialpadMode::VirtualSoftware
        );
        assert!(virtual_only
            .clone()
            .set_mode(DialpadMode::Hardware)
            .is_err());

        let none_capable = Dialpad {
            is_virtual_capable: false,
            ..virtual_only
        };
        assert!(none_capable.active_mode().is_err());
    }

    #[test]
    fn test_calculate_rotation_angle() {
        // Center (0,0), point 1 at (1, 0) [angle 0], point 2 at (0, 1) [angle pi/2]
        let delta = Dialpad::calculate_rotation_angle(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        assert!((delta - std::f64::consts::FRAC_PI_2).abs() < 1e-6);
    }
}
