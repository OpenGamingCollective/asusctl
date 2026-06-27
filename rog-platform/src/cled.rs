use std::path::PathBuf;

use log::{info, warn};

use crate::error::{PlatformError, Result};
use crate::{attr_num, to_device};

/// A generic class LED device from `/sys/class/leds/`.
///
/// This wraps a sysfs led device and provides read/write access to its
/// `brightness` and `max_brightness` attributes. It is the same abstraction
/// used by `Backlight` and `KeyboardBacklight`, but generalised to any LED.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct CledDevice {
    path: PathBuf,
}

impl CledDevice {
    attr_num!("brightness", path, u8);
    attr_num!("max_brightness", path, u8);

    /// Create a new `CledDevice` by exact match on the sysfs name.
    ///
    /// `name` is matched against `device.sysname()` (e.g. `"asus:xgm-..."`).
    pub fn new(name: &str) -> Result<Self> {
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("leds").map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for device in enumerator.scan_devices().map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("scan_devices failed".into(), err)
        })? {
            let sysname = device.sysname().to_string_lossy();
            if sysname == name {
                info!("Found class LED device at {sysname:?}");
                return Ok(Self {
                    path: device.syspath().to_path_buf(),
                });
            }
        }

        Err(PlatformError::MissingFunction(format!(
            "CledDevice::new(): no LED named '{name}' found"
        )))
    }

    /// Create a new `CledDevice` by substring match on the sysfs name.
    ///
    /// Useful when the exact name varies (e.g. includes a USB path suffix).
    /// Matches any device whose sysname **contains** `pattern`.
    pub fn new_from_pattern(pattern: &str) -> Result<Self> {
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("leds").map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for device in enumerator.scan_devices().map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("scan_devices failed".into(), err)
        })? {
            let sysname = device.sysname().to_string_lossy();
            if sysname.contains(pattern) {
                info!("Found class LED device matching '{pattern}' at {sysname:?}");
                return Ok(Self {
                    path: device.syspath().to_path_buf(),
                });
            }
        }

        Err(PlatformError::MissingFunction(format!(
            "CledDevice::new_from_pattern(): no LED containing '{pattern}' found"
        )))
    }
}
