use std::sync::atomic::Ordering;

use config_traits::StdConfig;
use log::{debug, error, warn};
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
use crate::Reloadable;
use crate::error::RogError;

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
    ) -> Result<(), RogError> {
        // let task = zbus.clone();
        self.reload()
            .await
            .unwrap_or_else(|err| warn!("Controller error: {}", err));
        connection
            .object_server()
            .at(path.clone(), self)
            .await
            .map_err(|e| {
                error!("Couldn't add server at path: {path}, {e:?}");
                e
            })?;
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

    async fn create_tasks(&self, _: SignalEmitter<'static>) -> Result<(), RogError> {
        let inner1 = self.0.clone();
        let inner2 = self.0.clone();
        let inner3 = self.0.clone();
        let inner4 = self.0.clone();
        self.create_sys_event_tasks(
            move |sleeping| {
                // on_sleep
                let inner = inner1.clone();
                async move {
                    let config = inner.config.lock().await.clone();
                    if config.display_enabled {
                        inner.thread_exit.store(true, Ordering::Release); // ensure clean slate

                        inner
                            .write_bytes(&pkt_set_enable_display(
                                !(sleeping && config.off_when_suspended),
                            ))
                            .await
                            .map_err(|err| {
                                warn!("create_sys_event_tasks::off_when_suspended {}", err);
                            })
                            .ok();

                        if config.builtin_anims_enabled {
                            inner
                                .write_bytes(&pkt_set_enable_powersave_anim(
                                    !(sleeping && config.off_when_suspended),
                                ))
                                .await
                                .map_err(|err| {
                                    warn!("create_sys_event_tasks::off_when_suspended {}", err);
                                })
                                .ok();
                        } else if !sleeping && !config.builtin_anims_enabled {
                            // Run custom wake animation
                            inner
                                .write_bytes(&pkt_set_enable_powersave_anim(false))
                                .await
                                .ok(); // ensure builtins are disabled

                            inner.run_thread(inner.cache.wake.clone(), true).await;
                        }
                    }
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
                // on lid change
                async move {
                    let AniMeConfig {
                        off_when_lid_closed,
                        builtin_anims_enabled,
                        ..
                    } = *inner.config.lock().await;
                    if off_when_lid_closed {
                        if builtin_anims_enabled {
                            inner
                                .write_bytes(&pkt_set_enable_powersave_anim(!lid_closed))
                                .await
                                .map_err(|err| {
                                    warn!("create_sys_event_tasks::off_when_suspended {}", err);
                                })
                                .ok();
                        }
                        inner
                            .write_bytes(&pkt_set_enable_display(!lid_closed))
                            .await
                            .map_err(|err| {
                                warn!("create_sys_event_tasks::off_when_lid_closed {}", err);
                            })
                            .ok();
                    }
                }
            },
            move |power_plugged| {
                let inner = inner4.clone();
                // on power change
                async move {
                    let AniMeConfig {
                        off_when_unplugged,
                        builtin_anims_enabled,
                        brightness_on_battery,
                        ..
                    } = *inner.config.lock().await;
                    if off_when_unplugged {
                        if builtin_anims_enabled {
                            inner
                                .write_bytes(&pkt_set_enable_powersave_anim(power_plugged))
                                .await
                                .map_err(|err| {
                                    warn!("create_sys_event_tasks::off_when_suspended {}", err);
                                })
                                .ok();
                        }
                        inner
                            .write_bytes(&pkt_set_enable_display(power_plugged))
                            .await
                            .map_err(|err| {
                                warn!("create_sys_event_tasks::off_when_unplugged {}", err);
                            })
                            .ok();
                    } else {
                        inner
                            .write_bytes(&pkt_set_brightness(brightness_on_battery))
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

        let turn_off =
            (lid_closed && off_when_lid_closed) || (!power_plugged && off_when_unplugged);
        self.0
            .write_bytes(&pkt_set_enable_display(!turn_off))
            .await
            .map_err(|err| {
                warn!("create_sys_event_tasks::reload {}", err);
            })
            .ok();

        if turn_off || !display_enabled {
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
