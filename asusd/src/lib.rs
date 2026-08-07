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

use dmi_id::DMIID;
use futures_util::stream::StreamExt;
use log::{debug, error, info, warn};
use logind_zbus::manager::ManagerProxy;
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
            let connection = Connection::system()
                .await
                .expect("Controller could not create dbus connection");

            let logind_manager = ManagerProxy::builder(&connection)
                .cache_properties(CacheProperties::Lazily)
                .build()
                .await
                .expect("Controller could not create ManagerProxy");

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
                async move {
                    if let Ok(mut notif) = logind_manager.receive_prepare_for_sleep().await {
                        while let Some(event) = notif.next().await {
                            // blocks thread :|
                            if let Ok(args) = event.args() {
                                debug!("Doing on_prepare_for_sleep({})", args.start);
                                on_prepare_for_sleep(args.start).await;
                            }
                        }
                    }
                }
            });

            tokio::spawn({
                let logind_manager = logind_manager.clone();
                async move {
                    // 1. Initial Lid State Fetch & Apply at Daemon Startup
                    let mut last_lid = match logind_manager.lid_closed().await {
                        Ok(closed) => {
                            debug!("Initial lid state on startup: {}", closed);
                            on_lid_change(closed).await;
                            closed
                        }
                        Err(e) => {
                            warn!("Failed to read initial lid state from logind: {}", e);
                            false
                        }
                    };

                    // 2. Subscribe to D-Bus Property Change Stream
                    let mut stream = logind_manager.receive_lid_closed_changed().await;

                    // 3. Process Signals with Event Deduplication
                    while let Some(change) = stream.next().await {
                        if let Ok(lid_closed) = change.get().await {
                            if lid_closed != last_lid {
                                last_lid = lid_closed;
                                debug!("Lid state changed: {}", lid_closed);
                                on_lid_change(lid_closed).await;
                            }
                        }
                    }
                }
            });

            // External power supply monitoring
            if let Some((external_power_supply_sysname, initial_power_supply_state)) =
                std::fs::read_dir("/sys/class/power_supply")
                    .ok()
                    .and_then(|dir| {
                        // Look up the external power supply sysname (e.g. "ACAD") by finding one
                        // with the type "mains"
                        dir.flatten().find_map(|entry| {
                            let type_path = entry.path().canonicalize().ok()?.join("type");
                            let supply_type = std::fs::read_to_string(type_path).ok()?;
                            supply_type
                                .trim()
                                .eq_ignore_ascii_case("mains")
                                .then(|| entry.file_name().to_string_lossy().to_string())
                        })
                    })
                    .and_then(|sysname| {
                        // Look up the initial state of the external power supply
                        let path = std::path::PathBuf::from("/sys/class/power_supply")
                            .join(&sysname)
                            .join("online");
                        let state = std::fs::read_to_string(&path)
                            .map_err(|e| {
                                error!(
                                    "Could not read external power supply state from {path:?}: {e}"
                                )
                            })
                            .ok()
                            .map(|s| s.trim() != "0")?;
                        Some((sysname, state))
                    })
            {
                debug!(
                    "External power supply plugged in on startup: {}",
                    initial_power_supply_state
                );
                on_external_power_change(initial_power_supply_state).await;

                let handle = tokio::runtime::Handle::current();
                std::thread::spawn(move || {
                    'external_power_monitor_thread: {
                        let mut power_supply_monitor = match udev::MonitorBuilder::new()
                            .and_then(|m| m.match_subsystem("power_supply"))
                            .and_then(|m| m.listen())
                        {
                            Ok(m) => m,
                            Err(e) => {
                                error!("Could not create udev power supply monitor: {e}");
                                break 'external_power_monitor_thread;
                            }
                        };

                        let mut poll = match mio::Poll::new() {
                            Ok(p) => p,
                            Err(e) => {
                                error!("Could not create mio Poll: {e}");
                                break 'external_power_monitor_thread;
                            }
                        };

                        if let Err(e) = poll.registry().register(
                            &mut power_supply_monitor,
                            mio::Token(0),
                            mio::Interest::READABLE,
                        ) {
                            error!("Could not register power supply monitor with mio: {e}");
                            break 'external_power_monitor_thread;
                        }

                        let mut events = mio::Events::with_capacity(8);

                        let mut last_power_supply_state = initial_power_supply_state;
                        loop {
                            match poll.poll(&mut events, None) {
                                Ok(_) => {}
                                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                                Err(e) => {
                                    error!("Power supply monitor poll error: {e}");
                                    break 'external_power_monitor_thread;
                                }
                            }

                            for event in power_supply_monitor.iter() {
                                if event.event_type() != udev::EventType::Change {
                                    continue;
                                }
                                if event.device().sysname().to_string_lossy()
                                    != external_power_supply_sysname
                                {
                                    continue;
                                }

                                let Some(current_power_supply_state) = event
                                    .device()
                                    .property_value("POWER_SUPPLY_ONLINE")
                                    .map(|v| v != "0")
                                else {
                                    warn!(
                                        "Power supply change event for external power supply \
                                         missing POWER_SUPPLY_ONLINE property, skipping..."
                                    );
                                    continue;
                                };

                                if current_power_supply_state != last_power_supply_state {
                                    last_power_supply_state = current_power_supply_state;
                                    debug!(
                                        "External power supply state changed: {}",
                                        current_power_supply_state
                                    );
                                    handle.block_on(on_external_power_change(
                                        current_power_supply_state,
                                    ));
                                }
                            }
                        }
                    }
                    error!(
                        "External power supply monitor exited unexpectedly, changes will no \
                         longer be detected."
                    );
                });
            } else {
                warn!("External power supply monitoring unavailable");
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
