use std::collections::BTreeMap;

use config_traits::StdConfig;
use log::{debug, error, info, warn};
use rog_aura::keyboard::{AuraLaptopUsbPackets, LaptopAuraPower};
use rog_aura::{AuraDeviceType, AuraEffect, AuraModeNum, AuraZone, LedBrightness, PowerZones};
use zbus::fdo::Error as ZbErr;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, interface};

use super::{Aura, supports_dynamic_zone};
use crate::error::RogError;
use crate::{CtrlTask, Reloadable};

pub const AURA_ZBUS_NAME: &str = "Aura";
pub const AURA_ZBUS_PATH: &str = "/xyz/ljones";

#[derive(Clone)]
pub struct AuraZbus(Aura);

impl AuraZbus {
    pub fn new(aura: Aura) -> Self {
        Self(aura)
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

/// The main interface for changing, reading, or notfying
///
/// LED commands are split between Brightness, Modes, Per-Key
#[interface(name = "xyz.ljones.Aura")]
impl AuraZbus {
    /// Return the device type for this Aura keyboard
    #[zbus(property)]
    async fn device_type(&self) -> AuraDeviceType {
        self.0.config.lock().await.led_type
    }

    /// Return the current LED brightness
    #[zbus(property)]
    async fn brightness(&self) -> Result<LedBrightness, ZbErr> {
        Ok(self.0.get_brightness().await?)
    }

    /// Set the keyboard brightness level (Off/Low/Med/High)
    #[zbus(property)]
    async fn set_brightness(&mut self, brightness: LedBrightness) -> Result<(), ZbErr> {
        self.0.set_brightness(brightness).await?;
        let mut config = self.0.config.lock().await;
        config.brightness = brightness;
        config.write();
        Ok(())
    }

    /// Total levels of brightness available
    #[zbus(property)]
    async fn supported_brightness(&self) -> Vec<LedBrightness> {
        vec![
            LedBrightness::Off,
            LedBrightness::Low,
            LedBrightness::Med,
            LedBrightness::High,
        ]
    }

    /// The total available modes
    #[zbus(property)]
    async fn supported_basic_modes(&self) -> Result<Vec<AuraModeNum>, ZbErr> {
        let config = self.0.config.lock().await;
        if self.0.has_dynamic_lighting() {
            let led_lock = if let Some(global) = &self.0.dynamic_global {
                Some(global.lock().await)
            } else if let Some(kbd) = &self.0.dynamic_kbd {
                Some(kbd.lock().await)
            } else {
                None
            };
            if let Some(led) = led_lock {
                let mut modes = Vec::new();
                for mode in config.builtins.keys() {
                    if let Some(eff_str) = mode.to_dynamic_effect_str()
                        && led.supports_effect(eff_str)?
                    {
                        modes.push(*mode);
                    }
                }
                return Ok(modes);
            }
        }
        Ok(config.builtins.keys().cloned().collect())
    }

    #[zbus(property)]
    async fn supported_basic_zones(&self) -> Result<Vec<AuraZone>, ZbErr> {
        if self.0.has_dynamic_lighting() {
            // The kernel nodes split keyboard from lightbar, but do not model
            // the historical four keyboard or left/right lightbar zones.
            return Ok(Vec::new());
        }
        let config = self.0.config.lock().await;
        Ok(config.support_data.basic_zones.clone())
    }

    #[zbus(property)]
    async fn supported_power_zones(&self) -> Result<Vec<PowerZones>, ZbErr> {
        if self.0.has_dynamic_lighting() {
            let has_power_states = if let Some(global) = &self.0.dynamic_global {
                global.lock().await.has_power_states()
            } else if let Some(kbd) = &self.0.dynamic_kbd {
                kbd.lock().await.has_power_states()
            } else {
                false
            };
            if !has_power_states {
                // Avoid advertising zones the UI cannot actually control.
                return Ok(Vec::new());
            }
        }
        let config = self.0.config.lock().await;
        Ok(config.support_data.power_zones.clone())
    }

    /// The current mode data
    #[zbus(property)]
    async fn led_mode(&self) -> Result<AuraModeNum, ZbErr> {
        // entirely possible to deadlock here, so use try instead of lock()
        if let Ok(config) = self.0.config.try_lock() {
            Ok(config.current_mode)
        } else {
            Err(ZbErr::Failed("Aura control couldn't lock self".to_string()))
        }
    }

    /// Set an Aura effect if the effect mode or zone is supported.
    ///
    /// On success the aura config file is read to refresh cached values, then
    /// the effect is stored and config written to disk.
    #[zbus(property)]
    async fn set_led_mode(&mut self, num: AuraModeNum) -> Result<(), ZbErr> {
        let mut config = self.0.config.lock().await;
        config.current_mode = num;
        self.0.write_current_config_mode(&mut config).await?;
        if config.brightness == LedBrightness::Off {
            config.brightness = LedBrightness::Med;
        }
        if let Err(e) = self.0.set_brightness(config.brightness).await {
            log::warn!("Could not set keyboard backlight brightness: {e}");
        }
        config.write();
        Ok(())
    }

    /// The current mode data
    #[zbus(property)]
    async fn led_mode_data(&self) -> Result<AuraEffect, ZbErr> {
        // entirely possible to deadlock here, so use try instead of lock()
        if let Ok(config) = self.0.config.try_lock() {
            let mode = config.current_mode;
            match config.builtins.get(&mode) {
                Some(effect) => Ok(effect.clone()),
                None => Err(ZbErr::Failed("Could not get the current effect".into())),
            }
        } else {
            Err(ZbErr::Failed("Aura control couldn't lock self".to_string()))
        }
    }

    /// Set an Aura effect if the effect mode or zone is supported.
    ///
    /// On success the aura config file is read to refresh cached values, then
    /// the effect is stored and config written to disk.
    #[zbus(property)]
    async fn set_led_mode_data(&mut self, effect: AuraEffect) -> Result<(), ZbErr> {
        let mut config = self.0.config.lock().await;
        let (is_mode_supported, is_zone_supported) = if self.0.has_dynamic_lighting() {
            let mode_ok = if let Some(eff_str) = effect.mode.to_dynamic_effect_str() {
                let led_lock = if let Some(global) = &self.0.dynamic_global {
                    Some(global.lock().await)
                } else if let Some(kbd) = &self.0.dynamic_kbd {
                    Some(kbd.lock().await)
                } else {
                    None
                };
                match led_lock {
                    Some(led) => led.supports_effect(eff_str)?,
                    None => false,
                }
            } else {
                false
            };
            let zone_ok = supports_dynamic_zone(effect.zone);
            (mode_ok, zone_ok)
        } else {
            (
                config.support_data.basic_modes.contains(&effect.mode),
                effect.zone == AuraZone::None
                    || config.support_data.basic_zones.contains(&effect.zone)
                    || (self.0.dynamic_lightbar.is_some()
                        && matches!(effect.zone, AuraZone::BarLeft | AuraZone::BarRight)),
            )
        };

        if !is_mode_supported || !is_zone_supported {
            return Err(ZbErr::NotSupported(format!(
                "The Aura effect is not supported: {effect:?}"
            )));
        }

        self.0
            .write_effect_and_apply(config.led_type, &effect)
            .await?;
        if config.brightness == LedBrightness::Off {
            config.brightness = LedBrightness::Med;
        }
        if let Err(e) = self.0.set_brightness(config.brightness).await {
            log::warn!("Could not set keyboard backlight brightness: {e}");
        }
        config.set_builtin(effect);
        config.write();
        Ok(())
    }

    /// Get the data set for every mode available
    async fn all_mode_data(&self) -> BTreeMap<AuraModeNum, AuraEffect> {
        let config = self.0.config.lock().await;
        config.builtins.clone()
    }

    // As property doesn't work for AuraPowerDev (complexity of serialization?)
    #[zbus(property)]
    async fn led_power(&self) -> LaptopAuraPower {
        let config = self.0.config.lock().await;
        config.enabled.clone()
    }

    /// Set a variety of states, input is array of enum.
    /// `enabled` sets if the sent array should be disabled or enabled
    ///
    /// For Modern ROG devices the "enabled" flag is ignored.
    #[zbus(property)]
    async fn set_led_power(&mut self, options: LaptopAuraPower) -> Result<(), ZbErr> {
        let mut config = self.0.config.lock().await;
        for opt in options.states {
            let zone = opt.zone;
            for state in config.enabled.states.iter_mut() {
                if state.zone == zone {
                    *state = opt;
                    break;
                }
            }
        }
        config.write();
        Ok(self.0.set_power_states(&config).await.map_err(|e| {
            warn!("{}", e);
            e
        })?)
    }

    /// On machine that have some form of either per-key keyboard or per-zone
    /// this can be used to write custom effects over dbus. The input is a
    /// nested `Vec<Vec<8>>` where `Vec<u8>` is a raw USB packet
    async fn direct_addressing_raw(&self, data: AuraLaptopUsbPackets) -> Result<(), ZbErr> {
        let mut config = self.0.config.lock().await;
        self.0.write_effect_block(&mut config, &data).await?;
        Ok(())
    }
}

impl CtrlTask for AuraZbus {
    fn zbus_path() -> &'static str {
        "/xyz/ljones"
    }

    async fn create_tasks(&self, _: SignalEmitter<'static>) -> Result<(), RogError> {
        let inner1 = self.0.clone();
        let inner3 = self.0.clone();
        self.create_sys_event_tasks(
            move |sleeping| {
                let inner1 = inner1.clone();
                // unwrap as we want to bomb out of the task
                async move {
                    if !sleeping {
                        info!("CtrlKbdLedTask reloading brightness and modes");
                        let brightness = inner1.config.lock().await.brightness;
                        if let Err(e) = inner1.set_brightness(brightness).await {
                            error!("CtrlKbdLedTask: {e}");
                        }
                        let mut config = inner1.config.lock().await;
                        if let Err(e) = inner1.write_current_config_mode(&mut config).await {
                            error!("CtrlKbdLedTask: {e}");
                        }
                    } else if let Err(e) = inner1.update_config().await {
                        error!("CtrlKbdLedTask: {e}");
                    }
                }
            },
            move |_shutting_down| {
                let inner3 = inner3.clone();
                async move {
                    info!("CtrlKbdLedTask reloading brightness and modes");
                    let brightness = inner3.config.lock().await.brightness;
                    if let Err(e) = inner3.set_brightness(brightness).await {
                        error!("CtrlKbdLedTask: {e}");
                    }
                }
            },
            move |_lid_closed| {
                // on lid change
                async move {}
            },
            move |_power_plugged| {
                // power change
                async move {}
            },
        )
        .await;

        Ok(())
    }
}

impl Reloadable for AuraZbus {
    async fn reload(&mut self) -> Result<(), RogError> {
        self.0.fix_ally_power().await?;
        let mut config = self.0.lock_config().await;
        debug!("reloading power states");
        self.0
            .set_power_states(&config)
            .await
            .map_err(|err| warn!("{err}"))
            .ok();
        debug!("reloading keyboard mode");
        self.0.write_current_config_mode(&mut config).await?;
        Ok(())
    }
}
