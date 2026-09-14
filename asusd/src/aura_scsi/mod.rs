use std::sync::Arc;

use config::ScsiConfig;
use rog_platform::ScsiLed;
use rog_scsi::{AuraEffect, AuraMode, Direction};
use tokio::sync::{Mutex, MutexGuard};

use crate::error::RogError;

pub mod config;
pub mod trait_impls;

#[derive(Clone)]
pub struct ScsiAura {
    pub led: ScsiLed,
    pub config: Arc<Mutex<ScsiConfig>>,
}

impl ScsiAura {
    pub fn new(led: ScsiLed, config: Arc<Mutex<ScsiConfig>>) -> Self {
        Self { led, config }
    }

    pub fn led(&self) -> &ScsiLed {
        &self.led
    }

    pub async fn lock_config(&self) -> MutexGuard<'_, ScsiConfig> {
        self.config.lock().await
    }

    pub async fn write_effect(&self, effect: &AuraEffect) -> Result<(), RogError> {
        Self::write_kernel_effect(&self.led, effect)
    }

    fn write_kernel_effect(led: &ScsiLed, effect: &AuraEffect) -> Result<(), RogError> {
        match effect.mode {
            AuraMode::Off => {
                led.set_effect("off").map_err(RogError::Platform)?;
            }
            AuraMode::Static => {
                let colors = [
                    (effect.colour1.r, effect.colour1.g, effect.colour1.b),
                    (effect.colour2.r, effect.colour2.g, effect.colour2.b),
                    (effect.colour3.r, effect.colour3.g, effect.colour3.b),
                    (effect.colour4.r, effect.colour4.g, effect.colour4.b),
                ];
                if led.dynamic().has_effects_palette() {
                    led.set_palette_colors(&colors)
                        .map_err(RogError::Platform)?;
                }
                led.set_effect("static").map_err(RogError::Platform)?;
            }
            AuraMode::Breathe => {
                let colors = [
                    (effect.colour1.r, effect.colour1.g, effect.colour1.b),
                    (effect.colour2.r, effect.colour2.g, effect.colour2.b),
                    (effect.colour3.r, effect.colour3.g, effect.colour3.b),
                    (effect.colour4.r, effect.colour4.g, effect.colour4.b),
                ];
                if led.dynamic().has_effects_palette() {
                    led.set_palette_colors(&colors)
                        .map_err(RogError::Platform)?;
                }
                if led.dynamic().has_speed() {
                    led.set_speed(effect.speed as u32)
                        .map_err(RogError::Platform)?;
                }
                led.set_effect("breathing").map_err(RogError::Platform)?;
            }
            AuraMode::Flashing => {
                let colors = [
                    (effect.colour1.r, effect.colour1.g, effect.colour1.b),
                    (effect.colour2.r, effect.colour2.g, effect.colour2.b),
                    (effect.colour3.r, effect.colour3.g, effect.colour3.b),
                    (effect.colour4.r, effect.colour4.g, effect.colour4.b),
                ];
                if led.dynamic().has_effects_palette() {
                    led.set_palette_colors(&colors)
                        .map_err(RogError::Platform)?;
                }
                if led.dynamic().has_speed() {
                    led.set_speed(effect.speed as u32)
                        .map_err(RogError::Platform)?;
                }
                led.set_effect("strobe").map_err(RogError::Platform)?;
            }
            AuraMode::RainbowCycle => {
                if led.dynamic().has_speed() {
                    led.set_speed(effect.speed as u32)
                        .map_err(RogError::Platform)?;
                }
                led.set_effect("spectrum_cycle")
                    .map_err(RogError::Platform)?;
            }
            AuraMode::RainbowWave => {
                if led.dynamic().has_speed() {
                    led.set_speed(effect.speed as u32)
                        .map_err(RogError::Platform)?;
                }
                let dir = match effect.direction {
                    Direction::Forward => "right",
                    Direction::Reverse => "left",
                };
                if led.dynamic().has_direction() {
                    led.set_direction(dir).map_err(RogError::Platform)?;
                }
                led.set_effect("rainbow").map_err(RogError::Platform)?;
            }
            _ => return Err(rog_platform::error::PlatformError::NotSupported.into()),
        }
        Ok(())
    }

    /// Initialise the device if required. Locks the internal config so be wary
    /// of deadlocks.
    pub async fn do_initialization(&self) -> Result<(), RogError> {
        let config = self.config.lock().await;
        let mode = config.current_mode;
        if let Some(effect) = config.modes.get(&mode) {
            self.write_effect(effect).await?;
        }
        Ok(())
    }
}
