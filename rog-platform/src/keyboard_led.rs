use std::path::PathBuf;

use log::{info, warn};

use crate::error::{PlatformError, Result};
use crate::{attr_num, has_attr, set_attr_u8_array, to_device};

/// The sysfs control for backlight levels. This is only for the 3-step
/// backlight setting, and for TUF laptops. It is not a hard requirement
/// for Aura keyboards
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Clone)]
pub struct KeyboardBacklight {
    path: PathBuf,
}

impl KeyboardBacklight {
    attr_num!("brightness", path, u8);

    has_attr!("kbd_rgb_mode" path);

    set_attr_u8_array!(
        /// kbd_rgb_mode can only be set, not read back
        "kbd_rgb_mode"
        path
    );

    has_attr!("kbd_rgb_state" path);

    set_attr_u8_array!(
        /// kbd_rgb_state can only be set, not read back
        "kbd_rgb_state"
        path
    );

    pub fn new() -> Result<Self> {
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
            let sys = device.sysname().to_string_lossy();
            if sys.contains("kbd_backlight") || sys.contains("ally:rgb:gamepad") {
                info!("Found keyboard LED controls at {:?}", device.sysname());
                return Ok(Self {
                    path: device.syspath().to_owned(),
                });
            }
        }
        Err(PlatformError::MissingFunction(
            "KeyboardLed:new(), asus::kbd_backlight not found".into(),
        ))
    }
}
