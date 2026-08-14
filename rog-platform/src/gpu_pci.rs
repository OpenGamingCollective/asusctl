//! GPU PCI device detection and power status monitoring.
//!
//! This module provides functionality to detect discrete GPUs via udev/PCI
//! and read their runtime power status from sysfs. It is used by
//! rog-control-center to color the tray icon and send status notifications.

use std::fmt::Display;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use log::{info, trace, warn};
use serde::{Deserialize, Serialize};
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::error::{PlatformError, Result};

// --- ASUS-specific sysfs paths (reused from rog-platform) ---

// Both locations read the same WMI devstate and report the same values. The
// `asus-nb-wmi` attributes are deprecated in the kernel and are compiled out
// with CONFIG_ASUS_WMI_DEPRECATED_ATTRS=n, so firmware-attributes comes first.
const ASUS_DGPU_DISABLE_PATHS: [&str; 2] = [
    "/sys/class/firmware-attributes/asus-armoury/attributes/dgpu_disable/current_value",
    "/sys/devices/platform/asus-nb-wmi/dgpu_disable",
];
const ASUS_GPU_MUX_PATHS: [&str; 2] = [
    "/sys/class/firmware-attributes/asus-armoury/attributes/gpu_mux_mode/current_value",
    "/sys/devices/platform/asus-nb-wmi/gpu_mux_mode",
];

/// The first of `paths` that this machine actually has.
fn first_existing(paths: &[&str]) -> Option<PathBuf> {
    paths
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
        .map(Path::to_path_buf)
}

/// Read an attribute whose value is a single digit.
fn read_digit(path: &Path) -> Result<u8> {
    let mut file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| PlatformError::Read(path.to_string_lossy().to_string(), e))?;
    let mut buf = [0u8; 1];
    file.read_exact(&mut buf)
        .map_err(|e| PlatformError::Read(path.to_string_lossy().to_string(), e))?;
    Ok(buf[0])
}

/// Path of the ASUS dgpu_disable attribute, if this machine has one.
pub fn asus_dgpu_disable_path() -> Option<PathBuf> {
    first_existing(&ASUS_DGPU_DISABLE_PATHS)
}

/// Path of the ASUS gpu_mux_mode attribute, if this machine has one.
pub fn asus_gpu_mux_path() -> Option<PathBuf> {
    first_existing(&ASUS_GPU_MUX_PATHS)
}

/// Check if the ASUS dgpu_disable attribute exists.
pub fn asus_dgpu_disable_exists() -> bool {
    asus_dgpu_disable_path().is_some()
}

/// Read the ASUS dgpu_disable value.
pub fn asus_dgpu_disabled() -> Result<bool> {
    let path = asus_dgpu_disable_path().ok_or(PlatformError::NotSupported)?;
    Ok(read_digit(&path)? == b'1')
}

/// Check if the ASUS gpu_mux_mode attribute exists.
pub fn asus_gpu_mux_exists() -> bool {
    asus_gpu_mux_path().is_some()
}

/// Read the ASUS gpu_mux_mode value. Returns true if in discreet (dGPU) mode.
pub fn asus_gpu_mux_discreet() -> Result<bool> {
    let path = asus_gpu_mux_path().ok_or(PlatformError::NotSupported)?;
    // gpu_mux_mode: 0 = dGPU (discreet), 1 = Optimus (hybrid)
    Ok(read_digit(&path)? == b'0')
}

// --- GfxPower ---

/// The runtime power status of the discrete GPU.
#[derive(
    Debug, Default, Type, Value, OwnedValue, PartialEq, Eq, Copy, Clone, Serialize, Deserialize,
)]
pub enum GfxPower {
    Active,
    Suspended,
    AsusDisabled,
    AsusMuxDiscreet,
    #[default]
    Unknown,
}

