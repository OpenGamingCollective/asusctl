//! GPU PCI device detection and power status monitoring.
//!
//! This module enumerates PCI display devices with udev, classifies each as
//! the integrated or the discrete GPU, and reads their runtime power status
//! from sysfs. The classification uses the vendor-specific rules fastfetch
//! uses, not display connector state. It is used by rog-control-center to
//! color the tray icon and send status notifications.

use std::fmt::Display;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::OnceLock;

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

/// Parse a PCI sysname such as `0000:01:00.0` (or `01:00.0` when the domain is
/// omitted) into domain, bus, device and function.
fn parse_pci_address(sysname: &str) -> Option<(u16, u8, u8, u8)> {
    let (prefix, function) = sysname.rsplit_once('.')?;
    let mut parts = prefix.split(':');
    let first = parts.next()?;
    let second = parts.next()?;
    let (domain, bus, device) = match parts.next() {
        Some(device) => (first, second, device),
        None => ("0", first, second),
    };
    if parts.next().is_some() {
        return None;
    }
    Some((
        u16::from_str_radix(domain, 16).ok()?,
        u8::from_str_radix(bus, 16).ok()?,
        u8::from_str_radix(device, 16).ok()?,
        u8::from_str_radix(function, 16).ok()?,
    ))
}

/// True if this is an auxiliary display controller rather than a GPU: a
/// function above 0 with the `other display` subclass (`0x80`).
///
/// fastfetch skips these so one GPU is not counted twice.
fn is_auxiliary_display(sysname: &str, pci_class: &str) -> bool {
    let function = parse_pci_address(sysname).map(|(_, _, _, function)| function);
    let subclass = u32::from_str_radix(pci_class, 16)
        .ok()
        .map(|class| ((class >> 8) & 0xff) as u8);
    matches!((function, subclass), (Some(function), Some(0x80)) if function > 0)
}

/// Probe the device's hwmon directory for the APU-only `in1_input` node.
///
/// The amdgpu driver registers that northbridge voltage sensor only on APUs,
/// which is how fastfetch tells an AMD APU from a discrete card when
/// `AMDGPU_IDS_FLAGS_FUSION` cannot be queried through libdrm (this crate does
/// not link libdrm, so this is the sysfs fallback it uses).
///
/// `Some(true)` is an APU and `Some(false)` is a readable hwmon directory with
/// at least one entry but no APU sensor (a discrete card). `None` means the
/// directory could not be read or held no hwmon entry, so the probe is
/// inconclusive and the caller uses the model name.
fn amd_apu_hwmon(dev_path: &Path) -> Option<bool> {
    let entries = fs::read_dir(dev_path.join("hwmon")).ok()?;
    let mut inspected_entry = false;
    for entry in entries.flatten() {
        inspected_entry = true;
        if entry.path().join("in1_input").exists() {
            return Some(true);
        }
    }
    // An empty directory is not evidence of a discrete card
    inspected_entry.then_some(false)
}

/// The vendor half of a udev `PCI_ID` (`vendor:device`).
fn pci_vendor(pci_id: &str) -> &str {
    pci_id.split_once(':').map_or(pci_id, |(vendor, _)| vendor)
}

/// Classify a display device as discrete (`Some(true)`) or integrated
/// (`Some(false)`) from vendor-specific facts, mirroring fastfetch's Linux
/// detection. `None` means the vendor rules cannot decide, and the caller
/// falls back to the model name.
///
/// * AMD: an APU-only `in1_input` hwmon node means integrated; a readable
///   hwmon directory without it, with amdgpu bound, is a discrete card. Other
///   drivers (radeon) and an unreadable hwmon directory are not guessed at,
///   so fastfetch's fallback applies there too.
/// * NVIDIA: every NVIDIA display device on an x86 laptop is a dGPU, so the
///   vendor decides. fastfetch uses the NVML brand with a
///   GeForce/Quadro/Tesla name fallback; the vendor rule is the equivalent
///   for a boolean result, and it also covers names such as `RTX A1000` that
///   an NVML-less name check would miss.
fn classify_by_vendor(vendor: &str, driver: Option<&str>, apu_hwmon: Option<bool>) -> Option<bool> {
    match vendor {
        AMD_PCI_VENDOR => match apu_hwmon {
            Some(true) => Some(false),
            Some(false) if driver == Some("amdgpu") => Some(true),
            _ => None,
        },
        NVIDIA_PCI_VENDOR => Some(true),
        _ => None,
    }
}

