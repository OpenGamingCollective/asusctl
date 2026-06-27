use std::path::PathBuf;

use log::{info, warn};

use crate::error::{PlatformError, Result};
use crate::{attr_num, to_device};

/// The "platform" device provides access to things like:
/// - `dgpu_disable`
/// - `egpu_enable`
/// - `panel_od`
/// - `gpu_mux`
/// - `keyboard_mode`, set keyboard RGB mode and speed
/// - `keyboard_state`, set keyboard power states
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct AsusPower {
    mains: PathBuf,
    battery: PathBuf,
    usb: Option<PathBuf>,
}

impl AsusPower {
    attr_num!("charge_control_end_threshold", battery, u8);

    attr_num!("online", mains, u8);

    /// When checking for battery this will look in order:
    /// - if attr `manufacturer` contains `asus`
    /// - if attr `charge_control_end_threshold` exists and `energy_full_design`
    ///   >= 50 watt
    /// - if syspath end conatins `BAT`
    /// - if attr `type` is `battery` (last resort)
    pub fn new() -> Result<Self> {
        let mut mains = PathBuf::new();
        let mut battery = None;
        let mut usb = None;

        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("power_supply").map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for device in enumerator.scan_devices().map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("scan_devices failed".into(), err)
        })? {
            if let Some(attr) = device.attribute_value("type") {
                info!("Power: Checking {:?}", device.syspath());
                match attr.to_string_lossy().to_ascii_lowercase().trim() {
                    "mains" => {
                        info!("Found mains power at {:?}", device.sysname());
                        mains = device.syspath().to_path_buf();
                    }
                    "battery" => {
                        // Priortised list of checks
                        info!("Found a battery");
                        if battery.is_none() {
                            info!("Checking battery attributes");
                            if let Some(current) =
                                device.attribute_value("charge_control_end_threshold")
                            {
                                info!(
                                    "Found battery power at {:?}, matched \
                                     charge_control_end_threshold. Current level: {current:?}",
                                    device.sysname()
                                );
                                battery = Some(device.syspath().to_path_buf());
                            } else if device.sysname().to_string_lossy().starts_with("BAT") {
                                info!(
                                    "Found battery power at {:?}, sysfs path ended with BAT<n>",
                                    device.sysname()
                                );
                                battery = Some(device.syspath().to_path_buf());
                            } else {
                                info!(
                                    "Last resort: Found battery power at {:?} using type = Battery",
                                    device.sysname()
                                );
                                battery = Some(device.syspath().to_path_buf());
                            }
                        }
                    }
                    "usb" => {
                        info!("Found USB-C power at {:?}", device.sysname());
                        usb = Some(device.syspath().to_path_buf());
                    }
                    _ => {}
                };
            }
        }

        if let Some(battery) = battery {
            return Ok(Self {
                mains,
                battery,
                usb,
            });
        }

        // No battery found. Return an AsusPower with an empty battery path so
        // callers can still be constructed and query `has_*` methods which
        // will correctly report absence. This avoids hard-failing on systems
        // where the asus-nb-wmi driver loads on desktops with no battery.
        info!("Did not find a battery, continuing without battery support");
        Ok(Self {
            mains,
            battery: PathBuf::new(),
            usb,
        })
    }
}
