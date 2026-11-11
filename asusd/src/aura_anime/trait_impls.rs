use std::sync::atomic::Ordering;

use config_traits::StdConfig;
use log::{debug, warn};
use logind_zbus::manager::ManagerProxy;
use rog_anime::usb::{
    Brightness, pkt_set_brightness, pkt_set_builtin_animations, pkt_set_enable_display,
    pkt_set_enable_powersave_anim,
};
use rog_anime::{Animations, AnimeDataBuffer, DeviceState};
use zbus::object_server::SignalEmitter;
use zbus::proxy::CacheProperties;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, interface};

use super::AniMe;
use super::config::AniMeConfig;
use crate::error::RogError;
use crate::start_hotplug_tasks;

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

/// Keep the panel off when the user disabled it or an active lid/power
/// restriction applies. Shared by `create_tasks` handlers and `reload`.
fn anime_display_allowed(
    display_enabled: bool,
    lid_closed: bool,
    off_when_lid_closed: bool,
    power_plugged: bool,
    off_when_unplugged: bool,
) -> bool {
    if !display_enabled {
        return false;
    }
    let restricted = (lid_closed && off_when_lid_closed) || (!power_plugged && off_when_unplugged);
    !restricted
}

/// Combine lid/power allowance with the suspend-off policy.
fn anime_sleep_enable(sleeping: bool, off_when_suspended: bool, allowed: bool) -> bool {
    allowed && !(sleeping && off_when_suspended)
}

#[derive(Clone)]
pub struct AniMeZbus(AniMe);

impl AniMeZbus {
    pub fn new(anime: AniMe) -> Self {
        Self(anime)
    }

