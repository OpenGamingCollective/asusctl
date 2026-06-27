use std::path::PathBuf;

use log::{info, warn};

use crate::error::{PlatformError, Result};
use crate::{attr_num, to_device};

/// The "backlight" device provides access to screen brightness control
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct Backlight {
    path: PathBuf,
    device_type: BacklightType,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub enum BacklightType {
    Primary,
    Screenpad,
}

impl Backlight {
    attr_num!("brightness", path, i32);

    attr_num!("max_brightness", path, i32);

    attr_num!("bl_power", path, i32);

    pub fn new(device_type: BacklightType) -> Result<Self> {
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("backlight").map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for device in enumerator.scan_devices().map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("scan_devices failed".into(), err)
        })? {
            info!("Backlight: Checking {:?}", device.syspath());
            match device_type {
                BacklightType::Primary => {
                    if device.sysname().to_string_lossy() == "intel_backlight" {
                        info!("Found primary backlight at {:?}", device.sysname());
                        return Ok(Self {
                            path: device.syspath().to_path_buf(),
                            device_type,
                        });
                    }
                }
                BacklightType::Screenpad => {
                    let name = device.sysname().to_string_lossy();
                    if name == "asus_screenpad" || name == "asus_screenpad_backlight" {
                        info!("Found screenpad backlight at {:?}", device.sysname());
                        return Ok(Self {
                            path: device.syspath().to_path_buf(),
                            device_type,
                        });
                    }
                }
            }
        }

        Err(PlatformError::MissingFunction(format!(
            "Backlight {device_type:?} not found"
        )))
    }

    pub fn device_type(&self) -> &BacklightType {
        &self.device_type
    }
}