impl FromStr for GfxPower {
    type Err = PlatformError;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s.to_lowercase().trim() {
            "active" => GfxPower::Active,
            "suspended" => GfxPower::Suspended,
            "dgpu_disabled" => GfxPower::AsusDisabled,
            "asus_mux_discreet" => GfxPower::AsusMuxDiscreet,
            _ => GfxPower::Unknown,
        })
    }
}

impl From<&GfxPower> for &str {
    fn from(gfx: &GfxPower) -> &'static str {
        match gfx {
            GfxPower::Active => "active",
            GfxPower::Suspended => "suspended",
            GfxPower::AsusDisabled => "dgpu_disabled",
            GfxPower::AsusMuxDiscreet => "asus_mux_discreet",
            GfxPower::Unknown => "unknown",
        }
    }
}

impl Display for GfxPower {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: &str = self.into();
        write!(f, "{}", s)
    }
}

// --- PCI GPU identification ---

/// Nvidia PCI vendor ID, as it appears in the udev `PCI_ID` property.
const NVIDIA_PCI_VENDOR: &str = "10DE";
/// AMD PCI vendor ID, as it appears in the udev `PCI_ID` property.
const AMD_PCI_VENDOR: &str = "1002";

/// True if a udev `PCI_ID` property (`vendor:device`) belongs to a GPU vendor
/// that is handled here.
pub fn is_gpu_vendor(pci_id: &str) -> bool {
    pci_id.starts_with(NVIDIA_PCI_VENDOR) || pci_id.starts_with(AMD_PCI_VENDOR)
}

/// True if a udev `PCI_CLASS` property is a display controller (base class
/// `0x03`).
///
/// `PCI_CLASS` is the 24-bit class code in hex without leading zeros, e.g.
/// `30000` for a VGA controller and `30200` for a 3D controller.
pub fn is_display_class(pci_class: &str) -> bool {
    u32::from_str_radix(pci_class, 16).is_ok_and(|class| class >> 16 == 0x03)
}

fn read_hwmon_temp(dir: &Path) -> Option<f32> {
    fs::read_to_string(dir.join("temp1_input"))
        .ok()?
        .trim()
        .parse::<f32>()
        .ok()
        .map(|t| t / 1000.0)
}

fn read_drm_busy(dir: &Path) -> Option<f32> {
    fs::read_to_string(dir.join("device/gpu_busy_percent"))
        .or_else(|_| fs::read_to_string(dir.join("gpu_busy_percent")))
        .ok()?
        .trim()
        .parse::<f32>()
        .ok()
}

fn read_nvml_temp() -> Option<f32> {
    let nvml = nvml_wrapper::Nvml::init().ok()?;
    let device = nvml.device_by_index(0).ok()?;
    let temp = device
        .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
        .ok()?;
    Some(temp as f32)
}

fn read_nvml_usage() -> Option<f32> {
    let nvml = nvml_wrapper::Nvml::init().ok()?;
    let device = nvml.device_by_index(0).ok()?;
    let rates = device.utilization_rates().ok()?;
    Some(rates.gpu as f32)
}

// --- Device ---

/// A PCI GPU device.
#[derive(Clone, Debug)]
pub struct Device {
    /// Path to the device sysfs entry.
    dev_path: PathBuf,
    /// Whether this device is the discrete GPU.
    is_dgpu: bool,
    /// Vendor:Device PCI ID string.
    pci_id: String,
}

impl Device {
    pub fn dev_path(&self) -> &PathBuf {
        &self.dev_path
    }

    pub fn is_dgpu(&self) -> bool {
        self.is_dgpu
    }

    pub fn pci_id(&self) -> &str {
        &self.pci_id
    }

    /// Read a file underneath the sys object.
    fn read_file(path: PathBuf) -> Result<String> {
        fs::read_to_string(&path)
            .map_err(|e| PlatformError::Read(path.to_string_lossy().to_string(), e))
    }

