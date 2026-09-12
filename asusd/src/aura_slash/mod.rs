use std::sync::Arc;

use config::SlashConfig;
use rog_platform::slash_led::SlashLed;
use tokio::sync::{Mutex, MutexGuard};

use crate::error::RogError;

pub mod config;
pub mod trait_impls;

#[derive(Debug, Clone)]
pub struct Slash {
    led: SlashLed,
    config: Arc<Mutex<SlashConfig>>,
}

impl Slash {
    pub fn new(led: SlashLed, config: Arc<Mutex<SlashConfig>>) -> Self {
        Self { led, config }
    }

    pub fn led(&self) -> &SlashLed {
        &self.led
    }

    pub async fn lock_config(&self) -> MutexGuard<'_, SlashConfig> {
        self.config.lock().await
    }

    /// Initialise the device if required. Locks the internal config so be wary
    /// of deadlocks.
    pub async fn do_initialization(&self) -> Result<(), RogError> {
        let config = self.config.lock().await;

        let brightness = if config.enabled { config.brightness } else { 0 };
        self.led.set_brightness(brightness)?;
        self.led.set_slash_interval(config.display_interval)?;
        self.led.set_slash_mode(&config.display_mode.to_string())?;
        Ok(())
    }
}
