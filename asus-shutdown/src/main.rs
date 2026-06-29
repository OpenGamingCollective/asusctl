use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use log::{debug, error, info, warn};
use logind_zbus::manager::{InhibitType, ManagerProxy};
use rog_dbus::asus_armoury::AsusArmouryProxy;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use zbus::proxy::CacheProperties;
use zbus::zvariant::OwnedFd;
use zbus::Connection;

const SERVICE_NAME: &str = "asus-shutdown";
const SHUTDOWN_REASON: &str = "defer risky ASUS GPU firmware writes until shutdown";
const WAIT_FOR_GPU_IDLE: Duration = Duration::from_secs(15);
const GPU_IDLE_POLL_INTERVAL: Duration = Duration::from_millis(500);
const GPU_IDLE_STABLE_FOR: Duration = Duration::from_secs(2);
const WAIT_FOR_NVIDIA_POWERD_EXIT: Duration = Duration::from_secs(10);
const WAIT_FOR_LOGIND_EXIT: Duration = Duration::from_secs(20);
const ASUSD_BUS_NAME: &str = "xyz.ljones.Asusd";
const ASUSD_ARMOURY_IFACE: &str = "xyz.ljones.AsusArmoury";
const SYSTEMD1_BUS_NAME: &str = "org.freedesktop.systemd1";
const SYSTEMD1_MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const SYSTEMD1_MANAGER_IFACE: &str = "org.freedesktop.systemd1.Manager";
const SYSTEMD1_UNIT_IFACE: &str = "org.freedesktop.systemd1.Unit";
const LOGIND_SERVICE: &str = "systemd-logind.service";
const NVIDIA_POWERD_SERVICE: &str = "nvidia-powerd.service";
const NVIDIA_SERVICES: &[&str] = &[
    NVIDIA_POWERD_SERVICE,
    "nvidia-persistenced.service",
    "nvidia-fabricmanager.service",
];
const NVIDIA_MODULE_PATHS: &[&str] = &[
    "/sys/module/nvidia",
    "/sys/module/nvidia_drm",
    "/sys/module/nvidia_modeset",
    "/sys/module/nvidia_uvm",
];

#[derive(Clone, Debug)]
struct PendingAction {
    path: String,
    name: String,
    value: i32,
}

#[derive(Debug)]
struct DiscreteGpu {
    card: String,
    runtime_status: Option<String>,
    devnodes: Vec<PathBuf>,
    busy: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut logger = env_logger::Builder::new();
    logger
        .parse_default_env()
        .target(env_logger::Target::Stdout)
        .format_timestamp(None)
        .filter_level(log::LevelFilter::Info)
        .init();

    let is_service = match env::var_os("IS_SERVICE") {
        Some(val) => val == "1",
        None => true,
    };

    if !is_service {
        print_dry_run_actions().await;
        return Ok(());
    }

    info!("Starting {}", SERVICE_NAME);

    let connection = Connection::system().await?;
    let manager = ManagerProxy::builder(&connection)
        .cache_properties(CacheProperties::No)
        .build()
        .await?;

    let inhibitor = Arc::new(Mutex::new(Some(
        acquire_shutdown_inhibitor(&manager).await?,
    )));
    let mut shutdown_events = manager.receive_prepare_for_shutdown().await?;
    let mut sigterm = signal(SignalKind::terminate())?;

