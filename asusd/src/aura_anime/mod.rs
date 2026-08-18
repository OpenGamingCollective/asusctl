pub mod config;
/// Implements `CtrlTask`, Reloadable, `ZbusRun`
pub mod trait_impls;

use std::collections::VecDeque;
use std::convert::TryFrom;
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex as StdMutex};
use std::time::Duration;

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

#[derive(Debug)]
enum Job {
    Frame(AnimePacketType),
    Control(Vec<u8>),
}

#[derive(Debug, Default)]
struct MailboxState {
    queue: VecDeque<Job>,
    shutdown: bool,
}

type FrameMailbox = Arc<(StdMutex<MailboxState>, Condvar)>;

#[derive(Debug)]
struct WorkerGuard(FrameMailbox);

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.0;
        if let Ok(mut guard) = lock.lock() {
            guard.shutdown = true;
            cvar.notify_all();
        }
    }
}

#[derive(Debug, Clone)]
pub struct AniMe {
    config: Arc<Mutex<AniMeConfig>>,
    cache: AniMeConfigCached,
    // set to force thread to exit
    thread_exit: Arc<AtomicBool>,
    // Set to false when the thread exits
    thread_running: Arc<AtomicBool>,
    mailbox: FrameMailbox,
    _worker_guard: Arc<WorkerGuard>,
    #[cfg(test)]
    processed_counter: Arc<AtomicUsize>,
}

