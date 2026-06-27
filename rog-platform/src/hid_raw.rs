use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use log::{info, warn};
use udev::Device;

use crate::error::{PlatformError, Result};

/// A USB device that utilizes hidraw for I/O
#[derive(Debug)]
pub struct HidRaw {
    /// The path to the `/dev/<name>` of the device
    devfs_path: PathBuf,
    /// The sysfs path
    syspath: PathBuf,
    /// The product ID. The vendor ID is not kept
    prod_id: String,
    _device_bcd: u32,
    /// Retaining a handle to the file for the duration of `HidRaw`
    file: RefCell<File>,
}

impl HidRaw {
    pub fn new(id_product: &str) -> Result<Self> {
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("enumerator failed".into(), err)
        })?;

        enumerator.match_subsystem("hidraw").map_err(|err| {
            warn!("{err}");
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for endpoint in enumerator
            .scan_devices()
            .map_err(|e| PlatformError::IoPath("enumerator".to_owned(), e))?
        {
            if let Some(usb_device) = endpoint
                .parent_with_subsystem_devtype("usb", "usb_device")
                .map_err(|e| {
                    PlatformError::IoPath(endpoint.devpath().to_string_lossy().to_string(), e)
                })?
                && let Some(dev_node) = endpoint.devnode()
                && let Some(this_id_product) = usb_device.attribute_value("idProduct")
            {
                if this_id_product != id_product {
                    continue;
                }
                let dev_path = endpoint.devpath().to_string_lossy();
                if dev_path.contains("virtual") {
                    info!("Using device at: {dev_node:?} for <TODO: label control> control");
                }
                return Ok(Self {
                    file: RefCell::new(OpenOptions::new().write(true).open(dev_node)?),
                    devfs_path: dev_node.to_owned(),
                    prod_id: this_id_product.to_string_lossy().into(),
                    syspath: endpoint.syspath().into(),
                    _device_bcd: usb_device
                        .attribute_value("bcdDevice")
                        .unwrap_or_default()
                        .to_string_lossy()
                        .parse()
                        .unwrap_or_default(),
                });
            }
        }
        Err(PlatformError::MissingFunction(format!(
            "hidraw dev {id_product} not found"
        )))
    }

    /// Make `HidRaw` device from a udev device
    pub fn from_device(endpoint: Device) -> Result<Self> {
        if let Some(parent) = endpoint
            .parent_with_subsystem_devtype("usb", "usb_device")
            .map_err(|e| {
                PlatformError::IoPath(endpoint.devpath().to_string_lossy().to_string(), e)
            })?
            && let Some(dev_node) = endpoint.devnode()
            && let Some(id_product) = parent.attribute_value("idProduct")
        {
            return Ok(Self {
                file: RefCell::new(OpenOptions::new().write(true).open(dev_node)?),
                devfs_path: dev_node.to_owned(),
                prod_id: id_product.to_string_lossy().into(),
                syspath: endpoint.syspath().into(),
                _device_bcd: endpoint
                    .attribute_value("bcdDevice")
                    .unwrap_or_default()
                    .to_string_lossy()
                    .parse()
                    .unwrap_or_default(),
            });
        }
        Err(PlatformError::MissingFunction(
            "hidraw dev no dev path".to_string(),
        ))
    }

    pub fn prod_id(&self) -> &str {
        &self.prod_id
    }

    /// Write an array of raw bytes to the device using the hidraw interface
    pub fn write_bytes(&self, message: &[u8]) -> Result<()> {
        if let Ok(mut file) = self.file.try_borrow_mut() {
            // TODO: re-get the file if error?
            file.write_all(message).map_err(|e| {
                PlatformError::IoPath(self.devfs_path.to_string_lossy().to_string(), e)
            })?;
        }
        Ok(())
    }

    /// This method was added for certain devices like AniMe to prevent them
    /// waking the laptop
    pub fn set_wakeup_disabled(&self) -> Result<()> {
        let mut dev = Device::from_syspath(&self.syspath)?;
        Ok(dev.set_attribute_value("power/wakeup", "disabled")?)
    }
}
