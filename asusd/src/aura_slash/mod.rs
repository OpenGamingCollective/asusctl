use std::sync::Arc;
use std::time::Duration;

use config::SlashConfig;

use log::warn;

use rog_platform::hid_raw::HidRaw;
use rog_platform::power::AsusPower;
use rog_platform::usb_raw::USBRaw;
use rog_slash::usb::{
    ENHANCED_SEGMENT_COUNT, segment_count, slash_pkt_custom_commit, slash_pkt_custom_enable,
    slash_pkt_custom_frame, slash_pkt_custom_select, slash_pkt_enable, slash_pkt_init,
    slash_pkt_options, slash_pkt_set_mode,
};
use rog_slash::{SlashType, battery_pattern};

use tokio::sync::{Mutex, MutexGuard, Notify};
use tokio::task::JoinHandle;

use crate::error::RogError;

pub mod config;
pub mod trait_impls;

type BatteryLevelTask = (JoinHandle<()>, Arc<Notify>);

#[derive(Debug, Clone)]
pub struct Slash {
    hid: Option<Arc<Mutex<HidRaw>>>,
    usb: Option<Arc<Mutex<USBRaw>>>,
    config: Arc<Mutex<SlashConfig>>,
    battery_level_task: Arc<Mutex<Option<BatteryLevelTask>>>,
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

        let armed = self
            .write_bytes(&slash_pkt_custom_select(slash_type))
            .await
            .and(self.write_bytes(&slash_pkt_custom_enable(slash_type)).await)
            .and(self.write_bytes(&slash_pkt_custom_commit(slash_type)).await);
        if let Err(e) = armed {
            warn!("slash: failed to arm custom-pattern buffer, aborting battery level task: {e}");
            self.do_initialization().await.ok();
            return;
        }

        let shutdown = Arc::new(Notify::new());
        let shutdown_task = shutdown.clone();
        let inner = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                let (slash_type, brightness) = {
                    let config = inner.config.lock().await;
                    (config.slash_type, config.brightness)
                };
                if let Err(e) = inner
                    .update_slash_battery_level_pattern(slash_type, brightness)
                    .await
                {
                    warn!("slash: failed to update battery level pattern: {e}");
                }
                tokio::select! {
                    _ = shutdown_task.notified() => break,
                    _ = interval.tick() => {}
                }
            }
        });
        *task = Some((handle, shutdown));
    }

    pub async fn update_slash_battery_level_pattern(
        &self,
        slash_type: SlashType,
        brightness: u8,
    ) -> Result<(), RogError> {
        let percentage = AsusPower::new()?.get_battery_capacity_percent()?;
        let length = segment_count(slash_type);
        let mut segments = [0u8; ENHANCED_SEGMENT_COUNT];
        battery_pattern(&mut segments[..length], percentage as f32, brightness);
        let frame = slash_pkt_custom_frame(slash_type, &segments[..length])?;

        self.write_bytes(&frame).await?;
        Ok(())
    }

    /// Stop the battery-level background task if running. When `restore_mode`
    /// is set, also switches the hardware back to the configured
    /// `display_mode` (skipped when the whole display is being turned off
    /// anyway, to avoid a redundant write).
    pub async fn stop_battery_level_task(&self, restore_mode: bool) {
        let mut task = self.battery_level_task.lock().await;
        if let Some((handle, shutdown)) = task.take() {
            shutdown.notify_one();
            handle.await.ok();
        }
        drop(task);

        if restore_mode {
            // mode-set + save isn't enough to pull the firmware out
            // of the custom-pattern render target armed by `start_battery_level_task`.
            let config = self.config.lock().await;
            for pkt in &slash_pkt_init(config.slash_type) {
                self.write_bytes(pkt).await.ok();
            }
        }
    }
}
