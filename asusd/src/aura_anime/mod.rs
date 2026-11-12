pub mod config;
/// Implements `CtrlTask`, Reloadable, `ZbusRun`
pub mod trait_impls;

use std::convert::TryFrom;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;

use config_traits::StdConfig;
use log::{debug, error, info, warn};
use rog_anime::error::AnimeError;
use rog_anime::usb::{
    AnimAwake, AnimBooting, AnimShutdown, AnimSleeping, Brightness, pkt_flush, pkt_set_brightness,
    pkt_set_builtin_animations, pkt_set_enable_display, pkt_set_enable_powersave_anim,
    pkts_for_init,
};
use rog_anime::{ActionData, Animations, AnimeDataBuffer, AnimePacketType};
use rog_platform::DynamicLed;
use rog_platform::usb_raw::USBRaw;
use tokio::sync::Mutex;

use self::config::{AniMeConfig, AniMeConfigCached};
use crate::error::RogError;

#[derive(Debug, Clone)]
pub struct AniMe {
    usb: Option<Arc<Mutex<USBRaw>>>,
    dynamic: Option<Arc<Mutex<DynamicLed>>>,
    config: Arc<Mutex<AniMeConfig>>,
    cache: AniMeConfigCached,
    // set to force thread to exit
    thread_exit: Arc<AtomicBool>,
    // Set to false when the thread exits
    thread_running: Arc<AtomicBool>,
}

