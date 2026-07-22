use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

use log::{error, info, warn};
use udev::Device;

use crate::error::{PlatformError, Result};

/// Matches the kernel `struct hidraw_devinfo` (8 bytes total).
#[repr(C)]
pub struct HidrawDevinfo {
    pub bustype: u32,
    pub vendor: i16,
    pub product: i16,
}

// rustix custom Ioctl trait implementations
struct GetRawInfo {
    info: HidrawDevinfo,
}

unsafe impl rustix::ioctl::Ioctl for GetRawInfo {
    type Output = HidrawDevinfo;
    const OPCODE: rustix::ioctl::Opcode = rustix::ioctl::Opcode::old(0x80084803);
    const IS_MUTATING: bool = true;

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        &mut self.info as *mut HidrawDevinfo as *mut std::ffi::c_void
    }

    unsafe fn output_from_ptr(
        _out: rustix::ioctl::IoctlOutput,
        extract_output: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(std::ptr::read(extract_output as *const HidrawDevinfo))
    }
}

struct SetFeatureReport<const N: usize> {
    payload: [u8; N],
}

unsafe impl<const N: usize> rustix::ioctl::Ioctl for SetFeatureReport<N> {
    type Output = ();
    const OPCODE: rustix::ioctl::Opcode =
        rustix::ioctl::Opcode::from_components(rustix::ioctl::Direction::ReadWrite, b'H', 0x06, N);
    const IS_MUTATING: bool = false;

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.payload.as_mut_ptr() as *mut std::ffi::c_void
    }

    unsafe fn output_from_ptr(
        _out: rustix::ioctl::IoctlOutput,
        _extract_output: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(())
    }
}

struct GetFeatureReport<const N: usize> {
    buf: [u8; N],
}

