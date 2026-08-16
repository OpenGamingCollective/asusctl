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
use zbus::Connection;
use zbus::object_server::{Interface, SignalEmitter};
use zbus::proxy::CacheProperties;
use zbus::zvariant::ObjectPath;

use crate::error::RogError;

const CONFIG_PATH_BASE: &str = "/etc/asusd/";
pub const ASUS_ZBUS_PATH: &str = "/xyz/ljones";

pub static DBUS_NAME: &str = "xyz.ljones.Asusd";
pub static DBUS_PATH: &str = "/xyz/ljones/Daemon";
pub static DBUS_IFACE: &str = "xyz.ljones.Asusd";

const POWER_SUPPLY_PATH: &str = "/sys/class/power_supply";

/// Check if a power supply type string represents an external power source.
fn is_external_power_type(supply_type: &str) -> bool {
    let t = supply_type.trim();
    t.eq_ignore_ascii_case("mains") || t.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("usb"))
}

/// Find all external power supplies (Mains AC or USB-C PD, e.g. `ADP0`, `AC0`, `ucsi-source-psy-*`).
fn find_external_power_supplies() -> Vec<(String, std::path::PathBuf)> {
    let mut enumerator = match udev::Enumerator::new() {
        Ok(e) => e,
        Err(e) => {
            warn!("Could not create a udev enumerator for power supplies: {e}");
            return Vec::new();
        }
    };
    if let Err(e) = enumerator.match_subsystem("power_supply") {
        warn!("Could not filter the udev enumerator to power supplies: {e}");
        return Vec::new();
    }
    let devices = match enumerator.scan_devices() {
        Ok(d) => d,
        Err(e) => {
            warn!("Could not scan for power supplies: {e}");
            return Vec::new();
        }
    };

    let supplies: Vec<_> = devices
        .filter_map(|device| {
            let supply_type = device.attribute_value("type")?;
            if is_external_power_type(&supply_type.to_string_lossy()) {
                let sysname = device.sysname().to_string_lossy().into_owned();
                let online_path = std::path::Path::new(POWER_SUPPLY_PATH)
                    .join(&sysname)
                    .join("online");
                Some((sysname, online_path))
            } else {
                None
            }
        })
        .collect();

    if supplies.is_empty() {
        warn!(
            "No power supplies with type 'Mains' or 'USB' found in {POWER_SUPPLY_PATH}, \
             external power changes will not be detected"
        );
    } else {
        debug!(
            "Power monitor: tracking external power supplies: {:?}",
            supplies
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>()
        );
    }

    supplies
}

fn read_power_supply_online(path: &std::path::Path) -> Option<bool> {
    std::fs::read_to_string(path)
        .map_err(|e| debug!("Could not read power supply state from {path:?}: {e}"))
        .ok()
        .and_then(|online| online.trim().parse::<u8>().ok())
        .map(|v| v != 0)
}

/// Returns `true` if any of the given external power supplies is currently online.
fn is_any_external_power_online(supplies: &[(String, std::path::PathBuf)]) -> bool {
    supplies
        .iter()
        .any(|(_, path)| read_power_supply_online(path).unwrap_or(false))
}

static POWER_MONITOR: tokio::sync::OnceCell<Option<tokio::sync::watch::Receiver<bool>>> =
    tokio::sync::OnceCell::const_new();

/// Start the power monitor on first use, shared by every `create_sys_event_tasks` caller.
async fn power_state_receiver() -> Option<tokio::sync::watch::Receiver<bool>> {
    POWER_MONITOR.get_or_init(start_power_monitor).await.clone()
}