    loop {
        tokio::select! {
            _ = sigterm.recv() => {
                warn!("Received SIGTERM, exiting cleanly");
                break;
            }
            event = shutdown_events.next() => {
                match event {
                    Some(event) => match event.args() {
                        Ok(args) if args.start => {
                            info!("Shutdown requested, applying deferred ASUS GPU settings");
                            if let Err(err) = apply_shutdown_settings().await {
                                error!("Failed to apply deferred GPU settings: {err}");
                            }

                            inhibitor.lock().await.take();
                            info!("Released shutdown delay inhibitor");

                            info!("Exiting after deferred shutdown apply completed");
                            break;
                        }
                        Ok(args) => {
                            debug!("PrepareForShutdown({})", args.start);
                            let mut guard = inhibitor.lock().await;
                            if guard.is_none() {
                                match acquire_shutdown_inhibitor(&manager).await {
                                    Ok(fd) => {
                                        *guard = Some(fd);
                                        info!("Reacquired shutdown delay inhibitor");
                                    }
                                    Err(err) => {
                                        error!("Failed to reacquire shutdown inhibitor: {err}");
                                    }
                                }
                            }
                        }
                        Err(err) => warn!("Failed to decode PrepareForShutdown signal: {err}"),
                    },
                    None => {
                        info!("PrepareForShutdown signal stream ended");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn acquire_shutdown_inhibitor(manager: &ManagerProxy<'_>) -> Result<OwnedFd, zbus::Error> {
    manager
        .inhibit(
            InhibitType::Shutdown,
            SERVICE_NAME,
            SHUTDOWN_REASON,
            "delay",
        )
        .await
}

async fn print_dry_run_actions() {
    println!("asus-shutdown dry-run mode (manual start)");
    println!("Planned shutdown actions from asusd queue:");

    match fetch_pending_actions().await {
        Ok(actions) if actions.is_empty() => {
            println!("- none");
        }
        Ok(actions) => {
            for action in actions {
                println!(
                    "- {} => {} (path: {})",
                    action.name, action.value, action.path
                );
            }
        }
        Err(err) => {
            println!("- could not query asusd queue: {err}");
        }
    }
}

async fn fetch_pending_actions() -> Result<Vec<PendingAction>, Box<dyn std::error::Error>> {
    let conn = Connection::system().await?;
    let manager = zbus::fdo::ObjectManagerProxy::new(&conn, ASUSD_BUS_NAME, "/").await?;
    let managed = manager.get_managed_objects().await?;

    let mut actions = Vec::new();
    for (path, ifaces) in managed {
        if !ifaces.contains_key(ASUSD_ARMOURY_IFACE) {
            continue;
        }

        let proxy = AsusArmouryProxy::builder(&conn)
            .path(path.clone())?
            .destination(ASUSD_BUS_NAME)?
            .build()
            .await?;

        let value = proxy.queued_gpu_value().await?;
        if value < 0 {
            continue;
        }

        let name = proxy.name().await?;
        actions.push(PendingAction {
            path: path.to_string(),
            name: <&str>::from(name).to_owned(),
            value,
        });
    }

    actions.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(actions)
}

async fn apply_shutdown_settings() -> Result<(), Box<dyn std::error::Error>> {
    info!("[phase 1/6] Checking for deferred GPU settings queued in asusd...");
    let queued = fetch_pending_actions().await?;

    info!(
        "[phase 2/6] Found {} deferred GPU settings queued in asusd",
        queued.len()
    );
    if queued.is_empty() {
        info!("No deferred GPU settings queued");
        return Ok(());
    }

    info!("Deferred GPU settings queued for shutdown apply:");
    for action in &queued {
        info!("  {} => {} ({})", action.name, action.value, action.path);
    }

    info!("[phase 3/6] Waiting for discrete GPU to become idle before applying settings...");
    wait_for_discrete_gpu_idle().await;

    info!(
        "[phase 4/6] Waiting for {} to exit before firmware writes...",
        LOGIND_SERVICE
    );
    let _ = wait_for_unit_to_exit(LOGIND_SERVICE, WAIT_FOR_LOGIND_EXIT).await;

    info!("[phase 5/6] Preparing NVIDIA stack safety gates before firmware writes...");
    prepare_nvidia_for_gpu_firmware_writes().await;

    info!("[phase 6/6] Proceeding with applying deferred GPU settings");

    let conn = Connection::system().await?;
    for action in queued {
        let proxy = AsusArmouryProxy::builder(&conn)
            .path(action.path.as_str())?
            .destination(ASUSD_BUS_NAME)?
            .build()
            .await?;

        info!(
            "Applying deferred GPU attribute {} = {}",
            action.name, action.value
        );
        let applied = proxy.apply_queued_gpu_value().await?;
        if !applied {
            warn!("No queued value remained for {}", action.name);
        }
    }

    Ok(())
}

async fn prepare_nvidia_for_gpu_firmware_writes() {
    let has_nvidia_modules = NVIDIA_MODULE_PATHS
        .iter()
        .any(|path| Path::new(path).exists());

    let mut saw_loaded_or_active_service = false;

    for unit in NVIDIA_SERVICES {
        let exited = wait_for_unit_to_exit(unit, WAIT_FOR_NVIDIA_POWERD_EXIT).await;
        if !exited {
            saw_loaded_or_active_service = true;
        }
    }

    if !has_nvidia_modules && !saw_loaded_or_active_service {
        info!("No NVIDIA modules/services detected, skipping NVIDIA-specific shutdown preparation");
        return;
    }

    if !has_nvidia_modules {
        info!("NVIDIA modules are not loaded, skipping module unload step");
        return;
    }

    info!("Preparing NVIDIA driver stack for firmware attribute apply");
    let output = Command::new("modprobe")
        .args([
            "-r", "nvidia_drm", "nvidia_modeset", "nvidia_uvm", "nvidia",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            info!("Successfully unloaded NVIDIA modules before firmware attribute apply");
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            warn!(
                "Failed to unload NVIDIA modules before firmware attribute apply (status {}): {}",
                out.status,
                if stderr.is_empty() {
                    "no stderr output"
                } else {
                    stderr.as_str()
                }
            );
        }
        Err(err) => {
            warn!("Failed to execute modprobe -r for NVIDIA modules: {err}");
        }
    }
}

async fn wait_for_unit_to_exit(unit_name: &str, timeout: Duration) -> bool {
    info!(
        "Waiting for {} to exit before applying firmware attributes...",
        unit_name
    );
    let started = Instant::now();

    let conn = match Connection::system().await {
        Ok(conn) => conn,
        Err(err) => {
            warn!("Failed to connect to system bus while waiting for {unit_name}: {err}");
            return false;
        }
    };

    let manager = match zbus::Proxy::new(
        &conn,
        SYSTEMD1_BUS_NAME,
        SYSTEMD1_MANAGER_PATH,
        SYSTEMD1_MANAGER_IFACE,
    )
    .await
    {
        Ok(proxy) => proxy,
        Err(err) => {
            warn!("Failed to create systemd manager proxy while waiting for {unit_name}: {err}");
            return false;
        }
    };

    let deadline = Instant::now() + timeout;

    loop {
        let unit_path: Result<zbus::zvariant::OwnedObjectPath, zbus::Error> =
            manager.call("GetUnit", &(unit_name)).await;

        let unit_path = match unit_path {
            Ok(path) => path,
            Err(err) => {
                let msg = err.to_string();
                if msg.contains("NoSuchUnit") || msg.contains("not loaded") {
                    info!(
                        "{} is not loaded, continuing after {:?}",
                        unit_name,
                        started.elapsed()
                    );
                    return true;
                }

                warn!("Failed to lookup {} in systemd: {err}", unit_name);
                return false;
            }
        };

        let props_builder =
            match zbus::fdo::PropertiesProxy::builder(&conn).destination(SYSTEMD1_BUS_NAME) {
                Ok(builder) => builder,
                Err(err) => {
                    warn!(
                        "Failed to create properties proxy destination for {}: {err}",
                        unit_name
                    );
                    return false;
                }
            };

        let props_builder = match props_builder.path(unit_path.as_str()) {
            Ok(builder) => builder,
            Err(err) => {
                warn!(
                    "Failed to set properties proxy path for {}: {err}",
                    unit_name
                );
                return false;
            }
        };

        let props = match props_builder.build().await {
            Ok(props) => props,
            Err(err) => {
                warn!(
                    "Failed to query {} properties via systemd API: {err}",
                    unit_name
                );
                return false;
            }
        };

        let unit_iface = match zbus::names::InterfaceName::try_from(SYSTEMD1_UNIT_IFACE) {
            Ok(iface) => iface,
            Err(err) => {
                warn!("Failed to parse systemd unit interface name: {err}");
                return false;
            }
        };

        let active_state: String = match props.get(unit_iface.clone(), "ActiveState").await {
            Ok(value) => match String::try_from(value) {
                Ok(state) => state,
                Err(err) => {
                    warn!("Failed to decode ActiveState for {}: {err}", unit_name);
                    return false;
                }
            },
            Err(err) => {
                warn!("Failed to read ActiveState for {}: {err}", unit_name);
                return false;
            }
        };

        let sub_state: String = match props.get(unit_iface, "SubState").await {
            Ok(value) => match String::try_from(value) {
                Ok(state) => state,
                Err(err) => {
                    warn!("Failed to decode SubState for {}: {err}", unit_name);
                    return false;
                }
            },
            Err(err) => {
                warn!("Failed to read SubState for {}: {err}", unit_name);
                return false;
            }
        };

        if active_state == "inactive" || active_state == "failed" {
            info!(
                "{} is {} ({}) and considered exited after {:?}",
                unit_name,
                active_state,
                sub_state,
                started.elapsed()
            );
            return true;
        }

        if Instant::now() >= deadline {
            warn!(
                "Timed out waiting for {} to exit (ActiveState={}, SubState={})",
                unit_name, active_state, sub_state
            );
            return false;
        }

        debug!(
            "{} still active (ActiveState={}, SubState={}), waiting...",
            unit_name, active_state, sub_state
        );
        sleep(GPU_IDLE_POLL_INTERVAL).await;
    }
}

async fn wait_for_discrete_gpu_idle() {
    let deadline = Instant::now() + WAIT_FOR_GPU_IDLE;
    let mut idle_since: Option<Instant> = None;

    loop {
        match collect_discrete_gpu_state() {
            Ok(gpus) if gpus.is_empty() => {
                info!("No discrete DRM devices detected, continuing shutdown apply");
                return;
            }
            Ok(gpus) => {
                if gpus.iter().all(discrete_gpu_is_idle) {
                    let now = Instant::now();
                    let idle_start = idle_since.get_or_insert(now);
                    let idle_for = now.saturating_duration_since(*idle_start);

                    if idle_for >= GPU_IDLE_STABLE_FOR {
                        info!(
                            "Discrete GPU nodes have stayed idle for {:?}",
                            GPU_IDLE_STABLE_FOR
                        );
                        return;
                    }

                    debug!(
                        "Discrete GPU nodes are idle, waiting {:?} more to avoid driver teardown race",
                        GPU_IDLE_STABLE_FOR.saturating_sub(idle_for)
                    );
                } else {
                    idle_since = None;
                }

                if Instant::now() >= deadline {
                    warn!("Timed out waiting for discrete GPU users to drain");
                    for gpu in &gpus {
                        if gpu.busy {
                            warn!(
                                "GPU {} still busy with runtime_status={:?} nodes={:?}",
                                gpu.card, gpu.runtime_status, gpu.devnodes
                            );
                        }
                    }
                    return;
                }
            }
            Err(err) => {
                warn!("Failed to inspect GPU state: {err}");
                return;
            }
        }

        sleep(GPU_IDLE_POLL_INTERVAL).await;
    }
}

fn discrete_gpu_is_idle(gpu: &DiscreteGpu) -> bool {
    // Only check /proc file descriptors to determine if GPU is in use.
    // Runtime power state (sysfs power/runtime_status) is unavailable on most
    // laptops, so we don't rely on it. If a process holds an open fd to the GPU,
    // it is definitely using it.
    !gpu.busy
}

fn collect_discrete_gpu_state() -> Result<Vec<DiscreteGpu>, Box<dyn std::error::Error>> {
    let render_map = render_node_map()?;
    let busy_nodes = busy_gpu_nodes()?;
    let mut gpus = Vec::new();

    for entry in fs::read_dir("/sys/class/drm")? {
        let entry = entry?;
        let file_name = entry.file_name();
        let card = file_name.to_string_lossy();
        if !is_card_entry(&card) {
            continue;
        }

        let device_path = entry.path().join("device");
        if read_trimmed(device_path.join("boot_vga")).as_deref() == Some("1") {
            continue;
        }

        let runtime_status = read_trimmed(device_path.join("power/runtime_status"));
        let mut devnodes = vec![PathBuf::from(format!("/dev/dri/{card}"))];

        let canonical_device = match fs::canonicalize(&device_path) {
            Ok(path) => path,
            Err(err) => {
                debug!(
                    "Skipping DRM entry {} without canonical device path: {err}",
                    card
                );
                continue;
            }
        };

        if let Some(render_nodes) = render_map.get(&canonical_device) {
            devnodes.extend(render_nodes.iter().cloned());
        }

        let busy = devnodes.iter().any(|node| busy_nodes.contains(node));
        gpus.push(DiscreteGpu {
            card: card.into_owned(),
            runtime_status,
            devnodes,
            busy,
        });
    }

    Ok(gpus)
}

fn render_node_map() -> Result<HashMap<PathBuf, Vec<PathBuf>>, Box<dyn std::error::Error>> {
    let mut render_map = HashMap::new();

    for entry in fs::read_dir("/sys/class/drm")? {
        let entry = entry?;
        let file_name = entry.file_name();
        let render = file_name.to_string_lossy();
        if !render.starts_with("renderD") {
            continue;
        }

        let device_path = match fs::canonicalize(entry.path().join("device")) {
            Ok(path) => path,
            Err(_) => continue,
        };

        render_map
            .entry(device_path)
            .or_insert_with(Vec::new)
            .push(PathBuf::from(format!("/dev/dri/{render}")));
    }

    Ok(render_map)
}

fn busy_gpu_nodes() -> Result<HashSet<PathBuf>, Box<dyn std::error::Error>> {
    let mut nodes = HashSet::new();

    for proc_entry in fs::read_dir("/proc")? {
        let proc_entry = proc_entry?;
        let pid = proc_entry.file_name();
        if !pid.to_string_lossy().chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }

        let fd_dir = proc_entry.path().join("fd");
        let Ok(fd_entries) = fs::read_dir(fd_dir) else {
            continue;
        };

        for fd_entry in fd_entries.flatten() {
            let Ok(target) = fs::read_link(fd_entry.path()) else {
                continue;
            };

            if target.starts_with("/dev/dri/") {
                nodes.insert(target);
            }
        }
    }

    Ok(nodes)
}

fn is_card_entry(name: &str) -> bool {
    name.starts_with("card") && name[4..].chars().all(|ch| ch.is_ascii_digit())
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|content| content.trim().to_string())
}
