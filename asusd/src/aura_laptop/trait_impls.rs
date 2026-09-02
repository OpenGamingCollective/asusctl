use std::collections::BTreeMap;

use config_traits::StdConfig;
use log::{debug, error, info, warn};
use rog_aura::keyboard::{AuraLaptopUsbPackets, LaptopAuraPower};
use rog_aura::{AuraDeviceType, AuraEffect, AuraModeNum, AuraZone, LedBrightness, PowerZones};
use zbus::fdo::Error as ZbErr;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, interface};

use super::Aura;
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
        // let task = zbus.clone();
        // let signal_ctx = signal_ctx.clone();
        self.reload()
            .await
            .unwrap_or_else(|err| warn!("Controller error: {}", err));
        let task = self.clone();
        connection
            .object_server()
            .at(path.clone(), self)
            .await
            .map_err(|e| error!("Couldn't add server at path: {path}, {e:?}"))
            .ok();
        // Subscribe to logind sleep/shutdown events. Without this call the
        // Aura interface is available, but the keyboard controller never
        // receives on_prepare_for_sleep notifications.
        let signal_ctx = SignalEmitter::new(connection, AURA_ZBUS_PATH)?;
        info!("Starting CtrlKbdLedTask system-event subscription");
        task.create_tasks(signal_ctx).await?;
        info!("Started CtrlKbdLedTask system-event subscription");
        Ok(())
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
        if let Some(bl) = self.0.backlight.as_ref() {
            return Ok(bl.lock().await.get_brightness().map(|n| n.into())?);
        }
        Err(ZbErr::Failed("No sysfs brightness control".to_string()))
    }

    /// Set the keyboard brightness level (0-3)
    #[zbus(property)]
    async fn set_brightness(&mut self, brightness: LedBrightness) -> Result<(), ZbErr> {
        if let Some(bl) = self.0.backlight.as_ref() {
            let res = bl.lock().await.set_brightness(brightness.into());
            if res.is_ok() {
                let mut config = self.0.config.lock().await;
                config.brightness = brightness;
                config.write();
            }
            return Ok(res?);
        }
        Err(ZbErr::Failed("No sysfs brightness control".to_string()))
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
        Ok(config.builtins.keys().cloned().collect())
    }

    #[zbus(property)]
    async fn supported_basic_zones(&self) -> Result<Vec<AuraZone>, ZbErr> {
        let config = self.0.config.lock().await;
        Ok(config.support_data.basic_zones.clone())
    }

    #[zbus(property)]
    async fn supported_power_zones(&self) -> Result<Vec<PowerZones>, ZbErr> {
        let config = self.0.config.lock().await;
        Ok(config.support_data.power_zones.clone())
    }

    /// The current mode data
    #[zbus(property)]
    async fn led_mode(&self) -> Result<AuraModeNum, ZbErr> {
        // entirely possible to deadlock here, so use try instead of lock()
        // let ctrl = self.0.lock().await;
        // Ok(config.current_mode)
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
        if let Err(e) = self.0.set_brightness(config.brightness.into()).await {
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
        if !config.support_data.basic_modes.contains(&effect.mode)
            || effect.zone != AuraZone::None
                && !config.support_data.basic_zones.contains(&effect.zone)
        {
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
        if let Err(e) = self.0.set_brightness(config.brightness.into()).await {
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
        info!("Creating Aura system-event callbacks");
        let inner1 = self.0.clone();
        let inner3 = self.0.clone();
        self.create_sys_event_tasks(
            move |sleeping| {
                let inner1 = inner1.clone();
                // unwrap as we want to bomb out of the task
                async move {
                    info!("CtrlKbdLedTask received prepare_for_sleep({sleeping})");
                    if sleeping {
                        // Re-write the user's configured power state right
                        // before suspend. The kernel patch re-asserts brightness
                        // and enables all power modes to ensure the sleep
                        // strobe works, but this overrides the user's "sleep
                        // backlight off" preference. By writing the actual
                        // user config here (after the kernel prepare callback
                        // has already run), we restore the user's intent while
                        // still allowing the kernel's brightness re-assertion
                        // to have set up the EC correctly for the strobe case.
                        let config = inner1.config.lock().await;
                        let sleep_enabled = config.enabled.states.iter()
                            .any(|s| s.zone == rog_aura::PowerZones::Keyboard && s.sleep);
                        drop(config);

                        if !sleep_enabled {
                            info!("CtrlKbdLedTask sleep: user disabled sleep backlight, re-writing power state");
                            let config = inner1.config.lock().await;
                            if let Err(e) = inner1.set_power_states(&config).await {
                                error!("CtrlKbdLedTask sleep power state rewrite: {e}");
                            }
                        } else {
                            info!("CtrlKbdLedTask sleep: sleep backlight enabled, no-op");
                        }
                    } else {
                        info!("CtrlKbdLedTask reloading brightness and modes");
                        let (brightness, led_type) = {
                            let config = inner1.config.lock().await;
                            (config.brightness.into(), config.led_type)
                        };
                        if let Some(backlight) = &inner1.backlight {
                            if let Err(e) = backlight.lock().await.set_brightness(brightness) {
                                error!("CtrlKbdLedTask wake brightness: {e}");
                                return;
                            }
                        }
                        let mut config = inner1.config.lock().await;
                        if let Err(e) = inner1.write_current_config_mode(&mut config).await {
                            error!("CtrlKbdLedTask wake mode: {e}");
                            return;
                        }
                        if led_type.is_tuf_laptop()
                            && let Err(e) = inner1.set_power_states(&config).await
                        {
                            error!("CtrlKbdLedTask wake power state: {e}");
                        }
                    }
                }
            },
            move |_shutting_down| {
                let inner3 = inner3.clone();
                async move {
                    info!("CtrlKbdLedTask reloading brightness and modes");
                    if let Some(backlight) = &inner3.backlight {
                        // unwrap as we want to bomb out of the task
                        backlight
                            .lock()
                            .await
                            .set_brightness(inner3.config.lock().await.brightness.into())
                            .map_err(|e| {
                                error!("CtrlKbdLedTask: {e}");
                                e
                            })
                            .unwrap();
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

        // let ctrl2 = self.0.clone();
        // let ctrl = self.0.lock().await;
        // if ctrl.led_node.has_brightness_control() {
        //     let watch = ctrl.led_node.monitor_brightness()?;
        //     tokio::spawn(async move {
        //         let mut buffer = [0; 32];
        //         watch
        //             .into_event_stream(&mut buffer)
        //             .unwrap()
        //             .for_each(|_| async {
        //                 if let Some(lock) = ctrl2.try_lock() {
        //                     load_save(true, lock).unwrap(); // unwrap as we want
        //                                                     // to
        //                                                     // bomb out of the
        //                                                     // task
        //                 }
        //             })
        //             .await;
        //     });
        // }

        Ok(())
    }
}

impl Reloadable for AuraZbus {
    async fn reload(&mut self) -> Result<(), RogError> {
        self.0.fix_ally_power().await?;
        debug!("reloading keyboard mode");
        let mut config = self.0.lock_config().await;
        self.0.write_current_config_mode(&mut config).await?;
        debug!("reloading power states");
        self.0
            .set_power_states(&config)
            .await
            .map_err(|err| warn!("{err}"))
            .ok();
        Ok(())
    }
}