/// Classify a whole udev `PCI_ID` (`vendor:device`), which is the value
/// [`Device::find`] actually has at hand.
fn classify_pci_id(pci_id: &str, driver: Option<&str>, apu_hwmon: Option<bool>) -> Option<bool> {
    classify_by_vendor(pci_vendor(pci_id), driver, apu_hwmon)
}

/// Collect the facts [`classify_pci_id`] needs from udev and sysfs.
fn detect_gpu_type(device: &udev::Device, pci_id: &str) -> Option<bool> {
    let driver = device.driver().map(|d| d.to_string_lossy().into_owned());
    // The APU sensor is only meaningful for AMD, so skip the sysfs read for
    // other vendors.
    let apu_hwmon = if pci_vendor(pci_id) == AMD_PCI_VENDOR {
        amd_apu_hwmon(device.syspath())
    } else {
        None
    };
    classify_pci_id(pci_id, driver.as_deref(), apu_hwmon)
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

/// The process-wide NVML handle, initialised on first successful use.
///
/// `Nvml::init` dlopens libnvidia-ml and costs milliseconds, so it is not
/// repeated per telemetry read. A failed init is not cached: the driver may
/// simply not be up yet, so the next call retries.
fn nvml() -> Option<&'static nvml_wrapper::Nvml> {
    static NVML: OnceLock<nvml_wrapper::Nvml> = OnceLock::new();
    if let Some(nvml) = NVML.get() {
        return Some(nvml);
    }
    let nvml = nvml_wrapper::Nvml::init().ok()?;
    Some(NVML.get_or_init(|| nvml))
}

fn read_nvml_temp() -> Option<f32> {
    let device = nvml()?.device_by_index(0).ok()?;
    let temp = device
        .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
        .ok()?;
    Some(temp as f32)
}

fn read_nvml_usage() -> Option<f32> {
    let device = nvml()?.device_by_index(0).ok()?;
    let rates = device.utilization_rates().ok()?;
    Some(rates.gpu as f32)
}

/// Graphics clock in MHz from an amdgpu hwmon directory, which reports the
/// shader clock as `freq1_input` in Hz.
fn read_hwmon_freq(dir: &Path) -> Option<f32> {
    let hz = fs::read_to_string(dir.join("freq1_input"))
        .ok()?
        .trim()
        .parse::<f32>()
        .ok()?;
    // 0 means the node exists but has no reading, not an idle clock
    (hz > 0.0).then(|| hz / 1_000_000.0)
}

