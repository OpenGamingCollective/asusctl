use std::path::PathBuf;

use log::{info, warn};

use crate::error::{PlatformError, Result};
use crate::{attr_num, to_device};

/// sysfs power_supply attributes are reported in micro-units (uW, uWh, uA,
/// uAh, uV)
const MICRO: f64 = 1_000_000.0;

/// The "platform" device provides access to things like:
/// - `dgpu_disable`
/// - `egpu_enable`
/// - `panel_od`
/// - `gpu_mux`
/// - `keyboard_mode`, set keyboard RGB mode and speed
/// - `keyboard_state`, set keyboard power states
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct AsusPower {
    mains: PathBuf,
    battery: PathBuf,
    usb: Option<PathBuf>,
}

impl AsusPower {
    attr_num!("charge_control_end_threshold", battery, u8);

    attr_num!("online", mains, u8);

    /// When checking for battery this will look in order:
    /// - if attr `manufacturer` contains `asus`
    /// - if attr `charge_control_end_threshold` exists and `energy_full_design`
    ///   >= 50 watt
    /// - if syspath end conatins `BAT`
    /// - if attr `type` is `battery` (last resort)
    pub fn new() -> Result<Self> {
        let mut mains = PathBuf::new();
        let mut battery = None;
        let mut usb = None;

        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("power_supply").map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for device in enumerator.scan_devices().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("scan_devices failed".into(), err)
        })? {
            if let Some(attr) = device.attribute_value("type") {
                info!("Power: Checking {:?}", device.syspath());
                match attr.to_string_lossy().to_ascii_lowercase().trim() {
                    "mains" => {
                        info!("Found mains power at {:?}", device.sysname());
                        mains = device.syspath().to_path_buf();
                    }
                    "battery" => {
                        // Priortised list of checks
                        info!("Found a battery");
                        if battery.is_none() {
                            info!("Checking battery attributes");
                            if let Some(current) =
                                device.attribute_value("charge_control_end_threshold")
                            {
                                info!(
                                    "Found battery power at {:?}, matched \
                                     charge_control_end_threshold. Current level: {current:?}",
                                    device.sysname()
                                );
                                battery = Some(device.syspath().to_path_buf());
                            } else if device.sysname().to_string_lossy().starts_with("BAT") {
                                info!(
                                    "Found battery power at {:?}, sysfs path ended with BAT<n>",
                                    device.sysname()
                                );
                                battery = Some(device.syspath().to_path_buf());
                            } else {
                                info!(
                                    "Last resort: Found battery power at {:?} using type = Battery",
                                    device.sysname()
                                );
                                battery = Some(device.syspath().to_path_buf());
                            }
                        }
                    }
                    "usb" => {
                        info!("Found USB-C power at {:?}", device.sysname());
                        usb = Some(device.syspath().to_path_buf());
                    }
                    _ => {}
                };
            }
        }

        if let Some(battery) = battery {
            return Ok(Self {
                mains,
                battery,
                usb,
            });
        }

        // No battery found. Return an AsusPower with an empty battery path so
        // callers can still be constructed and query `has_*` methods which
        // will correctly report absence. This avoids hard-failing on systems
        // where the asus-nb-wmi driver loads on desktops with no battery.
        info!("Did not find a battery, continuing without battery support");
        Ok(Self {
            mains,
            battery: PathBuf::new(),
            usb,
        })
    }

    pub fn has_battery(&self) -> bool {
        !self.battery.as_os_str().is_empty()
    }

    pub fn get_battery_cycle_count(&self) -> Result<i32> {
        let path = self.battery.join("cycle_count");
        if !path.exists() {
            return Err(PlatformError::Read(
                path.to_string_lossy().into(),
                std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
            ));
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| PlatformError::Read(path.to_string_lossy().into(), e))?;
        content.trim().parse::<i32>().map_err(|e| {
            PlatformError::Read(
                path.to_string_lossy().into(),
                std::io::Error::new(std::io::ErrorKind::InvalidData, e),
            )
        })
    }

    pub fn get_battery_health(&self) -> Result<u8> {
        // Ratio of raw values; energy vs charge units cancel out
        let full = self
            .read_battery_attr("energy_full")
            .or_else(|_| self.read_battery_attr("charge_full"))
            .ok();
        let design = self
            .read_battery_attr("energy_full_design")
            .or_else(|_| self.read_battery_attr("charge_full_design"))
            .ok();

        match (full, design) {
            (Some(f), Some(d)) if d > 0.0 => {
                let health = (f / d * 100.0).round().clamp(0.0, 100.0) as u8;
                Ok(health)
            }
            _ => Err(PlatformError::Read(
                self.battery.to_string_lossy().into(),
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "energy/charge attributes not found",
                ),
            )),
        }
    }

    fn read_battery_attr(&self, attr: &str) -> Result<f64> {
        let content = std::fs::read_to_string(self.battery.join(attr))
            .map_err(|e| PlatformError::Read(attr.into(), e))?;
        content.trim().parse::<f64>().map_err(|e| {
            PlatformError::Read(
                attr.into(),
                std::io::Error::new(std::io::ErrorKind::InvalidData, e),
            )
        })
    }

    /// Batteries report either in power units (`direct` attr, uW/uWh) or in
    /// charge units (`charge` attr, uA/uAh) which must be multiplied by
    /// `voltage_now` (uV). Returns plain watts/watt-hours either way.
    fn read_direct_or_charge_based(&self, direct: &str, charge: &str) -> Result<f64> {
        if self.battery.join(direct).exists() {
            Ok(self.read_battery_attr(direct)? / MICRO)
        } else if self.battery.join(charge).exists() && self.battery.join("voltage_now").exists() {
            Ok(
                self.read_battery_attr(charge)? * self.read_battery_attr("voltage_now")?
                    / (MICRO * MICRO),
            )
        } else {
            Err(PlatformError::Read(
                self.battery.to_string_lossy().into(),
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("{direct}/{charge} attributes not found"),
                ),
            ))
        }
    }

    pub fn get_battery_power_consumption(&self) -> Result<f32> {
        self.read_direct_or_charge_based("power_now", "current_now")
            .map(|w| w as f32)
    }

    pub fn get_battery_status(&self) -> Result<String> {
        let path = self.battery.join("status");
        if !path.exists() {
            return Err(PlatformError::Read(
                path.to_string_lossy().into(),
                std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
            ));
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| PlatformError::Read(path.to_string_lossy().into(), e))?;
        Ok(content.trim().to_string())
    }

    pub fn get_battery_remaining_energy_wh(&self) -> Result<f64> {
        self.read_direct_or_charge_based("energy_now", "charge_now")
    }

    pub fn get_battery_full_energy_wh(&self) -> Result<f64> {
        self.read_direct_or_charge_based("energy_full", "charge_full")
    }

    pub fn get_battery_time_estimate(&self) -> Result<Option<(bool, i32, i32)>> {
        let status = self.get_battery_status()?;
        let is_charging = match status.as_str() {
            "Charging" => true,
            "Discharging" => false,
            _ => return Ok(None),
        };

        let power_draw = self.get_battery_power_consumption().unwrap_or(0.0).abs();
        if power_draw < 0.1 {
            return Ok(None);
        }

        let energy_now = self.get_battery_remaining_energy_wh()?;
        let energy_full = self.get_battery_full_energy_wh()?;

        let remaining_wh = if is_charging {
            let limit = self.get_charge_control_end_threshold().unwrap_or(100) as f64;
            let target_wh = energy_full * (limit / 100.0);
            if energy_now >= target_wh {
                return Ok(None);
            }
            target_wh - energy_now
        } else {
            energy_now
        };

        let hours_float = remaining_wh / power_draw as f64;
        if hours_float < 0.0 || hours_float.is_nan() || hours_float.is_infinite() {
            return Ok(None);
        }

        let total_minutes = (hours_float * 60.0).round() as i32;
        let hours = total_minutes / 60;
        let minutes = total_minutes % 60;

        Ok(Some((is_charging, hours, minutes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_battery_methods() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp_dir = std::env::temp_dir().join("fake_battery");
        fs::create_dir_all(&temp_dir)?;

        // Write fake files
        fs::write(temp_dir.join("cycle_count"), "42\n")?;
        fs::write(temp_dir.join("energy_full"), "80000000\n")?;
        fs::write(temp_dir.join("energy_full_design"), "100000000\n")?;
        fs::write(temp_dir.join("energy_now"), "45000000\n")?;
        fs::write(temp_dir.join("power_now"), "15000000\n")?;
        fs::write(temp_dir.join("status"), "Discharging\n")?;

        let power = AsusPower {
            mains: PathBuf::new(),
            battery: temp_dir.clone(),
            usb: None,
        };

        assert!(power.has_battery());
        assert_eq!(power.get_battery_cycle_count()?, 42);
        assert_eq!(power.get_battery_health()?, 80);
        assert_eq!(power.get_battery_power_consumption()?, 15.0);
        assert_eq!(power.get_battery_status()?, "Discharging");
        assert_eq!(power.get_battery_remaining_energy_wh()?, 45.0);
        assert_eq!(power.get_battery_full_energy_wh()?, 80.0);
        assert_eq!(power.get_battery_time_estimate()?, Some((false, 3, 0)));

        // Test charging estimation
        fs::write(temp_dir.join("status"), "Charging\n")?;
        assert_eq!(power.get_battery_time_estimate()?, Some((true, 2, 20)));

        // Clean up
        let _ = fs::remove_dir_all(temp_dir);
        Ok(())
    }

    #[test]
    fn test_charge_based_battery_methods() {
        let temp_dir = std::env::temp_dir().join("fake_battery_charge");
        fs::create_dir_all(&temp_dir).unwrap();

        // Charge-unit battery (mA/mAh firmware): no power_now/energy_* attrs
        fs::write(temp_dir.join("current_now"), "2870000\n").unwrap();
        fs::write(temp_dir.join("voltage_now"), "15811000\n").unwrap();
        fs::write(temp_dir.join("charge_now"), "3322000\n").unwrap();
        fs::write(temp_dir.join("charge_full"), "4700000\n").unwrap();
        fs::write(temp_dir.join("status"), "Discharging\n").unwrap();

        let power = AsusPower {
            mains: PathBuf::new(),
            battery: temp_dir.clone(),
            usb: None,
        };

        // 2.87 A * 15.811 V = 45.377... W
        let watts = power.get_battery_power_consumption().unwrap();
        assert!((watts - 45.377).abs() < 0.01, "got {watts} W");

        // 3.322 Ah * 15.811 V = 52.524... Wh
        let wh = power.get_battery_remaining_energy_wh().unwrap();
        assert!((wh - 52.524).abs() < 0.01, "got {wh} Wh");

        let full_wh = power.get_battery_full_energy_wh().unwrap();
        assert!((full_wh - 74.311).abs() < 0.01, "got {full_wh} Wh");

        fs::remove_dir_all(&temp_dir).ok();
    }
}