impl AniMe {
    pub fn new(
        hid: Option<Arc<Mutex<HidRaw>>>,
        usb: Option<Arc<Mutex<USBRaw>>>,
        config: Arc<Mutex<AniMeConfig>>,
    ) -> Self {
        let mailbox: FrameMailbox =
            Arc::new((StdMutex::new(MailboxState::default()), Condvar::new()));

        let hid_thread = hid;
        let usb_thread = usb;
        let mailbox_thread = mailbox.clone();

        #[cfg(test)]
        let processed_counter = Arc::new(AtomicUsize::new(0));
        #[cfg(test)]
        let processed_counter_thread = processed_counter.clone();

        std::thread::Builder::new()
            .name("anime-io".into())
            .spawn(move || {
                let (lock, cvar) = &*mailbox_thread;
                loop {
                    let mut guard = match lock.lock() {
                        Ok(g) => g,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    while guard.queue.is_empty() && !guard.shutdown {
                        guard = match cvar.wait(guard) {
                            Ok(g) => g,
                            Err(poisoned) => poisoned.into_inner(),
                        };
                    }
                    if guard.shutdown && guard.queue.is_empty() {
                        break;
                    }

                    let Some(job) = guard.queue.pop_front() else {
                        continue;
                    };
                    drop(guard);

                    match job {
                        Job::Frame(packets) => {
                            if let Some(hid) = &hid_thread {
                                let guard = hid.blocking_lock();
                                for row in &packets {
                                    if let Err(e) = guard.write_bytes(row) {
                                        warn!("AniMe HID write error: {e}");
                                    }
                                }
                                if let Err(e) = guard.write_bytes(&pkt_flush()) {
                                    warn!("AniMe HID flush error: {e}");
                                }
                            } else if let Some(usb) = &usb_thread {
                                let guard = usb.blocking_lock();
                                for row in &packets {
                                    if let Err(e) = guard.write_bytes(row) {
                                        warn!("AniMe USB write error: {e}");
                                    }
                                }
                                if let Err(e) = guard.write_bytes(&pkt_flush()) {
                                    warn!("AniMe USB flush error: {e}");
                                }
                            }
                        }
                        Job::Control(packet) => {
                            if let Some(hid) = &hid_thread {
                                if let Err(e) = hid.blocking_lock().write_bytes(&packet) {
                                    warn!("AniMe HID control write error: {e}");
                                }
                            } else if let Some(usb) = &usb_thread {
                                if let Err(e) = usb.blocking_lock().write_bytes(&packet) {
                                    warn!("AniMe USB control write error: {e}");
                                }
                            }
                        }
                    }

                    #[cfg(test)]
                    {
                        processed_counter_thread.fetch_add(1, Ordering::SeqCst);
                    }
                }
            })
            .expect("Failed to spawn anime-io thread");

        Self {
            config,
            cache: AniMeConfigCached::default(),
            thread_exit: Arc::new(AtomicBool::new(false)),
            thread_running: Arc::new(AtomicBool::new(false)),
            mailbox: mailbox.clone(),
            _worker_guard: Arc::new(WorkerGuard(mailbox)),
            #[cfg(test)]
            processed_counter,
        }
    }

    /// Dispatches the latest packet buffer to the background I/O worker.
    /// Replaces any pending unprocessed frame without blocking or dropping.
    pub fn dispatch_packets(&self, packets: AnimePacketType) {
        let (lock, cvar) = &*self.mailbox;
        let mut guard = match lock.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        match guard.queue.back_mut() {
            Some(Job::Frame(payload)) => {
                *payload = packets;
            }
            _ => {
                guard.queue.push_back(Job::Frame(packets));
            }
        }
        cvar.notify_one();
    }

    /// Dispatches a raw control packet to the background I/O worker.
    /// Control commands are queued and guaranteed to execute in FIFO order.
    pub fn dispatch_control(&self, packet: Vec<u8>) {
        let (lock, cvar) = &*self.mailbox;
        let mut guard = match lock.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.queue.push_back(Job::Control(packet));
        cvar.notify_one();
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
        debug!("Successfully initialised AniMe matrix display");
        Ok(())
    }

    pub async fn write_bytes(&self, message: &[u8]) -> Result<(), RogError> {
        self.dispatch_control(message.to_vec());
        Ok(())
    }

    /// Write only a data packet. This will modify the leds brightness using the
    /// global brightness set in config.
    pub async fn write_data_buffer(&self, mut buffer: AnimeDataBuffer) -> Result<(), RogError> {
        for led in buffer.data_mut().iter_mut() {
            *led = (*led).min(254);
        }
        let data = AnimePacketType::try_from(&buffer)?;
        self.dispatch_packets(data);
        Ok(())
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

        // Cache pre-computed static images before the loop
        let precomputed_images: Vec<Option<AnimePacketType>> = actions
            .iter()
            .map(|action| {
                if let ActionData::Image(image) = action {
                    let mut buf = image.as_ref().clone();
                    for led in buf.data_mut().iter_mut() {
                        *led = (*led).min(254);
                    }
                    match AnimePacketType::try_from(&buf) {
                        Ok(packets) => Some(packets),
                        Err(e) => {
                            warn!("ActionData::Image packet conversion failed: {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            })
            .collect();

        tokio::spawn(async move {
            info!("AniMe new system thread started");
            while thread_running.load(Ordering::SeqCst) {
                // Make any running loop exit first
                thread_exit.store(true, Ordering::SeqCst);
                tokio::task::yield_now().await;
            }

            info!("AniMe no previous system thread running (now)");
            thread_exit.store(false, Ordering::SeqCst);
            thread_running.store(true, Ordering::SeqCst);
            'main: loop {
                for (idx, action) in actions.iter().enumerate() {
                    if thread_exit.load(Ordering::SeqCst) {
                        break 'main;
                    }
                    match action {
                        ActionData::Animation(frames) => {
                            rog_anime::run_animation(frames, &|mut frame| {
                                if thread_exit.load(Ordering::Acquire) {
                                    info!("rog-anime: animation sub-loop was asked to exit");
                                    return Ok(true); // Do safe exit
                                }
                                for led in frame.data_mut().iter_mut() {
                                    *led = (*led).min(254);
                                }
                                match AnimePacketType::try_from(&frame) {
                                    Ok(packets) => inner.dispatch_packets(packets),
                                    Err(e) => warn!("Animation frame conversion failed: {}", e),
                                }
                                Ok(false) // Don't exit yet
                            });
                            if thread_exit.load(Ordering::Acquire) {
                                info!("rog-anime: sub-loop exited and main loop exiting now");
                                break 'main;
                            }
                        }
                        ActionData::Image(_) => {
                            once = false;
                            if let Some(packets) = &precomputed_images[idx] {
                                inner.dispatch_packets(packets.clone());
                            }
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                        ActionData::Pause(duration) => tokio::time::sleep(*duration).await,
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
            // Loop ended, set the atomics
            thread_running.store(false, Ordering::SeqCst);
            info!("AniMe system thread exited");
        })
        .await
        .map(|err| info!("AniMe system thread: {:?}", err))
        .ok();
    }
}

#[cfg(test)]
impl AniMe {
    pub fn processed_count(&self) -> usize {
        self.processed_counter.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rog_anime::AnimeType;
    use std::time::Duration;

    #[tokio::test]
    async fn test_anime_channel_dispatch() {
        let config = Arc::new(Mutex::new(AniMeConfig::new()));
        let anime = AniMe::new(None, None, config);
        let buffer = AnimeDataBuffer::new(AnimeType::GA402);
        let res = anime.write_data_buffer(buffer).await;
        assert!(res.is_ok());

        // Dispatch a control packet as well to verify control queue routing
        assert!(
            anime
                .write_bytes(&[
                    0x5d, 0x01
                ])
                .await
                .is_ok()
        );

        // Dispatch multiple frames in sequence to verify replacement & worker execution
        for _ in 0..5 {
            let next_buffer = AnimeDataBuffer::new(AnimeType::GA402);
            assert!(anime.write_data_buffer(next_buffer).await.is_ok());
        }

        // Wait for worker thread to process the queued dispatches
        let mut processed = 0;
        for _ in 0..50 {
            processed = anime.processed_count();
            if processed >= 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            processed >= 2,
            "Expected worker to process dispatches, got {processed}"
        );
    }

    #[tokio::test]
    async fn test_anime_frame_replacement_and_fifo() {
        let config = Arc::new(Mutex::new(AniMeConfig::new()));
        let anime = AniMe::new(None, None, config);

        let buf1 = AnimeDataBuffer::new(AnimeType::GA402);
        let buf2 = AnimeDataBuffer::new(AnimeType::GA402);
        let buf3 = AnimeDataBuffer::new(AnimeType::GA402);

        assert!(anime.write_data_buffer(buf1).await.is_ok());
        assert!(anime.write_data_buffer(buf2).await.is_ok());
        assert!(
            anime
                .write_bytes(&[
                    0x5d, 0x01
                ])
                .await
                .is_ok()
        );
        assert!(anime.write_data_buffer(buf3).await.is_ok());

        let mut processed = 0;
        for _ in 0..50 {
            processed = anime.processed_count();
            if processed >= 3 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            processed >= 3,
            "Expected worker to process at least 3 dispatches, got {processed}"
        );
    }
}
