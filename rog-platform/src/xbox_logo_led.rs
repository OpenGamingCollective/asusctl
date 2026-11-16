use std::path::{Path, PathBuf};

use log::{info, warn};

use crate::dynamic_led::DynamicLed;
use crate::error::{PlatformError, Result};
use crate::{attr_num, to_device};

/// Xbox Guide LED on Xbox Ally X (`asus::xbox_logo`, brightness 0–3).
///
/// Prefers a Dynamic Lighting node when `effect`/`effect_index` validate;
/// otherwise uses brightness-only class LED attributes.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct XboxLogoLed {
    path: PathBuf,
    dynamic: Option<DynamicLed>,
}

impl XboxLogoLed {
    attr_num!("brightness", path, u8);
    attr_num!("max_brightness", path, u8);

    pub fn new() -> Result<Self> {
        let std_path = Path::new("/sys/class/leds/asus::xbox_logo");
        if std_path.exists() {
            return Self::from_path(std_path.to_owned());
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
            if sys == "asus::xbox_logo" || sys.contains("xbox_logo") {
                return Self::from_path(device.syspath().to_owned());
            }
        }

        Err(PlatformError::MissingFunction(
            "XboxLogoLed:new(), asus::xbox_logo not found".into(),
        ))
    }

    fn from_path(path: PathBuf) -> Result<Self> {
        let dynamic = DynamicLed::from_sysfs_path(path.clone()).ok();
        if dynamic.is_some() {
            info!("Found Xbox Guide Dynamic Lighting LED at {:?}", path);
        } else {
            info!("Found Xbox Guide LED at {:?}", path);
        }
        Ok(Self { path, dynamic })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_dynamic(&self) -> bool {
        self.dynamic.is_some()
    }

    pub fn set_effect(&self, effect: &str) -> Result<()> {
        let led = self
            .dynamic
            .as_ref()
            .ok_or_else(|| PlatformError::AttrNotFound("effect".into()))?;
        led.set_effect(effect)
    }

    pub fn get_effect(&self) -> Result<String> {
        let led = self
            .dynamic
            .as_ref()
            .ok_or_else(|| PlatformError::AttrNotFound("effect".into()))?;
        led.get_effect()
    }

    pub fn set_speed(&self, speed: u32) -> Result<()> {
        let led = self
            .dynamic
            .as_ref()
            .ok_or_else(|| PlatformError::AttrNotFound("speed".into()))?;
        led.set_supported_speed(speed)
    }
}
