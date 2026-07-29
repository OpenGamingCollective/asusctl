use std::sync::Arc;

use config::SlashConfig;
use rog_platform::hid_raw::HidRaw;
use rog_platform::slash::usb::{
    slash_pkt_enable, slash_pkt_init, slash_pkt_options, slash_pkt_set_mode,
};
use rog_platform::usb_raw::USBRaw;
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

    /// Initialise the device if required. Locks the internal config so be wary
    /// of deadlocks.
    pub async fn do_initialization(&self) -> Result<(), RogError> {
        // Don't try to initialise these models as the asus drivers already did
        let config = self.config.lock().await;
        for pkt in &slash_pkt_init(config.slash_type) {
            self.write_bytes(pkt).await?;
        }
        self.write_bytes(&slash_pkt_enable(config.slash_type, config.enabled))
            .await?;

        // Apply config upon initialization
        let option_packets = slash_pkt_options(
            config.slash_type,
            config.enabled,
            config.brightness,
            config.display_interval,
        );
        self.write_bytes(&option_packets).await?;

        let mode_packets = slash_pkt_set_mode(config.slash_type, config.display_mode);
        // self.node.write_bytes(&mode_packets[0])?;
        self.write_bytes(&mode_packets[1]).await?;

        Ok(())
    }
}
