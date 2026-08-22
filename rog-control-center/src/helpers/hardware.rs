use std::{fs, io, path::Path};

use crate::helpers::types::{BatteryInfo, CpuTelemetry};
use anyhow::Result;

pub fn get_dmi_product_name() -> io::Result<String> {
    let path = Path::new("/sys/class/dmi/id/product_name");
    Ok(fs::read_to_string(path)?.trim().to_owned())
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
pub fn calculate_cpu_usage(
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

pub fn get_current_ram() -> f32 {
    rog_platform::cpu::get_ram_usage_pct()
}

pub fn get_cpu_telemetry() -> CpuTelemetry {
    CpuTelemetry {
        temp: rog_platform::cpu::get_cpu_temp(),
        freq_mhz: rog_platform::cpu::get_cpu_frequency_mhz(),
        usage_pct: 99.99,
    }
}
