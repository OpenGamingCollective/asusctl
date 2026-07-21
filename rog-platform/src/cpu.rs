use std::path::PathBuf;

use log::{info, warn};
use serde::{Deserialize, Serialize};
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::error::{PlatformError, Result};
use crate::platform::PlatformProfile;
use crate::{read_attr_string, to_device};

const ATTR_AVAILABLE_GOVERNORS: &str = "cpufreq/scaling_available_governors";
const ATTR_GOVERNOR: &str = "cpufreq/scaling_governor";
const ATTR_AVAILABLE_EPP: &str = "cpufreq/energy_performance_available_preferences";
const ATTR_EPP: &str = "cpufreq/energy_performance_preference";

/// Both modern AMD and Intel have cpufreq control if using `powersave`
/// governor. What interests us the most here is `energy_performance_preference`
/// which can drastically alter CPU performance.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct CPUControl {
    paths: Vec<PathBuf>,
}

impl CPUControl {
    pub fn new() -> Result<Self> {
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("cpu").map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        let mut supported = false;
        let mut cpu = CPUControl { paths: Vec::new() };
        for device in enumerator.scan_devices().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("CPU: scan_devices failed".into(), err)
        })? {
            if !supported {
                info!(
                    "Found CPU support at {:?}, checking supported items",
                    device.sysname()
                );

                match device.attribute_value(ATTR_AVAILABLE_GOVERNORS) {
                    Some(g) => info!("{ATTR_AVAILABLE_GOVERNORS}: {g:?}"),
                    None => {
                        return Err(PlatformError::CPU(format!(
                            "{ATTR_AVAILABLE_GOVERNORS} not found"
                        )))
                    }
                }
                match device.attribute_value(ATTR_GOVERNOR) {
                    Some(g) => info!("{ATTR_GOVERNOR}: {g:?}"),
                    None => return Err(PlatformError::CPU(format!("{ATTR_GOVERNOR} not found"))),
                }
                match device.attribute_value(ATTR_AVAILABLE_EPP) {
                    Some(g) => info!("{ATTR_AVAILABLE_EPP}: {g:?}"),
                    None => {
                        return Err(PlatformError::CPU(format!(
                            "{ATTR_AVAILABLE_EPP} not found"
                        )))
                    }
                }
                match device.attribute_value(ATTR_EPP) {
                    Some(g) => info!("{ATTR_EPP}: {g:?}"),
                    None => return Err(PlatformError::CPU(format!("{ATTR_EPP} not found"))),
                }
                supported = true;
            }
            if supported {
                info!("Adding: {:?}", device.syspath());
                cpu.paths.push(device.syspath().to_owned());
            }
        }
        if cpu.paths.is_empty() {
            return Err(PlatformError::MissingFunction(
                "asus-nb-wmi not found".into(),
            ));
        }
        Ok(cpu)
    }

    pub fn get_governor(&self) -> Result<CPUGovernor> {
        if let Some(path) = self.paths.first() {
            let s = read_attr_string(&to_device(path)?, ATTR_GOVERNOR)?;
            Ok(s.as_str().into())
            // TODO: check cpu are sync
        } else {
            Err(PlatformError::CPU("No CPU's?".to_string()))
        }
    }

    pub fn get_available_governors(&self) -> Result<Vec<CPUGovernor>> {
        if let Some(path) = self.paths.first() {
            read_attr_string(&to_device(path)?, ATTR_AVAILABLE_GOVERNORS)
                .map(|s| s.split_whitespace().map(|s| s.into()).collect())
            // TODO: check cpu are sync
        } else {
            Err(PlatformError::CPU("No CPU's?".to_string()))
        }
    }

    pub fn set_governor(&self, gov: CPUGovernor) -> Result<()> {
        if !self.get_available_governors()?.contains(&gov) {
            return Err(PlatformError::CPU(format!("{gov:?} is not available")));
        }
        for path in &self.paths {
            let mut dev = to_device(path)?;
            dev.set_attribute_value(ATTR_GOVERNOR, String::from(gov))?;
        }
        Ok(())
    }

    pub fn get_epp(&self) -> Result<CPUEPP> {
        if let Some(path) = self.paths.first() {
            let s = read_attr_string(&to_device(path)?, ATTR_EPP)?;
            Ok(s.as_str().into())
            // TODO: check cpu are sync
        } else {
            Err(PlatformError::CPU("No CPU's?".to_string()))
        }
    }

    pub fn get_available_epp(&self) -> Result<Vec<CPUEPP>> {
        if let Some(path) = self.paths.first() {
            read_attr_string(&to_device(path)?, ATTR_AVAILABLE_EPP)
                .map(|s| s.split_whitespace().map(|s| s.into()).collect())
            // TODO: check cpu are sync
        } else {
            Err(PlatformError::CPU("No CPU's?".to_string()))
        }
    }

    pub fn set_epp(&self, epp: CPUEPP) -> Result<()> {
        if !self.get_available_epp()?.contains(&epp) {
            return Err(PlatformError::CPU(format!("{epp:?} is not available")));
        }
        for path in &self.paths {
            let mut dev = to_device(path)?;
            dev.set_attribute_value(ATTR_EPP, String::from(epp))?;
        }
        Ok(())
    }
}

