//! GPU power status controller for D-Bus.
//!
//! Monitors the discrete GPU's runtime power status via inotify on sysfs
//! and exposes it over D-Bus for the tray icon and other clients.

use log::{error, info, warn};
use rog_platform::gpu_pci::{GfxPower, GfxVendor};
use zbus::object_server::SignalEmitter;
use zbus::{interface, Connection};

use crate::error::RogError;

const GPU_ZBUS_PATH: &str = "/xyz/ljones/Gpu";

/// GPU power status controller. Detects the dGPU and monitors its power state.
///
/// The controller is non-fatal: if no dGPU is found or detection fails,
/// it reports `GfxPower::Unknown` and `GfxVendor::Unknown`.
#[derive(Clone)]
pub struct CtrlGpu {
    #[allow(dead_code)]
    connection: Connection,
    /// Current GPU power status.
    power_status: GfxPower,
    /// Current GPU vendor.
    vendor: GfxVendor,
    /// Path to the dGPU's sysfs entry (if found), used for inotify.
    dgpu_runtime_status_path: Option<std::path::PathBuf>,
}

impl CtrlGpu {
    pub fn new(connection: Connection) -> Self {
        let (power_status, vendor, dgpu_runtime_status_path) = Self::detect_gpu();

        info!(
            "CtrlGpu: detected GPU vendor={vendor}, power={power_status}, path={:?}",
            dgpu_runtime_status_path
        );

        Self {
            connection,
            power_status,
            vendor,
            dgpu_runtime_status_path,
        }
    }

    /// Detect GPU and return (power_status, vendor, optional runtime_status_path).
    fn detect_gpu() -> (GfxPower, GfxVendor, Option<std::path::PathBuf>) {
        use rog_platform::gpu_pci::Device;

        let devices = Device::find().unwrap_or_default();

        if let Some(dgpu) = devices.iter().find(|d| d.is_dgpu()) {
            let vendor = dgpu.vendor();
            let runtime_path = {
                let mut p = dgpu.dev_path().clone();
                p.push("power");
                p.push("runtime_status");
                if p.exists() {
                    Some(p)
                } else {
                    None
                }
            };
            if let Ok(power) = dgpu.get_runtime_status() {
                return (power, vendor, runtime_path);
            }
            return (GfxPower::Unknown, vendor, runtime_path);
        }

        // No dGPU devices — check ASUS-specific attributes
        if rog_platform::gpu_pci::asus_dgpu_disable_exists() {
            if let Ok(disabled) = rog_platform::gpu_pci::asus_dgpu_disabled() {
                if disabled {
                    return (GfxPower::AsusDisabled, GfxVendor::AsusDgpuDisabled, None);
                }
            }
        }
        if rog_platform::gpu_pci::asus_gpu_mux_exists() {
            if let Ok(discreet) = rog_platform::gpu_pci::asus_gpu_mux_discreet() {
                if discreet {
                    return (GfxPower::AsusMuxDiscreet, GfxVendor::Nvidia, None);
                }
            }
        }

        (GfxPower::Unknown, GfxVendor::Unknown, None)
    }

    /// Re-detect GPU status (e.g. after hotplug). Returns true if status changed.
    fn redetect(&mut self) -> bool {
        let (new_power, new_vendor, new_path) = Self::detect_gpu();
        let changed = new_power != self.power_status || new_vendor != self.vendor;
        if changed {
            info!(
                "CtrlGpu: status changed from ({}, {}) to ({}, {})",
                self.power_status, self.vendor, new_power, new_vendor
            );
        }
        self.power_status = new_power;
        self.vendor = new_vendor;
        // Update the inotify path if it changed (e.g. dGPU appeared/disappeared)
        if self.dgpu_runtime_status_path != new_path {
            self.dgpu_runtime_status_path = new_path;
        }
        changed
    }
}

#[interface(name = "xyz.ljones.Gpu")]
impl CtrlGpu {
    /// The current GPU power status.
    #[zbus(property)]
    fn power_status(&self) -> &str {
        let s: &str = (&self.power_status).into();
        s
    }

    /// The GPU vendor name.
    #[zbus(property)]
    fn vendor(&self) -> String {
        self.vendor.to_string()
    }

