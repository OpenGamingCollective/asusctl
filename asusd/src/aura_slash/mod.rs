use std::sync::Arc;

use config::SlashConfig;
use log::warn;
use rog_platform::hid_raw::HidRaw;
use rog_platform::usb_raw::USBRaw;
use rog_slash::usb::{
    slash_pkt_battery_saver, slash_pkt_boot, slash_pkt_enable, slash_pkt_init,
    slash_pkt_low_battery, slash_pkt_options, slash_pkt_set_mode, slash_pkt_shutdown,
    slash_pkt_sleep,
};
use tokio::sync::{Mutex, MutexGuard};

use crate::error::RogError;

pub mod config;
pub mod trait_impls;

#[derive(Debug, Clone)]
pub struct Slash {
    hid: Option<Arc<Mutex<HidRaw>>>,
    usb: Option<Arc<Mutex<USBRaw>>>,
    config: Arc<Mutex<SlashConfig>>,
}

impl Slash {
    pub fn new(
        hid: Option<Arc<Mutex<HidRaw>>>,
        usb: Option<Arc<Mutex<USBRaw>>>,
        config: Arc<Mutex<SlashConfig>>,
    ) -> Self {
        Self { hid, usb, config }
    }

    pub async fn lock_config(&self) -> MutexGuard<'_, SlashConfig> {
        self.config.lock().await
    }

    pub async fn write_bytes(&self, message: &[u8]) -> Result<(), RogError> {
        if let Some(hid) = &self.hid {
            hid.lock().await.write_bytes(message)?;
        } else if let Some(usb) = &self.usb {
            usb.lock().await.write_bytes(message)?;
        }
        Ok(())
    }

    /// Reload slash settings (re-initializes mode, options, brightness, and system state flags)
    pub async fn reload(&self) -> Result<(), RogError> {
        self.do_initialization().await?;

        let (boot_pkt, sleep_pkt, shutdown_pkt, battery_saver_pkt, low_battery_pkt) = {
            let config = self.config.lock().await;
            (
                slash_pkt_boot(config.slash_type, config.show_on_boot),
                slash_pkt_sleep(config.slash_type, config.show_on_sleep),
                slash_pkt_shutdown(config.slash_type, config.show_on_shutdown),
                slash_pkt_battery_saver(config.slash_type, config.show_on_battery),
                slash_pkt_low_battery(config.slash_type, config.show_battery_warning),
            )
        };

        let state_pkts: [(&[u8], &str); 5] = [
            (&boot_pkt, "show_on_boot"),
            (&sleep_pkt, "show_on_sleep"),
            (&shutdown_pkt, "show_on_shutdown"),
            (&battery_saver_pkt, "show_on_battery"),
            (&low_battery_pkt, "show_battery_warning"),
        ];

        for (pkt, name) in state_pkts {
            if let Err(err) = self.write_bytes(pkt).await {
                warn!("{name} failed on reload: {err}");
            }
        }

        Ok(())
    }

    /// Initialise the device if required. Locks the internal config so be wary
    /// of deadlocks.
    pub async fn do_initialization(&self) -> Result<(), RogError> {
        let (init_pkts, enable_pkt, option_packets, mode_packet) = {
            let config = self.config.lock().await;
            (
                slash_pkt_init(config.slash_type),
                slash_pkt_enable(config.slash_type, config.enabled),
                slash_pkt_options(
                    config.slash_type,
                    config.enabled,
                    config.brightness,
                    config.display_interval,
                ),
                slash_pkt_set_mode(config.slash_type, config.display_mode)[1],
            )
        };

        for pkt in &init_pkts {
            self.write_bytes(pkt).await?;
        }
        self.write_bytes(&enable_pkt).await?;
        self.write_bytes(&option_packets).await?;
        self.write_bytes(&mode_packet).await?;

        Ok(())
    }
}