#[repr(u8)]
#[derive(
    Deserialize, Serialize, Type, Value, OwnedValue, Debug, PartialEq, PartialOrd, Clone, Copy,
)]
#[zvariant(signature = "s")]
pub enum CPUGovernor {
    Performance = 0,
    Powersave = 1,
    BadValue = 2,
}

impl From<&str> for CPUGovernor {
    fn from(s: &str) -> Self {
        match s {
            "performance" => Self::Performance,
            "powersave" => Self::Powersave,
            _ => Self::BadValue,
        }
    }
}

impl From<CPUGovernor> for String {
    fn from(g: CPUGovernor) -> Self {
        match g {
            CPUGovernor::Performance => "performance".to_string(),
            CPUGovernor::Powersave => "powersave".to_string(),
            CPUGovernor::BadValue => "bad_value".to_string(),
        }
    }
}

#[repr(u32)]
#[derive(
    Deserialize,
    Serialize,
    Type,
    Value,
    OwnedValue,
    Default,
    Debug,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
)]
#[zvariant(signature = "u")]
pub enum CPUEPP {
    #[default]
    Default = 0,
    Performance = 1,
    BalancePerformance = 2,
    BalancePower = 3,
    Power = 4,
}

impl From<PlatformProfile> for CPUEPP {
    fn from(value: PlatformProfile) -> Self {
        match value {
            PlatformProfile::Balanced => CPUEPP::BalancePerformance,
            PlatformProfile::Performance => CPUEPP::Performance,
            PlatformProfile::Quiet => CPUEPP::Power,
            PlatformProfile::LowPower => CPUEPP::Power,
            PlatformProfile::Custom => CPUEPP::BalancePower,
        }
    }
}

impl From<&str> for CPUEPP {
    fn from(s: &str) -> Self {
        match s {
            "default" => Self::Default,
            "performance" => Self::Performance,
            "balance_performance" => Self::BalancePerformance,
            "balance_power" => Self::BalancePower,
            "power" => Self::Power,
            _ => Self::Default,
        }
    }
}

impl From<CPUEPP> for String {
    fn from(g: CPUEPP) -> Self {
        match g {
            CPUEPP::Default => "default".to_string(),
            CPUEPP::Performance => "performance".to_string(),
            CPUEPP::BalancePerformance => "balance_performance".to_string(),
            CPUEPP::BalancePower => "balance_power".to_string(),
            CPUEPP::Power => "power".to_string(),
        }
    }
}

impl From<i32> for CPUEPP {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Default,
            1 => Self::Performance,
            2 => Self::BalancePerformance,
            3 => Self::BalancePower,
            4 => Self::Power,
            _ => Self::Default,
        }
    }
}