    /// Read the runtime power status from sysfs.
    pub fn get_runtime_status(&self) -> Result<GfxPower> {
        let mut path = self.dev_path.clone();
        path.push("power");
        path.push("runtime_status");
        trace!("get_runtime_status: {path:?}");
        match Self::read_file(path) {
            Ok(inner) => GfxPower::from_str(inner.as_str()),
            // The device is gone or its runtime PM state is unreadable. `off` is
            // not a value runtime_status ever reports, so don't invent it.
            Err(_) => Ok(GfxPower::Unknown),
        }
    }

    /// Read the temperature (°C) of this GPU from sysfs hwmon.
    ///
    /// If this is a discrete GPU and it is not in the `Active` power state,
    /// this immediately returns `Some(0.0)` without reading sysfs hwmon
    /// nodes to prevent waking the PCIe device from runtime PM sleep.
    pub fn get_temp(&self) -> Option<f32> {
        if self.is_dgpu
            && self.get_runtime_status().unwrap_or(GfxPower::Unknown) != GfxPower::Active
        {
            return Some(0.0);
        }

        // 1. Direct hwmon directory under device path
        if let Ok(entries) = fs::read_dir(self.dev_path.join("hwmon")) {
            for entry in entries.flatten() {
                if let Some(temp) = read_hwmon_temp(&entry.path()) {
                    return Some(temp);
                }
            }
        }

        // 2. Global /sys/class/hwmon matching this device's sysfs path
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_match = path.join("device").canonicalize().ok().is_some_and(|p| {
                    p == self.dev_path
                        || self.dev_path.starts_with(&p)
                        || p.starts_with(&self.dev_path)
                });
                if is_match {
                    if let Some(temp) = read_hwmon_temp(&path) {
                        return Some(temp);
                    }
                }
            }
        }

        // 3. Fallback to NVML if this is an NVIDIA device and hwmon is not available
        if self.pci_id.to_uppercase().starts_with(NVIDIA_PCI_VENDOR) {
            if let Some(temp) = read_nvml_temp() {
                return Some(temp);
            }
        }

        None
    }

    /// Read the GPU utilization percentage (0.0 - 100.0) from sysfs DRM nodes.
    ///
    /// If this is a discrete GPU and it is not in the `Active` power state,
    /// this immediately returns `Some(0.0)` without reading sysfs DRM
    /// nodes to prevent waking the PCIe device from runtime PM sleep.
    pub fn get_usage_pct(&self) -> Option<f32> {
        if self.is_dgpu
            && self.get_runtime_status().unwrap_or(GfxPower::Unknown) != GfxPower::Active
        {
            return Some(0.0);
        }

        // 1. Direct gpu_busy_percent under device path
        if let Some(busy) = read_drm_busy(&self.dev_path) {
            return Some(busy);
        }

        // 2. DRM card directories under device path
        if let Ok(entries) = fs::read_dir(self.dev_path.join("drm")) {
            for entry in entries.flatten() {
                if let Some(busy) = read_drm_busy(&entry.path()) {
                    return Some(busy);
                }
            }
        }

        // 3. Global /sys/class/drm matching this device's sysfs path
        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_match = path.join("device").canonicalize().ok().is_some_and(|p| {
                    p == self.dev_path
                        || self.dev_path.starts_with(&p)
                        || p.starts_with(&self.dev_path)
                });
                if is_match {
                    if let Some(busy) = read_drm_busy(&path) {
                        return Some(busy);
                    }
                }
            }
        }

        // 4. Fallback to NVML if this is an NVIDIA device and DRM busy is not available
        if self.pci_id.to_uppercase().starts_with(NVIDIA_PCI_VENDOR) {
            if let Some(usage) = read_nvml_usage() {
                return Some(usage);
            }
        }

        None
    }

    /// Enumerate PCI GPU devices via udev and identify the dGPU.
    pub fn find() -> Result<Vec<Self>> {
        let mut devices = Vec::new();
        let mut parent = String::new();

        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("enumerator failed".into(), err)
        })?;

        enumerator.match_subsystem("pci").map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        let get_parent = |dev: &udev::Device| -> String {
            dev.sysname()
                .to_string_lossy()
                .trim_end_matches(char::is_numeric)
                .trim_end_matches('.')
                .to_string()
        };

        for device in enumerator.scan_devices().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("scan_devices failed".into(), err)
        })? {
            let sysname = device.sysname().to_string_lossy();
            trace!("Looking at PCI device {:?}", sysname);
            if let Some(id) = device.property_value("PCI_ID") {
                if let Some(class) = device.property_value("PCI_CLASS") {
                    let id = id.to_string_lossy();
                    let class = class.to_string_lossy();
                    // Match only Nvidia or AMD
                    if is_gpu_vendor(&id) {
                        let mut dgpu = false;
                        // Check connected displays to distinguish dGPU from iGPU.
                        // eDP-1 is the internal panel, always on iGPU.
                        let displays =
                            find_connected_displays(device.syspath()).unwrap_or_default();
                        if !displays.contains(&"eDP-1".to_string()) {
                            trace!(
                                "Matched dGPU {id} at {:?} by checking display connections",
                                device.sysname()
                            );
                            dgpu = is_display_class(&class);
                        } else {
                            trace!(
                                "Device {id} at {:?} appears to be the iGPU",
                                device.sysname()
                            );
                        }
                        if !dgpu && id.starts_with(AMD_PCI_VENDOR) {
                            trace!(
                                "Found dGPU Device {id} without boot_vga attribute at {:?}",
                                device.sysname()
                            );
                            // Fallback: check hwmon for AMD iGPU detection
                            let mut dev_path = PathBuf::from(device.syspath());
                            dev_path.push("hwmon");

                            let hwmon_n_opt = match dev_path.read_dir() {
                                Ok(mut entries) => entries.next(),
                                Err(e) => {
                                    trace!("Error reading hwmon directory: {}", e);
                                    None
                                }
                            };

                            if let Some(Ok(hwmon_n)) = hwmon_n_opt {
                                let mut hwmon_path = hwmon_n.path();
                                hwmon_path.push("in1_input");
                                dgpu = !hwmon_path.exists();
                            }
                        }
                        if !dgpu {
                            if let Some(label) = device.property_value("ID_MODEL_FROM_DATABASE") {
                                trace!(
                                    "Found ID_MODEL_FROM_DATABASE property {id} at {:?} : {label:?}",
                                    device.sysname()
                                );
                                dgpu = lscpi_dgpu_check(&label.to_string_lossy());
                            } else {
                                trace!(
                                    "Didn't find dGPU with standard methods, using last resort for id:{id} at {:?}",
                                    device.sysname()
                                );
                                dgpu = lscpi_dgpu_check(&lscpi(&id).unwrap_or_default());
                            }
                        }

                        if dgpu || (!parent.is_empty() && sysname.contains(&parent)) {
                            if dgpu {
                                info!("Found dgpu {id} at {:?}", device.sysname());
                            } else {
                                info!("Found additional device {id} at {:?}", device.sysname());
                            }
                            parent = get_parent(&device);
                            devices.push(Self {
                                dev_path: PathBuf::from(device.syspath()),
                                is_dgpu: dgpu,
                                pci_id: id.to_string(),
                            });
                        }
                    }
                }
            }
            if !parent.is_empty() && !sysname.contains(&parent) {
                break;
            }
        }

        Ok(devices)
    }
}

