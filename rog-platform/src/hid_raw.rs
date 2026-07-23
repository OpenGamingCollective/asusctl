use std::cell::RefCell;
#[cfg(target_os = "linux")]
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Write;
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
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
            {
                if let Some(dev_node) = endpoint.devnode() {
                    if let Some(this_id_product) = usb_device.attribute_value("idProduct") {
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
                            file: RefCell::new(open_hidraw(dev_node)?),
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
        {
            if let Some(dev_node) = endpoint.devnode() {
                if let Some(id_product) = parent.attribute_value("idProduct") {
                    return Ok(Self {
                        file: RefCell::new(open_hidraw(dev_node)?),
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
            }
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

    /// Query the ASUS notebook Aura firmware capability report.
    ///
    /// Armoury Crate uses HID feature reports rather than an output report
    /// for this readback.  Unsupported kernels/devices are deliberately
    /// reported as `Ok(None)` so the caller can retain the static support DB.
    #[cfg(target_os = "linux")]
    pub fn query_aura_capability_report(&self) -> Result<Option<Vec<u8>>> {
        const REPORT_ID: u8 = 0x5d;
        const COMMAND: u8 = 0x9e;
        const SELECTORS: [u8; 2] = [
            0x20, 0x15,
        ];

        for selector in SELECTORS {
            let request = [
                REPORT_ID, COMMAND, 1, selector,
            ];
            let response = match self.aura_feature_transaction(&request) {
                Ok(response) => response,
                Err(error) => {
                    log::debug!(
                        "Aura firmware capability selector {selector:#04x} transaction failed on {:?}: {error}",
                        self.devfs_path
                    );
                    continue;
                }
            };

            if response.len() >= 5
                && response[0] == REPORT_ID
                && response[1] == COMMAND
                && response[2] == 1
                && response[3] == selector
                && response[4] == 1
            {
                return Ok(Some(response));
            }
            log::debug!(
                "Aura firmware capability selector {selector:#04x} returned an unrecognised response on {:?}",
                self.devfs_path
            );
        }

        Ok(None)
    }

    /// Query the physical keyboard/layout status used by Armoury Crate.
    #[cfg(target_os = "linux")]
    pub fn query_aura_status_report(&self) -> Result<Option<Vec<u8>>> {
        const REPORT_ID: u8 = 0x5d;
        const COMMAND: u8 = 0x05;

        self.prime_aura_status_report();
        // GetRGBKBStatus uses the six-byte request below.  Byte 5 is the
        // requested payload length; the remaining report is zero padding.
        let request = [
            REPORT_ID, COMMAND, 0x20, 0x31, 0x00, 0x20,
        ];
        let response = self.aura_feature_transaction(&request)?;
        if response.len() >= 5
            && response[0] == REPORT_ID
            && response[1] == COMMAND
            && response[2] == 0x20
            && response[3] == 0x31
            && response[4] == 0
        {
            return Ok(Some(response));
        }

        log::debug!(
            "Aura hardware status returned an unrecognised response on {:?}",
            self.devfs_path
        );
        Ok(None)
    }

    #[cfg(not(target_os = "linux"))]
    pub fn query_aura_status_report(&self) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    #[cfg(target_os = "linux")]
    fn prime_aura_status_report(&self) {
        // GetRGBKBStatus performs this signature SetFeature/GetFeature pair
        // before its 0x05 query.  NBFWRunmodeReadback (0x9e) does not.
        let report_len = self.aura_feature_report_len();
        let signature = [
            0x5d, b'A', b'S', b'U', b'S', b' ', b'T', b'e', b'c', b'h', b'.', b'I', b'n', b'c',
            b'.',
        ];
        let mut set_report = vec![0u8; report_len];
        let len = signature.len().min(set_report.len());
        set_report[..len].copy_from_slice(&signature[..len]);
        if let Err(error) = self.set_feature_report(&mut set_report) {
            log::debug!(
                "Aura status signature SetFeature failed on {:?}: {error}",
                self.devfs_path
            );
        }

        let mut response = vec![0u8; report_len];
        response[0] = 0x5d;
        if let Err(error) = self.get_feature_report(&mut response) {
            log::debug!(
                "Aura status signature GetFeature failed on {:?}: {error}",
                self.devfs_path
            );
        }
    }

    #[cfg(target_os = "linux")]
    fn aura_feature_transaction(&self, request: &[u8]) -> Result<Vec<u8>> {
        let report_len = self.aura_feature_report_len();
        let mut set_report = vec![0u8; report_len];
        let len = request.len().min(report_len);
        set_report[..len].copy_from_slice(&request[..len]);
        self.set_feature_report(&mut set_report)?;

        let mut response = vec![0u8; report_len];
        if let Some(report_id) = request.first() {
            response[0] = *report_id;
        }
        self.get_feature_report(&mut response)?;
        Ok(response)
    }

    #[cfg(target_os = "linux")]
    fn aura_feature_report_len(&self) -> usize {
        const REPORT_ID: u8 = 0x5d;
        const FALLBACK_LEN: usize = 64;
        const MAX_REPORT_LEN: usize = 4096;

        // `syspath` points at .../<hid-device>/hidraw/hidrawN.  The HID
        // descriptor is stored on the parent HID device.
        let descriptor = self
            .syspath
            .parent()
            .and_then(|path| path.parent())
            .map(|path| path.join("report_descriptor"));
        descriptor
            .and_then(|path| fs::read(path).ok())
            .and_then(|bytes| feature_report_len(&bytes, REPORT_ID))
            .filter(|len| *len > 0 && *len <= MAX_REPORT_LEN)
            .unwrap_or(FALLBACK_LEN)
    }

    #[cfg(not(target_os = "linux"))]
    pub fn query_aura_capability_report(&self) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    #[cfg(target_os = "linux")]
    fn set_feature_report(&self, report: &mut [u8]) -> Result<()> {
        self.feature_report_ioctl(0x06, report)
    }

    #[cfg(target_os = "linux")]
    fn get_feature_report(&self, report: &mut [u8]) -> Result<()> {
        self.feature_report_ioctl(0x07, report)
    }

    #[cfg(target_os = "linux")]
    fn feature_report_ioctl(&self, number: u32, report: &mut [u8]) -> Result<()> {
        if report.is_empty() {
            return Err(PlatformError::MissingFunction(
                "hidraw feature report cannot be empty".to_owned(),
            ));
        }

        let file = self.file.try_borrow_mut().map_err(|_| {
            PlatformError::MissingFunction("hidraw file is already borrowed".to_owned())
        })?;
        let request = hidraw_ioctl(3, number, report.len());
        let result = unsafe {
            libc::ioctl(
                file.as_raw_fd(),
                request,
                report.as_mut_ptr().cast::<libc::c_void>(),
            )
        };
        if result < 0 {
            return Err(PlatformError::IoPath(
                self.devfs_path.to_string_lossy().to_string(),
                std::io::Error::last_os_error(),
            ));
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

fn open_hidraw(path: &std::path::Path) -> std::io::Result<File> {
    match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => Ok(file),
        Err(read_write_error) => OpenOptions::new()
            .write(true)
            .open(path)
            // Preserve the feature-open error when the compatibility fallback
            // also fails; it is more useful than the retry error.
            .map_err(|_| read_write_error),
    }
}

#[cfg(target_os = "linux")]
fn hidraw_ioctl(direction: u32, number: u32, length: usize) -> libc::c_ulong {
    // Linux asm-generic _IOC layout used by the x86_64 ASUS hardware targeted
    // here.  Keep the length dynamic instead of hard-coding a 64-byte request.
    const NR_BITS: u32 = 8;
    const TYPE_BITS: u32 = 8;
    const SIZE_BITS: u32 = 14;
    const NR_SHIFT: u32 = 0;
    const TYPE_SHIFT: u32 = NR_SHIFT + NR_BITS;
    const SIZE_SHIFT: u32 = TYPE_SHIFT + TYPE_BITS;
    const DIR_SHIFT: u32 = SIZE_SHIFT + SIZE_BITS;

    ((direction << DIR_SHIFT)
        | ((b'H' as u32) << TYPE_SHIFT)
        | (number << NR_SHIFT)
        | ((length as u32) << SIZE_SHIFT)) as libc::c_ulong
}

/// Return the byte length (including a non-zero report ID) of a Feature
/// report declared by a HID report descriptor.
#[cfg(target_os = "linux")]
pub fn feature_report_len(descriptor: &[u8], report_id: u8) -> Option<usize> {
    #[derive(Clone, Copy, Default)]
    struct GlobalState {
        report_size_bits: usize,
        report_count: usize,
        report_id: u8,
    }

    let mut index = 0;
    let mut global = GlobalState::default();
    let mut global_stack = Vec::new();
    let mut feature_bits = 0usize;

    while index < descriptor.len() {
        let prefix = descriptor[index];
        index += 1;

        if prefix == 0xfe {
            // Long item: size, long-tag, payload.
            let size = *descriptor.get(index)? as usize;
            index = index.checked_add(2 + size)?;
            if index > descriptor.len() {
                return None;
            }
            continue;
        }

        let size = match prefix & 0x03 {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 4,
        };
        let end = index.checked_add(size)?;
        if end > descriptor.len() {
            return None;
        }
        let data = &descriptor[index..end];
        let value = data
            .iter()
            .enumerate()
            .fold(0usize, |value, (offset, byte)| {
                value | (*byte as usize) << (offset * 8)
            });
        let item = prefix & 0xfc;

        match item {
            // Global: Report Size, Report ID, Report Count, Push, Pop.
            0x74 => global.report_size_bits = value,
            0x84 => global.report_id = value as u8,
            0x94 => global.report_count = value,
            0xa4 => global_stack.push(global),
            0xb4 => global = global_stack.pop()?,
            // Main: Feature. Input/Output are intentionally ignored.
            0xb0 if global.report_id == report_id => {
                feature_bits = feature_bits
                    .checked_add(global.report_size_bits.checked_mul(global.report_count)?)?;
            }
            _ => {}
        }
        index = end;
    }

    if feature_bits == 0 {
        None
    } else {
        Some(feature_bits.div_ceil(8) + usize::from(report_id != 0))
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{feature_report_len, hidraw_ioctl};

    #[test]
    fn hidraw_feature_ioctl_numbers_match_linux() {
        assert_eq!(hidraw_ioctl(3, 0x06, 64), 0xc0404806);
        assert_eq!(hidraw_ioctl(3, 0x07, 64), 0xc0404807);
    }

    #[test]
    fn finds_feature_report_size_for_selected_id() {
        // Report 0x5d has 63 bytes of Feature data plus its report ID; the
        // neighbouring 0x5a report must not affect the result.
        let descriptor = [
            0x85, 0x5a, 0x75, 0x08, 0x95, 0x01, 0xb1, 0x00, 0x85, 0x5d, 0x75, 0x08, 0x95, 0x3f,
            0xb1, 0x00,
        ];
        assert_eq!(feature_report_len(&descriptor, 0x5d), Some(64));
        assert_eq!(feature_report_len(&descriptor, 0x5a), Some(2));
        assert_eq!(feature_report_len(&descriptor, 0x41), None);
    }

    #[test]
    fn restores_report_globals_after_push_and_pop() {
        let descriptor = [
            0x85, 0x5d, 0x75, 0x08, 0x95, 0x01, 0xa4, // Push
            0x75, 0x10, 0x95, 0x02, 0xb1, 0x00, // Feature: 4 bytes
            0xb4, // Pop
            0xb1, 0x00, // Feature: 1 byte
        ];
        assert_eq!(feature_report_len(&descriptor, 0x5d), Some(6));
    }
}
