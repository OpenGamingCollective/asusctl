#![deny(unused_must_use)]
/// Configuration loading, saving
pub mod config;
pub mod ctrl_backlight;
/// Control platform profiles + fan-curves if available
pub mod ctrl_fancurves;
/// Control ASUS bios function such as boot sound, Optimus/Dedicated gfx mode
pub mod ctrl_platform;
pub mod ctrl_xgm_led;

pub mod asus_armoury;
pub mod aura_anime;
pub mod aura_laptop;
pub mod aura_manager;
pub mod aura_scsi;
pub mod aura_slash;
pub mod aura_types;
pub mod error;

use std::future::Future;
use std::sync::Arc;

use dmi_id::DMIID;
use futures_util::stream::StreamExt;
use log::{debug, error, info, warn};
use logind_zbus::manager::ManagerProxy;
use rog_platform::power::AsusPower;
use zbus::object_server::{Interface, SignalEmitter};
use zbus::proxy::CacheProperties;
use zbus::zvariant::ObjectPath;
use zbus::Connection;

use crate::error::RogError;

const CONFIG_PATH_BASE: &str = "/etc/asusd/";
pub const ASUS_ZBUS_PATH: &str = "/xyz/ljones";

pub static DBUS_NAME: &str = "xyz.ljones.Asusd";
pub static DBUS_PATH: &str = "/xyz/ljones/Daemon";
pub static DBUS_IFACE: &str = "xyz.ljones.Asusd";

/// This macro adds a function which spawns an `inotify` task on the passed in
/// `Executor`.
///
/// The generated function is `watch_<name>()`. Self requires the following
/// methods to be available:
/// - `<name>() -> SomeValue`, functionally is a getter, but is allowed to have
///   side effects.
/// - `notify_<name>(SignalEmitter, SomeValue)`
///
/// In most cases if `SomeValue` is stored in a config then `<name>()` getter is
/// expected to update it. The getter should *never* write back to the path or
/// attribute that is being watched or an infinite loop will occur.
///
/// # Example
///
/// ```ignore
/// impl RogPlatform {
///     task_watch_item!(panel_od platform);
///     task_watch_item!(gpu_mux_mode platform);
/// }
/// ```\
/// // TODO: this is kind of useless if it can't trigger some action
#[macro_export]
macro_rules! task_watch_item {
    ($name:ident $name_str:literal $self_inner:ident) => {
        concat_idents::concat_idents!(fn_name = watch_, $name {
        async fn fn_name(
            &self,
            signal_ctxt: SignalEmitter<'static>,
        ) -> Result<(), RogError> {
            use futures_util::StreamExt;

            let ctrl = self.clone();
            concat_idents::concat_idents!(watch_fn = monitor_, $name {
                match self.$self_inner.watch_fn() {
                    Ok(watch) => {
                        tokio::spawn(async move {
                            let mut buffer = [0; 32];
                            if let Ok(stream) = watch.into_event_stream(&mut buffer) {
                                stream.for_each(|_| async {
                                    if let Ok(value) = ctrl.$name() { // get new value from zbus method
                                        if ctrl.config.lock().await.$name != value {
                                            log::debug!("{} was changed to {} externally", $name_str, value);
                                            concat_idents::concat_idents!(notif_fn = $name, _changed {
                                                ctrl.notif_fn(&signal_ctxt).await.ok();
                                            });
                                            let mut lock = ctrl.config.lock().await;
                                            lock.$name = value;
                                            lock.write();
                                        }
                                    }
                                }).await;
                            } else {
                                log::error!("Failed to create event stream for {}", $name_str);
                            }
                        });
                    }
                    Err(e) => info!("inotify watch failed: {}. You can ignore this if your device does not support the feature", e),
                }
            });
            Ok(())
        }
        });
    };
}

