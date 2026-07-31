use std::fs;
use std::path::PathBuf;

use log::{info, warn};

use crate::error::{PlatformError, Result};
use crate::{attr_num, to_device};

/// ASUS WMI Device ID for DialPad hardware toggle (`IIA0 == 0x00100063`)
pub const ASUS_WMI_DEVID_DIALPAD: u32 = 0x00100063;

/// The Dialpad device provides access to ASUS DialPad backlight and hardware status.
///
/// Note: The hardware enabled state is inferred via `brightness > 0` as a heuristic,
/// supplemented by optional WMI ACPI state toggle calls (`0x00100063`).
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct Dialpad {
    path: PathBuf,
    wmi_dev_id_path: Option<PathBuf>,
}

impl Dialpad {
    attr_num!("brightness", path, u8);
    attr_num!("max_brightness", path, u8);

    pub fn new() -> Result<Self> {
        let wmi_path = PathBuf::from("/sys/devices/platform/asus-wmi/dev_id");
        let wmi_dev_id_path = if wmi_path.exists() {
            Some(wmi_path)
        } else {
            None
        };

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
            if name == "asus::dialpad" || name == "asus_dialpad" || name.contains("dialpad") {
                info!("Found DialPad LED device at {:?}", device.syspath());
                return Ok(Self {
                    path: device.syspath().to_path_buf(),
                    wmi_dev_id_path,
                });
            }
        }

        let fallback_path = PathBuf::from("/sys/class/leds/asus::dialpad");
        if fallback_path.exists() {
            info!("Found DialPad LED at fallback path {:?}", fallback_path);
            return Ok(Self {
                path: fallback_path,
                wmi_dev_id_path,
            });
        }

        Err(PlatformError::MissingFunction(
            "DialPad LED device not found".into(),
        ))
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Send WMI command `0x00100063` to toggle hardware DialPad ACPI state.
    /// Returns an error if the WMI write operation fails.
    pub fn set_wmi_hardware_state(&self, enabled: bool) -> Result<()> {
        if let Some(ref wmi_path) = self.wmi_dev_id_path {
            let val = if enabled { 1 } else { 0 };
            let cmd = format!("{ASUS_WMI_DEVID_DIALPAD:#x} {val}");
            info!("Sending WMI command to {wmi_path:?}: {cmd}");
            fs::write(wmi_path, &cmd).map_err(|e| {
                warn!("WMI DialPad toggle write failed: {e}");
                PlatformError::IoPath(wmi_path.to_string_lossy().into(), e)
            })?;
        }
        Ok(())
    }
}
