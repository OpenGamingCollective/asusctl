use std::sync::atomic::Ordering;

use config_traits::StdConfig;
use log::{debug, error, warn};
use logind_zbus::manager::ManagerProxy;
use rog_anime::usb::{
    pkt_set_brightness, pkt_set_builtin_animations, pkt_set_enable_display,
    pkt_set_enable_powersave_anim, Brightness,
};
use rog_anime::{Animations, AnimeDataBuffer, DeviceState};
use zbus::object_server::SignalEmitter;
use zbus::proxy::CacheProperties;
use zbus::zvariant::OwnedObjectPath;
use zbus::{interface, Connection};

use tokio_util::sync::CancellationToken;

use super::config::AniMeConfig;
use super::AniMe;
use crate::error::RogError;
use crate::{CtrlTask, Reloadable};

async fn get_logind_manager<'a>() -> ManagerProxy<'a> {
    let connection = Connection::system()
        .await
        .expect("Controller could not create dbus connection");

    ManagerProxy::builder(&connection)
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .expect("Controller could not create ManagerProxy")
}

#[derive(Clone)]
pub struct AniMeZbus(AniMe);

impl AniMeZbus {
    pub fn new(anime: AniMe) -> Self {
        Self(anime)
    }

    pub async fn start_tasks(
        mut self,
        connection: &Connection,
        path: OwnedObjectPath,
        cancel_token: CancellationToken,
    ) -> Result<(), RogError> {
        *self.0.cancel_token.lock().await = Some(cancel_token.clone());
        self.reload()
            .await
            .unwrap_or_else(|err| warn!("Controller error: {}", err));
        connection
            .object_server()
            .at(path.clone(), self.clone())
            .await
            .map_err(|e| {
                error!("Couldn't add server at path: {path}, {e:?}");
                e
            })?;

        let res = async {
            let signal_ctx = SignalEmitter::new(connection, path.clone().into_inner())?;
            self.create_tasks(signal_ctx).await?;
            Ok::<(), RogError>(())
        }
        .await;

        if let Err(err) = res {
            error!("Failed post-registration tasks for {path}: {err:?}, removing object");
            cancel_token.cancel();
            connection
                .object_server()
                .remove::<AniMeZbus, _>(&path)
                .await
                .ok();
            return Err(err);
        }

        debug!("start_tasks was successful");
        Ok(())
    }
}

// None of these calls can be guarnateed to succeed unless we loop until okay
// If the try_lock *does* succeed then any other thread trying to lock will not
// grab it until we finish.
#[interface(name = "xyz.ljones.Anime")]
impl AniMeZbus {
    /// Writes a data stream of length. Will force system thread to exit until
    /// it is restarted
    async fn write(&self, input: AnimeDataBuffer) -> zbus::fdo::Result<()> {
        let mut config = self.0.config.lock().await;
        let bright = config.display_brightness;
        if config.builtin_anims_enabled {
            // This clears the display, causing flickers if done indiscriminately on every
            // write. Therefore, we guard it behind a config check.
            self.0.set_builtins_enabled(false, bright).await?;
            config.builtin_anims_enabled = false;
            config.write();
        }
        drop(config);
        self.0.thread_exit.store(true, Ordering::SeqCst);
        self.0.write_data_buffer(input).await.map_err(|err| {
            warn!("ctrl_anime::run_animation:callback {}", err);
            err
        })?;
        Ok(())
    }

    /// Set base brightness level
    #[zbus(property)]
    async fn brightness(&self) -> Brightness {
        if let Ok(config) = self.0.config.try_lock() {
            return config.display_brightness;
        }
        Brightness::Off
    }

    /// Set base brightness level
    #[zbus(property)]
    async fn set_brightness(&self, brightness: Brightness) {
        self.0
            .write_bytes(&pkt_set_brightness(brightness))
            .await
            .map_err(|err| {
                warn!("ctrl_anime::set_brightness {}", err);
            })
            .ok();
        self.0
            .write_bytes(&pkt_set_enable_display(brightness != Brightness::Off))
            .await
            .map_err(|err| {
                warn!("ctrl_anime::set_brightness {}", err);
            })
            .ok();

        let mut config = self.0.config.lock().await;
        config.display_enabled = brightness != Brightness::Off;
        config.display_brightness = brightness;
        config.write();
    }

    #[zbus(property)]
    async fn builtins_enabled(&self) -> bool {
        if let Ok(config) = self.0.config.try_lock() {
            return config.builtin_anims_enabled;
        }
        false
    }

