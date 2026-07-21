use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use log::error;

use config_traits::StdConfig;
use rog_anime::error::AnimeError;
use rog_anime::{ActionData, ActionLoader, AnimTime, Fade, Sequences, Vec2};
use rog_dbus::zbus_anime::AnimeProxyBlocking;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use zbus::interface;
use zbus::zvariant::{ObjectPath, Type};

use crate::config::ConfigAnime;
use crate::error::Error;

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
pub struct Timer {
    type_of: TimeType,
    /// If time type is Timer then this is milliseonds, otherwise it is
    /// animation loop count
    count: u64,
    /// Used only for `TimeType::Timer`, milliseonds to fade the image in for
    fade_in: u64,
    /// Used only for `TimeType::Timer`, milliseonds to fade the image out for
    fade_out: u64,
}

impl From<Timer> for AnimTime {
    fn from(time: Timer) -> Self {
        match time.type_of {
            TimeType::Timer => {
                if time.fade_in != 0 && time.fade_out != 0 {
                    let fade_in = Duration::from_millis(time.fade_in);
                    let fade_out = Duration::from_millis(time.fade_out);
                    let show_for = if time.count != 0 {
                        Some(Duration::from_millis(time.count))
                    } else {
                        None
                    };
                    AnimTime::Fade(Fade::new(fade_in, show_for, fade_out))
                } else {
                    AnimTime::Time(Duration::from_millis(time.count))
                }
            }
            TimeType::Count => AnimTime::Count(time.count as u32),
            TimeType::Infinite => AnimTime::Infinite,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
pub enum TimeType {
    Timer,
    Count,
    Infinite,
}

/// The inner object exists to allow the zbus proxy to share it with a runner
/// thread and a zbus server behind `Arc<Mutex<T>>`
pub struct CtrlAnimeInner<'a> {
    sequences: Sequences,
    client: AnimeProxyBlocking<'a>,
    do_early_return: Arc<AtomicBool>,
}

impl CtrlAnimeInner<'static> {
    pub fn new(
        sequences: Sequences,
        client: AnimeProxyBlocking<'static>,
        do_early_return: Arc<AtomicBool>,
    ) -> Result<Self, Error> {
        Ok(Self {
            sequences,
            client,
            do_early_return,
        })
    }

    /// To be called on each main loop iteration to pump out commands to the
    /// anime
    pub fn run(&self) -> Result<(), Error> {
        if self.do_early_return.load(Ordering::SeqCst) {
            return Ok(());
        }

        for action in self.sequences.iter() {
            match action {
                ActionData::Animation(frames) => {
                    rog_anime::run_animation(frames, &|output| {
                        if self.do_early_return.load(Ordering::Acquire) {
                            return Ok(true); // Do safe exit
                        }
                        self.client
                            .write(output)
                            .map_err(|e| AnimeError::Dbus(format!("{}", e)))
                            .map(|_| false)
                    });
                }
                ActionData::Image(image) => {
                    self.client.write(image.as_ref().clone()).ok();
                }
                ActionData::Pause(duration) => {
                    let start = Instant::now();
                    'pause: loop {
                        if self.do_early_return.load(Ordering::SeqCst) {
                            return Ok(());
                        }
                        if Instant::now().duration_since(start) > *duration {
                            break 'pause;
                        }
                        sleep(Duration::from_millis(1));
                    }
                }
                ActionData::AudioEq
                | ActionData::SystemInfo
                | ActionData::TimeDate
                | ActionData::Matrix => {}
            }
        }

        Ok(())
    }
}

pub struct CtrlAnime<'a> {
    config: Arc<Mutex<ConfigAnime>>,
    client: AnimeProxyBlocking<'a>,
    inner: Arc<Mutex<CtrlAnimeInner<'a>>>,
    /// Must be the same Atomic as in CtrlAnimeInner
    inner_early_return: Arc<AtomicBool>,
}

impl CtrlAnime<'static> {
    pub fn new(
        config: Arc<Mutex<ConfigAnime>>,
        inner: Arc<Mutex<CtrlAnimeInner<'static>>>,
        client: AnimeProxyBlocking<'static>,
        inner_early_return: Arc<AtomicBool>,
    ) -> Result<Self, Error> {
        Ok(CtrlAnime {
            config,
            client,
            inner,
            inner_early_return,
        })
    }

    pub async fn add_to_server(self, server: &mut zbus::Connection) {
        server
            .object_server()
            .at(&ObjectPath::from_str_unchecked("/xyz/ljones/Anime"), self)
            .await
            .map_err(|err| {
                error!("CtrlAnime: add_to_server {}", err);
                err
            })
            .ok();
    }
}

