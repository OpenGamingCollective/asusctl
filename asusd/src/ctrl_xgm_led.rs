use std::sync::Arc;

use config_traits::StdConfig;
use log::{error, info, warn};
use rog_platform::cled::CledDevice;
use tokio::sync::Mutex;
use zbus::fdo::Error as FdoErr;
use zbus::{Connection, interface};

use crate::ASUS_ZBUS_PATH;
use crate::config::Config;
use crate::error::RogError;

/// Controller for the XG Mobile external GPU LED.
///
/// The XG Mobile exposes a class LED device in `/sys/class/leds/` with the
/// name pattern `asus:xgm-*` and `max_brightness=1`. This controller provides
/// a simple on/off toggle on dbus and persists the last state so it can be
/// re-applied when the device is reconnected.
#[derive(Clone)]
pub struct CtrlXgmLed {
    cled: CledDevice,
    config: Arc<Mutex<Config>>,
}

impl CtrlXgmLed {
    /// Try to find the XG Mobile LED device.
    ///
    /// Returns `Ok(None)` (not an error) when the device is not connected, so
    /// the daemon can skip registration gracefully.
    pub fn try_new(config: Arc<Mutex<Config>>) -> Result<Option<Self>, RogError> {
        match CledDevice::new_from_pattern("asus:xgm") {
            Ok(cled) => {
                info!("Found XG Mobile LED device");

                // Restore last user-set state (if any) so a bright LED is
                // silenced immediately on plug-in.
                let desired = config.try_lock().ok().and_then(|c| c.xgm_led_enabled);
                if let Some(enabled) = desired {
                    let value: u8 = enabled.into();
                    info!("Restoring XG Mobile LED to {enabled}");
                    if let Err(e) = cled.set_brightness(value) {
                        warn!("Failed to restore XG Mobile LED brightness: {e}");
                    }
                }

                Ok(Some(Self { cled, config }))
            }
            Err(e) => {
                info!("XG Mobile LED not found: {e}");
                Ok(None)
            }
        }
    }

    /// Return the current brightness state of the LED.
    fn get_led_enabled(&self) -> bool {
        self.cled.get_brightness().map(|v| v > 0).unwrap_or(false)
    }

    /// Set the LED brightness and persist to config.
    fn set_led_enabled_inner(&self, enabled: bool) -> Result<(), FdoErr> {
        let value: u8 = enabled.into();
        self.cled.set_brightness(value).map_err(|e| {
            warn!("Failed to set XG Mobile LED: {e}");
            FdoErr::Failed(format!("Failed to set XG Mobile LED: {e}"))
        })?;
        // Persist for re-apply on reconnect or daemon restart
        if let Ok(mut config) = self.config.try_lock() {
            config.xgm_led_enabled = Some(enabled);
            config.write();
        }
        Ok(())
    }
}

#[interface(name = "xyz.ljones.XgmLed")]
impl CtrlXgmLed {
    /// Whether the XG Mobile LED is enabled (on).
    #[zbus(property)]
    async fn xgm_led_enabled(&self) -> Result<bool, FdoErr> {
        Ok(self.get_led_enabled())
    }

    /// Enable or disable the XG Mobile LED.
    #[zbus(property)]
    async fn set_xgm_led_enabled(&self, enabled: bool) -> Result<(), zbus::Error> {
        self.set_led_enabled_inner(enabled).map_err(Into::into)
    }
}

impl crate::ZbusRun for CtrlXgmLed {
    async fn add_to_server(self, server: &mut Connection) {
        Self::add_to_server_helper(self, ASUS_ZBUS_PATH, server).await;
    }
}

impl crate::Reloadable for CtrlXgmLed {
    async fn reload(&mut self) -> Result<(), RogError> {
        info!("Reloading XG Mobile LED settings");
        // Re-apply persisted state on reload
        let enabled = self.config.lock().await.xgm_led_enabled.unwrap_or(false);
        let value: u8 = enabled.into();
        if let Err(e) = self.cled.set_brightness(value) {
            error!("Failed to restore XG Mobile LED on reload: {e}");
        }
        Ok(())
    }
}
