use std::sync::Arc;

use config_traits::StdConfig;
use log::{error, info, warn};
use rog_platform::dialpad::Dialpad;
use tokio::sync::Mutex;
use zbus::fdo::Error as FdoErr;
use zbus::{interface, Connection};

use crate::config::Config;
use crate::error::RogError;

pub const DIALPAD_ZBUS_PATH: &str = "/xyz/ljones/Dialpad";

/// Controller for the ASUS Touchpad DialPad LED and hardware state.
#[derive(Clone)]
pub struct CtrlDialpad {
    dialpad: Dialpad,
    config: Arc<Mutex<Config>>,
}

impl CtrlDialpad {
    pub async fn try_new(config: Arc<Mutex<Config>>) -> Result<Option<Self>, RogError> {
        match Dialpad::new() {
            Ok(dialpad) => {
                info!("Found ASUS DialPad hardware device");
                let ctrl = Self { dialpad, config };

                let c = ctrl.config.lock().await;
                if let Err(e) = ctrl.apply_saved_state(&c) {
                    error!("Failed to apply saved DialPad state on startup: {e}");
                }
                drop(c);

                Ok(Some(ctrl))
            }
            Err(e) => {
                info!("ASUS DialPad device not found: {e}");
                Ok(None)
            }
        }
    }

    /// Helper to compute the target brightness based on saved config and max hardware brightness.
    fn desired_brightness(&self, config: &Config) -> u8 {
        let max_b = self.dialpad.get_max_brightness().unwrap_or(255);
        config.dialpad_brightness.unwrap_or(max_b).min(max_b)
    }

    /// Helper to apply saved config state to hardware (used by try_new and reload).
    ///
    /// LED brightness is the primary control mechanism. WMI ACPI state toggle is attempted,
    /// but WMI write failures are treated as non-fatal warnings since not all hardware revisions
    /// require or support WMI signaling.
    fn apply_saved_state(&self, config: &Config) -> Result<(), RogError> {
        let enabled = config.dialpad_enabled.unwrap_or(true);
        let target_brightness = if enabled {
            self.desired_brightness(config)
        } else {
            0
        };

        self.dialpad
            .set_brightness(target_brightness)
            .map_err(RogError::Platform)?;

        if let Err(e) = self.dialpad.set_wmi_hardware_state(enabled) {
            warn!("WMI DialPad hardware ACPI toggle failed (non-fatal): {e}");
        }
        Ok(())
    }

    /// Check if the DialPad is currently enabled (inferred via LED brightness > 0).
    fn is_enabled(&self) -> bool {
        self.dialpad
            .get_brightness()
            .map(|v| v > 0)
            .unwrap_or(false)
    }

    async fn set_enabled_inner(&self, enabled: bool) -> Result<(), FdoErr> {
        let mut config = self.config.lock().await;
        config.dialpad_enabled = Some(enabled);

        let brightness_to_set = if enabled {
            self.desired_brightness(&config)
        } else {
            0
        };

        self.dialpad
            .set_brightness(brightness_to_set)
            .map_err(|e| {
                warn!("Failed to set DialPad brightness: {e}");
                FdoErr::Failed(format!("Failed to set DialPad brightness: {e}"))
            })?;

        if let Err(e) = self.dialpad.set_wmi_hardware_state(enabled) {
            warn!("WMI DialPad hardware ACPI toggle failed (non-fatal): {e}");
        }

        config.write();
        Ok(())
    }

    fn get_brightness_inner(&self) -> u8 {
        self.dialpad.get_brightness().unwrap_or(0)
    }

    async fn set_brightness_inner(&self, value: u8) -> Result<(), FdoErr> {
        let max_b = self.dialpad.get_max_brightness().unwrap_or(255);
        let clamped_value = value.min(max_b);

        self.dialpad.set_brightness(clamped_value).map_err(|e| {
            warn!("Failed to set DialPad brightness: {e}");
            FdoErr::Failed(format!("Failed to set DialPad brightness: {e}"))
        })?;

        let enabled = clamped_value > 0;
        if let Err(e) = self.dialpad.set_wmi_hardware_state(enabled) {
            warn!("WMI DialPad hardware ACPI toggle failed (non-fatal): {e}");
        }

        let mut config = self.config.lock().await;
        config.dialpad_brightness = Some(clamped_value);
        config.dialpad_enabled = Some(enabled);
        config.write();
        Ok(())
    }
}

#[interface(name = "xyz.ljones.Dialpad")]
impl CtrlDialpad {
    #[zbus(property)]
    async fn enabled(&self) -> Result<bool, FdoErr> {
        Ok(self.is_enabled())
    }

    #[zbus(property)]
    async fn set_enabled(&self, enabled: bool) -> Result<(), zbus::Error> {
        self.set_enabled_inner(enabled).await.map_err(Into::into)
    }

    #[zbus(property)]
    async fn brightness(&self) -> Result<u8, FdoErr> {
        Ok(self.get_brightness_inner())
    }

    #[zbus(property)]
    async fn set_brightness(&self, value: u8) -> Result<(), zbus::Error> {
        self.set_brightness_inner(value).await.map_err(Into::into)
    }

    #[zbus(property)]
    async fn supported(&self) -> Result<bool, FdoErr> {
        Ok(true)
    }
}

impl crate::ZbusRun for CtrlDialpad {
    async fn add_to_server(self, server: &mut Connection) {
        Self::add_to_server_helper(self, DIALPAD_ZBUS_PATH, server).await;
    }
}

impl crate::Reloadable for CtrlDialpad {
    async fn reload(&mut self) -> Result<(), RogError> {
        info!("Reloading DialPad settings");
        let lock = self.config.lock().await;
        self.apply_saved_state(&lock)
    }
}
