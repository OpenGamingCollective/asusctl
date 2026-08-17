use std::{fs, io, path::Path};

use crate::ui::helpers::types::BatteryInfo;
use anyhow::Result;

pub fn get_dmi_product_name() -> io::Result<String> {
    let path = Path::new("/sys/class/dmi/id/product_name");
    fs::read_to_string(path)
}

/// Get the battery informations from rog_platform
pub fn battery_infos() -> Result<BatteryInfo> {
    // Get the power informations from the rog_platform, return if fail
    let power = rog_platform::power::AsusPower::new()?;

    let health = power.get_battery_health()?;

    let consumption = power.get_battery_power_consumption()?;

    let status = power
        .get_battery_status()
        .unwrap_or_else(|_| String::from("Unknown"));
    // Get both the charging state and estimated time
    let (is_charging, estimated_time) = power
        .get_battery_time_estimate()?
        .map(|(charging, hours, minutes)| (charging, (hours, minutes)))
        .unwrap_or((false, (0, 0)));

    Ok(BatteryInfo {
        health,
        consumption,
        status,
        estimated_time,
        is_charging,
    })
}

/// Helper to calculate the CPU usage given the previous and current ticks
pub fn calculate_cpu_sage(
    prev: Option<&rog_platform::cpu::CpuTicks>,
    curr: Option<&rog_platform::cpu::CpuTicks>,
) -> f32 {
    if let (Some(p), Some(c)) = (prev, curr) {
        let idle_diff = c.idle.saturating_sub(p.idle) as f32;
        let total_diff = c.total.saturating_sub(p.total) as f32;
        if total_diff > 0.0 {
            return ((1.0 - (idle_diff / total_diff)) * 100.0).clamp(0.0, 100.0);
        }
    }
    0.0
}
