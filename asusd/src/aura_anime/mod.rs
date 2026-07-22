pub mod config;
/// Implements `CtrlTask`, Reloadable, `ZbusRun`
pub mod trait_impls;

use std::convert::TryFrom;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;

use config_traits::StdConfig;
use log::{debug, error, info, warn};
use rog_anime::usb::{
    Brightness, pkt_flush, pkt_set_brightness, pkt_set_enable_display,
    pkt_set_enable_powersave_anim, pkts_for_init,
};
use rog_anime::{ActionData, AnimeDataBuffer, AnimePacketType};
use rog_platform::hid_raw::HidRaw;
use rog_platform::usb_raw::USBRaw;
use tokio::sync::Mutex;

use self::config::{AniMeConfig, AniMeConfigCached};
use crate::error::RogError;

#[derive(Debug, Clone)]
pub struct AniMe {
    hid: Option<Arc<Mutex<HidRaw>>>,
    usb: Option<Arc<Mutex<USBRaw>>>,
    config: Arc<Mutex<AniMeConfig>>,
    cache: AniMeConfigCached,
    // set to force thread to exit
    thread_exit: Arc<AtomicBool>,
    // Set to false when the thread exits
    thread_running: Arc<AtomicBool>,
}

impl AniMe {
    pub fn new(
        hid: Option<Arc<Mutex<HidRaw>>>,
        usb: Option<Arc<Mutex<USBRaw>>>,
        config: Arc<Mutex<AniMeConfig>>,
    ) -> Self {
        Self {
            hid,
            usb,
            config,
            cache: AniMeConfigCached::default(),
            thread_exit: Arc::new(AtomicBool::new(false)),
            thread_running: Arc::new(AtomicBool::new(false)),
        }
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
        let pkts = pkts_for_init();
        self.write_bytes(&pkts[0]).await?;
        self.write_bytes(&pkts[1]).await?;
        debug!("Succesfully initialised AniMe matrix display");
        Ok(())
    }

    pub async fn write_bytes(&self, message: &[u8]) -> Result<(), RogError> {
        if let Some(hid) = &self.hid {
            hid.lock().await.write_bytes(message)?;
        } else if let Some(usb) = &self.usb {
            usb.lock().await.write_bytes(message)?;
        }
        Ok(())
    }

    /// Write only a data packet. This will modify the leds brightness using the
    /// global brightness set in config.
    async fn write_data_buffer(&self, mut buffer: AnimeDataBuffer) -> Result<(), RogError> {
        for led in buffer.data_mut().iter_mut() {
            let mut bright = *led as f32;
            if bright > 254.0 {
                bright = 254.0;
            }
            *led = bright as u8;
        }
        let data = AnimePacketType::try_from(buffer)?;
        for row in &data {
            self.write_bytes(row).await?;
        }
        self.write_bytes(&pkt_flush()).await
    }

    pub async fn set_builtins_enabled(
        &self,
        enabled: bool,
        bright: Brightness,
    ) -> Result<(), RogError> {
        self.write_bytes(&pkt_set_enable_powersave_anim(enabled))
            .await?;
        self.write_bytes(&pkt_set_enable_display(enabled)).await?;
        self.write_bytes(&pkt_set_brightness(bright)).await?;
        self.write_bytes(&pkt_set_enable_powersave_anim(enabled))
            .await
    }

    /// Start an action thread. This is classed as a singleton and there should
    /// be only one running - so the thread uses atomics to signal run/exit.
    ///
    /// Because this also writes to the usb device, other write tries (display
    /// only) *must* get the mutex lock and set the `thread_exit` atomic.
    async fn run_thread(&self, actions: Vec<ActionData>, mut once: bool) {
        if actions.is_empty() {
            warn!("AniMe system actions was empty");
            return;
        }

        self.write_bytes(&pkt_set_enable_powersave_anim(false))
            .await
            .map_err(|err| {
                warn!("rog_anime::run_animation:callback {}", err);
            })
            .ok();

        let thread_exit = self.thread_exit.clone();
        let thread_running = self.thread_running.clone();
        let anime_type = self.config.lock().await.anime_type;
        let inner = self.clone();

        // Loop rules:
        // - Lock the mutex **only when required**. That is, the lock must be held for
        //   the shortest duration possible.
        // - An AtomicBool used for thread exit should be checked in every loop,
        //   including nested

        // The only reason for this outer thread is to prevent blocking while waiting
        // for the next spawned thread to exit
        // TODO: turn this in to async task (maybe? COuld still risk blocking main
        // thread)
        tokio::spawn(async move {
            info!("AniMe new system thread started");
            // First two loops are to ensure we *do* aquire a lock on the mutex
            // The reason the loop is required is because the USB writes can block
            // for up to 10ms. We can't fail to get the atomics.
            while thread_running.load(Ordering::SeqCst) {
                // Make any running loop exit first
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
                            // TODO: sort all this out
                            rog_anime::run_animation(frames, &|frame| {
                                if thread_exit.load(Ordering::Acquire) {
                                    info!("rog-anime: animation sub-loop was asked to exit");
                                    return Ok(true); // Do safe exit
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
                                Ok(false) // Don't exit yet
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
            // Clear the display on exit
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
            // A write can block for many milliseconds so lets not hold the config lock for
            // the same period
            let enabled = inner.config.lock().await.builtin_anims_enabled;
            inner
                .write_bytes(&pkt_set_enable_powersave_anim(enabled))
                .await
                .map_err(|err| {
                    warn!("rog_anime::run_animation:callback {}", err);
                })
                .ok();
            // Loop ended, set the atmonics
            thread_running.store(false, Ordering::SeqCst);
            info!("AniMe system thread exited");
        })
        .await
        .map(|err| info!("AniMe system thread: {:?}", err))
        .ok();
    }
}