fn read_nvml_freq() -> Option<f32> {
    let device = nvml()?.device_by_index(0).ok()?;
    let clock = device
        .clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics)
        .ok()?;
    Some(clock as f32)
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

    /// Probe this device's hwmon directories with `read`, falling back to
    /// `nvml` on NVIDIA hardware whose proprietary driver registers no hwmon.
    ///
    /// If this is a discrete GPU and it is not in the `Active` power state,
    /// this immediately returns `None` without accessing hwmon or NVML to prevent
    /// waking the PCIe device from runtime PM sleep.
    fn probe_hwmon(
        &self,
        read: fn(&Path) -> Option<f32>,
        nvml: fn() -> Option<f32>,
    ) -> Option<f32> {
        if self.is_dgpu && self.get_runtime_status().unwrap_or_default() != GfxPower::Active {
            return None;
        }

        // 1. Direct hwmon directory under device path
        if let Ok(entries) = fs::read_dir(self.dev_path.join("hwmon")) {
            for entry in entries.flatten() {
                if let Some(value) = read(&entry.path()) {
                    return Some(value);
                }
            }
        }

        // 2. Global /sys/class/hwmon matching this device's sysfs path
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let path = entry.path();
                // This GPU or something under it, never an ancestor bridge
                let is_match = path
                    .join("device")
                    .canonicalize()
                    .ok()
                    .is_some_and(|p| p == self.dev_path || p.starts_with(&self.dev_path));
                if is_match && let Some(value) = read(&path) {
                    return Some(value);
                }
            }
        }

        // 3. Fallback to NVML if this is an NVIDIA device and hwmon is not available
        if self.pci_id.to_uppercase().starts_with(NVIDIA_PCI_VENDOR)
            && self.get_runtime_status().unwrap_or_default() == GfxPower::Active
        {
            return nvml();
        }

        None
    }

    /// Read the temperature (°C) of this GPU from sysfs hwmon with NVML fallback.
    pub fn get_temp(&self) -> Option<f32> {
        self.probe_hwmon(read_hwmon_temp, read_nvml_temp)
    }

    /// Probe this device's DRM directories for usage percentage, falling back to
    /// `nvml` on NVIDIA hardware whose proprietary driver registers no DRM usage node.
    ///
    /// If this is a discrete GPU and it is not in the `Active` power state,
    /// this immediately returns `None` without accessing DRM sysfs or NVML to prevent
    /// waking the PCIe device from runtime PM sleep.
    fn probe_usage(&self, nvml: fn() -> Option<f32>) -> Option<f32> {
        if self.is_dgpu && self.get_runtime_status().unwrap_or_default() != GfxPower::Active {
            return None;
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
                // This GPU or something under it, never an ancestor bridge
                let is_match = path
                    .join("device")
                    .canonicalize()
                    .ok()
                    .is_some_and(|p| p == self.dev_path || p.starts_with(&self.dev_path));
                if is_match && let Some(busy) = read_drm_busy(&path) {
                    return Some(busy);
                }
            }
        }

        // 4. Fallback to NVML if this is an NVIDIA device and DRM busy is not available
        if self.pci_id.to_uppercase().starts_with(NVIDIA_PCI_VENDOR)
            && self.get_runtime_status().unwrap_or_default() == GfxPower::Active
            && let Some(usage) = nvml()
        {
            return Some(usage);
        }

        None
    }

    /// Read the GPU utilization percentage (0.0 - 100.0) from sysfs DRM nodes with NVML fallback.
    ///
    /// If this is a discrete GPU and it is not in the `Active` power state,
    /// this immediately returns `None` without accessing DRM sysfs or NVML to prevent
    /// waking the PCIe device from runtime PM sleep.
    pub fn get_usage_pct(&self) -> Option<f32> {
        self.probe_usage(read_nvml_usage)
    }

    /// Read the current graphics clock (MHz) of this GPU from sysfs hwmon with
    /// NVML fallback.
    pub fn get_freq_mhz(&self) -> Option<f32> {
        self.probe_hwmon(read_hwmon_freq, read_nvml_freq)
    }

    /// Enumerate PCI GPU devices via udev and identify the dGPU.
    pub fn find() -> Result<Vec<Self>> {
        let mut devices = Vec::new();

        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("enumerator failed".into(), err)
        })?;

        enumerator.match_subsystem("pci").map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for device in enumerator.scan_devices().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("scan_devices failed".into(), err)
        })? {
            let sysname = device.sysname().to_string_lossy();
            trace!("Looking at PCI device {:?}", sysname);
            if let Some(id) = device.property_value("PCI_ID")
                && let Some(class) = device.property_value("PCI_CLASS")
            {
                let id = id.to_string_lossy();
                let class = class.to_string_lossy();
                // Match only display devices of a vendor we can classify
                if is_gpu_vendor(&id) && is_display_class(&class) {
                    // An "other display" controller above function 0 is an
                    // auxiliary node of a GPU, not a second GPU.
                    if is_auxiliary_display(&sysname, &class) {
                        trace!("Skipping auxiliary display controller {id} at {sysname:?}");
                        continue;
                    }

                    let model = device
                        .property_value("ID_MODEL_FROM_DATABASE")
                        .or_else(|| device.property_value("ID_MODEL"))
                        .map(|model| model.to_string_lossy());

                    // Classify from vendor-specific facts, exactly like
                    // fastfetch: never from which connector a panel is on, as
                    // a mux switch puts the internal panel on the dGPU.
                    let dgpu = match detect_gpu_type(&device, &id) {
                        Some(is_dgpu) => is_dgpu,
                        // Only an AMD card on a driver we cannot interrogate
                        // (e.g. radeon) reaches this: use the model name.
                        None => model.as_deref().is_some_and(lspci_dgpu_check),
                    };

                    if dgpu {
                        info!("Found dgpu {id} at {sysname:?}");
                    } else {
                        info!("Found igpu {id} at {sysname:?}");
                    }
                    devices.push(Self {
                        dev_path: PathBuf::from(device.syspath()),
                        is_dgpu: dgpu,
                        pci_id: id.to_string(),
                    });
                }
            }
        }

        Ok(devices)
    }
}

