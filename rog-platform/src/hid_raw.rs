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
    /// The product ID. The vendor ID is not kept
    prod_id: String,
    _device_bcd: u32,
    /// Retaining a handle to the file for the duration of `HidRaw`
    file: RefCell<File>,
}

impl HidRaw {
    /// Check whether a hidraw endpoint descriptor declares an output report ID.
    pub fn supports_output_report(endpoint: &Device, report_id: u8) -> bool {
        std::fs::read(endpoint.syspath().join("device/report_descriptor"))
            .is_ok_and(|descriptor| descriptor_has_output_report(&descriptor, report_id))
    }

    pub fn new(id_product: &str) -> Result<Self> {
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("enumerator failed".into(), err)
        })?;

        enumerator.match_subsystem("hidraw").map_err(|err| {
            warn!("{}", err);
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
                    info!(
                        "Using device at: {:?} for <TODO: label control> control",
                        dev_node
                    );
                }
                return Ok(Self {
                    file: RefCell::new(OpenOptions::new().write(true).open(dev_node)?),
                    devfs_path: dev_node.to_owned(),
                    prod_id: this_id_product.to_string_lossy().into(),
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
            "hidraw dev {} not found",
            id_product
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
                _device_bcd: parent
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
        if message.is_empty() {
            return Err(PlatformError::InvalidValue);
        }
        let mut file = self
            .file
            .try_borrow_mut()
            .map_err(|_| PlatformError::InvalidValue)?;
        file.write_all(message)
            .map_err(|e| PlatformError::IoPath(self.devfs_path.to_string_lossy().to_string(), e))
    }
}

fn descriptor_has_output_report(descriptor: &[u8], wanted_id: u8) -> bool {
    let mut offset = 0;
    let mut report_id = 0;
    while let Some(prefix) = descriptor.get(offset).copied() {
        if prefix == 0xfe {
            let Some(size) = descriptor.get(offset + 1).copied() else {
                return false;
            };
            offset = match offset.checked_add(3 + usize::from(size)) {
                Some(next) => next,
                None => return false,
            };
            continue;
        }
        let size = match prefix & 0x03 {
            3 => 4,
            value => usize::from(value),
        };
        let end = match offset.checked_add(1 + size) {
            Some(end) => end,
            None => return false,
        };
        let Some(data) = descriptor.get(offset + 1..end) else {
            return false;
        };
        let item_type = (prefix >> 2) & 0x03;
        let tag = prefix >> 4;
        if item_type == 1 && tag == 8 && size == 1 {
            report_id = data[0];
        } else if item_type == 0 && tag == 9 && report_id == wanted_id {
            return true;
        }
        offset = end;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::descriptor_has_output_report;

    #[test]
    fn parses_output_report_id_safely() {
        assert!(descriptor_has_output_report(
            &[
                0x85, 0x5d, 0x09, 0x01, 0x91, 0x02
            ],
            0x5d
        ));
        assert!(!descriptor_has_output_report(
            &[
                0x85, 0x5d, 0x09, 0x01, 0x81, 0x02
            ],
            0x5d
        ));
        assert!(!descriptor_has_output_report(&[0x85], 0x5d));
    }
}
