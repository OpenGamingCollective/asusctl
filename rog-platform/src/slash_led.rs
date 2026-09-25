use std::path::{Path, PathBuf};

use log::{info, warn};

use crate::error::{PlatformError, Result};
use crate::{attr_num, attr_string, to_device};

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Clone)]
pub struct SlashLed {
    path: PathBuf,
}

impl SlashLed {
    attr_num!("brightness", path, u8);
    attr_num!("max_brightness", path, u8);

    attr_string!("slash_mode", path);
    attr_string!("slash_mode_index", path);
    attr_num!("slash_interval", path, u8);

    pub fn new() -> Result<Self> {
        let std_path = Path::new("/sys/class/leds/asus::slash");
        if std_path.exists() {
            info!("Found Slash LED at {:?}", std_path);
            return Ok(Self {
                path: std_path.to_owned(),
            });
        }

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
            let sys = device.sysname().to_string_lossy();
            if sys.contains("slash") {
                info!("Found Slash LED controls at {:?}", device.sysname());
                return Ok(Self {
                    path: device.syspath().to_owned(),
                });
            }
        }

        Err(PlatformError::MissingFunction(
            "SlashLed:new(), asus::slash not found".into(),
        ))
    }

    pub fn is_available() -> bool {
        Self::new().is_ok()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