// The pattern for a zbus method is:
// - Get config lock if required
// - Set inner_early_return to stop the inner run loop temporarily
// - Do actions
// - Write config if required
// - Unset inner_early_return
#[interface(name = "xyz.ljones.Asusd")]
impl CtrlAnime<'static> {
    pub fn insert_asus_gif(
        &mut self,
        index: u32,
        file: &str,
        time: Timer,
        brightness: f32,
    ) -> zbus::fdo::Result<String> {
        if let Ok(mut config) = self.config.try_lock() {
            let time: AnimTime = time.into();
            let file = Path::new(&file);
            let action = ActionLoader::AsusAnimation {
                file: file.into(),
                brightness,
                time,
            };

            // Must make the inner run loop return early
            self.inner_early_return.store(true, Ordering::SeqCst);

            if let Ok(mut controller) = self.inner.lock() {
                controller
                    .sequences
                    .insert(index as usize, &action)
                    .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
            }
            config.anime.push(action);
            config.write();

            let ron = ron::ser::to_string_pretty(&*config, PrettyConfig::new().depth_limit(4))
                .expect("Parse config to RON failed");

            // Release the inner run loop again
            self.inner_early_return.store(false, Ordering::SeqCst);
            return Ok(ron);
        }
        Err(zbus::fdo::Error::Failed("UserConfig lock fail".into()))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_image_gif(
        &mut self,
        index: u32,
        file: &str,
        scale: f32,
        angle: f32,
        xy: (f32, f32),
        time: Timer,
        brightness: f32,
    ) -> zbus::fdo::Result<String> {
        if let Ok(mut config) = self.config.try_lock() {
            let time: AnimTime = time.into();
            let file = Path::new(&file);
            let translation = Vec2::new(xy.0, xy.1);
            let action = ActionLoader::ImageAnimation {
                file: file.into(),
                scale,
                angle,
                translation,
                brightness,
                time,
            };

            // Must make the inner run loop return early
            self.inner_early_return.store(true, Ordering::SeqCst);

            if let Ok(mut controller) = self.inner.lock() {
                controller
                    .sequences
                    .insert(index as usize, &action)
                    .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
            }
            config.anime.push(action);
            config.write();

            let ron = ron::ser::to_string_pretty(&*config, PrettyConfig::new().depth_limit(4))
                .expect("Parse config to RON failed");
            // Release the inner run loop again
            self.inner_early_return.store(false, Ordering::SeqCst);
            return Ok(ron);
        }
        Err(zbus::fdo::Error::Failed("UserConfig lock fail".into()))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_image(
        &mut self,
        index: u32,
        file: &str,
        scale: f32,
        angle: f32,
        xy: (f32, f32),
        time: Timer,
        brightness: f32,
    ) -> zbus::fdo::Result<String> {
        if let Ok(mut config) = self.config.try_lock() {
            let file = Path::new(&file);
            let time = time.into();
            let action = ActionLoader::Image {
                file: file.into(),
                scale,
                angle,
                translation: Vec2::new(xy.0, xy.1),
                brightness,
                time,
            };

            // Must make the inner run loop return early
            self.inner_early_return.store(true, Ordering::SeqCst);

            if let Ok(mut controller) = self.inner.lock() {
                controller
                    .sequences
                    .insert(index as usize, &action)
                    .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
            }
            config.anime.push(action);
            config.write();

            let ron = ron::ser::to_string_pretty(&*config, PrettyConfig::new().depth_limit(4))
                .expect("Parse config to RON failed");
            // Release the inner run loop again
            self.inner_early_return.store(false, Ordering::SeqCst);
            return Ok(ron);
        }
        Err(zbus::fdo::Error::Failed("UserConfig lock fail".into()))
    }

    pub fn insert_pause(&mut self, index: u32, millis: u64) -> zbus::fdo::Result<String> {
        if let Ok(mut config) = self.config.try_lock() {
            let action = ActionLoader::Pause(Duration::from_millis(millis));
            // Must make the inner run loop return early
            self.inner_early_return.store(true, Ordering::SeqCst);

            if let Ok(mut controller) = self.inner.lock() {
                controller
                    .sequences
                    .insert(index as usize, &action)
                    .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
            }
            config.anime.push(action);
            config.write();

            let ron = ron::ser::to_string_pretty(&*config, PrettyConfig::new().depth_limit(4))
                .expect("Parse config to RON failed");
            // Release the inner run loop again
            self.inner_early_return.store(false, Ordering::SeqCst);
            return Ok(ron);
        }
        Err(zbus::fdo::Error::Failed("UserConfig lock fail".into()))
    }

    pub fn remove_item(&mut self, index: u32) -> zbus::fdo::Result<String> {
        if let Ok(mut config) = self.config.try_lock() {
            // Must make the inner run loop return early
            self.inner_early_return.store(true, Ordering::SeqCst);

            if let Ok(mut controller) = self.inner.lock() {
                controller.sequences.remove_item(index as usize);
            }
            if (index as usize) < config.anime.len() {
                config.anime.remove(index as usize);
            }
            config.write();

            let ron = ron::ser::to_string_pretty(&*config, PrettyConfig::new().depth_limit(4))
                .expect("Parse config to RON failed");
            // Release the inner run loop again
            self.inner_early_return.store(false, Ordering::SeqCst);
            return Ok(ron);
        }
        Err(zbus::fdo::Error::Failed("UserConfig lock fail".into()))
    }

    pub fn set_state(&mut self, on: bool) -> zbus::fdo::Result<()> {
        // Operations here need to be in specific order
        if on {
            self.client.set_enable_display(on).ok();
            // Let the inner loop run
            self.inner_early_return.store(false, Ordering::SeqCst);
        } else {
            // Must make the inner run loop return early
            self.inner_early_return.store(true, Ordering::SeqCst);
            self.client.set_enable_display(on).ok();
        }
        Ok(())
    }
}
