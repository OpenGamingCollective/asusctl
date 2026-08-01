use std::str::FromStr;
use std::sync::Arc;

use config_traits::StdConfig;
use log::{error, info, warn};
use rog_platform::dialpad::{Dialpad, DialpadMode, DEFAULT_MAX_BRIGHTNESS};
use tokio::sync::Mutex;
use zbus::fdo::Error as FdoErr;
use zbus::object_server::SignalEmitter;
use zbus::{interface, Connection};

use crate::config::Config;
use crate::error::RogError;

pub const DIALPAD_ZBUS_PATH: &str = "/xyz/ljones/Dialpad";

/// Controller for the ASUS Touchpad DialPad LED and hardware/software state.
#[derive(Clone)]
pub struct CtrlDialpad {
    dialpad: Arc<Mutex<Dialpad>>,
    config: Arc<Mutex<Config>>,
}

/// Helper to calculate target brightness from daemon config and maximum allowed hardware brightness.
fn compute_target_brightness(config: &Config, max_b: u8) -> u8 {
    let enabled = config.dialpad_enabled.unwrap_or(true);
    let last_nonzero = config
        .dialpad_brightness
        .filter(|&b| b > 0)
        .unwrap_or(max_b);
    if enabled {
        last_nonzero
    } else {
        0
    }
}

impl CtrlDialpad {
    pub async fn try_new(config: Arc<Mutex<Config>>) -> Result<Option<Self>, RogError> {
        match Dialpad::new() {
            Ok(dialpad) => {
                info!("Found ASUS DialPad controller capability");
                let ctrl = Self {
                    dialpad: Arc::new(Mutex::new(dialpad)),
                    config,
                };

                if let Err(e) = ctrl.apply_saved_state_internal().await {
                    error!("Failed to apply saved DialPad state on startup: {e}");
                }

                Ok(Some(ctrl))
            }
            Err(e) => {
                info!("ASUS DialPad device not found: {e}");
                Ok(None)
            }
        }
    }

    /// Internal helper enforcing strict lock ordering: dialpad lock FIRST, config lock SECOND.
    async fn apply_saved_state_internal(&self) -> Result<(), RogError> {
        let mut dialpad = self.dialpad.lock().await;
        let config = self.config.lock().await;

        // Restore mode
        if let Some(mode) = config.dialpad_mode {
            if let Err(e) = dialpad.set_mode(mode) {
                warn!("Failed to apply saved DialPad mode '{mode}': {e}");
            }
        }

        let enabled = config.dialpad_enabled.unwrap_or(true);
        let max_b = dialpad
            .get_max_brightness()
            .unwrap_or(DEFAULT_MAX_BRIGHTNESS);
        let target_brightness = compute_target_brightness(&config, max_b);

        // 1. WMI ACPI Enable FIRST (Some ASUS models expose only sysfs LED control; WMI toggle may legitimately fail or be unsupported)
        if let Err(e) = dialpad.set_wmi_hardware_state(enabled) {
            warn!("WMI DialPad hardware ACPI toggle failed (non-fatal): {e}");
        }

        // 2. Brightness SECOND
        dialpad
            .set_brightness(target_brightness)
            .map_err(RogError::Platform)?;

        Ok(())
    }

    /// Check if the DialPad is explicitly enabled via daemon configuration.
    async fn is_enabled(&self) -> bool {
        self.config.lock().await.dialpad_enabled.unwrap_or(true)
    }

    /// Lock ordering maintained: dialpad lock FIRST, config lock SECOND. Standard asusd config write under lock.
    async fn set_enabled_inner(&self, enabled: bool) -> Result<(), FdoErr> {
        let mut dialpad = self.dialpad.lock().await;
        let mut config = self.config.lock().await;

        let max_b = dialpad
            .get_max_brightness()
            .unwrap_or(DEFAULT_MAX_BRIGHTNESS);
        let brightness_to_set = if enabled {
            config
                .dialpad_brightness
                .filter(|&b| b > 0)
                .unwrap_or(max_b)
        } else {
            0
        };

        if let Err(e) = dialpad.set_wmi_hardware_state(enabled) {
            warn!("WMI DialPad hardware ACPI toggle failed (non-fatal): {e}");
        }

        dialpad.set_brightness(brightness_to_set).map_err(|e| {
            warn!("Failed to set DialPad brightness: {e}");
            FdoErr::Failed(format!("Failed to set DialPad brightness: {e}"))
        })?;

        config.dialpad_enabled = Some(enabled);
        config.write();
        Ok(())
    }

