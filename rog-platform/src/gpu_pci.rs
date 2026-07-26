//! GPU PCI device detection and power status monitoring.
//!
//! This module provides functionality to detect discrete GPUs via udev/PCI
//! and read their runtime power status from sysfs. It is used by asusd to
//! expose GPU power status over D-Bus for the tray icon color.

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

const PCI_BUS_PATH: &str = "/sys/bus/pci";
const SLOTS: &str = "/sys/bus/pci/slots";

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
    Off,
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
            "off" => GfxPower::Off,
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
            GfxPower::Off => "off",
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

// --- GfxVendor ---

/// GPU vendor identification.
#[derive(
    Debug, Default, Type, Value, OwnedValue, PartialEq, Eq, Copy, Clone, Serialize, Deserialize,
)]
pub enum GfxVendor {
    Nvidia,
    Amd,
    Intel,
    #[default]
    Unknown,
    AsusDgpuDisabled,
}

impl From<u16> for GfxVendor {
    fn from(vendor: u16) -> Self {
        match vendor {
            0x1002 => GfxVendor::Amd,
            0x10DE => GfxVendor::Nvidia,
            0x8086 => GfxVendor::Intel,
            _ => GfxVendor::Unknown,
        }
    }
}

impl Display for GfxVendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GfxVendor::Nvidia => write!(f, "Nvidia"),
            GfxVendor::Amd => write!(f, "AMD"),
            GfxVendor::Intel => write!(f, "Intel"),
            GfxVendor::Unknown => write!(f, "Unknown"),
            GfxVendor::AsusDgpuDisabled => write!(f, "ASUS dGPU disabled"),
        }
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

// --- Device ---

/// A PCI GPU device.
#[derive(Clone, Debug)]
pub struct Device {
    /// Path to the device sysfs entry.
    dev_path: PathBuf,
    /// Vendor of this device.
    vendor: GfxVendor,
    /// Whether this device is the discrete GPU.
    is_dgpu: bool,
    /// Kernel name, e.g. `0000:01:00.0`.
    #[allow(dead_code)]
    name: String,
    /// Vendor:Device PCI ID string.
    pci_id: String,
}

impl Device {
    pub fn dev_path(&self) -> &PathBuf {
        &self.dev_path
    }

    pub fn vendor(&self) -> GfxVendor {
        self.vendor
    }

    pub fn is_dgpu(&self) -> bool {
        self.is_dgpu
    }

    pub fn pci_id(&self) -> &str {
        &self.pci_id
    }