impl AniMe {
    pub fn new(
        usb: Option<Arc<Mutex<USBRaw>>>,
        dynamic: Option<Arc<Mutex<DynamicLed>>>,
        config: Arc<Mutex<AniMeConfig>>,
    ) -> Self {
        Self {
            usb,
            dynamic,
            config,
            cache: AniMeConfigCached::default(),
            thread_exit: Arc::new(AtomicBool::new(false)),
            thread_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn has_dynamic_lighting(&self) -> bool {
        self.dynamic.is_some()
    }

    /// Will fail if something is already holding the config lock
    async fn do_init_cache(&mut self) {
        if let Ok(mut config) = self.config.try_lock() {
            if let Err(e) = self.cache.init_from_config(&config, config.anime_type) {
                error!(
                    "Trying to cache the Anime Config failed, will reset to default config: {e:?}"
                );
                config.rename_file_old();
                *config = AniMeConfig::new();
                config.write();
            } else {
                debug!("Initialised AniMe cache");
            }
        } else {
            error!("AniMe Matrix could not init cache")
        }
    }

    /// Initialise the device if required.
    pub async fn do_initialization(&mut self) -> Result<(), RogError> {
        self.do_init_cache().await;
        if self.has_dynamic_lighting() {
            // hid-asus already ran AniMe init when registering asus::anime.
            debug!("AniMe Dynamic Lighting ready (kernel owns init)");
            return Ok(());
        }
        let pkts = pkts_for_init();
        self.write_usb(&pkts[0]).await?;
        self.write_usb(&pkts[1]).await?;
        debug!("Succesfully initialised AniMe matrix display");
        Ok(())
    }

    async fn write_usb(&self, message: &[u8]) -> Result<(), RogError> {
        let Some(usb) = &self.usb else {
            return Err(RogError::Anime(AnimeError::NoDevice));
        };
        usb.lock().await.write_bytes(message)?;
        Ok(())
    }

    /// Legacy USB packet write. Prefer the typed helpers when Dual-path aware.
    pub async fn write_bytes(&self, message: &[u8]) -> Result<(), RogError> {
        if self.has_dynamic_lighting() {
            return Err(RogError::MissingFunction(
                "AniMe Dynamic Lighting owns the device; USB packets are not used".into(),
            ));
        }
        self.write_usb(message).await
    }

    pub async fn apply_brightness(&self, brightness: Brightness) -> Result<(), RogError> {
        if let Some(led) = &self.dynamic {
            let led = led.lock().await;
            let off = brightness == Brightness::Off;
            if led.has_enabled() {
                led.set_lighting_enabled(!off)?;
            }
            if !off || !led.has_enabled() {
                let max = led.get_max_brightness().unwrap_or(3).max(1);
                // AniMe Brightness is already 0..=3; scale only when the node is wider.
                let level = if max <= 3 {
                    brightness as u8
                } else {
                    match brightness {
                        Brightness::Off => 0,
                        Brightness::Low => (max / 3).max(1),
                        Brightness::Med => ((max as u16 * 2) / 3) as u8,
                        Brightness::High => max,
                    }
                };
                led.set_brightness(level)?;
            }
            return Ok(());
        }
        self.write_usb(&pkt_set_brightness(brightness)).await?;
        self.write_usb(&pkt_set_enable_display(brightness != Brightness::Off))
            .await
    }

    pub async fn apply_display_enabled(&self, enabled: bool) -> Result<(), RogError> {
        if let Some(led) = &self.dynamic {
            let led = led.lock().await;
            if led.has_enabled() {
                led.set_lighting_enabled(enabled)?;
            }
            return Ok(());
        }
        self.write_usb(&pkt_set_enable_display(enabled)).await
    }

    pub async fn apply_builtin_animations(&self, settings: Animations) -> Result<(), RogError> {
        if let Some(led) = &self.dynamic {
            let led = led.lock().await;
            // Each stage write updates only that firmware slot.
            for name in [
                boot_effect_name(settings.boot),
                awake_effect_name(settings.awake),
                sleep_effect_name(settings.sleep),
                shutdown_effect_name(settings.shutdown),
            ] {
                if led.supports_effect(name)? {
                    led.set_effect(name)?;
                }
            }
            return Ok(());
        }
        self.write_usb(&pkt_set_builtin_animations(
            settings.boot, settings.awake, settings.sleep, settings.shutdown,
        ))
        .await
    }

    /// Enable or disable builtin stage animations.
    ///
    /// `settings` is required when enabling so callers that already hold the
    /// config lock can pass a copy instead of re-entering the mutex.
    pub async fn set_builtins_enabled(
        &self,
        enabled: bool,
        bright: Brightness,
        settings: Animations,
    ) -> Result<(), RogError> {
        if let Some(led) = &self.dynamic {
            let led = led.lock().await;
            if enabled {
                for name in [
                    boot_effect_name(settings.boot),
                    awake_effect_name(settings.awake),
                    sleep_effect_name(settings.sleep),
                    shutdown_effect_name(settings.shutdown),
                ] {
                    if led.supports_effect(name)? {
                        led.set_effect(name)?;
                    }
                }
            } else if led.supports_effect("direct")? {
                led.set_effect("direct")?;
            }
            drop(led);
            self.apply_brightness(bright).await?;
            return Ok(());
        }
        // Custom frames need powersave anims off without blanking the panel.
        self.write_usb(&pkt_set_enable_powersave_anim(enabled))
            .await?;
        if enabled {
            self.write_usb(&pkt_set_enable_display(true)).await?;
            self.write_usb(&pkt_set_brightness(bright)).await?;
            self.write_usb(&pkt_set_builtin_animations(
                settings.boot, settings.awake, settings.sleep, settings.shutdown,
            ))
            .await?;
            return self.write_usb(&pkt_set_enable_powersave_anim(true)).await;
        }
        self.write_usb(&pkt_set_brightness(bright)).await
    }

    /// Write a direct intensity frame. Scales pixel values and uses DL
    /// `direct_buffer` (1 byte/LED) when present.
    pub(crate) async fn write_data_buffer(
        &self,
        mut buffer: AnimeDataBuffer,
    ) -> Result<(), RogError> {
        for led in buffer.data_mut().iter_mut() {
            let mut bright = *led as f32;
            if bright > 254.0 {
                bright = 254.0;
            }
            *led = bright as u8;
        }

        if let Some(led) = &self.dynamic {
            let led = led.lock().await;
            // Do not force the panel on: Off stays Off until brightness/enable.
            if led.supports_effect("direct")?
                && led.get_effect().ok().as_deref().map(str::trim) != Some("direct")
            {
                led.set_effect("direct")?;
            }
            let led_count = led.get_led_count()? as usize;
            let data = buffer.data();
            if data.len() != led_count {
                return Err(RogError::Anime(AnimeError::DataBufferLength));
            }
            led.write_direct(data)?;
            return Ok(());
        }

        let data = AnimePacketType::try_from(buffer)?;
        for row in &data {
            self.write_usb(row).await?;
        }
        self.write_usb(&pkt_flush()).await
    }

    /// Start an action thread. This is classed as a singleton and there should
    /// be only one running - so the thread uses atomics to signal run/exit.
    ///
    /// Because this also writes to the usb device, other write tries (display
    /// only) *must* get the mutex lock and set the `thread_exit` atomic.
    pub async fn run_thread(&self, actions: Vec<ActionData>, mut once: bool) {
        if actions.is_empty() {
            warn!("AniMe system actions was empty");
            return;
        }

        let (bright, settings, anime_type) = {
            let config = self.config.lock().await;
            (
                config.display_brightness,
                config.builtin_anims,
                config.anime_type,
            )
        };
        if let Err(err) = self.set_builtins_enabled(false, bright, settings).await {
            warn!("rog_anime::run_animation: disable builtins {}", err);
        }

        let thread_exit = self.thread_exit.clone();
        let thread_running = self.thread_running.clone();
        let inner = self.clone();

        // Loop rules:
        // - Lock the mutex **only when required**. That is, the lock must be held for
        //   the shortest duration possible.
        // - An AtomicBool used for thread exit should be checked in every loop,
        //   including nested

        // The only reason for this outer thread is to prevent blocking while waiting
        // for the next spawned thread to exit
        tokio::spawn(async move {
            info!("AniMe new system thread started");
            while thread_running.load(Ordering::SeqCst) {
                thread_exit.store(true, Ordering::SeqCst);
            }

            info!("AniMe no previous system thread running (now)");
            thread_exit.store(false, Ordering::SeqCst);
            thread_running.store(true, Ordering::SeqCst);
            'main: loop {
                for action in &actions {
                    if thread_exit.load(Ordering::SeqCst) {
                        break 'main;
                    }
                    match action {
                        ActionData::Animation(frames) => {
                            rog_anime::run_animation(frames, &|frame| {
                                if thread_exit.load(Ordering::Acquire) {
                                    info!("rog-anime: animation sub-loop was asked to exit");
                                    return Ok(true);
                                }
                                let inner = inner.clone();
                                tokio::task::spawn(async move {
                                    inner
                                        .write_data_buffer(frame)
                                        .await
                                        .map_err(|err| {
                                            warn!("rog_anime::run_animation:callback {}", err);
                                        })
                                        .ok();
                                });
                                Ok(false)
                            });
                            if thread_exit.load(Ordering::Acquire) {
                                info!("rog-anime: sub-loop exited and main loop exiting now");
                                break 'main;
                            }
                        }
                        ActionData::Image(image) => {
                            once = false;
                            inner
                                .write_data_buffer(image.as_ref().clone())
                                .await
                                .map_err(|e| error!("{}", e))
                                .ok();
                        }
                        ActionData::Pause(duration) => sleep(*duration),
                        ActionData::AudioEq
                        | ActionData::SystemInfo
                        | ActionData::TimeDate
                        | ActionData::Matrix => {}
                    }
                }
                if thread_exit.load(Ordering::SeqCst) {
                    break 'main;
                }
                if once || actions.is_empty() {
                    break 'main;
                }
            }
            if let Ok(data) =
                AnimeDataBuffer::from_vec(anime_type, vec![0u8; anime_type.data_length()])
                    .map_err(|e| error!("{}", e))
            {
                inner
                    .write_data_buffer(data)
                    .await
                    .map_err(|err| {
                        warn!("rog_anime::run_animation:callback {}", err);
                    })
                    .ok();
            }
            let (enabled, bright, settings) = {
                let config = inner.config.lock().await;
                (
                    config.builtin_anims_enabled,
                    config.display_brightness,
                    config.builtin_anims,
                )
            };
            inner
                .set_builtins_enabled(enabled, bright, settings)
                .await
                .map_err(|err| {
                    warn!("rog_anime::run_animation:callback {}", err);
                })
                .ok();
            thread_running.store(false, Ordering::SeqCst);
            info!("AniMe system thread exited");
        })
        .await
        .map(|err| info!("AniMe system thread: {:?}", err))
        .ok();
    }
}

fn boot_effect_name(anim: AnimBooting) -> &'static str {
    match anim {
        AnimBooting::GlitchConstruction => "boot_glitch_construction",
        AnimBooting::StaticEmergence => "boot_static_emergence",
    }
}

fn awake_effect_name(anim: AnimAwake) -> &'static str {
    match anim {
        AnimAwake::BinaryBannerScroll => "awake_binary_banner_scroll",
        AnimAwake::RogLogoGlitch => "awake_rog_logo_glitch",
    }
}

fn sleep_effect_name(anim: AnimSleeping) -> &'static str {
    match anim {
        AnimSleeping::BannerSwipe => "sleep_banner_swipe",
        AnimSleeping::Starfield => "sleep_starfield",
    }
}

fn shutdown_effect_name(anim: AnimShutdown) -> &'static str {
    match anim {
        AnimShutdown::GlitchOut => "shutdown_glitch_out",
        AnimShutdown::SeeYa => "shutdown_see_ya",
    }
}