#[macro_export]
macro_rules! task_watch_item_notify {
    ($name:ident $self_inner:ident) => {
        concat_idents::concat_idents!(fn_name = watch_, $name {
        async fn fn_name(
            &self,
            signal_ctxt: SignalEmitter<'static>,
        ) -> Result<(), RogError> {
            use futures_util::StreamExt;

            let ctrl = self.clone();
            concat_idents::concat_idents!(watch_fn = monitor_, $name {
                match self.$self_inner.watch_fn() {
                    Ok(watch) => {
                        tokio::spawn(async move {
                            let mut buffer = [0; 32];
                            if let Ok(stream) = watch.into_event_stream(&mut buffer) {
                                stream.for_each(|_| async {
                                    concat_idents::concat_idents!(notif_fn = $name, _changed {
                                        ctrl.notif_fn(&signal_ctxt).await.ok();
                                    });
                                }).await;
                            }
                        });
                    }
                    Err(e) => info!("inotify watch failed: {}. You can ignore this if your device does not support the feature", e),
                }
            });
            Ok(())
        }
        });
    };
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn print_board_info() {
    let dmi = DMIID::new().unwrap_or_default();
    info!("Product family: {}", dmi.product_family);
    info!("Board name: {}", dmi.board_name);
}

pub trait Reloadable {
    fn reload(&mut self) -> impl Future<Output = Result<(), RogError>> + Send;
}

pub trait ReloadAndNotify {
    type Data: Send;

    fn reload_and_notify(
        &mut self,
        signal_context: &SignalEmitter<'static>,
        data: Self::Data,
    ) -> impl Future<Output = Result<(), RogError>> + Send;
}

pub trait ZbusRun {
    fn add_to_server(self, server: &mut Connection) -> impl Future<Output = ()> + Send;

    fn add_to_server_helper(
        iface: impl Interface,
        path: &str,
        server: &mut Connection,
    ) -> impl Future<Output = ()> + Send {
        async move {
            server
                .object_server()
                .at(&ObjectPath::from_str_unchecked(path), iface)
                .await
                .map_err(|err| {
                    warn!("{}: add_to_server {}", path, err);
                    err
                })
                .ok();
        }
    }
}

/// Locate the mains (AC adapter) supply and its `online` attribute. Goes
/// through `AsusPower` so the monitor watches the same supply `get_online()`
/// reads.
fn mains_power_supply() -> Option<(String, std::path::PathBuf)> {
    let power = AsusPower::new()
        .map_err(|e| error!("Could not enumerate power supplies: {e}"))
        .ok()?;
    match (power.mains_sysname(), power.mains_syspath()) {
        (Some(sysname), Some(syspath)) => Some((sysname, syspath.join("online"))),
        _ => {
            error!(
                "No power supply with type 'Mains' was found, external power changes will not be \
                 detected"
            );
            None
        }
    }
}

fn read_power_supply_online(path: &std::path::Path) -> Option<bool> {
    std::fs::read_to_string(path)
        .map_err(|e| error!("Could not read the external power supply state from {path:?}: {e}"))
        .ok()
        .map(|online| online.trim() != "0")
}

/// Watch the mains power supply for udev change events, publishing the new
/// `online` state. State is published rather than acted on here so a slow
/// consumer can never stall the poll loop.
fn spawn_mains_power_monitor(
    sysname: String,
    online_path: std::path::PathBuf,
    power_state: Arc<tokio::sync::watch::Sender<Option<bool>>>,
) {
    std::thread::spawn(move || {
        let mut monitor = match udev::MonitorBuilder::new()
            .and_then(|monitor| monitor.match_subsystem("power_supply"))
            .and_then(|monitor| monitor.listen())
        {
            Ok(monitor) => monitor,
            Err(e) => {
                error!("Could not create a udev power supply monitor: {e}");
                return;
            }
        };

        let mut poll = match mio::Poll::new() {
            Ok(poll) => poll,
            Err(e) => {
                error!("Could not create a mio poll for the power supply monitor: {e}");
                return;
            }
        };

        if let Err(e) =
            poll.registry()
                .register(&mut monitor, mio::Token(0), mio::Interest::READABLE)
        {
            error!("Could not register the power supply monitor with mio: {e}");
            return;
        }

        let mut events = mio::Events::with_capacity(8);
        loop {
            match poll.poll(&mut events, None) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    error!(
                        "Power supply monitor poll error, external power changes will no longer \
                         be detected: {e}"
                    );
                    return;
                }
            }

            for event in monitor.iter() {
                if event.event_type() != udev::EventType::Change {
                    continue;
                }
                let device = event.device();
                if device.sysname().to_string_lossy() != sysname {
                    continue;
                }

                // A change uevent is not guaranteed to carry the full property set
                let online = match device.property_value("POWER_SUPPLY_ONLINE") {
                    Some(value) => Some(value != "0"),
                    None => {
                        debug!("Power supply uevent had no POWER_SUPPLY_ONLINE, reading sysfs");
                        read_power_supply_online(&online_path)
                    }
                };

                if online.is_some() && power_state.send(online).is_err() {
                    return;
                }
            }
        }
    });
}

