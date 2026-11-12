use std::sync::Arc;

use config::SlashConfig;
use rog_platform::hid_raw::HidRaw;
use rog_platform::slash_led::SlashLed;
use rog_platform::usb_raw::USBRaw;
use rog_slash::usb::{slash_pkt_enable, slash_pkt_init, slash_pkt_options, slash_pkt_set_mode};
use tokio::sync::{Mutex, MutexGuard};

use crate::error::RogError;

pub mod config;
pub mod trait_impls;

#[derive(Debug, Clone)]
pub struct Slash {
    /// Kernel `asus::slash` LED when Dynamic Lighting owns the device.
    led: Option<SlashLed>,
    hid: Option<Arc<Mutex<HidRaw>>>,
    usb: Option<Arc<Mutex<USBRaw>>>,
    config: Arc<Mutex<SlashConfig>>,
}

impl Slash {
    pub fn new(
        led: Option<SlashLed>,
        hid: Option<Arc<Mutex<HidRaw>>>,
        usb: Option<Arc<Mutex<USBRaw>>>,
        config: Arc<Mutex<SlashConfig>>,
    ) -> Self {
        Self {
            led,
            hid,
            usb,
            config,
        }
    }

    pub fn has_sysfs_led(&self) -> bool {
        self.led.is_some()
    }

    pub fn led(&self) -> Option<&SlashLed> {
        self.led.as_ref()
    }

    pub async fn lock_config(&self) -> MutexGuard<'_, SlashConfig> {
        self.config.lock().await
    }

    pub async fn write_bytes(&self, message: &[u8]) -> Result<(), RogError> {
        if self.led.is_some() {
            return Err(RogError::MissingFunction(
                "Slash Dynamic Lighting owns the device; USB/HID packets are not used".into(),
            ));
        }
        if let Some(hid) = &self.hid {
            hid.lock().await.write_bytes(message)?;
        } else if let Some(usb) = &self.usb {
            usb.lock().await.write_bytes(message)?;
        } else {
            return Err(RogError::MissingFunction("No Slash backend".into()));
        }
        Ok(())
    }

    /// Initialise the device if required. Locks the internal config so be wary
    /// of deadlocks.
    pub async fn do_initialization(&self) -> Result<(), RogError> {
        let config = self.config.lock().await;
        if let Some(led) = &self.led {
            let brightness = if config.enabled { config.brightness } else { 0 };
            led.set_brightness(brightness)?;
            led.set_interval(config.display_interval)?;
            led.set_mode(&config.display_mode.to_string())?;
            return Ok(());
        }

        for pkt in &slash_pkt_init(config.slash_type) {
            self.write_bytes(pkt).await?;
        }
        self.write_bytes(&slash_pkt_enable(config.slash_type, config.enabled))
            .await?;
        let option_packets = slash_pkt_options(
            config.slash_type,
            config.enabled,
            config.brightness,
            config.display_interval,
        );
        self.write_bytes(&option_packets).await?;
        let mode_packets = slash_pkt_set_mode(config.slash_type, config.display_mode);
        self.write_bytes(&mode_packets[1]).await?;
        Ok(())
    }
}