// --- Utility functions ---

/// Check a device model or lspci label string for dGPU patterns.
pub fn lspci_dgpu_check(label: &str) -> bool {
    for pat in [
        "Radeon RX", "AMD/ATI", "GeForce", "Geforce", "Quadro", "T1200",
    ] {
        if label.contains(pat) {
            return true;
        }
    }
    false
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

    if let Ok(devs) = Device::find()
        && let Some(dgpu) = devs.into_iter().find(|d| d.is_dgpu())
    {
        return dgpu.get_runtime_status().unwrap_or_default();
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

    if let Ok(mut enumerator) = udev::Enumerator::new()
        && enumerator.match_subsystem("pci").is_ok()
        && let Ok(devices) = enumerator.scan_devices()
    {
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
                    if vendor.eq_ignore_ascii_case(AMD_PCI_VENDOR) && !device_id.is_empty() {
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

                    if model_name.is_empty()
                        && let Some(model) = device.property_value("ID_MODEL_FROM_DATABASE")
                    {
                        model_name = model.to_string_lossy().into_owned();
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
    pub dgpu_suspended: bool,
    pub dgpu_freq_mhz: f32,
}

impl Default for GpuTelemetry {
    fn default() -> Self {
        Self {
            igpu_temp: -1.0,
            igpu_usage: -1.0,
            dgpu_temp: -1.0,
            dgpu_usage: -1.0,
            dgpu_suspended: false,
            dgpu_freq_mhz: -1.0,
        }
    }
}

/// Retrieve telemetry metrics for all detected GPUs in a single udev scan.
pub fn get_gpu_telemetry() -> GpuTelemetry {
    let mut telemetry = GpuTelemetry::default();
    let power_status = get_gpu_power_status();
    let dgpu_active = power_status == GfxPower::Active;
    telemetry.dgpu_suspended = power_status == GfxPower::Suspended;

    if let Ok(devices) = Device::find() {
        for device in devices {
            if device.is_dgpu() {
                if dgpu_active {
                    telemetry.dgpu_temp = device.get_temp().unwrap_or(-1.0);
                    telemetry.dgpu_usage = device.get_usage_pct().unwrap_or(-1.0);
                    telemetry.dgpu_freq_mhz = device.get_freq_mhz().unwrap_or(-1.0);
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

    /// A dGPU that reads `active`, so the suspend guard does not short-circuit.
    fn fake_active_device(dev_path: PathBuf, pci_id: &str) -> Device {
        fs::create_dir_all(dev_path.join("power")).expect("failed to create power dir");
        fs::write(dev_path.join("power/runtime_status"), "active\n").expect("write");
        Device {
            dev_path,
            is_dgpu: true,
            pci_id: pci_id.to_string(),
        }
    }

    #[test]
    fn freq_reads_amdgpu_hwmon_hz() {
        let dir = TestDir::new("freq_amdgpu");
        fs::create_dir_all(dir.join("hwmon/hwmon1")).expect("mkdir");
        fs::write(dir.join("hwmon/hwmon1/freq1_input"), "2100000000\n").expect("write");
        let dev = fake_active_device(dir.0.clone(), "1002:1638");
        assert_eq!(dev.get_freq_mhz(), Some(2100.0));
    }

    #[test]
    fn freq_ignores_zero_hwmon_reading() {
        let dir = TestDir::new("freq_zero");
        fs::create_dir_all(dir.join("hwmon/hwmon1")).expect("mkdir");
        fs::write(dir.join("hwmon/hwmon1/freq1_input"), "0\n").expect("write");
        let dev = fake_active_device(dir.0.clone(), "1002:1638");
        assert_eq!(dev.get_freq_mhz(), None);
    }

    #[test]
    fn freq_none_when_no_nodes() {
        // AMD, so the NVML fallback is skipped and the result does not depend
        // on whether the test box has a live NVIDIA driver
        let dir = TestDir::new("freq_empty");
        let dev = fake_active_device(dir.0.clone(), "1002:1638");
        assert_eq!(dev.get_freq_mhz(), None);
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
        assert!(!is_display_class("40300")); // Audio controller
        assert!(!is_display_class("040300")); // Audio controller with leading zero
        assert!(!is_display_class("20000")); // network controller
        assert!(!is_display_class("c0330")); // USB controller
        assert!(!is_display_class("3")); // base class alone is not a class code
        assert!(!is_display_class(""));
        assert!(!is_display_class("not-hex"));
    }

    #[test]
    fn pci_address_parsing() {
        assert_eq!(parse_pci_address("0000:01:00.0"), Some((0, 1, 0, 0)));
        assert_eq!(parse_pci_address("0000:00:02.0"), Some((0, 0, 2, 0)));
        assert_eq!(parse_pci_address("0001:03:1f.7"), Some((1, 3, 0x1f, 7)));
        // a sysname without a domain
        assert_eq!(parse_pci_address("00:02.0"), Some((0, 0, 2, 0)));
        assert_eq!(parse_pci_address("card0"), None);
        assert_eq!(parse_pci_address("0000:00:02"), None);
        assert_eq!(parse_pci_address("0000:00:02.0.1"), None);
        assert_eq!(parse_pci_address("not-hex:00:02.0"), None);
    }

    #[test]
    fn auxiliary_display_function_is_skipped() {
        // A second function with the "other display" subclass is not a GPU
        assert!(is_auxiliary_display("0000:01:00.1", "38000"));
        // function 0 is the GPU itself
        assert!(!is_auxiliary_display("0000:01:00.0", "38000"));
        // a VGA or 3D controller function is a GPU
        assert!(!is_auxiliary_display("0000:01:00.1", "30000"));
        assert!(!is_auxiliary_display("0000:01:00.1", "30200"));
        assert!(!is_auxiliary_display("garbage", "38000"));
    }

    #[test]
    fn vendor_classification_follows_fastfetch() {
        // AMD APU: the APU-only in1_input hwmon node decides
        assert_eq!(
            classify_by_vendor(AMD_PCI_VENDOR, Some("amdgpu"), Some(true)),
            Some(false)
        );
        // AMD discrete card: a readable hwmon directory without the APU sensor
        assert_eq!(
            classify_by_vendor(AMD_PCI_VENDOR, Some("amdgpu"), Some(false)),
            Some(true)
        );
        // A driver we cannot interrogate is not guessed at
        assert_eq!(
            classify_by_vendor(AMD_PCI_VENDOR, Some("radeon"), Some(false)),
            None
        );
        assert_eq!(classify_by_vendor(AMD_PCI_VENDOR, None, Some(false)), None);
        // An unreadable hwmon directory is not evidence of a discrete card
        assert_eq!(
            classify_by_vendor(AMD_PCI_VENDOR, Some("amdgpu"), None),
            None
        );

        // Every NVIDIA display device on an x86 laptop is a dGPU
        assert_eq!(
            classify_by_vendor(NVIDIA_PCI_VENDOR, Some("nvidia"), None),
            Some(true)
        );
        assert_eq!(
            classify_by_vendor(NVIDIA_PCI_VENDOR, Some("nouveau"), None),
            Some(true)
        );
    }

    #[test]
    fn full_pci_id_is_classified() {
        // Device::find() passes the udev PCI_ID (`vendor:device`), not a bare
        // vendor ID; this is the path that the vendor rules must see.
        assert_eq!(
            classify_pci_id("1002:1114", Some("amdgpu"), Some(true)),
            Some(false),
            "an AMD APU must be integrated on the real call path"
        );
        assert_eq!(
            classify_pci_id("1002:73ff", Some("amdgpu"), Some(false)),
            Some(true)
        );
        assert_eq!(
            classify_pci_id("10DE:28b8", Some("nvidia"), None),
            Some(true)
        );
        // A bare vendor ID still works (PCI_ID without a device part)
        assert_eq!(
            classify_pci_id(AMD_PCI_VENDOR, Some("amdgpu"), Some(true)),
            Some(false)
        );
        // Intel is not handled at all
        assert_eq!(
            classify_pci_id("8086:a7a0", Some("i915"), Some(false)),
            None
        );
        // An unreadable AMD hwmon directory must stay undecided so that
        // Device::find() can fall back to the model name
        assert_eq!(
            classify_pci_id("1002:1114", Some("amdgpu"), None),
            None,
            "an unreadable hwmon directory is not a dGPU verdict"
        );
    }

    #[test]
    fn amd_apu_hwmon_detection() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = TestDir::new("amd_apu_hwmon");
        assert_eq!(amd_apu_hwmon(&dir.0), None);

        // An empty hwmon directory is inconclusive, not a dGPU verdict
        fs::create_dir_all(dir.join("hwmon"))?;
        assert_eq!(amd_apu_hwmon(&dir.0), None);

        // A discrete card exposes temperature and clock sensors only
        fs::create_dir_all(dir.join("hwmon/hwmon0"))?;
        fs::write(dir.join("hwmon/hwmon0/temp1_input"), "40000\n")?;
        fs::write(dir.join("hwmon/hwmon0/freq1_input"), "0\n")?;
        assert_eq!(amd_apu_hwmon(&dir.0), Some(false));

        // The northbridge voltage node only exists on APUs
        fs::write(dir.join("hwmon/hwmon0/in1_input"), "850\n")?;
        assert_eq!(amd_apu_hwmon(&dir.0), Some(true));
        Ok(())
    }

    #[test]
    fn first_existing_returns_first_present_path()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
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
    fn unreadable_runtime_status_is_unknown_not_off()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
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
    fn device_get_temp_and_usage_when_suspended()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = TestDir::new("asusctl_test_temp_suspended");
        fs::create_dir_all(dir.join("power"))?;
        fs::write(dir.join("power/runtime_status"), "suspended\n")?;

        let hwmon_dir = dir.join("hwmon/hwmon0");
        fs::create_dir_all(&hwmon_dir)?;
        fs::write(hwmon_dir.join("temp1_input"), "55000\n")?;
        fs::write(dir.join("gpu_busy_percent"), "80\n")?;

        let device = fake_device(dir.0.clone());
        // Discrete GPU in suspended state must return None without querying hwmon/drm/nvml
        assert_eq!(device.get_temp(), None);
        assert_eq!(device.get_usage_pct(), None);
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
    fn device_get_temp_and_usage_non_dgpu_when_suspended()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = TestDir::new("asusctl_test_non_dgpu_suspended");
        fs::create_dir_all(dir.join("power"))?;
        fs::write(dir.join("power/runtime_status"), "suspended\n")?;

        let device = Device {
            dev_path: dir.0.clone(),
            is_dgpu: false,
            pci_id: "10DE:228E".to_string(),
        };

        fn panic_on_nvml() -> Option<f32> {
            panic!("NVML fallback must not be called when runtime_status is not Active");
        }

        // Non-dgpu Nvidia function without hwmon/drm in suspended state must not call NVML
        assert_eq!(device.probe_hwmon(read_hwmon_temp, panic_on_nvml), None);
        assert_eq!(device.probe_usage(panic_on_nvml), None);
        assert_eq!(device.probe_hwmon(read_hwmon_freq, panic_on_nvml), None);
        assert_eq!(device.get_temp(), None);
        assert_eq!(device.get_usage_pct(), None);
        assert_eq!(device.get_freq_mhz(), None);
        Ok(())
    }

    #[test]
    #[ignore = "requires ASUS hardware with a dGPU"]
    fn live_dgpu_detection() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let devices = Device::find()?;
        println!("Found {} display devices:", devices.len());
        for dev in &devices {
            println!(
                "  - Device {} (is_dgpu: {}, path: {:?}, status: {:?})",
                dev.pci_id(),
                dev.is_dgpu(),
                dev.dev_path(),
                dev.get_runtime_status()?
            );
        }
        let (igpu_name, dgpu_name) = get_gpu_names();
        println!("GPU Names: iGPU = '{igpu_name}', dGPU = '{dgpu_name}'");
        let telemetry = get_gpu_telemetry();
        println!("Telemetry: {telemetry:?}");

        let dgpu = devices.iter().find(|d| d.is_dgpu()).expect("no dGPU found");
        assert!(is_gpu_vendor(dgpu.pci_id()));
        Ok(())
    }
}