/// Set up a task to run on the async executor
pub trait CtrlTask {
    fn zbus_path() -> &'static str;

    fn signal_context(connection: &Connection) -> Result<SignalEmitter<'static>, zbus::Error> {
        SignalEmitter::new(connection, Self::zbus_path())
    }

    /// Implement to set up various tasks that may be required, using the
    /// `Executor`. No blocking loops are allowed, or they must be run on a
    /// separate thread.
    fn create_tasks(
        &self,
        signal: SignalEmitter<'static>,
    ) -> impl Future<Output = Result<(), RogError>> + Send;

    // /// Create a timed repeating task
    // async fn repeating_task(&self, millis: u64, mut task: impl FnMut() + Send +
    // 'static) {     use std::time::Duration;
    //     use tokio::time;
    //     let mut timer = time::interval(Duration::from_millis(millis));
    //     tokio::spawn(async move {
    //         timer.tick().await;
    //         task();
    //     });
    // }

    /// Free helper method to create tasks to run on: sleep, wake, shutdown,
    /// boot
    ///
    /// The closures can potentially block, so execution time should be the
    /// minimal possible such as save a variable.
    ///
    /// `on_lid_change` and `on_external_power_change` are also called once with
    /// the current state so that the hardware is configured at daemon start,
    /// not only on the next transition.
    fn create_sys_event_tasks<Fut1, Fut2, Fut3, Fut4, F1, F2, F3, F4>(
        &self,
        mut on_prepare_for_sleep: F1,
        mut on_prepare_for_shutdown: F2,
        mut on_lid_change: F3,
        mut on_external_power_change: F4,
    ) -> impl Future<Output = ()> + Send
    where
        F1: FnMut(bool) -> Fut1 + Send + 'static,
        F2: FnMut(bool) -> Fut2 + Send + 'static,
        F3: FnMut(bool) -> Fut3 + Send + 'static,
        F4: FnMut(bool) -> Fut4 + Send + 'static,
        Fut1: Future<Output = ()> + Send,
        Fut2: Future<Output = ()> + Send,
        Fut3: Future<Output = ()> + Send,
        Fut4: Future<Output = ()> + Send,
    {
        async {
            // The system bus can still be coming up during early boot
            let connection = match Connection::system().await {
                Ok(connection) => connection,
                Err(e) => {
                    error!("Controller could not create dbus connection: {e}");
                    return;
                }
            };

            let logind_manager = match ManagerProxy::builder(&connection)
                .cache_properties(CacheProperties::Lazily)
                .build()
                .await
            {
                Ok(manager) => manager,
                Err(e) => {
                    error!("Controller could not create ManagerProxy: {e}");
                    return;
                }
            };

            // Caching is required for the change stream to fire, but it also makes
            // property reads return cache. A resync must bypass it or it reads back
            // the same stale value a missed signal left behind.
            let logind_direct = match ManagerProxy::builder(&connection)
                .cache_properties(CacheProperties::No)
                .build()
                .await
            {
                Ok(manager) => Some(manager),
                Err(e) => {
                    warn!("Could not create an uncached ManagerProxy, lid state will not resync after resume: {e}");
                    None
                }
            };

            // Bumped after every resume. Change detection below is edge triggered,
            // so both watchers re-read their source when this ticks.
            let (resumed, _) = tokio::sync::watch::channel(0u64);
            let resumed = Arc::new(resumed);

            tokio::spawn({
                let logind_manager = logind_manager.clone();
                async move {
                    if let Ok(mut notif) = logind_manager.receive_prepare_for_shutdown().await {
                        while let Some(event) = notif.next().await {
                            // blocks thread :|
                            if let Ok(args) = event.args() {
                                debug!("Doing on_prepare_for_shutdown({})", args.start);
                                on_prepare_for_shutdown(args.start).await;
                            }
                        }
                    }
                }
            });

            tokio::spawn({
                let logind_manager = logind_manager.clone();
                let resumed = resumed.clone();
                async move {
                    if let Ok(mut notif) = logind_manager.receive_prepare_for_sleep().await {
                        while let Some(event) = notif.next().await {
                            // blocks thread :|
                            if let Ok(args) = event.args() {
                                debug!("Doing on_prepare_for_sleep({})", args.start);
                                on_prepare_for_sleep(args.start).await;
                                if !args.start {
                                    resumed.send_modify(|tick| *tick += 1);
                                }
                            }
                        }
                    }
                }
            });

            tokio::spawn({
                let logind_manager = logind_manager.clone();
                let mut resumed = resumed.subscribe();
                async move {
                    // Subscribe before the initial read so a startup change is not lost
                    let mut stream = logind_manager.receive_lid_closed_changed().await;
                    let resync = logind_direct.as_ref().unwrap_or(&logind_manager);

                    let mut last_lid = match logind_manager.lid_closed().await {
                        Ok(closed) => {
                            debug!("Initial lid state on startup: {closed}");
                            on_lid_change(closed).await;
                            Some(closed)
                        }
                        Err(e) => {
                            warn!("Failed to read initial lid state from logind: {e}");
                            None
                        }
                    };

                    loop {
                        let lid_closed = tokio::select! {
                            Some(change) = stream.next() => change.get().await,
                            Ok(()) = resumed.changed() => {
                                debug!("Re-reading lid state after resume");
                                resync.lid_closed().await
                            }
                            else => break,
                        };

                        match lid_closed {
                            Ok(lid_closed) if last_lid != Some(lid_closed) => {
                                last_lid = Some(lid_closed);
                                debug!("Lid state changed: {lid_closed}");
                                on_lid_change(lid_closed).await;
                            }
                            Ok(_) => {}
                            Err(e) => {
                                // The tracked state is now unknown, let the next read through
                                last_lid = None;
                                warn!("Failed to read lid state from logind: {e}");
                            }
                        }
                    }
                }
            });

            // logind's OnExternalPower is annotated EmitsChangedSignal=false so it can
            // only be polled. The kernel emits a udev change event for the mains supply
            // instead, so watch that. Kept separate from the `aura_manager` monitor,
            // which belongs to device discovery.
            if let Some((sysname, online_path)) = mains_power_supply() {
                // Start the monitor before the initial read so a startup change is not lost
                let (power_state, mut power_changed) = tokio::sync::watch::channel(None);
                spawn_mains_power_monitor(sysname, online_path.clone(), Arc::new(power_state));

                let mut last_power = read_power_supply_online(&online_path);
                if let Some(online) = last_power {
                    debug!("External power supply plugged in on startup: {online}");
                    on_external_power_change(online).await;
                }

                let mut resumed = resumed.subscribe();
                tokio::spawn(async move {
                    loop {
                        let online = tokio::select! {
                            Ok(()) = power_changed.changed() => *power_changed.borrow_and_update(),
                            Ok(()) = resumed.changed() => {
                                debug!("Re-reading external power state after resume");
                                read_power_supply_online(&online_path)
                            }
                            else => break,
                        };

                        if let Some(online) = online {
                            if last_power != Some(online) {
                                last_power = Some(online);
                                debug!("External power supply state changed: {online}");
                                on_external_power_change(online).await;
                            }
                        }
                    }
                });
            }
        }
    }
}

pub trait GetSupported {
    type A;

    fn get_supported() -> Self::A;
}

pub async fn start_tasks<T>(
    mut zbus: T,
    connection: &mut Connection,
    signal_ctx: SignalEmitter<'static>,
) -> Result<(), RogError>
where
    T: ZbusRun + Reloadable + CtrlTask + Clone,
{
    let zbus_clone = zbus.clone();

    zbus.reload()
        .await
        .unwrap_or_else(|err| warn!("Controller error: {}", err));
    zbus.add_to_server(connection).await;

    zbus_clone.create_tasks(signal_ctx).await.ok();
    Ok(())
}