    /// Read a file underneath the sys object.
    fn read_file(path: PathBuf) -> Result<String> {
        let path = path
            .canonicalize()
            .map_err(|e| PlatformError::Read(path.to_string_lossy().to_string(), e))?;
        let mut data = String::new();
        let mut file = fs::OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|e| PlatformError::Read(path.to_string_lossy().to_string(), e))?;
        trace!("read_file: {file:?}");
        file.read_to_string(&mut data)
            .map_err(|e| PlatformError::Read(path.to_string_lossy().to_string(), e))?;
        Ok(data)
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
                        if let Some(vendor) = id.split(':').next() {
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
                                if let Some(label) = device.property_value("ID_MODEL_FROM_DATABASE")
                                {
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
                                let vendor_id: u16 = u16::from_str_radix(vendor, 16).unwrap_or(0);
                                devices.push(Self {
                                    dev_path: PathBuf::from(device.syspath()),
                                    vendor: vendor_id.into(),
                                    is_dgpu: dgpu,
                                    name: sysname.to_string(),
                                    pci_id: id.to_string(),
                                });
                            }
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

    // Find card directory (card0 or card1)
    let card_dir = drm_path
        .read_dir()
        .map_err(|e| PlatformError::Read(drm_path.to_string_lossy().to_string(), e))?
        .find_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().into_string().ok()?;
            if name.starts_with("card") {
                Some(entry.path())
            } else {
                None
            }
        })
        .ok_or(PlatformError::NotSupported)?;

    // Collect display names
    let displays: Vec<String> = card_dir
        .read_dir()
        .map_err(|e| PlatformError::Read(card_dir.to_string_lossy().to_string(), e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().into_string().ok()?;

            if name.contains('-') {
                // Check connection status
                let status_path = entry.path().join("status");
                let status = fs::read_to_string(status_path).ok()?;

                if status.trim() == "connected" {
                    name.split_once('-').map(|(_, display)| display.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    Ok(displays)
}

/// Find the PCI hotplug slot power control file for a device.
pub fn find_slot_power(address: &str) -> Result<PathBuf> {
    let mut buf = Vec::new();
    let path = PathBuf::from(SLOTS);
    if !path.exists() {
        return Err(PlatformError::NotSupported);
    }
    for path in path.read_dir().map_err(PlatformError::Io)? {
        let path = path.map_err(PlatformError::Io)?.path();

        let mut address_path = path.to_path_buf();
        address_path.push("address");

        let mut file = OpenOptions::new()
            .read(true)
            .open(&address_path)
            .map_err(PlatformError::Io)?;
        file.read_to_end(&mut buf).map_err(PlatformError::Io)?;

        if address.contains(String::from_utf8_lossy(&buf).trim_end()) {
            address_path.pop();
            address_path.push("power");
            info!("Found hotplug power slot at {:?}", address_path);
            return Ok(address_path);
        }
        buf.clear();
    }
    Err(PlatformError::NotSupported)
}

/// Rescan the PCI bus to add all removed devices back.
pub fn rescan_pci_bus() -> Result<()> {
    let path = PathBuf::from(PCI_BUS_PATH).join("rescan");
    std::fs::write(&path, "1")
        .map_err(|e| PlatformError::Write(path.to_string_lossy().to_string(), e))
}

/// Get the current GPU power status, using all available detection methods.
///
/// This is the main entry point for determining dGPU power state. It tries:
/// 1. Direct PCI device detection (if dGPU devices are found)
/// 2. ASUS dgpu_disable attribute
/// 3. ASUS gpu_mux_mode attribute
pub fn get_gpu_power_status() -> (GfxPower, GfxVendor) {
    let devices = Device::find().unwrap_or_default();

    if let Some(dgpu) = devices.iter().find(|d| d.is_dgpu()) {
        let vendor = dgpu.vendor();
        if let Ok(power) = dgpu.get_runtime_status() {
            return (power, vendor);
        }
        return (GfxPower::Unknown, vendor);
    }

    // No dGPU devices found — check ASUS-specific attributes
    if asus_dgpu_disable_exists() {
        if let Ok(disabled) = asus_dgpu_disabled() {
            if disabled {
                return (GfxPower::AsusDisabled, GfxVendor::AsusDgpuDisabled);
            }
        }
    }
    if asus_gpu_mux_exists() {
        if let Ok(discreet) = asus_gpu_mux_discreet() {
            if discreet {
                return (GfxPower::AsusMuxDiscreet, GfxVendor::Nvidia);
            }
        }
    }

    (GfxPower::Unknown, GfxVendor::Unknown)
}

fn lookup_amdgpu_name(device_id: &str, revision: &str) -> Option<String> {
    if let Ok(content) = std::fs::read_to_string("/usr/share/libdrm/amdgpu.ids") {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 3 {
                let d_id = parts[0].trim().to_lowercase();
                let r_id = parts[1].trim().to_lowercase();
                let name = parts[2].trim().to_string();
                if d_id == device_id && r_id == revision && !name.is_empty() {
                    return Some(name);
                }
            }
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
                            if vendor == "1002" && !device_id.is_empty() {
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

                            let is_dgpu = id_val.starts_with("10DE")
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

pub fn get_igpu_temp() -> f32 {
    if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(name) = std::fs::read_to_string(path.join("name")) {
                let name = name.trim();
                if name == "amdgpu" {
                    if let Ok(temp_str) = std::fs::read_to_string(path.join("temp1_input")) {
                        if let Ok(temp_val) = temp_str.trim().parse::<f32>() {
                            return temp_val / 1000.0;
                        }
                    }
                }
            }
        }
    }
    -1.0
}

pub fn get_igpu_usage_pct() -> f32 {
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if name.starts_with("card") {
                let busy_path = path.join("device/gpu_busy_percent");
                if busy_path.exists() {
                    if let Ok(vendor_str) = std::fs::read_to_string(path.join("device/vendor")) {
                        let vendor = vendor_str.trim();
                        if vendor == "0x1002" {
                            if let Ok(val_str) = std::fs::read_to_string(busy_path) {
                                if let Ok(val) = val_str.trim().parse::<f32>() {
                                    return val;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    -1.0
}

pub fn get_gpu_temp() -> f32 {
    if let Ok(nvml) = nvml_wrapper::Nvml::init() {
        if let Ok(device) = nvml.device_by_index(0) {
            if let Ok(temp) =
                device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
            {
                return temp as f32;
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(name) = std::fs::read_to_string(path.join("name")) {
                let name = name.trim();
                if name == "amdgpu" || name == "nouveau" {
                    if let Ok(temp_str) = std::fs::read_to_string(path.join("temp1_input")) {
                        if let Ok(temp_val) = temp_str.trim().parse::<f32>() {
                            return temp_val / 1000.0;
                        }
                    }
                }
            }
        }
    }
    0.0
}

pub fn get_gpu_usage_pct() -> f32 {
    if let Ok(nvml) = nvml_wrapper::Nvml::init() {
        if let Ok(device) = nvml.device_by_index(0) {
            if let Ok(rates) = device.utilization_rates() {
                return rates.gpu as f32;
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let path = entry.path().join("device/gpu_busy_percent");
            if path.exists() {
                if let Ok(val_str) = std::fs::read_to_string(path) {
                    if let Ok(val) = val_str.trim().parse::<f32>() {
                        return val;
                    }
                }
            }
        }
    }
    0.0
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
            vendor: GfxVendor::Nvidia,
            is_dgpu: true,
            name: "0000:01:00.0".to_string(),
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
    #[ignore = "requires ASUS hardware with a dGPU"]
    fn live_dgpu_detection() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let devices = Device::find()?;
        let dgpu = devices.iter().find(|d| d.is_dgpu()).expect("no dGPU found");
        assert_ne!(dgpu.vendor(), GfxVendor::Unknown);
        println!(
            "dGPU {} runtime status: {:?}",
            dgpu.pci_id(),
            dgpu.get_runtime_status()?
        );
        Ok(())
    }
}
