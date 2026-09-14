// Plan:
// - Manager has udev monitor on USB looking for ROG devices
// - If a device is found, add it to watch
// - Add it to Zbus server
// - If udev sees device removed then remove the zbus path

use std::collections::HashSet;
use std::sync::Arc;

use log::{debug, error, info, warn};
use mio::{Events, Interest, Poll, Token};
use rog_aura::AuraDeviceType;
use rog_platform::error::PlatformError;
use rog_platform::hid_raw::HidRaw;
use tokio::sync::Mutex;
use udev::{Device, MonitorBuilder};
use zbus::Connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath};

use crate::ASUS_ZBUS_PATH;
use crate::aura_anime::trait_impls::AniMeZbus;
use crate::aura_laptop::trait_impls::AuraZbus;
use crate::aura_scsi::trait_impls::ScsiZbus;
use crate::aura_slash::trait_impls::SlashZbus;
use crate::aura_types::DeviceHandle;
use crate::error::RogError;

const MOD_NAME: &str = "aura";

/// Returns only the Device details concatenated in a form usable for
/// adding/appending to a filename
pub fn filename_partial(parent: &Device) -> Option<OwnedObjectPath> {
    if let Some(id_product) = parent.attribute_value("idProduct") {
        let id_product = id_product.to_string_lossy();
        let identity = if let Some(serial) = parent.attribute_value("serial") {
            serial.to_string_lossy()
        } else if let Some(devpath) = parent.attribute_value("devpath") {
            devpath.to_string_lossy()
        } else {
            parent.sysname().to_string_lossy()
        };
        let path = sanitize_path_component(&format!("{id_product}_{identity}"));
        return Some(ObjectPath::from_str_unchecked(&path).into());
    }
    None
}

