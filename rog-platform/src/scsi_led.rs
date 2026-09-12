use std::path::Path;

use log::{info, warn};

use crate::dynamic_led::DynamicLed;
use crate::error::{PlatformError, Result};

/// Generic control interface for ASUS Aura SCSI-attached lighting devices
/// backed by the kernel `leds-asus-aura-scsi` Dynamic Lighting driver.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct ScsiLed {
    dynamic: DynamicLed,
}

impl ScsiLed {
    /// Discover the first available ASUS SCSI dynamic lighting LED node.
    pub fn new() -> Result<Self> {
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("ScsiLed udev enumerator failed: {err}");
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("leds").map_err(|err| {
            warn!("ScsiLed match_subsystem failed: {err}");
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for device in enumerator.scan_devices().map_err(|err| {
            warn!("ScsiLed scan_devices failed: {err}");
            PlatformError::Udev("scan_devices failed".into(), err)
        })? {
            let sysname = device.sysname().to_string_lossy();
            if sysname.contains("asus-scsi")
                || sysname.contains("asus-aura-scsi")
                || sysname.contains("asus-arion")
            {
                info!(
                    "Found ASUS SCSI Dynamic Lighting LED device at {:?}",
                    sysname
                );
                let dynamic = DynamicLed::find(&sysname)?;
                return Ok(Self { dynamic });
            }
        }

        Err(PlatformError::MissingFunction(
            "ScsiLed::new(): no asus scsi dynamic LED device found".into(),
        ))
    }

    /// Find the LED whose sysfs ancestry belongs to this exact SCSI device.
    pub fn find_for_block(device: &udev::Device) -> Result<Self> {
        let mut current = device.parent();
        let mut scsi_path = None;
        while let Some(d) = current {
            if let Some(sub) = d.subsystem()
                && sub == "scsi"
            {
                let s = d.sysname().to_string_lossy();
                if s.contains(':') {
                    scsi_path = Some(d.syspath().to_path_buf());
                    break;
                }
            }
            current = d.parent();
        }

        let scsi_path = scsi_path.ok_or_else(|| {
            PlatformError::MissingFunction(format!(
                "No SCSI parent found for {}",
                device.syspath().display()
            ))
        })?;
        let mut enumerator = udev::Enumerator::new()
            .map_err(|err| PlatformError::Udev("enumerator failed".into(), err))?;
        enumerator
            .match_subsystem("leds")
            .map_err(|err| PlatformError::Udev("match_subsystem failed".into(), err))?;

        for dev in enumerator
            .scan_devices()
            .map_err(|err| PlatformError::Udev("scan_devices failed".into(), err))?
        {
            if belongs_to_scsi(dev.syspath(), &scsi_path) {
                let dynamic = DynamicLed::from_syspath(dev.syspath().to_path_buf())?;
                info!(
                    "Found exact SCSI Dynamic Lighting LED {:?} for {}",
                    dev.sysname(),
                    scsi_path.display()
                );
                return Ok(Self { dynamic });
            }
        }

        Err(PlatformError::MissingFunction(format!(
            "Dynamic Lighting LED for {} is not registered yet",
            scsi_path.display()
        )))
    }

    /// Find a `ScsiLed` for a specific `/dev/sdX` or `/dev/sgN` path.
    pub fn find_for_dev(dev_node: &str) -> Result<Self> {
        let mut enumerator = udev::Enumerator::new().map_err(|e| {
            warn!("ScsiLed udev enumerator failed: {e}");
            PlatformError::Udev("enumerator failed".into(), e)
        })?;
        enumerator.match_subsystem("block").map_err(|e| {
            warn!("ScsiLed match_subsystem failed: {e}");
            PlatformError::Udev("match block failed".into(), e)
        })?;

        for dev in enumerator.scan_devices().map_err(|e| {
            warn!("ScsiLed scan_devices failed: {e}");
            PlatformError::Udev("scan failed".into(), e)
        })? {
            if let Some(node) = dev.devnode()
                && node.to_string_lossy() == dev_node
            {
                return Self::find_for_block(&dev);
            }
        }

        Err(PlatformError::MissingFunction(format!(
            "No block device found for {dev_node}"
        )))
    }

    /// Check if an ASUS SCSI Dynamic Lighting LED is available on the system.
    pub fn is_available() -> bool {
        Self::new().is_ok()
    }

    /// Return reference to inner `DynamicLed`.
    pub fn dynamic(&self) -> &DynamicLed {
        &self.dynamic
    }

    /// Return path to the sysfs node.
    pub fn path(&self) -> &Path {
        self.dynamic.path()
    }

    /// Set animation effect string.
    pub fn set_effect(&self, effect: &str) -> Result<()> {
        self.dynamic.set_effect(effect)
    }

    /// Set effect animation speed.
    pub fn set_speed(&self, speed: u32) -> Result<()> {
        self.dynamic.set_supported_speed(speed)
    }

    /// Set effect animation direction ("right" or "left").
    pub fn set_direction(&self, direction: &str) -> Result<()> {
        self.dynamic.set_supported_direction(direction)
    }

    /// Set palette colors formatted as `(r, g, b)`.
    pub fn set_palette_colors(&self, colors: &[(u8, u8, u8)]) -> Result<()> {
        self.dynamic.set_palette_colors(colors)
    }

    /// Write raw RGB bytes to the direct buffer.
    pub fn write_direct(&self, data: &[u8]) -> Result<()> {
        self.dynamic.write_direct(data)
    }

    /// Set brightness (0..=255).
    pub fn set_brightness(&self, brightness: u8) -> Result<()> {
        self.dynamic.set_brightness(brightness)
    }

    /// Get brightness (0..=255).
    pub fn get_brightness(&self) -> Result<u8> {
        self.dynamic.get_brightness()
    }
}

fn belongs_to_scsi(led_path: &Path, scsi_path: &Path) -> bool {
    led_path.starts_with(scsi_path)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn matches_only_exact_scsi_ancestry() {
        let scsi = Path::new("/sys/devices/usb/2:0:0:0");
        assert!(super::belongs_to_scsi(
            Path::new("/sys/devices/usb/2:0:0:0/leds/asus-aura-scsi"),
            scsi
        ));
        assert!(!super::belongs_to_scsi(
            Path::new("/sys/devices/usb/2:0:0:01/leds/asus-aura-scsi"),
            scsi
        ));
    }
}