    pub async fn start_tasks(
        self,
        connection: &Connection,
        path: OwnedObjectPath,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), RogError> {
        start_hotplug_tasks(self, connection, path, cancel).await?;
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

impl crate::CtrlTask for AniMeZbus {
    fn zbus_path() -> &'static str {
        "ANIME_ZBUS_PATH"
    }

    async fn create_tasks(
        &self,
        _: SignalEmitter<'static>,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), RogError> {
        let inner1 = self.0.clone();
        let inner2 = self.0.clone();
        let inner3 = self.0.clone();
        let inner4 = self.0.clone();
        // One system-bus proxy for all event handlers (Clone is cheap).
        let manager = get_logind_manager().await;
        let sleep_mgr = manager.clone();
        let lid_mgr = manager.clone();
        let power_mgr = manager;
        self.create_sys_event_tasks(
            cancel,
            move |sleeping| {
                // on_sleep
                let inner = inner1.clone();
                let manager = sleep_mgr.clone();
                async move {
                    let config = inner.config.lock().await.clone();
                    if !config.display_enabled {
                        return;
                    }
                    inner.thread_exit.store(true, Ordering::Release); // ensure clean slate

                    // Suspend-off must not depend on D-Bus: a failed query must not
                    // skip the off write and leave the panel lit during sleep.
                    let enable = if sleeping && config.off_when_suspended {
                        false
                    } else {
                        match (
                            manager.lid_closed().await,
                            manager.on_external_power().await,
                        ) {
                            (Ok(lid_closed), Ok(power_plugged)) => anime_sleep_enable(
                                sleeping,
                                config.off_when_suspended,
                                anime_display_allowed(
                                    true,
                                    lid_closed,
                                    config.off_when_lid_closed,
                                    power_plugged,
                                    config.off_when_unplugged,
                                ),
                            ),
                            _ => {
                                // Resume (or sleep without suspend-off): do not leave
                                // the panel stuck off after a failed lid/power query.
                                warn!(
                                    "create_sys_event_tasks::sleep: lid/power query failed; \
                                     using display_enabled only"
                                );
                                true
                            }
                        }
                    };

                    if config.builtin_anims_enabled {
                        inner
                            .write_bytes(&pkt_set_enable_powersave_anim(enable))
                            .await
                            .map_err(|err| {
                                warn!("create_sys_event_tasks::off_when_suspended {}", err);
                            })
                            .ok();
                    } else if !sleeping && enable {
                        // Run custom wake animation
                        inner
                            .write_bytes(&pkt_set_enable_powersave_anim(false))
                            .await
                            .ok(); // ensure builtins are disabled

                        inner.run_thread(inner.cache.wake.clone(), true).await;
                    }

                    inner
                        .write_bytes(&pkt_set_enable_display(enable))
                        .await
                        .map_err(|err| {
                            warn!("create_sys_event_tasks::off_when_suspended {}", err);
                        })
                        .ok();
                }
            },
            move |shutting_down| {
                // on_shutdown
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
            move |lid_closed| {
                let inner = inner3.clone();
                let manager = lid_mgr.clone();
                // on lid change
                async move {
                    let AniMeConfig {
                        display_enabled,
                        off_when_lid_closed,
                        off_when_unplugged,
                        builtin_anims_enabled,
                        ..
                    } = *inner.config.lock().await;
                    if !off_when_lid_closed {
                        return;
                    }
                    let Ok(power_plugged) = manager.on_external_power().await else {
                        warn!("create_sys_event_tasks::lid: on_external_power query failed");
                        return;
                    };
                    let enable = anime_display_allowed(
                        display_enabled, lid_closed, true, power_plugged, off_when_unplugged,
                    );
                    if builtin_anims_enabled {
                        inner
                            .write_bytes(&pkt_set_enable_powersave_anim(enable))
                            .await
                            .map_err(|err| {
                                warn!("create_sys_event_tasks::off_when_lid_closed {}", err);
                            })
                            .ok();
                    }
                    inner
                        .write_bytes(&pkt_set_enable_display(enable))
                        .await
                        .map_err(|err| {
                            warn!("create_sys_event_tasks::off_when_lid_closed {}", err);
                        })
                        .ok();
                }
            },
            move |power_plugged| {
                let inner = inner4.clone();
                let manager = power_mgr.clone();
                // on power change
                async move {
                    let AniMeConfig {
                        display_enabled,
                        off_when_lid_closed,
                        off_when_unplugged,
                        builtin_anims_enabled,
                        brightness_on_battery,
                        display_brightness,
                        ..
                    } = *inner.config.lock().await;
                    if off_when_unplugged {
                        let Ok(lid_closed) = manager.lid_closed().await else {
                            warn!("create_sys_event_tasks::power: lid_closed query failed");
                            return;
                        };
                        let enable = anime_display_allowed(
                            display_enabled, lid_closed, off_when_lid_closed, power_plugged, true,
                        );
                        if builtin_anims_enabled {
                            inner
                                .write_bytes(&pkt_set_enable_powersave_anim(enable))
                                .await
                                .map_err(|err| {
                                    warn!("create_sys_event_tasks::off_when_unplugged {}", err);
                                })
                                .ok();
                        }
                        inner
                            .write_bytes(&pkt_set_enable_display(enable))
                            .await
                            .map_err(|err| {
                                warn!("create_sys_event_tasks::off_when_unplugged {}", err);
                            })
                            .ok();
                    } else {
                        let brightness = if power_plugged {
                            display_brightness
                        } else {
                            brightness_on_battery
                        };
                        inner
                            .write_bytes(&pkt_set_brightness(brightness))
                            .await
                            .map_err(|err| {
                                warn!("create_sys_event_tasks::off_when_unplugged {}", err);
                            })
                            .ok();
                    }
                }
            },
        )
        .await;

        Ok(())
    }
}

impl crate::Reloadable for AniMeZbus {
    async fn reload(&mut self) -> Result<(), RogError> {
        let AniMeConfig {
            builtin_anims_enabled,
            builtin_anims,
            display_enabled,
            display_brightness,
            off_when_lid_closed,
            off_when_unplugged,
            ..
        } = *self.0.config.lock().await;

        // Set builtins
        if builtin_anims_enabled {
            self.0
                .write_bytes(&pkt_set_builtin_animations(
                    builtin_anims.boot,
                    builtin_anims.awake,
                    builtin_anims.sleep,
                    builtin_anims.shutdown,
                ))
                .await?;
        }
        // Builtins enabled or na?
        self.0
            .set_builtins_enabled(builtin_anims_enabled, display_brightness)
            .await?;

        let manager = get_logind_manager().await;
        let lid_closed = manager.lid_closed().await.unwrap_or_default();
        let power_plugged = manager.on_external_power().await.unwrap_or_default();

        let allowed = anime_display_allowed(
            display_enabled, lid_closed, off_when_lid_closed, power_plugged, off_when_unplugged,
        );
        self.0
            .write_bytes(&pkt_set_enable_display(allowed))
            .await
            .map_err(|err| {
                warn!("create_sys_event_tasks::reload {}", err);
            })
            .ok();

        if !allowed {
            self.0.write_bytes(&pkt_set_enable_display(false)).await?;
            // early return so we don't run animation thread
            return Ok(());
        }

        if !builtin_anims_enabled && !self.0.cache.boot.is_empty() {
            self.0
                .write_bytes(&pkt_set_enable_powersave_anim(false))
                .await
                .ok();

            let action = self.0.cache.boot.clone();
            self.0.run_thread(action, true).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{anime_display_allowed, anime_sleep_enable};

    #[test]
    fn rejects_when_display_disabled() {
        assert!(!anime_display_allowed(false, false, true, true, true));
    }

    #[test]
    fn rejects_lid_closed_with_policy() {
        assert!(!anime_display_allowed(true, true, true, true, true));
    }

    #[test]
    fn allows_lid_closed_without_policy() {
        assert!(anime_display_allowed(true, true, false, true, true));
    }

    #[test]
    fn rejects_unplugged_with_policy() {
        assert!(!anime_display_allowed(true, false, false, false, true));
    }

    #[test]
    fn allows_unplugged_without_policy() {
        assert!(anime_display_allowed(true, false, false, false, false));
    }

    #[test]
    fn allows_when_clear() {
        assert!(anime_display_allowed(true, false, true, true, true));
    }

    #[test]
    fn sleep_enable_four_combinations() {
        assert!(!anime_sleep_enable(true, true, true));
        assert!(anime_sleep_enable(true, false, true));
        assert!(anime_sleep_enable(false, true, true));
        assert!(anime_sleep_enable(false, false, true));
        assert!(!anime_sleep_enable(false, false, false));
    }

    #[test]
    fn resume_keeps_lid_restriction_when_suspend_off_disabled() {
        // Regression: lid closed + off_when_lid_closed, off_when_suspended=false, resume.
        let allowed = anime_display_allowed(true, true, true, true, true);
        assert!(!anime_sleep_enable(false, false, allowed));
    }
}
