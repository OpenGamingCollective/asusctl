use config_traits::StdConfig;
use log::{debug, error, warn};
use rog_slash::usb::{
    slash_pkt_battery_saver, slash_pkt_boot, slash_pkt_enable, slash_pkt_lid_closed,
    slash_pkt_low_battery, slash_pkt_options, slash_pkt_save, slash_pkt_set_mode,
    slash_pkt_shutdown, slash_pkt_sleep,
};
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

fn slash_sysfs_power_supported(slash: &Slash) -> bool {
    slash.led().is_some_and(|led| led.has_power_states())
}

/// Update one DL `power_states` token (`boot` / `awake` / `sleep` / `shutdown`).
fn slash_set_dl_power_token(slash: &Slash, token: &str, enable: bool) -> Result<(), RogError> {
    let led = slash
        .led()
        .ok_or_else(|| RogError::MissingFunction("Slash LED missing for power_states".into()))?;
    let raw = led.get_power_states().map_err(RogError::from)?;
    let owned: Vec<String> = raw.split_whitespace().map(str::to_owned).collect();
    let mut states: Vec<&str> = owned.iter().map(String::as_str).collect();
    let present = states.contains(&token);
    if enable && !present {
        states.push(token);
    } else if !enable {
        states.retain(|s| *s != token);
    }
    led.set_power_states_list(&states).map_err(RogError::from)?;
    Ok(())
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
    /// True when boot/sleep/... persistence toggles are backed by HID/USB
    /// or by Slash Dynamic Lighting `power_states`.
    #[zbus(property)]
    async fn power_triggers_supported(&self) -> bool {
        !self.0.has_sysfs_led() || slash_sysfs_power_supported(&self.0)
    }

    #[zbus(property)]
    async fn enabled(&self) -> bool {
        let lock = self.0.lock_config().await;
        lock.enabled
    }

    #[zbus(property)]
    async fn set_enabled(&self, enabled: bool) {
        let mut config = self.0.lock_config().await;
        let brightness = if enabled && config.brightness == 0 {
            0x88
        } else {
            config.brightness
        };

        if let Some(led) = self.0.led() {
            let b = if enabled { brightness } else { 0 };
            if let Err(err) = led.set_brightness(b) {
                warn!("ctrl_slash::set_enabled via sysfs: {err}");
            }
        } else {
            self.0
                .write_bytes(&slash_pkt_enable(config.slash_type, enabled))
                .await
                .map_err(|err| warn!("ctrl_slash::enable {err}"))
                .ok();
            self.0
                .write_bytes(&slash_pkt_options(
                    config.slash_type,
                    enabled,
                    brightness,
                    config.display_interval,
                ))
                .await
                .map_err(|err| warn!("ctrl_slash::set_options {err}"))
                .ok();
        }

        config.enabled = enabled;
        config.brightness = brightness;
        config.write();
    }

    #[zbus(property)]
    async fn brightness(&self) -> u8 {
        let config = self.0.lock_config().await;
        config.brightness
    }

    #[zbus(property)]
    async fn set_brightness(&self, brightness: u8) {
        let mut config = self.0.lock_config().await;
        let enabled = brightness > 0;

        if let Some(led) = self.0.led() {
            if let Err(err) = led.set_brightness(brightness) {
                warn!("ctrl_slash::set_brightness via sysfs: {err}");
            }
        } else {
            self.0
                .write_bytes(&slash_pkt_options(
                    config.slash_type,
                    enabled,
                    brightness,
                    config.display_interval,
                ))
                .await
                .map_err(|err| warn!("ctrl_slash::set_options {err}"))
                .ok();
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

    #[zbus(property)]
    async fn set_interval(&self, interval: u8) {
        let mut config = self.0.lock_config().await;

        if let Some(led) = self.0.led() {
            if let Err(err) = led.set_interval(interval) {
                warn!("ctrl_slash::set_interval via sysfs: {err}");
            }
        } else {
            self.0
                .write_bytes(&slash_pkt_options(
                    config.slash_type, config.enabled, config.brightness, interval,
                ))
                .await
                .map_err(|err| warn!("ctrl_slash::set_options {err}"))
                .ok();
        }

        config.display_interval = interval;
        config.write();
    }

    #[zbus(property)]
    async fn mode(&self) -> zbus::fdo::Result<u8> {
        let config = self.0.lock_config().await;
        Ok(config.display_mode as u8)
    }

    #[zbus(property)]
    async fn set_mode(&self, mode: u8) -> zbus::Result<()> {
        let mode = SlashMode::try_from(mode)
            .map_err(|err| zbus::fdo::Error::InvalidArgs(format!("ctrl_slash::set_mode {err}")))?;
        let mut config = self.0.lock_config().await;

        if let Some(led) = self.0.led() {
            led.set_mode(&mode.to_string()).map_err(|err| {
                zbus::fdo::Error::Failed(format!("ctrl_slash::set_mode sysfs: {err}"))
            })?;
        } else {
            let command_packets = slash_pkt_set_mode(config.slash_type, mode);
            self.0.write_bytes(&command_packets[1]).await?;
            self.0
                .write_bytes(&slash_pkt_save(config.slash_type))
                .await?;
        }

        config.display_mode = mode;
        config.write();
        Ok(())
    }

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
    async fn set_show_on_boot(&self, enable: bool) -> zbus::Result<()> {
        if slash_sysfs_power_supported(&self.0) {
            slash_set_dl_power_token(&self.0, "boot", enable)?;
            let mut config = self.0.lock_config().await;
            config.show_on_boot = enable;
            config.write();
            return Ok(());
        }
        if self.0.has_sysfs_led() {
            return Err(slash_power_not_supported().into());
        }
        let mut config = self.0.lock_config().await;
        self.0
            .write_bytes(&slash_pkt_boot(config.slash_type, enable))
            .await?;
        config.show_on_boot = enable;
        config.write();
        Ok(())
    }

    #[zbus(property)]
    async fn show_on_sleep(&self) -> zbus::fdo::Result<bool> {
        let config = self.0.lock_config().await;
        Ok(config.show_on_sleep)
    }

    #[zbus(property)]
    async fn set_show_on_sleep(&self, enable: bool) -> zbus::Result<()> {
        if slash_sysfs_power_supported(&self.0) {
            slash_set_dl_power_token(&self.0, "sleep", enable)?;
            let mut config = self.0.lock_config().await;
            config.show_on_sleep = enable;
            config.write();
            return Ok(());
        }
        if self.0.has_sysfs_led() {
            return Err(slash_power_not_supported().into());
        }
        let mut config = self.0.lock_config().await;
        self.0
            .write_bytes(&slash_pkt_sleep(config.slash_type, enable))
            .await?;
        config.show_on_sleep = enable;
        config.write();
        Ok(())
    }

    #[zbus(property)]
    async fn show_on_shutdown(&self) -> zbus::fdo::Result<bool> {
        let config = self.0.lock_config().await;
        Ok(config.show_on_shutdown)
    }

    #[zbus(property)]
    async fn set_show_on_shutdown(&self, enable: bool) -> zbus::Result<()> {
        if slash_sysfs_power_supported(&self.0) {
            slash_set_dl_power_token(&self.0, "shutdown", enable)?;
            let mut config = self.0.lock_config().await;
            config.show_on_shutdown = enable;
            config.write();
            return Ok(());
        }
        if self.0.has_sysfs_led() {
            return Err(slash_power_not_supported().into());
        }
        let mut config = self.0.lock_config().await;
        self.0
            .write_bytes(&slash_pkt_shutdown(config.slash_type, enable))
            .await?;
        config.show_on_shutdown = enable;
        config.write();
        Ok(())
    }

    #[zbus(property)]
    async fn show_on_battery(&self) -> zbus::fdo::Result<bool> {
        let config = self.0.lock_config().await;
        Ok(config.show_on_battery)
    }

    #[zbus(property)]
    async fn set_show_on_battery(&self, enable: bool) -> zbus::Result<()> {
        if self.0.has_sysfs_led() {
            return Err(slash_power_not_supported().into());
        }
        let mut config = self.0.lock_config().await;
        self.0
            .write_bytes(&slash_pkt_battery_saver(config.slash_type, enable))
            .await?;
        config.show_on_battery = enable;
        config.write();
        Ok(())
    }

    #[zbus(property)]
    async fn show_battery_warning(&self) -> zbus::fdo::Result<bool> {
        let config = self.0.lock_config().await;
        Ok(config.show_battery_warning)
    }

    #[zbus(property)]
    async fn set_show_battery_warning(&self, enable: bool) -> zbus::Result<()> {
        if self.0.has_sysfs_led() {
            return Err(slash_power_not_supported().into());
        }
        let mut config = self.0.lock_config().await;
        self.0
            .write_bytes(&slash_pkt_low_battery(config.slash_type, enable))
            .await?;
        config.show_battery_warning = enable;
        config.write();
        Ok(())
    }

    #[zbus(property)]
    async fn show_on_lid_closed(&self) -> zbus::fdo::Result<bool> {
        let config = self.0.lock_config().await;
        Ok(config.show_on_lid_closed)
    }

    #[zbus(property)]
    async fn set_show_on_lid_closed(&self, enable: bool) -> zbus::Result<()> {
        if self.0.has_sysfs_led() {
            return Err(slash_power_not_supported().into());
        }
        let mut config = self.0.lock_config().await;
        self.0
            .write_bytes(&slash_pkt_lid_closed(config.slash_type, enable))
            .await?;
        config.show_on_lid_closed = enable;
        config.write();
        Ok(())
    }
}

impl Reloadable for SlashZbus {
    async fn reload(&mut self) -> Result<(), RogError> {
        debug!("reloading slash settings");
        self.0.do_initialization().await
    }
}