    /// Enable the builtin animations or not. This is quivalent to "Powersave
    /// animations" in Armory crate
    #[zbus(property)]
    async fn set_builtins_enabled(&self, enabled: bool) {
        let mut config = self.0.config.lock().await;
        let brightness = config.display_brightness;
        self.0
            .set_builtins_enabled(enabled, brightness)
            .await
            .map_err(|err| {
                warn!("ctrl_anime::set_builtins_enabled {}", err);
            })
            .ok();

        if !enabled {
            let anime_type = config.anime_type;
            let data = vec![255u8; anime_type.data_length()];
            if let Ok(tmp) = AnimeDataBuffer::from_vec(anime_type, data).map_err(|err| {
                warn!("ctrl_anime::set_builtins_enabled {}", err);
            }) {
                self.0
                    .write_bytes(tmp.data())
                    .await
                    .map_err(|err| {
                        warn!("ctrl_anime::set_builtins_enabled {}", err);
                    })
                    .ok();
            }
        }

        config.builtin_anims_enabled = enabled;
        config.write();
        if enabled {
            self.0.thread_exit.store(true, Ordering::Release);
        }
    }

    #[zbus(property)]
    async fn builtin_animations(&self) -> Animations {
        if let Ok(config) = self.0.config.try_lock() {
            return config.builtin_anims;
        }
        Animations::default()
    }

    /// Set which builtin animation is used for each stage
    #[zbus(property)]
    async fn set_builtin_animations(&self, settings: Animations) {
        self.0
            .write_bytes(&pkt_set_builtin_animations(
                settings.boot, settings.awake, settings.sleep, settings.shutdown,
            ))
            .await
            .map_err(|err| {
                warn!("ctrl_anime::run_animation:callback {}", err);
            })
            .ok();
        self.0
            .write_bytes(&pkt_set_enable_powersave_anim(true))
            .await
            .map_err(|err| {
                warn!("ctrl_anime::run_animation:callback {}", err);
            })
            .ok();
        let mut config = self.0.config.lock().await;
        config.display_enabled = true;
        config.builtin_anims = settings;
        config.write();
    }

    #[zbus(property)]
    async fn enable_display(&self) -> bool {
        if let Ok(config) = self.0.config.try_lock() {
            return config.display_enabled;
        }
        false
    }

    /// Set whether the AniMe is enabled at all
    #[zbus(property)]
    async fn set_enable_display(&self, enabled: bool) {
        self.0
            .write_bytes(&pkt_set_enable_display(enabled))
            .await
            .map_err(|err| {
                warn!("ctrl_anime::run_animation:callback {}", err);
            })
            .ok();
        let mut config = self.0.config.lock().await;
        config.display_enabled = enabled;
        config.write();
    }

    #[zbus(property)]
    async fn off_when_unplugged(&self) -> bool {
        if let Ok(config) = self.0.config.try_lock() {
            return config.off_when_unplugged;
        }
        false
    }

    /// Set if to turn the AniMe Matrix off when external power is unplugged
    #[zbus(property)]
    async fn set_off_when_unplugged(&self, enabled: bool) {
        let manager = get_logind_manager().await;
        let pow = manager.on_external_power().await.unwrap_or_default();

        self.0
            .write_bytes(&pkt_set_enable_display(!pow && !enabled))
            .await
            .map_err(|err| {
                warn!("create_sys_event_tasks::off_when_lid_closed {}", err);
            })
            .ok();

        let mut config = self.0.config.lock().await;
        config.off_when_unplugged = enabled;
        config.write();
    }

    #[zbus(property)]
    async fn off_when_suspended(&self) -> bool {
        if let Ok(config) = self.0.config.try_lock() {
            return config.off_when_suspended;
        }
        false
    }

    /// Set if to turn the AniMe Matrix off when the laptop is suspended
    #[zbus(property)]
    async fn set_off_when_suspended(&self, enabled: bool) {
        let mut config = self.0.config.lock().await;
        config.off_when_suspended = enabled;
        config.write();
    }

    #[zbus(property)]
    async fn off_when_lid_closed(&self) -> bool {
        if let Ok(config) = self.0.config.try_lock() {
            return config.off_when_lid_closed;
        }
        false
    }

    /// Set if to turn the AniMe Matrix off when the lid is closed
    #[zbus(property)]
    async fn set_off_when_lid_closed(&self, enabled: bool) {
        let manager = get_logind_manager().await;
        let lid = manager.lid_closed().await.unwrap_or_default();

        self.0
            .write_bytes(&pkt_set_enable_display(lid && !enabled))
            .await
            .map_err(|err| {
                warn!("create_sys_event_tasks::off_when_lid_closed {}", err);
            })
            .ok();

        let mut config = self.0.config.lock().await;
        config.off_when_lid_closed = enabled;
        config.write();
    }

    /// The main loop is the base system set action if the user isn't running
    /// the user daemon
    async fn run_main_loop(&self, start: bool) {
        if start {
            self.0.thread_exit.store(true, Ordering::SeqCst);
            self.0.run_thread(self.0.cache.system.clone(), false).await;
        }
    }