unsafe impl<const N: usize> rustix::ioctl::Ioctl for GetFeatureReport<N> {
    type Output = [u8; N];
    const OPCODE: rustix::ioctl::Opcode =
        rustix::ioctl::Opcode::from_components(rustix::ioctl::Direction::ReadWrite, b'H', 0x07, N);
    const IS_MUTATING: bool = true;

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.buf.as_mut_ptr() as *mut std::ffi::c_void
    }

    unsafe fn output_from_ptr(
        _out: rustix::ioctl::IoctlOutput,
        extract_output: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(std::ptr::read(extract_output as *const [u8; N]))
    }
}

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
            }
        }
        Err(PlatformError::MissingFunction(
            "hidraw dev no dev path".to_string(),
        ))
    }

    /// Build a `HidRaw` from an I2C-HID hidraw endpoint. Opens R/W so that we
    /// can use HIDIOCGFEATURE / HIDIOCSFEATURE on LampArray devices.
    pub fn from_i2c_device(endpoint: Device, prod_id: &str) -> Result<Self> {
        let sysname_dbg = endpoint.sysname().to_string_lossy().to_string();
        info!(
            "HidRaw::from_i2c_device: begin sysname={} prod_id={}",
            sysname_dbg, prod_id
        );
        info!(
            "HidRaw::from_i2c_device: querying devnode for sysname={}",
            sysname_dbg
        );
        let dev_node = endpoint.devnode().ok_or_else(|| {
            PlatformError::MissingFunction("I2C-HID endpoint has no devnode".to_string())
        })?;
        info!(
            "HidRaw::from_i2c_device: devnode={:?} sysname={}",
            dev_node, sysname_dbg
        );
        info!(
            "HidRaw::from_i2c_device: opening {:?} R/W (O_NONBLOCK) for prod_id={}",
            dev_node, prod_id
        );
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32)
            .open(dev_node)
            .map_err(|e| PlatformError::IoPath(dev_node.to_string_lossy().to_string(), e))?;
        let fd = file.as_raw_fd();
        info!(
            "HidRaw::from_i2c_device: file opened fd={} dev_node={:?}",
            fd, dev_node
        );
        info!(
            "HidRaw::from_i2c_device: about to query syspath for sysname={}",
            sysname_dbg
        );
        let syspath = endpoint.syspath().to_path_buf();
        info!(
            "HidRaw::from_i2c_device: syspath={:?} sysname={}",
            syspath, sysname_dbg
        );
        info!(
            "HidRaw::from_i2c_device: returning OK for sysname={} fd={}",
            sysname_dbg, fd
        );
        Ok(Self {
            file: RefCell::new(file),
            devfs_path: dev_node.to_owned(),
            prod_id: prod_id.to_string(),
            syspath,
            _device_bcd: 0,
        })
    }

    pub fn prod_id(&self) -> &str {
        &self.prod_id
    }

    pub fn devfs_path(&self) -> &PathBuf {
        &self.devfs_path
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

    /// Write to `use_leds_uapi` sysfs attribute of the HID device if present,
    /// enabling or disabling the kernel-managed multicolor LED classdev.
    pub fn set_use_leds_uapi(&self, enable: bool) -> Result<()> {
        let mut dev = Device::from_syspath(&self.syspath)?;
        if crate::has_attr(&dev, "use_leds_uapi") {
            crate::write_attr_bool(&mut dev, "use_leds_uapi", enable)?;
        }
        Ok(())
    }

    /// HIDIOCGRAWINFO -> kernel hidraw_devinfo (bustype, vendor, product).
    pub fn raw_info(&self) -> Result<HidrawDevinfo> {
        let file = self
            .file
            .try_borrow()
            .map_err(|_| PlatformError::MissingFunction("hidraw file busy".into()))?;
        let fd = file.as_raw_fd();
        info!(
            "HidRaw::raw_info: fd={} struct_size={}",
            fd,
            std::mem::size_of::<HidrawDevinfo>()
        );

        let op = GetRawInfo {
            info: HidrawDevinfo {
                bustype: 0,
                vendor: 0,
                product: 0,
            },
        };
        // SAFETY: We pass a pointer to a 8-byte struct matching the kernel's
        // hidraw_devinfo layout; the ioctl number encodes that size.
        let info = unsafe { rustix::ioctl::ioctl(&*file, op) }.map_err(|err| {
            let err = std::io::Error::from(err);
            error!(
                "HidRaw::raw_info: ioctl HIDIOCGRAWINFO failed on {:?}: {}",
                self.devfs_path, err
            );
            PlatformError::IoPath(self.devfs_path.to_string_lossy().to_string(), err)
        })?;

        info!(
            "HidRaw::raw_info: ok bus={:#x} vendor={:#06x} product={:#06x}",
            info.bustype, info.vendor as u16, info.product as u16
        );
        Ok(info)
    }

    /// HIDIOCSFEATURE(len) - send a feature report.
    pub fn set_feature_report(&self, payload: &[u8]) -> Result<()> {
        let file = self
            .file
            .try_borrow()
            .map_err(|_| PlatformError::MissingFunction("hidraw file busy".into()))?;
        let len = payload.len();
        match len {
            2 => {
                let mut data = [0u8; 2];
                data.copy_from_slice(payload);
                let op = SetFeatureReport { payload: data };
                unsafe {
                    rustix::ioctl::ioctl(&*file, op)
                }.map_err(|err| {
                    let err = std::io::Error::from(err);
                    error!(
                        "HidRaw::set_feature_report: ioctl HIDIOCSFEATURE(len=2) failed on {:?}: {}",
                        self.devfs_path,
                        err
                    );
                    PlatformError::IoPath(
                        self.devfs_path.to_string_lossy().to_string(),
                        err,
                    )
                })?;
            }
            10 => {
                let mut data = [0u8; 10];
                data.copy_from_slice(payload);
                let op = SetFeatureReport { payload: data };
                unsafe {
                    rustix::ioctl::ioctl(&*file, op)
                }.map_err(|err| {
                    let err = std::io::Error::from(err);
                    error!(
                        "HidRaw::set_feature_report: ioctl HIDIOCSFEATURE(len=10) failed on {:?}: {}",
                        self.devfs_path,
                        err
                    );
                    PlatformError::IoPath(
                        self.devfs_path.to_string_lossy().to_string(),
                        err,
                    )
                })?;
            }
            _ => {
                return Err(PlatformError::MissingFunction(format!(
                    "Unsupported set_feature_report payload length: {}",
                    len
                )));
            }
        }
        Ok(())
    }

    /// HIDIOCGFEATURE(len) - read a feature report. Buffer[0] must hold the
    /// report ID before the call.
    pub fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize> {
        let file = self
            .file
            .try_borrow()
            .map_err(|_| PlatformError::MissingFunction("hidraw file busy".into()))?;
        let len = buf.len();
        match len {
            23 => {
                let mut data = [0u8; 23];
                data.copy_from_slice(buf);
                let op = GetFeatureReport { buf: data };
                let res = unsafe {
                    rustix::ioctl::ioctl(&*file, op)
                }.map_err(|err| {
                    let err = std::io::Error::from(err);
                    error!(
                        "HidRaw::get_feature_report: ioctl HIDIOCGFEATURE(len=23) failed on {:?}: {}",
                        self.devfs_path,
                        err
                    );
                    PlatformError::IoPath(
                        self.devfs_path.to_string_lossy().to_string(),
                        err,
                    )
                })?;
                buf.copy_from_slice(&res);
                Ok(23)
            }
            _ => Err(PlatformError::MissingFunction(format!(
                "Unsupported get_feature_report buffer length: {}",
                len
            ))),
        }
    }
}