impl From<CPUEPP> for i32 {
    fn from(value: CPUEPP) -> Self {
        value as i32
    }
}

pub fn get_cpu_model() -> String {
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in cpuinfo.lines() {
            if line.starts_with("model name") {
                if let Some(pos) = line.find(':') {
                    let model = line[pos + 1..].trim().to_string();
                    if !model.is_empty() {
                        return model;
                    }
                }
            }
        }
    }
    "CPU".to_string()
}

#[derive(Debug, Clone, Copy)]
pub struct CpuTicks {
    pub idle: u64,
    pub total: u64,
}

pub fn read_cpu_ticks() -> Option<CpuTicks> {
    let stat = std::fs::read_to_string("/proc/stat").ok()?;
    let first_line = stat.lines().next()?;
    if first_line.starts_with("cpu ") {
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        let mut total = 0u64;
        let mut idle = 0u64;
        for (i, part) in parts.iter().skip(1).enumerate() {
            if let Ok(ticks) = part.parse::<u64>() {
                total += ticks;
                if i == 3 || i == 4 {
                    idle += ticks;
                }
            }
        }
        return Some(CpuTicks { idle, total });
    }
    None
}

pub fn get_cpu_temp() -> f32 {
    if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(name) = std::fs::read_to_string(path.join("name")) {
                let name = name.trim();
                if name == "k10temp" || name == "coretemp" || name == "zenpower" {
                    if let Ok(temp_str) = std::fs::read_to_string(path.join("temp1_input")) {
                        if let Ok(temp_val) = temp_str.trim().parse::<f32>() {
                            return temp_val / 1000.0;
                        }
                    }
                }
            }
        }
    }
    if let Ok(temp_str) = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
        if let Ok(temp_val) = temp_str.trim().parse::<f32>() {
            return temp_val / 1000.0;
        }
    }
    0.0
}

pub fn get_cpu_frequency_mhz() -> f32 {
    let mut total_freq = 0.0;
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit()) {
                let freq_path = entry.path().join("cpufreq/scaling_cur_freq");
                if let Ok(freq_str) = std::fs::read_to_string(freq_path) {
                    if let Ok(freq_khz) = freq_str.trim().parse::<f32>() {
                        total_freq += freq_khz / 1000.0;
                        count += 1;
                    }
                }
            }
        }
    }
    if count == 0 {
        if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in cpuinfo.lines() {
                if line.starts_with("cpu MHz") {
                    if let Some(pos) = line.find(':') {
                        if let Ok(val) = line[pos + 1..].trim().parse::<f32>() {
                            total_freq += val;
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    if count > 0 {
        total_freq / count as f32
    } else {
        0.0
    }
}

pub fn get_ram_usage_pct() -> f32 {
    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        let mut total = 0.0;
        let mut available = 0.0;
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(val) = parts.get(1) {
                    total = val.parse::<f32>().unwrap_or(0.0);
                }
            } else if line.starts_with("MemAvailable:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(val) = parts.get(1) {
                    available = val.parse::<f32>().unwrap_or(0.0);
                }
            }
        }
        if total > 0.0 {
            return ((total - available) / total) * 100.0;
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::CPUControl;
    use crate::cpu::{CPUGovernor, CPUEPP};

    #[test]
    #[ignore = "Can't run this in a docker image"]
    fn check_cpu() {
        let cpu = CPUControl::new().unwrap();
        assert_eq!(cpu.get_governor().unwrap(), CPUGovernor::Powersave);
        assert_eq!(
            cpu.get_available_governors().unwrap(),
            vec![
                CPUGovernor::Performance,
                CPUGovernor::Powersave
            ]
        );

        assert_eq!(cpu.get_epp().unwrap(), CPUEPP::BalancePower);
        assert_eq!(
            cpu.get_available_epp().unwrap(),
            vec![
                CPUEPP::Default,
                CPUEPP::Performance,
                CPUEPP::BalancePerformance,
                CPUEPP::BalancePower,
                CPUEPP::Power,
            ]
        );
    }
}