    /// The current GPU mode, derived from platform sysfs attributes.
    /// Returns "Optimus", "Integrated", "Vfio", "Ultimate", "Egpu", or "Unknown".
    #[zbus(property)]
    fn mode(&self) -> String {
        use rog_platform::gpu_pci::{
            asus_dgpu_disable_exists, asus_dgpu_disabled, asus_gpu_mux_discreet,
            asus_gpu_mux_exists,
        };

        if asus_dgpu_disable_exists() {
            if let Ok(disabled) = asus_dgpu_disabled() {
                if disabled {
                    return "Integrated".to_string();
                }
            }
        }
        if asus_gpu_mux_exists() {
            if let Ok(discreet) = asus_gpu_mux_discreet() {
                if discreet {
                    return "Ultimate".to_string();
                }
            }
        }
        // If a dGPU is active, it's in Optimus/hybrid mode
        match self.power_status {
            GfxPower::Active | GfxPower::Suspended | GfxPower::Off => "Optimus".to_string(),
            _ => "Unknown".to_string(),
        }
    }
}

impl crate::ZbusRun for CtrlGpu {
    async fn add_to_server(self, server: &mut Connection) {
        Self::add_to_server_helper(self, GPU_ZBUS_PATH, server).await;
    }
}

impl CtrlGpu {
    /// Spawn the inotify watcher for GPU power status changes.
    ///
    /// This watches the dGPU's `runtime_status` sysfs file. If no dGPU path
    /// is available (e.g. dGPU disabled via ASUS attribute), we fall back to
    /// periodic re-detection every 3 seconds to handle hotplug scenarios.
    pub async fn start_watcher(&self, signal_ctxt: SignalEmitter<'static>) -> Result<(), RogError> {
        let ctrl_inotify = self.clone();
        let ctrl_fallback = self.clone();
        let path = self.dgpu_runtime_status_path.clone();

        tokio::spawn(async move {
            if let Some(runtime_path) = path {
                // inotify-based monitoring of the runtime_status sysfs file
                info!("CtrlGpu: starting inotify watcher on {:?}", runtime_path);
                let mut ctrl = ctrl_inotify;
                let mut buffer = [0u8; 32];

                loop {
                    let inotify = match inotify::Inotify::init() {
                        Ok(i) => i,
                        Err(e) => {
                            error!("CtrlGpu: failed to init inotify: {e}");
                            // Fall back to polling
                            break;
                        }
                    };

                    if let Err(e) = inotify.watches().add(
                        &runtime_path,
                        inotify::WatchMask::MODIFY | inotify::WatchMask::DELETE_SELF,
                    ) {
                        warn!(
                            "CtrlGpu: inotify watch failed on {:?}: {e}. Re-detecting...",
                            runtime_path
                        );
                        // The file might not exist yet (dGPU removed). Re-detect.
                        if ctrl.redetect() {
                            let status_str: &str = (&ctrl.power_status).into();
                            let _ = signal_ctxt
                                .emit("xyz.ljones.Gpu", "PowerStatusChanged", &(status_str,))
                                .await;
                        }
                        // Wait a bit before retrying
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        continue;
                    }

                    use futures_util::StreamExt;
                    let mut events = match inotify.into_event_stream(&mut buffer) {
                        Ok(s) => s,
                        Err(e) => {
                            error!("CtrlGpu: failed to create event stream: {e}");
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                            continue;
                        }
                    };

                    while let Some(event) = events.next().await {
                        match event {
                            Ok(ev) => {
                                if ev.mask.contains(inotify::EventMask::DELETE_SELF) {
                                    warn!("CtrlGpu: runtime_status deleted, re-detecting");
                                    break;
                                }
                                // Read new power status
                                if let Some(dgpu) = rog_platform::gpu_pci::Device::find()
                                    .unwrap_or_default()
                                    .iter()
                                    .find(|d| d.is_dgpu())
                                {
                                    if let Ok(new_power) = dgpu.get_runtime_status() {
                                        if new_power != ctrl.power_status {
                                            info!("CtrlGpu: power status changed to {new_power:?}");
                                            ctrl.power_status = new_power;
                                            let status_str: &str = (&ctrl.power_status).into();
                                            let _ = signal_ctxt
                                                .emit(
                                                    "xyz.ljones.Gpu",
                                                    "PowerStatusChanged",
                                                    &(status_str,),
                                                )
                                                .await;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("CtrlGpu: inotify event error: {e}");
                                break;
                            }
                        }
                    }
                }
            }

            // Fallback: periodic re-detection (for when no inotify path is available,
            // or when inotify loop exited)
            info!("CtrlGpu: starting periodic re-detection (every 3s)");
            let mut ctrl = ctrl_fallback;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                if ctrl.redetect() {
                    let status_str: &str = (&ctrl.power_status).into();
                    let _ = signal_ctxt
                        .emit("xyz.ljones.Gpu", "PowerStatusChanged", &(status_str,))
                        .await;
                }
            }
        });

        Ok(())
    }
}
