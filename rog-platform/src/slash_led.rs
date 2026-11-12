use std::path::{Path, PathBuf};

use log::{info, warn};

use crate::dynamic_led::DynamicLed;
use crate::error::{PlatformError, Result};
use crate::{attr_num, attr_string, to_device};

/// Slash chassis LED (`asus::slash`).
///
/// Supports both the legacy custom attributes (`slash_mode` /
/// `slash_interval`) and a full Dynamic Lighting node (`effect` / `speed` /
/// optional `power_states`). Detection prefers Dynamic Lighting when the
/// generic attrs validate.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct SlashLed {
    path: PathBuf,
    dynamic: Option<DynamicLed>,
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
            // Exact chassis node only — do not bind peripheral LEDs whose
            // name merely contains "slash".
            if sys == "asus::slash" {
                return Self::from_path(device.syspath().to_owned());
            }
        }

        Err(PlatformError::MissingFunction(
            "SlashLed:new(), asus::slash not found".into(),
        ))
    }

    fn from_path(path: PathBuf) -> Result<Self> {
        let dynamic = DynamicLed::from_sysfs_path(path.clone()).ok();
        if dynamic.is_some() {
            info!("Found Slash Dynamic Lighting LED at {:?}", path);
        } else {
            info!("Found Slash LED (legacy attrs) at {:?}", path);
        }
        Ok(Self { path, dynamic })
    }

    pub fn is_available() -> bool {
        Self::new().is_ok()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// True when the node speaks the generic Dynamic Lighting ABI.
    pub fn is_dynamic(&self) -> bool {
        self.dynamic.is_some()
    }

    pub fn has_power_states(&self) -> bool {
        self.dynamic
            .as_ref()
            .is_some_and(|led| led.has_power_states())
    }

    pub fn get_power_states(&self) -> Result<String> {
        let led = self
            .dynamic
            .as_ref()
            .ok_or_else(|| PlatformError::AttrNotFound("power_states".into()))?;
        led.get_power_states()
    }

    pub fn set_power_states_list(&self, states: &[&str]) -> Result<()> {
        let led = self
            .dynamic
            .as_ref()
            .ok_or_else(|| PlatformError::AttrNotFound("power_states".into()))?;
        led.set_power_states_list(states)
    }

    /// Select Slash animation mode (`effect` under DL, else `slash_mode`).
    pub fn set_mode(&self, mode: &str) -> Result<()> {
        if let Some(led) = &self.dynamic {
            return led.set_effect(mode);
        }
        self.set_slash_mode(mode)
    }

    /// Read current mode name.
    pub fn get_mode(&self) -> Result<String> {
        if let Some(led) = &self.dynamic {
            return led.get_effect();
        }
        self.get_slash_mode()
    }

    /// Space-separated mode index.
    pub fn get_mode_index(&self) -> Result<String> {
        if let Some(led) = &self.dynamic {
            return led.get_effect_index();
        }
        self.get_slash_mode_index()
    }

    /// Animation interval / speed.
    pub fn set_interval(&self, interval: u8) -> Result<()> {
        if let Some(led) = &self.dynamic {
            if led.has_speed() {
                return led.set_supported_speed(u32::from(interval));
            }
            if self.has_slash_interval() {
                return self.set_slash_interval(interval);
            }
            return Err(PlatformError::AttrNotFound("speed".into()));
        }
        self.set_slash_interval(interval)
    }

    pub fn get_interval(&self) -> Result<u8> {
        if let Some(led) = &self.dynamic {
            if led.has_speed() {
                return Ok(led.get_speed()? as u8);
            }
            if self.has_slash_interval() {
                return self.get_slash_interval();
            }
            return Err(PlatformError::AttrNotFound("speed".into()));
        }
        self.get_slash_interval()
    }
}