    async fn get_brightness_inner(&self) -> Result<u8, FdoErr> {
        self.dialpad
            .lock()
            .await
            .get_brightness()
            .map_err(|e| FdoErr::Failed(format!("Failed to read DialPad brightness: {e}")))
    }

    /// Lock ordering maintained: dialpad lock FIRST, config lock SECOND. Standard asusd config write under lock.
    async fn set_brightness_inner(&self, value: u8) -> Result<(), FdoErr> {
        let mut dialpad = self.dialpad.lock().await;
        let max_b = dialpad
            .get_max_brightness()
            .unwrap_or(DEFAULT_MAX_BRIGHTNESS);
        let clamped_value = value.min(max_b);

        let enabled = clamped_value > 0;
        if let Err(e) = dialpad.set_wmi_hardware_state(enabled) {
            warn!("WMI DialPad hardware ACPI toggle failed (non-fatal): {e}");
        }

        dialpad.set_brightness(clamped_value).map_err(|e| {
            warn!("Failed to set DialPad brightness: {e}");
            FdoErr::Failed(format!("Failed to set DialPad brightness: {e}"))
        })?;

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
        Ok(self.is_enabled().await)
    }

    #[zbus(property)]
    async fn set_enabled(
        &self,
        enabled: bool,
        #[zbus(signal_context)] ctxt: SignalEmitter<'_>,
    ) -> Result<(), zbus::Error> {
        self.set_enabled_inner(enabled).await?;
        let _ = self.enabled_changed(&ctxt).await;
        let _ = self.brightness_changed(&ctxt).await;
        Ok(())
    }

    #[zbus(property)]
    async fn brightness(&self) -> Result<u8, FdoErr> {
        self.get_brightness_inner().await
    }

    #[zbus(property)]
    async fn set_brightness(
        &self,
        value: u8,
        #[zbus(signal_context)] ctxt: SignalEmitter<'_>,
    ) -> Result<(), zbus::Error> {
        self.set_brightness_inner(value).await?;
        let _ = self.brightness_changed(&ctxt).await;
        let _ = self.enabled_changed(&ctxt).await;
        Ok(())
    }

    #[zbus(property)]
    async fn mode(&self) -> Result<String, FdoErr> {
        Ok(self.dialpad.lock().await.mode().to_string())
    }

    #[zbus(property)]
    async fn set_mode(
        &self,
        mode_str: String,
        #[zbus(signal_context)] ctxt: SignalEmitter<'_>,
    ) -> Result<(), zbus::Error> {
        let mode = DialpadMode::from_str(&mode_str)
            .map_err(|e| FdoErr::Failed(format!("Invalid mode: {e}")))?;

        let mut dialpad = self.dialpad.lock().await;
        dialpad
            .set_mode(mode)
            .map_err(|e| FdoErr::Failed(e.to_string()))?;

        let mut config = self.config.lock().await;
        let max_b = dialpad
            .get_max_brightness()
            .unwrap_or(DEFAULT_MAX_BRIGHTNESS);
        let target_brightness = compute_target_brightness(&config, max_b);
        let enabled = config.dialpad_enabled.unwrap_or(true);

        if let Err(e) = dialpad.set_wmi_hardware_state(enabled) {
            warn!("WMI DialPad hardware ACPI toggle failed in set_mode: {e}");
        }
        if let Err(e) = dialpad.set_brightness(target_brightness) {
            warn!("DialPad set_brightness failed in set_mode: {e}");
        }

        config.dialpad_mode = Some(mode);
        config.write();

        let _ = self.mode_changed(&ctxt).await;
        let _ = self.brightness_changed(&ctxt).await;
        Ok(())
    }

    #[zbus(property)]
    async fn supported(&self) -> Result<bool, FdoErr> {
        let dialpad = self.dialpad.lock().await;
        Ok(dialpad.is_hardware_capable() || dialpad.is_virtual_capable())
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
        self.apply_saved_state_internal().await
    }
}
