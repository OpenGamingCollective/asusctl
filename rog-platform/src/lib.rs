//! This crate functions as a wrapper of all the relevant ASUS functionality
//! on ROG, Strix, and TUF laptops.

pub mod asus_armoury;
pub mod backlight;
pub mod cled;
pub mod cpu;
pub mod error;
pub mod gpu_pci;
pub mod hid_raw;
pub mod keyboard_led;
pub(crate) mod macros;
pub mod platform;
pub mod power;
pub mod usb_raw;

use std::path::Path;

use error::{PlatformError, Result};
use log::warn;
use platform::PlatformProfile;
use udev::Device;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn to_device(sys_path: &Path) -> Result<Device> {
    Device::from_syspath(sys_path)
        .map_err(|e| PlatformError::Udev("Couldn't transform syspath to device".to_owned(), e))
}

pub fn has_attr(device: &Device, attr_name: &str) -> bool {
    for attr in device.attributes() {
        if attr.name() == attr_name {
            return true;
        }
    }
    false
}

pub fn read_attr_bool(device: &Device, attr_name: &str) -> Result<bool> {
    if let Some(value) = device.attribute_value(attr_name) {
        let tmp = value.to_string_lossy();
        if tmp.trim() == "0" {
            return Ok(false);
        }
        return Ok(true);
    }
    Err(PlatformError::AttrNotFound(attr_name.to_owned()))
}

pub fn write_attr_bool(device: &mut Device, attr: &str, value: bool) -> Result<()> {
    let value = if value { 1 } else { 0 };
    device
        .set_attribute_value(attr, value.to_string())
        .map_err(|e| {
            warn!("attr write error: {e:?}");
            PlatformError::IoPath(attr.into(), e)
        })
}

pub fn read_attr_num<T>(device: &Device, attr_name: &str) -> Result<T>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    if let Some(value) = device.attribute_value(attr_name) {
        let tmp = value.to_string_lossy();
        return tmp.parse::<T>().map_err(|_e| PlatformError::ParseNum);
    }
    Err(PlatformError::AttrNotFound(attr_name.to_owned()))
}

pub fn write_attr_num<T>(device: &mut Device, attr_name: &str, value: T) -> Result<()>
where
    T: std::fmt::Display,
{
    if device
        .set_attribute_value(attr_name, format!("{value}"))
        .is_err()
    {
        return Err(PlatformError::AttrNotFound(attr_name.to_owned()));
    }
    Ok(())
}

pub fn read_attr_u8_array(device: &Device, attr_name: &str) -> Result<Vec<u8>> {
    if let Some(value) = device.attribute_value(attr_name) {
        let tmp = value.to_string_lossy();
        let tmp = tmp
            .split(' ')
            .map(|v| v.parse::<u8>().unwrap_or(0))
            .collect();
        return Ok(tmp);
    }
    Err(PlatformError::AttrNotFound(attr_name.to_owned()))
}

pub fn write_attr_u8_array(device: &mut Device, attr: &str, values: &[u8]) -> Result<()> {
    let mut tmp = String::new();
    for n in values {
        tmp.push_str(&n.to_string());
        tmp.push(' '); // space padding required
    }
    tmp.pop();
    device
        .set_attribute_value(attr, tmp.trim())
        .map_err(|e| PlatformError::IoPath(attr.into(), e))
}

pub fn read_attr_string(device: &Device, attr_name: &str) -> Result<String> {
    if let Some(value) = device.attribute_value(attr_name) {
        let tmp = value.to_string_lossy().to_string();
        return Ok(tmp);
    }
    Err(PlatformError::AttrNotFound(attr_name.to_owned()))
}

pub fn write_attr_string(device: &mut Device, attr: &str, value: &str) -> Result<()> {
    let tmp = value.trim();
    device
        .set_attribute_value(attr, tmp)
        .map_err(|e| PlatformError::IoPath(attr.into(), e))
}

pub fn read_attr_string_array(device: &Device, attr_name: &str) -> Result<Vec<PlatformProfile>> {
    if let Some(value) = device.attribute_value(attr_name) {
        let tmp: Vec<PlatformProfile> = value
            .to_string_lossy()
            .split(' ')
            .map(PlatformProfile::from)
            .collect();
        return Ok(tmp);
    }
    Err(PlatformError::AttrNotFound(attr_name.to_owned()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn check() {
        let data = [
            1, 2, 3, 4, 5,
        ];
        let mut tmp = String::new();
        for n in data {
            tmp.push_str(&n.to_string());
            tmp.push(' '); // space padding required
        }
        tmp.pop();
        assert_eq!(tmp, "1 2 3 4 5");

        let tmp: Vec<u8> = tmp
            .split(' ')
            .map(|v| v.parse::<u8>().unwrap_or(0))
            .collect();
        assert_eq!(
            tmp,
            &[
                1, 2, 3, 4, 5
            ]
        );
    }
}