    /// Get the device state as stored by asusd
    // #[zbus(property)]
    async fn device_state(&self) -> DeviceState {
        DeviceState::from(&*self.0.config.lock().await)
    }
}

/// Computes the effective display enable state and brightness according to policy precedence:
/// 1. Master switch (`display_enabled == false`) -> Always disabled (`false, Brightness::Off`).
/// 2. System suspend (`sleeping == true` and `off_when_suspended == true`) -> Disabled (`false, Brightness::Off`).
/// 3. Lid closed (`lid_closed == true` and `off_when_lid_closed == true`) -> Disabled (`false, Brightness::Off`).
/// 4. Battery / Unplugged (`power_plugged == false`):
///    - If `off_when_unplugged == true` -> Disabled (`false, Brightness::Off`).
///    - Else -> Enabled with `brightness_on_battery` (`true, brightness_on_battery`).
/// 5. AC Power / Default -> Enabled with `display_brightness` (`true, display_brightness`).
pub fn compute_effective_state(
    config: &AniMeConfig,
    lid_closed: bool,
    power_plugged: bool,
    sleeping: bool,
) -> (bool, Brightness) {
    if !config.display_enabled {
        return (false, Brightness::Off);
    }
    if sleeping && config.off_when_suspended {
        return (false, Brightness::Off);
    }
    if lid_closed && config.off_when_lid_closed {
        return (false, Brightness::Off);
    }
    if !power_plugged && config.off_when_unplugged {
        return (false, Brightness::Off);
    }

    let brightness = if power_plugged {
        config.display_brightness
    } else {
        config.brightness_on_battery
    };

    if brightness == Brightness::Off {
        (false, Brightness::Off)
    } else {
        (true, brightness)
    }
}

impl AniMe {
    pub async fn apply_effective_state(
        &self,
        lid_closed: bool,
        power_plugged: bool,
        sleeping: bool,
        resuming: bool,
    ) -> Result<(), RogError> {
        let config = self.config.lock().await.clone();
        let (enable, brightness) =
            compute_effective_state(&config, lid_closed, power_plugged, sleeping);

        self.thread_exit.store(true, Ordering::Release);

        self.write_bytes(&pkt_set_brightness(brightness))
            .await
            .map_err(|err| warn!("apply_effective_state::brightness {}", err))
            .ok();

        self.write_bytes(&pkt_set_enable_display(enable))
            .await
            .map_err(|err| warn!("apply_effective_state::enable_display {}", err))
            .ok();

        if config.builtin_anims_enabled {
            self.write_bytes(&pkt_set_enable_powersave_anim(enable))
                .await
                .map_err(|err| warn!("apply_effective_state::powersave_anim {}", err))
                .ok();
        } else if resuming && enable {
            self.write_bytes(&pkt_set_enable_powersave_anim(false))
                .await
                .ok();
            let inner = self.clone();
            let action = self.cache.wake.clone();
            let token = self.cancel_token.lock().await.clone();
            let thread_exit = self.thread_exit.clone();
            // Non-blocking wake animation task tied to CancellationToken
            tokio::spawn(async move {
                // allow the new animation thread to start by clearing the exit flag
                thread_exit.store(false, Ordering::Release);
                if let Some(token) = token {
                    tokio::select! {
                        _ = token.cancelled() => {
                            debug!("AniMe wake animation task cancelled due to device removal");
                            thread_exit.store(true, Ordering::Release);
                        }
                        _ = inner.run_thread(action, true) => {}
                    }
                } else {
                    inner.run_thread(action, true).await;
                }
            });
        }

        Ok(())
    }
}

impl crate::CtrlTask for AniMeZbus {
    fn zbus_path() -> &'static str {
        "/xyz/ljones/aura/anime"
    }

    async fn create_tasks(&self, _: SignalEmitter<'static>) -> Result<(), RogError> {
        let cancel_token = match self.0.cancel_token.lock().await.clone() {
            Some(token) => token,
            None => {
                error!("AniMeZbus::create_tasks failed: no CancellationToken associated with AniMe instance");
                return Err(RogError::Zbus(
                    zbus::fdo::Error::Failed("Missing CancellationToken".into()).into(),
                ));
            }
        };

        let inner1 = self.0.clone();
        let inner2 = self.0.clone();
        let inner3 = self.0.clone();
        let inner4 = self.0.clone();

        let _handles = self
            .create_sys_event_tasks(
                cancel_token,
                move |sleeping: bool, lid_closed: bool, power_plugged: bool| {
                    let inner = inner1.clone();
                    async move {
                        inner
                            .apply_effective_state(lid_closed, power_plugged, sleeping, !sleeping)
                            .await
                            .ok();
                    }
                },
                move |shutting_down: bool, _lid_closed: bool, _power_plugged: bool| {
                    let inner = inner2.clone();
                    async move {
                        let AniMeConfig {
                            display_enabled,
                            builtin_anims_enabled,
                            ..
                        } = *inner.config.lock().await;
                        if display_enabled && !builtin_anims_enabled {
                            if shutting_down {
                                inner.run_thread(inner.cache.shutdown.clone(), true).await;
                            } else {
                                inner.run_thread(inner.cache.boot.clone(), true).await;
                            }
                        }
                    }
                },
                move |lid_closed: bool, power_plugged: bool| {
                    let inner = inner3.clone();
                    async move {
                        inner
                            .apply_effective_state(lid_closed, power_plugged, false, false)
                            .await
                            .ok();
                    }
                },
                move |power_plugged: bool, lid_closed: bool| {
                    let inner = inner4.clone();
                    async move {
                        inner
                            .apply_effective_state(lid_closed, power_plugged, false, false)
                            .await
                            .ok();
                    }
                },
            )
            .await;

        Ok(())
    }
}