// --- Utility functions ---

/// Check an lspci label string for dGPU patterns.
pub fn lscpi_dgpu_check(label: &str) -> bool {
    for pat in [
        "Radeon RX", "AMD/ATI", "GeForce", "Geforce", "Quadro", "T1200",
    ] {
        if label.contains(pat) {
            return true;
        }
    }
    false
}

fn lscpi(vendor_device: &str) -> Result<String> {
    let mut cmd = Command::new("lspci");
    cmd.args([
        "-d", vendor_device,
    ]);
    let output = cmd.output().map_err(PlatformError::Io)?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Find connected displays for a GPU by scanning its DRM card directory.
pub fn find_connected_displays(gpu_path: &Path) -> Result<Vec<String>> {
    let drm_path = gpu_path.join("drm");

    let card_dir = drm_path
        .read_dir()
        .map_err(|e| PlatformError::Read(drm_path.to_string_lossy().to_string(), e))?
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("card"))
        .map(|entry| entry.path())
        .ok_or(PlatformError::NotSupported)?;

    let displays = card_dir
        .read_dir()
        .map_err(|e| PlatformError::Read(card_dir.to_string_lossy().to_string(), e))?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if name.contains('-')
                && fs::read_to_string(entry.path().join("status"))
                    .is_ok_and(|status| status.trim() == "connected")
            {
                name.split_once('-').map(|(_, display)| display.to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(displays)
}

/// Get the current GPU power status, using all available detection methods.
///
/// This is the main entry point for determining dGPU power state. It tries:
/// 1. ASUS dgpu_disable attribute — writing 1 does not remove the device from
///    the PCI bus, so in integrated mode it must win over a still-enumerated
///    dGPU
/// 2. Direct PCI device detection (if dGPU devices are found)
/// 3. ASUS gpu_mux_mode attribute
pub fn get_gpu_power_status() -> GfxPower {
    if asus_dgpu_disabled().unwrap_or(false) {
        return GfxPower::AsusDisabled;
    }

    if let Some(dgpu) = Device::find()
        .ok()
        .and_then(|devs| devs.into_iter().find(|d| d.is_dgpu()))
    {
        return dgpu.get_runtime_status().unwrap_or(GfxPower::Unknown);
    }

    if asus_gpu_mux_discreet().unwrap_or(false) {
        return GfxPower::AsusMuxDiscreet;
    }

    GfxPower::Unknown
}

fn lookup_amdgpu_name(device_id: &str, revision: &str) -> Option<String> {
    let content = fs::read_to_string("/usr/share/libdrm/amdgpu.ids").ok()?;
    for line in content.lines().map(str::trim) {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').map(str::trim).collect();
        if parts.len() >= 3
            && parts[0].eq_ignore_ascii_case(device_id)
            && parts[1].eq_ignore_ascii_case(revision)
            && !parts[2].is_empty()
        {
            return Some(parts[2].to_string());
        }
    }
    None
}

pub fn get_gpu_names() -> (String, String) {
    let mut igpu = None;
    let mut dgpu = None;

    if let Ok(mut enumerator) = udev::Enumerator::new() {
        if enumerator.match_subsystem("pci").is_ok() {
            if let Ok(devices) = enumerator.scan_devices() {
                for device in devices {
                    if let Some(class) = device.property_value("PCI_CLASS") {
                        let class_str = class.to_string_lossy();
                        if class_str.starts_with("03") || class_str.starts_with("3") {
                            let id_val = device
                                .property_value("PCI_ID")
                                .map(|s| s.to_string_lossy().into_owned())
                                .unwrap_or_default();

                            let mut parts = id_val.split(':');
                            let vendor = parts.next().unwrap_or("").to_lowercase();
                            let device_id = parts.next().unwrap_or("").to_lowercase();

                            let mut model_name = String::new();
                            if vendor.eq_ignore_ascii_case(AMD_PCI_VENDOR) && !device_id.is_empty()
                            {
                                let revision_path = device.syspath().join("revision");
                                let revision = std::fs::read_to_string(revision_path)
                                    .unwrap_or_default()
                                    .trim()
                                    .trim_start_matches("0x")
                                    .to_lowercase();
                                if let Some(amd_name) = lookup_amdgpu_name(&device_id, &revision) {
                                    model_name = amd_name;
                                }
                            }

                            if model_name.is_empty() {
                                if let Some(model) = device.property_value("ID_MODEL_FROM_DATABASE")
                                {
                                    model_name = model.to_string_lossy().into_owned();
                                }
                            }
                            if model_name.is_empty() {
                                model_name = id_val.clone();
                            }
                            if model_name.is_empty() {
                                model_name = "Unknown GPU".to_string();
                            }

                            let is_dgpu = vendor.eq_ignore_ascii_case(NVIDIA_PCI_VENDOR)
                                || model_name.contains("GeForce")
                                || model_name.contains("Radeon RX")
                                || model_name.contains("Discrete");

                            if is_dgpu {
                                dgpu = Some(model_name);
                            } else {
                                igpu = Some(model_name);
                            }
                        }
                    }
                }
            }
        }
    }

    (
        igpu.unwrap_or_else(|| "Integrated GPU".to_string()),
        dgpu.unwrap_or_else(|| "Discrete GPU".to_string()),
    )
}

/// Telemetry metrics for both integrated and discrete GPUs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpuTelemetry {
    pub igpu_temp: f32,
    pub igpu_usage: f32,
    pub dgpu_temp: f32,
    pub dgpu_usage: f32,
}

impl Default for GpuTelemetry {
    fn default() -> Self {
        Self {
            igpu_temp: -1.0,
            igpu_usage: -1.0,
            dgpu_temp: 0.0,
            dgpu_usage: 0.0,
        }
    }
}

/// Retrieve telemetry metrics for all detected GPUs in a single udev scan.
pub fn get_gpu_telemetry() -> GpuTelemetry {
    let mut telemetry = GpuTelemetry::default();
    let dgpu_active = get_gpu_power_status() == GfxPower::Active;

    if let Ok(devices) = Device::find() {
        for device in devices {
            if device.is_dgpu() {
                if dgpu_active {
                    telemetry.dgpu_temp = device.get_temp().unwrap_or(0.0);
                    telemetry.dgpu_usage = device.get_usage_pct().unwrap_or(0.0);
                }
            } else {
                telemetry.igpu_temp = device.get_temp().unwrap_or(-1.0);
                telemetry.igpu_usage = device.get_usage_pct().unwrap_or(-1.0);
            }
        }
    }

    telemetry
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A scratch directory unique to this test and process, removed on drop so
    /// nothing is left behind even when the test panics.
    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("{name}_{}", std::process::id()));
            fs::create_dir_all(&dir).expect("failed to create test dir");
            Self(dir)
        }

        fn join(&self, path: &str) -> PathBuf {
            self.0.join(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fake_device(dev_path: PathBuf) -> Device {
        Device {
            dev_path,
            is_dgpu: true,
            pci_id: "10DE:2820".to_string(),
        }
    }

    #[test]
    fn gpu_vendor_matching() {
        assert!(is_gpu_vendor("10DE:2820"));
        assert!(is_gpu_vendor("1002:1638"));
        assert!(!is_gpu_vendor("8086:A7A0"));
        assert!(!is_gpu_vendor(""));
        // udev reports PCI_ID in uppercase hex, lowercase is not a valid input
        assert!(!is_gpu_vendor("10de:2820"));
    }

    #[test]
    fn display_class_matching() {
        assert!(is_display_class("30000")); // VGA controller
        assert!(is_display_class("30200")); // 3D controller
        assert!(is_display_class("38000")); // other display controller
        assert!(!is_display_class("20000")); // network controller
        assert!(!is_display_class("c0330")); // USB controller
        assert!(!is_display_class("3")); // base class alone is not a class code
        assert!(!is_display_class(""));
        assert!(!is_display_class("not-hex"));
    }

    #[test]
    fn first_existing_returns_first_present_path(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = TestDir::new("asusctl_test_first_existing");
        let real = dir.join("present");
        fs::write(&real, "1")?;
        let missing = dir.join("missing").to_string_lossy().to_string();
        let real_str = real.to_string_lossy().to_string();

        assert_eq!(
            first_existing(&[
                missing.as_str(),
                real_str.as_str()
            ]),
            Some(real.clone())
        );
        assert_eq!(
            first_existing(&[
                real_str.as_str(),
                missing.as_str()
            ]),
            Some(real)
        );
        assert_eq!(first_existing(&[missing.as_str()]), None);
        assert_eq!(first_existing(&[]), None);
        Ok(())
    }

    #[test]
    fn read_digit_reads_first_byte() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = TestDir::new("asusctl_test_read_digit");
        let attr = dir.join("current_value");

        fs::write(&attr, "1\n")?;
        assert_eq!(read_digit(&attr)?, b'1');
        fs::write(&attr, "0")?;
        assert_eq!(read_digit(&attr)?, b'0');

        fs::write(&attr, "")?;
        assert!(read_digit(&attr).is_err());
        assert!(read_digit(&dir.join("missing")).is_err());
        Ok(())
    }

    #[test]
    fn unreadable_runtime_status_is_unknown_not_off(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = TestDir::new("asusctl_test_runtime_status");
        fs::create_dir_all(dir.join("power"))?;
        let device = fake_device(dir.0.clone());

        // no runtime_status file at all: the device is gone, not powered off
        assert_eq!(device.get_runtime_status()?, GfxPower::Unknown);

        fs::write(dir.join("power/runtime_status"), "active\n")?;
        assert_eq!(device.get_runtime_status()?, GfxPower::Active);

        fs::write(dir.join("power/runtime_status"), "suspended\n")?;
        assert_eq!(device.get_runtime_status()?, GfxPower::Suspended);

        fs::write(dir.join("power/runtime_status"), "unsupported\n")?;
        assert_eq!(device.get_runtime_status()?, GfxPower::Unknown);
        Ok(())
    }

    #[test]
    fn device_get_temp_and_usage_when_suspended(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = TestDir::new("asusctl_test_temp_suspended");
        fs::create_dir_all(dir.join("power"))?;
        fs::write(dir.join("power/runtime_status"), "suspended\n")?;

        let hwmon_dir = dir.join("hwmon/hwmon0");
        fs::create_dir_all(&hwmon_dir)?;
        fs::write(hwmon_dir.join("temp1_input"), "55000\n")?;
        fs::write(dir.join("gpu_busy_percent"), "80\n")?;

        let device = fake_device(dir.0.clone());
        // Discrete GPU in suspended state must return 0.0 without querying hwmon/drm
        assert_eq!(device.get_temp(), Some(0.0));
        assert_eq!(device.get_usage_pct(), Some(0.0));
        Ok(())
    }

    #[test]
    fn device_get_temp_and_usage_when_active() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        let dir = TestDir::new("asusctl_test_temp_active");
        fs::create_dir_all(dir.join("power"))?;
        fs::write(dir.join("power/runtime_status"), "active\n")?;

        let hwmon_dir = dir.join("hwmon/hwmon0");
        fs::create_dir_all(&hwmon_dir)?;
        fs::write(hwmon_dir.join("temp1_input"), "62500\n")?;
        fs::write(dir.join("gpu_busy_percent"), "45\n")?;

        let device = fake_device(dir.0.clone());
        assert_eq!(device.get_temp(), Some(62.5));
        assert_eq!(device.get_usage_pct(), Some(45.0));
        Ok(())
    }

    #[test]
    #[ignore = "requires ASUS hardware with a dGPU"]
    fn live_dgpu_detection() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let devices = Device::find()?;
        let dgpu = devices.iter().find(|d| d.is_dgpu()).expect("no dGPU found");
        assert!(is_gpu_vendor(dgpu.pci_id()));
        println!(
            "dGPU {} runtime status: {:?}",
            dgpu.pci_id(),
            dgpu.get_runtime_status()?
        );
        Ok(())
    }
}