/// Watch external power supplies for udev change events, reporting whether ANY
/// external supply is online over a coalescing watch channel.
///
/// The udev socket listener is bound inside the worker thread before taking the initial
/// power state snapshot so that any hardware transitions occurring during startup are
/// buffered and never lost.
async fn start_power_monitor() -> Option<tokio::sync::watch::Receiver<bool>> {
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

    std::thread::spawn(move || {
        let mut monitor = match udev::MonitorBuilder::new()
            .and_then(|monitor| monitor.match_subsystem("power_supply"))
            .and_then(|monitor| monitor.listen())
        {
            Ok(monitor) => monitor,
            Err(e) => {
                warn!("Could not create a udev power supply monitor: {e}");
                let _ = ready_tx.send(None);
                return;
            }
        };

        let mut poll = match mio::Poll::new() {
            Ok(poll) => poll,
            Err(e) => {
                warn!("Could not create a mio poll for the power supply monitor: {e}");
                let _ = ready_tx.send(None);
                return;
            }
        };

        if let Err(e) =
            poll.registry()
                .register(&mut monitor, mio::Token(0), mio::Interest::READABLE)
        {
            warn!("Could not register the power supply monitor with mio: {e}");
            let _ = ready_tx.send(None);
            return;
        }

        // Establish initial snapshot after the udev listener is bound
        let mut supplies = find_external_power_supplies();
        let initial_online = is_any_external_power_online(&supplies);
        let (power_state_tx, power_state_rx) = tokio::sync::watch::channel(initial_online);

        let _ = ready_tx.send(Some(power_state_rx));

        let mut events = mio::Events::with_capacity(8);

        loop {
            match poll.poll(&mut events, None) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    warn!(
                        "Power supply monitor poll error, external power changes will no longer \
                         be detected: {e}"
                    );
                    return;
                }
            }

            let mut has_changes = false;
            for event in monitor.iter() {
                let dev = event.device();
                let sysname = dev.sysname().to_string_lossy();
                let is_relevant = supplies.iter().any(|(name, _)| name == sysname.as_ref())
                    || dev
                        .property_value("POWER_SUPPLY_TYPE")
                        .or_else(|| dev.attribute_value("type"))
                        .is_some_and(|t| is_external_power_type(&t.to_string_lossy()));

                if is_relevant {
                    match event.event_type() {
                        udev::EventType::Add | udev::EventType::Remove => {
                            supplies = find_external_power_supplies();
                        }
                        _ => {}
                    }
                    has_changes = true;
                }
            }

            if has_changes {
                let online = is_any_external_power_online(&supplies);
                power_state_tx.send_replace(online);
            }
        }
    });

    ready_rx.await.ok().flatten()
}

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

    /// Free helper method to create tasks to run on: sleep, wake, shutdown, boot
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
            let connection = match Connection::system().await {
                Ok(conn) => conn,
                Err(err) => {
                    error!("Controller could not create dbus connection: {err}");
                    return;
                }
            };

            let logind_manager = match ManagerProxy::builder(&connection)
                .cache_properties(CacheProperties::Lazily)
                .build()
                .await
            {
                Ok(manager) => manager,
                Err(err) => {
                    warn!("Controller could not create logind ManagerProxy: {err}");
                    return;
                }
            };

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
                    // Subscribe before the initial read so a change during startup is
                    // queued rather than lost
                    let mut stream = logind_manager.receive_lid_closed_changed().await;

                    let mut last_lid = match logind_manager.lid_closed().await {
                        Ok(closed) => {
                            debug!("Initial lid state on startup: {closed}");
                            Some(closed)
                        }
                        Err(e) => {
                            debug!("Failed to read initial lid state from logind: {e}");
                            None
                        }
                    };

                    while let Some(change) = stream.next().await {
                        match change.get().await {
                            Ok(lid_closed) if last_lid != Some(lid_closed) => {
                                last_lid = Some(lid_closed);
                                debug!("Lid state changed: {lid_closed}");
                                on_lid_change(lid_closed).await;
                            }
                            Ok(_) => {}
                            Err(e) => {
                                // The tracked state is now unknown, let the next signal through
                                last_lid = None;
                                debug!("Failed to read lid state after a logind change: {e}");
                            }
                        }
                    }
                }
            });

            // logind's OnExternalPower is annotated EmitsChangedSignal=false so it can
            // only be polled. The kernel emits a udev change event for power supplies
            // instead, so watch all external supplies (Mains and USB/USB_PD).
            if let Some(mut power_state_rx) = power_state_receiver().await {
                tokio::spawn(async move {
                    let mut last_power = *power_state_rx.borrow_and_update();
                    while power_state_rx.changed().await.is_ok() {
                        let online = *power_state_rx.borrow_and_update();
                        if online != last_power {
                            last_power = online;
                            debug!("External power supply state changed: {online}");
                            on_external_power_change(online).await;
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