impl crate::Reloadable for AniMeZbus {
    async fn reload(&mut self) -> Result<(), RogError> {
        let config = self.0.config.lock().await.clone();

        if config.builtin_anims_enabled {
            self.0
                .write_bytes(&pkt_set_builtin_animations(
                    config.builtin_anims.boot,
                    config.builtin_anims.awake,
                    config.builtin_anims.sleep,
                    config.builtin_anims.shutdown,
                ))
                .await?;
        }

        let connection = match Connection::system().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Reloadable::reload: could not create dbus connection: {e:?}");
                return Ok(());
            }
        };
        let manager = match ManagerProxy::builder(&connection)
            .cache_properties(CacheProperties::No)
            .build()
            .await
        {
            Ok(m) => m,
            Err(e) => {
                warn!("Reloadable::reload: could not create ManagerProxy: {e:?}");
                return Ok(());
            }
        };

        let lid_closed = manager.lid_closed().await.unwrap_or_default();
        let power_plugged = manager.on_external_power().await.unwrap_or_default();

        let (enable, brightness) =
            compute_effective_state(&config, lid_closed, power_plugged, false);

        if config.builtin_anims_enabled {
            self.0.set_builtins_enabled(enable, brightness).await?;
        } else {
            self.0
                .apply_effective_state(lid_closed, power_plugged, false, false)
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rog_anime::usb::{pkt_set_brightness, pkt_set_enable_display, Brightness};

    #[test]
    fn test_compute_effective_state_policy_precedence_and_transitions() {
        // Master switch disabled
        let config_disabled = AniMeConfig {
            display_enabled: false,
            display_brightness: Brightness::High,
            ..Default::default()
        };
        assert_eq!(
            compute_effective_state(&config_disabled, false, true, false),
            (false, Brightness::Off)
        );

        // Policy precedence and state transitions
        let config = AniMeConfig {
            display_enabled: true,
            display_brightness: Brightness::High,
            brightness_on_battery: Brightness::Low,
            off_when_suspended: true,
            off_when_lid_closed: true,
            off_when_unplugged: true,
            ..Default::default()
        };

        // AC, Lid open -> (true, High)
        assert_eq!(
            compute_effective_state(&config, false, true, false),
            (true, Brightness::High)
        );

        // Suspend -> (false, Off)
        assert_eq!(
            compute_effective_state(&config, false, true, true),
            (false, Brightness::Off)
        );

        // Lid closed -> (false, Off)
        assert_eq!(
            compute_effective_state(&config, true, true, false),
            (false, Brightness::Off)
        );

        // Unplugged on battery -> (false, Off)
        assert_eq!(
            compute_effective_state(&config, false, false, false),
            (false, Brightness::Off)
        );
    }

    #[test]
    fn test_compute_effective_state_off_when_unplugged_false_battery_low() {
        let config = AniMeConfig {
            display_enabled: true,
            off_when_unplugged: false,
            brightness_on_battery: Brightness::Low,
            ..Default::default()
        };

        let (enable, brightness) = compute_effective_state(&config, false, false, false);
        assert!(enable);
        assert_eq!(brightness, Brightness::Low);
    }

    #[test]
    fn test_compute_effective_state_brightness_on_battery_off() {
        let config = AniMeConfig {
            display_enabled: true,
            off_when_unplugged: false,
            brightness_on_battery: Brightness::Off,
            ..Default::default()
        };

        let (enable, brightness) = compute_effective_state(&config, false, false, false);
        assert!(!enable);
        assert_eq!(brightness, Brightness::Off);

        let bright_pkt = pkt_set_brightness(brightness);
        assert_eq!(bright_pkt[3], 0x00); // Brightness::Off

        let display_pkt = pkt_set_enable_display(enable);
        assert_eq!(display_pkt[3], 0x80); // Disabled
    }
}
