use std::sync::Arc;
use std::time::Duration;

use config::SlashConfig;
use log::warn;
use rog_platform::hid_raw::HidRaw;
use rog_platform::power::AsusPower;
use rog_platform::usb_raw::USBRaw;
use rog_slash::battery_pattern;
use rog_slash::usb::{
    segment_count, slash_pkt_custom_commit, slash_pkt_custom_enable, slash_pkt_custom_frame,
    slash_pkt_custom_select, slash_pkt_enable, slash_pkt_init, slash_pkt_options,
    slash_pkt_set_mode,
};
use tokio::sync::{Mutex, MutexGuard};
use tokio::task::JoinHandle;

use crate::error::RogError;

pub mod config;
pub mod trait_impls;

#[derive(Debug, Clone)]
pub struct Slash {
    hid: Option<Arc<Mutex<HidRaw>>>,
    usb: Option<Arc<Mutex<USBRaw>>>,
    config: Arc<Mutex<SlashConfig>>,
    battery_level_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Slash {
    pub fn new(
        hid: Option<Arc<Mutex<HidRaw>>>,
        usb: Option<Arc<Mutex<USBRaw>>>,
        config: Arc<Mutex<SlashConfig>>,
    ) -> Self {
        Self {
            hid,
            usb,
            config,
            battery_level_task: Arc::new(Mutex::new(None)),
        }
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

    /// Arm the custom-pattern buffer and spawn a background task that renders
    /// the battery level to it every couple of seconds.
    pub async fn start_battery_level_task(&self) {
        let mut task = self.battery_level_task.lock().await;
        if task.is_some() {
            return;
        }

        let slash_type = self.config.lock().await.slash_type;
        self.write_bytes(&slash_pkt_custom_select(slash_type))
            .await
            .ok();
        self.write_bytes(&slash_pkt_custom_enable(slash_type))
            .await
            .ok();
        self.write_bytes(&slash_pkt_custom_commit(slash_type))
            .await
            .ok();

        let inner = self.clone();
        *task = Some(tokio::spawn(async move {
            let power = match AsusPower::new() {
                Ok(power) => power,
                Err(e) => {
                    warn!("Slash battery-level task: could not access battery info: {e}");
                    return;
                }
            };

            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;

                if !power.has_battery() {
                    continue;
                }
                let Ok(percent) = power.get_battery_capacity_percent() else {
                    continue;
                };

                let (slash_type, brightness) = {
                    let config = inner.config.lock().await;
                    (config.slash_type, config.brightness)
                };
                let length = segment_count(slash_type);
                let segments = battery_pattern(length, percent as f32, brightness);
                if let Ok(frame) = slash_pkt_custom_frame(slash_type, &segments) {
                    inner.write_bytes(&frame).await.ok();
                }
            }
        }));
    }

    /// Stop the battery-level background task if running. When `restore_mode`
    /// is set, also switches the hardware back to the configured
    /// `display_mode` (skipped when the whole display is being turned off
    /// anyway, to avoid a redundant write).
    pub async fn stop_battery_level_task(&self, restore_mode: bool) {
        let mut task = self.battery_level_task.lock().await;
        if let Some(handle) = task.take() {
            handle.abort();
        }
        drop(task);

        if restore_mode {
            // mode-set + save isn't enough to pull the firmware out
            // of the custom-pattern render target armed by `start_battery_level_task`.
            self.do_initialization().await.ok();
        }
    }
}
