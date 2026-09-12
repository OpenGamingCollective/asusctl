use config_traits::StdConfig;
use log::{debug, error, warn};
use rog_slash::{DeviceState, SlashMode};
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, interface};

use super::Slash;
use crate::Reloadable;
use crate::error::RogError;

const SLASH_POWER_UNSUPPORTED: &str = concat!(
    "Kernel Slash LED has no power_states; ",
    "boot/sleep/shutdown/battery/lid persistence is not supported"
);

fn slash_power_not_supported() -> zbus::fdo::Error {
    zbus::fdo::Error::NotSupported(SLASH_POWER_UNSUPPORTED.into())
}

#[derive(Clone)]
pub struct SlashZbus(Slash);

impl SlashZbus {
    pub fn new(slash: Slash) -> Self {
        Self(slash)
    }

    pub async fn start_tasks(
        mut self,
        connection: &Connection,
        path: OwnedObjectPath,
    ) -> Result<(), RogError> {
        self.reload()
            .await
            .unwrap_or_else(|err| warn!("Controller error: {}", err));
        connection
            .object_server()
            .at(path.clone(), self)
            .await
            .map_err(|e| {
                error!("Couldn't add server at path: {path}, {e:?}");
                RogError::from(e)
            })
            .map(|_| ())
    }
}

#[interface(name = "xyz.ljones.Slash")]
impl SlashZbus {
    /// Get enabled or not
    #[zbus(property)]
    async fn enabled(&self) -> bool {
        let lock = self.0.lock_config().await;
        lock.enabled
    }

    /// Set enabled true or false
    #[zbus(property)]
    async fn set_enabled(&self, enabled: bool) {
        let mut config = self.0.lock_config().await;
        let brightness = if enabled && config.brightness == 0 {
            0x88
        } else {
            config.brightness
        };

        let b = if enabled { brightness } else { 0 };
        if let Err(err) = self.0.led().set_brightness(b) {
            warn!("ctrl_slash::set_enabled via sysfs: {err}");
        }

        config.enabled = enabled;
        config.brightness = brightness;
        config.write();
    }

    /// Get brightness level
    #[zbus(property)]
    async fn brightness(&self) -> u8 {
        let config = self.0.lock_config().await;
        config.brightness
    }

    /// Set brightness level
    #[zbus(property)]
    async fn set_brightness(&self, brightness: u8) {
        let mut config = self.0.lock_config().await;
        let enabled = brightness > 0;

        if let Err(err) = self.0.led().set_brightness(brightness) {
            warn!("ctrl_slash::set_brightness via sysfs: {err}");
        }

        config.enabled = enabled;
        config.brightness = brightness;
        config.write();
    }

    #[zbus(property)]
    async fn interval(&self) -> u8 {
        let config = self.0.lock_config().await;
        config.display_interval
    }

    /// Set interval between slash animations (0-255)
    #[zbus(property)]
    async fn set_interval(&self, interval: u8) {
        let mut config = self.0.lock_config().await;

        if let Err(err) = self.0.led().set_slash_interval(interval) {
            warn!("ctrl_slash::set_interval via sysfs: {err}");
        }

        config.display_interval = interval;
        config.write();
    }

    #[zbus(property)]
    async fn mode(&self) -> zbus::fdo::Result<u8> {
        let config = self.0.lock_config().await;
        Ok(config.display_mode as u8)
    }

    /// Set animation mode
    #[zbus(property)]
    async fn set_mode(&self, mode: u8) -> zbus::Result<()> {
        let mode = SlashMode::try_from(mode).map_err(|err| {
            zbus::fdo::Error::InvalidArgs(format!("ctrl_slash::set_mode {}", err))
        })?;
        let mut config = self.0.lock_config().await;

        self.0
            .led()
            .set_slash_mode(&mode.to_string())
            .map_err(|err| {
                zbus::fdo::Error::Failed(format!("ctrl_slash::set_mode sysfs: {err}"))
            })?;

        config.display_mode = mode;
        config.write();
        Ok(())
    }

    /// Get the device state as stored by asusd
    async fn device_state(&self) -> DeviceState {
        let config = self.0.lock_config().await;
        DeviceState::from(&*config)
    }

    #[zbus(property)]
    async fn show_on_boot(&self) -> zbus::fdo::Result<bool> {
        let config = self.0.lock_config().await;
        Ok(config.show_on_boot)
    }

    #[zbus(property)]
    async fn set_show_on_boot(&self, _enable: bool) -> zbus::Result<()> {
        Err(slash_power_not_supported().into())
    }

    #[zbus(property)]
    async fn show_on_sleep(&self) -> zbus::fdo::Result<bool> {
        let config = self.0.lock_config().await;
        Ok(config.show_on_sleep)
    }

    #[zbus(property)]
    async fn set_show_on_sleep(&self, _enable: bool) -> zbus::Result<()> {
        Err(slash_power_not_supported().into())
    }

    #[zbus(property)]
    async fn show_on_shutdown(&self) -> zbus::fdo::Result<bool> {
        let config = self.0.lock_config().await;
        Ok(config.show_on_shutdown)
    }

    #[zbus(property)]
    async fn set_show_on_shutdown(&self, _enable: bool) -> zbus::Result<()> {
        Err(slash_power_not_supported().into())
    }

    #[zbus(property)]
    async fn show_on_battery(&self) -> zbus::fdo::Result<bool> {
        let config = self.0.lock_config().await;
        Ok(config.show_on_battery)
    }

    #[zbus(property)]
    async fn set_show_on_battery(&self, _enable: bool) -> zbus::Result<()> {
        Err(slash_power_not_supported().into())
    }

    #[zbus(property)]
    async fn show_battery_warning(&self) -> zbus::fdo::Result<bool> {
        let config = self.0.lock_config().await;
        Ok(config.show_battery_warning)
    }

    #[zbus(property)]
    async fn set_show_battery_warning(&self, _enable: bool) -> zbus::Result<()> {
        Err(slash_power_not_supported().into())
    }

    #[zbus(property)]
    async fn show_on_lid_closed(&self) -> zbus::fdo::Result<bool> {
        let config = self.0.lock_config().await;
        Ok(config.show_on_lid_closed)
    }

    #[zbus(property)]
    async fn set_show_on_lid_closed(&self, _enable: bool) -> zbus::Result<()> {
        Err(slash_power_not_supported().into())
    }
}

impl Reloadable for SlashZbus {
    async fn reload(&mut self) -> Result<(), RogError> {
        debug!("reloading slash settings");
        let config = self.0.lock_config().await;

        let brightness = if config.enabled { config.brightness } else { 0 };
        self.0.led().set_brightness(brightness)?;
        self.0.led().set_slash_interval(config.display_interval)?;
        self.0
            .led()
            .set_slash_mode(&config.display_mode.to_string())?;

        Ok(())
    }
}