fn sanitize_path_component(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn dbus_path_for_dev(parent: &Device) -> Option<OwnedObjectPath> {
    if let Some(filename) = filename_partial(parent) {
        return Some(
            ObjectPath::from_str_unchecked(&format!("{ASUS_ZBUS_PATH}/{MOD_NAME}/{filename}"))
                .into(),
        );
    }
    None
}

fn dbus_path_for_tuf() -> OwnedObjectPath {
    ObjectPath::from_str_unchecked(&format!("{ASUS_ZBUS_PATH}/{MOD_NAME}/tuf")).into()
}

fn dbus_path_for_slash() -> OwnedObjectPath {
    ObjectPath::from_str_unchecked(&format!("{ASUS_ZBUS_PATH}/{MOD_NAME}/slash")).into()
}

fn dbus_path_for_anime() -> OwnedObjectPath {
    ObjectPath::from_str_unchecked(&format!("{ASUS_ZBUS_PATH}/{MOD_NAME}/anime")).into()
}

fn dbus_path_for_scsi(prod_id: &str) -> OwnedObjectPath {
    ObjectPath::from_str_unchecked(&format!("{ASUS_ZBUS_PATH}/{MOD_NAME}/{prod_id}_scsi")).into()
}

fn dev_prop_matches(dev: &Device, prop: &str, value: &str) -> bool {
    if let Some(p) = dev.property_value(prop) {
        return p == value;
    }
    false
}

/// A device.
///
/// Each controller within should track its dbus path so it can be removed if
/// required.
pub struct AsusDevice {
    device: DeviceHandle,
    dbus_path: OwnedObjectPath,
    hid_key: Option<String>,
}

pub struct DeviceManager {
    _dbus_connection: Connection,
}

/// Returns true if this hidraw device is a non-Aura interface on the
/// ASUS Zephyrus Duo (0b05:1ce6) composite device and should be skipped.
///
/// 0b05:1ce6 exposes multiple HID interfaces; only interface 1.2 is the
/// real Aura/vendor control interface (report ID 0x5d, Output/Feature
/// capable). Other interfaces reuse that report ID for unrelated
/// Input-only data. Used by both startup enumeration and hotplug
/// add-event handling so the wrong interface can never be selected,
/// including after USB reset/resume.
fn is_non_aura_1ce6_interface(device: &Device) -> bool {
    if let Ok(Some(usb_parent)) = device.parent_with_subsystem_devtype("usb", "usb_device") {
        let vendor = usb_parent.attribute_value("idVendor");
        let product = usb_parent.attribute_value("idProduct");

        if vendor == Some(std::ffi::OsStr::new("0b05"))
            && product == Some(std::ffi::OsStr::new("1ce6"))
        {
            let syspath = device.syspath().to_string_lossy().to_string();
            if !syspath.contains(":1.2/") {
                debug!("Skipping non-Aura 1ce6 HID interface: {}", syspath);
                return true;
            }
            debug!(
                "Selected ASUS 0b05:1ce6 Aura HID interface 1.2: {}",
                syspath
            );
        }
    }
    false
}

impl DeviceManager {
    async fn init_hid_devices(
        connection: &Connection,
        device: Device,
    ) -> Result<Vec<AsusDevice>, RogError> {
        let mut devices = Vec::new();
        if let Some(usb_device) = device.parent_with_subsystem_devtype("usb", "usb_device")?
            && let Some(usb_id) = usb_device.attribute_value("idProduct")
            && let Some(vendor_id) = usb_device.attribute_value("idVendor")
        {
            if vendor_id != "0b05" {
                debug!("Not ASUS vendor ID: {}", vendor_id.to_string_lossy());
                return Ok(devices);
            }
            // Almost all devices are identified by the productId.
            // So let's see what we have and:
            // 1. Generate an interface path
            // 2. Create the device
            let usb_id_str = usb_id.to_str().unwrap_or_default();
            let aura_type = AuraDeviceType::from(usb_id_str);
            if matches!(
                aura_type,
                AuraDeviceType::LaptopKeyboard2021
                    | AuraDeviceType::LaptopKeyboardPre2021
                    | AuraDeviceType::LaptopKeyboardTuf
                    | AuraDeviceType::Ally
            ) {
                let hid_key = device
                    .devnode()
                    .map(|path| path.to_string_lossy().into_owned());
                let hid = if HidRaw::supports_output_report(&device, 0x5d) {
                    HidRaw::from_device(device)
                        .map(|hid| Arc::new(Mutex::new(hid)))
                        .ok()
                } else {
                    None
                };
                if let Ok(dev_type) = DeviceHandle::maybe_laptop_aura(hid, usb_id_str).await
                    && let DeviceHandle::Aura(aura) = dev_type.clone()
                {
                    let path = dbus_path_for_dev(&usb_device).unwrap_or(dbus_path_for_tuf());
                    let ctrl = AuraZbus::new(aura);
                    if ctrl
                        .start_tasks(connection, path.clone())
                        .await
                        .map_err(|e| {
                            error!("Failed to start Aura tasks: {e:?}, not adding this device")
                        })
                        .is_ok()
                    {
                        devices.push(AsusDevice {
                            device: dev_type,
                            dbus_path: path,
                            hid_key,
                        });
                    }
                }
                return Ok(devices);
            }
        }
        Ok(devices)
    }

    /// To be called on daemon startup
    async fn init_all_hid(connection: &Connection) -> Result<Vec<AsusDevice>, RogError> {
        // Ensure we only process one hidraw interface per physical USB device.
        // A USB device can expose multiple HID interfaces (and thus multiple hidraw
        // nodes). Processing more than one causes duplicate device
        // initialisation which can interfere with the kernel's own HID driver
        // and trigger a USB reset loop.
        let mut seen_usb_parents: HashSet<String> = HashSet::new();
        let mut devices: Vec<AsusDevice> = Vec::new();

        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("enumerator failed".into(), err)
        })?;

        enumerator.match_subsystem("hidraw").map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for device in enumerator
            .scan_devices()
            .map_err(|e| PlatformError::IoPath("enumerator".to_owned(), e))?
        {
            // ASUS Zephyrus Duo keyboard 0b05:1ce6 is a composite HID device;
            // see is_non_aura_1ce6_interface for why interface 1.2 is required.
            if is_non_aura_1ce6_interface(&device) {
                continue;
            }

            let parent_path = if let Ok(Some(usb_parent)) =
                device.parent_with_subsystem_devtype("usb", "usb_device")
            {
                let parent_path = usb_parent.syspath().to_string_lossy().to_string();
                if seen_usb_parents.contains(&parent_path) {
                    debug!("Skipping duplicate ASUS hidraw for USB parent already processed");
                    continue;
                }
                Some(parent_path)
            } else {
                None
            };

            let mut found = Self::init_hid_devices(connection, device).await?;
            if !found.is_empty() {
                if let Some(parent_path) = parent_path {
                    seen_usb_parents.insert(parent_path);
                }
                devices.append(&mut found);
            }
        }

        Ok(devices)
    }

    async fn init_scsi(
        connection: &Connection,
        device: &Device,
        path: OwnedObjectPath,
    ) -> Option<AsusDevice> {
        // "ID_MODEL_ID" "1932"
        // "ID_VENDOR_ID" "0b05"
        if dev_prop_matches(device, "ID_VENDOR_ID", "0b05")
            && let Some(dev_node) = device.devnode()
        {
            let prod_id = device
                .property_value("ID_MODEL_ID")
                .unwrap_or_default()
                .to_string_lossy();

            let dev_str = dev_node.to_string_lossy();
            if let Ok(dev_type) = DeviceHandle::maybe_scsi(&dev_str, &prod_id).await
                && let DeviceHandle::Scsi(scsi) = dev_type.clone()
            {
                let ctrl = ScsiZbus::new(scsi);
                if ctrl
                    .start_tasks(connection, path.clone())
                    .await
                    .map_err(|e| {
                        error!("Failed to start SCSI tasks: {e:?}, not adding this device")
                    })
                    .is_ok()
                {
                    return Some(AsusDevice {
                        device: dev_type,
                        dbus_path: path,
                        hid_key: None,
                    });
                }
            }
        }

        None
    }

    async fn init_all_scsi(connection: &Connection) -> Result<Vec<AsusDevice>, RogError> {
        // track and ensure we use only one hidraw per prod_id
        // let mut interfaces = HashSet::new();
        let mut devices: Vec<AsusDevice> = Vec::new();

        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("enumerator failed".into(), err)
        })?;

        enumerator.match_subsystem("block").map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        let mut found = Vec::new();
        for device in enumerator
            .scan_devices()
            .map_err(|e| PlatformError::IoPath("enumerator".to_owned(), e))?
        {
            if let Some(serial) = device.property_value("ID_SERIAL_SHORT") {
                let serial = serial.to_string_lossy().to_string();
                let path = dbus_path_for_scsi(&serial);
                if found.contains(&path) {
                    continue;
                }

                if let Some(dev) = Self::init_scsi(connection, &device, path.clone()).await {
                    devices.push(dev);
                    found.push(path);
                }
            } else {
                debug!("No serial for SCSI device: {:?}", device.devpath());
            }
        }

        Ok(devices)
    }

    pub async fn find_all_devices(connection: &Connection) -> Vec<AsusDevice> {
        let mut devices: Vec<AsusDevice> = Vec::new();
        // HID first, always
        if let Ok(devs) = &mut Self::init_all_hid(connection).await {
            devices.append(devs);
        }
        // USB after, need to check if HID picked something up and if so, skip it
        let mut do_anime = true;
        let mut do_slash = true;
        let mut do_kb_backlight = true;
        for dev in devices.iter() {
            if matches!(dev.device, DeviceHandle::Slash(_)) {
                do_slash = false;
            }
            if matches!(dev.device, DeviceHandle::AniMe(_)) {
                do_anime = false;
            }
            if matches!(dev.device, DeviceHandle::Aura(_)) {
                do_kb_backlight = false;
            }
        }

        if do_slash {
            if let Ok(dev_type) = DeviceHandle::maybe_slash().await {
                if let DeviceHandle::Slash(slash) = dev_type.clone() {
                    let path = dbus_path_for_slash();
                    let ctrl = SlashZbus::new(slash);
                    if ctrl
                        .start_tasks(connection, path.clone())
                        .await
                        .map_err(|e| {
                            error!("Failed to start Slash tasks: {e:?}, not adding this device")
                        })
                        .is_ok()
                    {
                        devices.push(AsusDevice {
                            device: dev_type,
                            dbus_path: path,
                            hid_key: None,
                        });
                    }
                }
            } else {
                info!("Tested device was not Slash");
            }
        }

        if do_anime {
            if let Ok(dev_type) = DeviceHandle::maybe_anime_usb().await {
                // TODO: this is copy/pasted
                if let DeviceHandle::AniMe(anime) = dev_type.clone() {
                    let path = dbus_path_for_anime();
                    let ctrl = AniMeZbus::new(anime);
                    if ctrl
                        .start_tasks(connection, path.clone())
                        .await
                        .map_err(|e| error!("Failed to start tasks: {e:?}, not adding this device"))
                        .is_ok()
                    {
                        devices.push(AsusDevice {
                            device: dev_type,
                            dbus_path: path,
                            hid_key: None,
                        });
                    }
                }
            } else {
                info!("Tested device was not AniMe Matrix");
            }
        }

        if do_kb_backlight {
            info!("No USB keyboard aura, try sysfs backlight / Dynamic Lighting");
            if let Ok(dev_type) = DeviceHandle::maybe_laptop_aura(None, "tuf").await
                && let DeviceHandle::Aura(aura) = dev_type.clone()
            {
                let path = dbus_path_for_tuf();
                let ctrl = AuraZbus::new(aura);
                if ctrl
                    .start_tasks(connection, path.clone())
                    .await
                    .map_err(|e| {
                        error!("Failed to start TUF Aura tasks: {e:?}, not adding this device")
                    })
                    .is_ok()
                {
                    devices.push(AsusDevice {
                        device: dev_type,
                        dbus_path: path,
                        hid_key: None,
                    });
                }
            }
        }

        if let Ok(devs) = &mut Self::init_all_scsi(connection).await {
            devices.append(devs);
        }

        devices
    }

    pub async fn new(connection: Connection) -> Result<Self, RogError> {
        let conn_copy = connection.clone();
        let devices = Self::find_all_devices(&conn_copy).await;
        info!("Found {} valid devices on startup", devices.len());
        let devices = Arc::new(Mutex::new(devices));
        let manager = Self {
            _dbus_connection: connection,
        };

        // TODO: The /sysfs/ LEDs don't cause events, so they need to be manually
        // checked for and added

        std::thread::spawn(move || {
            let mut monitor = MonitorBuilder::new()?.listen()?;
            let mut poll = Poll::new()?;
            let mut events = Events::with_capacity(1024);
            poll.registry()
                .register(&mut monitor, Token(0), Interest::READABLE)?;

            let rt = tokio::runtime::Runtime::new().expect("Unable to create Runtime");
            let _enter = rt.enter();
            loop {
                if poll.poll(&mut events, None).is_err() {
                    continue;
                }
                for event in monitor.iter() {
                    let action = event
                        .action()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    let subsys = if let Some(subsys) = event.subsystem() {
                        subsys.to_string_lossy().to_string()
                    } else {
                        continue;
                    };

                    let devices = devices.clone();
                    let conn_copy = conn_copy.clone();
                    rt.block_on(async move {
                        // SCSCI devs
                        if subsys == "block" {
                            if action == "remove" {
                                if let Some(serial) =
                                    event.device().property_value("ID_SERIAL_SHORT")
                                {
                                    let serial = serial.to_string_lossy().to_string();
                                    let path = dbus_path_for_scsi(&serial);

                                    let index = if let Some(index) = devices
                                        .lock()
                                        .await
                                        .iter()
                                        .position(|dev| dev.dbus_path == path)
                                    {
                                        index
                                    } else {
                                        if dev_prop_matches(&event.device(), "ID_VENDOR_ID", "0b05")
                                        {
                                            warn!("No device for dbus path: {path:?}");
                                        }
                                        return Ok(());
                                    };
                                    info!("removing: {path:?}");
                                    let dev = devices.lock().await.remove(index);
                                    let path = path.clone();
                                    if let DeviceHandle::Scsi(_) = dev.device {
                                        conn_copy
                                            .object_server()
                                            .remove::<ScsiZbus, _>(&path)
                                            .await?;
                                    }
                                }
                            } else if action == "add" {
                                let evdev = event.device();
                                if let Some(serial) = evdev.property_value("ID_SERIAL_SHORT") {
                                    let serial = serial.to_string_lossy().to_string();
                                    let path = dbus_path_for_scsi(&serial);
                                    if devices.lock().await.iter().any(|d| d.dbus_path == path) {
                                        debug!(
                                            "SCSI hotplug add: device {path:?} already registered"
                                        );
                                        return Ok(());
                                    }
                                    if let Some(new_devs) =
                                        Self::init_scsi(&conn_copy, &evdev, path).await
                                    {
                                        devices.lock().await.append(&mut vec![new_devs]);
                                    }
                                }
                            };
                        }

                        if subsys == "hidraw" {
                            if action == "remove" {
                                // Key cleanup off the removed hidraw node itself, NOT the
                                // USB parent. By the time a `remove` uevent arrives the USB
                                // parent is usually already detached from sysfs, so
                                // `parent_with_subsystem_devtype` returns None; gating the
                                // cleanup on it meant we kept the `Arc<Mutex<HidRaw>>` (held
                                // by both the handle map and the live zbus object) alive, so
                                // the open hidraw `File` was never dropped. The kernel only
                                // frees a hidraw minor once no fd remains open on it, so every
                                // re-enumeration (s2idle resume, dock/undock) leaked one
                                // minor until the 64-entry pool was exhausted system-wide.
                                // The uevent always carries DEVNAME, so the devnode is the
                                // stable key (and is exactly what we store as `hid_key`).
                                let removed_node = event
                                    .device()
                                    .devnode()
                                    .map(|n| n.to_string_lossy().to_string());
                                if let Some(removed_node) = removed_node {
                                    // Tear down any zbus objects backed by this node.
                                    let removals: Vec<usize> = devices
                                        .lock()
                                        .await
                                        .iter()
                                        .enumerate()
                                        .filter_map(|(i, dev)| {
                                            if dev.hid_key.as_deref() == Some(removed_node.as_str())
                                            {
                                                Some(i)
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();
                                    // Iter in reverse so as to not screw up indexing
                                    for index in removals.iter().rev() {
                                        let dev = devices.lock().await.remove(*index);
                                        let path = dev.dbus_path.clone();
                                        let res = match dev.device {
                                            DeviceHandle::Aura(_) => {
                                                conn_copy
                                                    .object_server()
                                                    .remove::<AuraZbus, _>(&path)
                                                    .await?
                                            }
                                            DeviceHandle::Slash(_) => {
                                                conn_copy
                                                    .object_server()
                                                    .remove::<SlashZbus, _>(&path)
                                                    .await?
                                            }
                                            DeviceHandle::AniMe(_) => {
                                                conn_copy
                                                    .object_server()
                                                    .remove::<AniMeZbus, _>(&path)
                                                    .await?
                                            }
                                            DeviceHandle::Scsi(_) => {
                                                conn_copy
                                                    .object_server()
                                                    .remove::<ScsiZbus, _>(&path)
                                                    .await?
                                            }
                                            // sysfs/USB-backed handles (e.g. OldAura) own no
                                            // shared hidraw fd; nothing to remove here.
                                            _ => false,
                                        };
                                        info!("AuraManager removed: {path:?}, {res}");
                                    }
                                }
                            } else if action == "add"
                                && let Some(parent) =
                                    event.parent_with_subsystem_devtype("usb", "usb_device")?
                            {
                                // Guard against initialising a second hidraw interface for a
                                // USB device we already track. Without this, a USB reset
                                // (e.g. triggered by an earlier duplicate init) fires
                                // remove+add events that cause another duplicate init and a
                                // permanent reset loop.
                                if let Some(path) = dbus_path_for_dev(&parent)
                                    && devices.lock().await.iter().any(|d| d.dbus_path == path)
                                {
                                    debug!(
                                        "Hotplug add: device {path:?} already registered, skipping"
                                    );
                                    return Ok(());
                                }
                                let evdev = event.device();
                                if is_non_aura_1ce6_interface(&evdev) {
                                    return Ok(());
                                }
                                if let Ok(mut new_devs) = Self::init_hid_devices(&conn_copy, evdev)
                                    .await
                                    .map_err(|e| error!("Couldn't add new device: {e:?}"))
                                {
                                    devices.lock().await.append(&mut new_devs);
                                }
                            }
                        }
                        Ok::<(), RogError>(())
                    })
                    .map_err(|e| error!("{e:?}"))
                    .ok();
                }
            }
            // Required for return type on spawn
            #[allow(unreachable_code)]
            Ok::<(), RogError>(())
        });
        Ok(manager)
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_path_component;

    #[test]
    fn physical_identity_is_a_valid_stable_object_component() {
        assert_eq!(sanitize_path_component("19b6_1-3.2:1.0"), "19b6_1_3_2_1_0");
    }
}
