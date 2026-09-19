use std::{fs, io, path::Path};

use crate::helpers::types::{
    BatteryInfo, CpuTelemetry, FanTelemetry, GpuTelemetry, SystemInfoData, SystemTelemetry,
};
use anyhow::Result;
use rog_platform::cpu::{CpuTicks, get_cpu_model, get_ram_usage_pct};
use rog_platform::gpu_pci::{Device, GfxPower, get_gpu_names};
use rog_platform::platform::get_fan_rpms;

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
    prev: Option<rog_platform::cpu::CpuTicks>,
    curr: Option<rog_platform::cpu::CpuTicks>,
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

pub fn get_cpu_telemetry(prev_tick: Option<CpuTicks>) -> (CpuTelemetry, Option<CpuTicks>) {
    let current_cpu_tick = rog_platform::cpu::read_cpu_ticks();
    let cpu_usage = calculate_cpu_usage(prev_tick, current_cpu_tick);
    (
        CpuTelemetry {
            temp: rog_platform::cpu::get_cpu_temp(),
            freq_mhz: rog_platform::cpu::get_cpu_frequency_mhz(),
            usage_pct: cpu_usage,
        },
        current_cpu_tick,
    )
}

/// Collect the full system telemetry
pub fn get_system_telemetry(prev_tick: Option<CpuTicks>) -> (SystemTelemetry, Option<CpuTicks>) {
    let (cpu, tick) = get_cpu_telemetry(prev_tick);
    let (cpu_fan, gpu_fan, mid_fan) = get_fan_rpms();

    let mut dgpu = GpuTelemetry {
        temp: -1.0,
        freq_mhz: -1.0,
        usage_pct: -1.0,
        suspended: false,
    };
    let mut igpu_temp = -1.0;
    let mut igpu_usage = -1.0;

    if let Ok(devices) = Device::find() {
        for device in &devices {
            if device.is_dgpu() {
                match device.get_runtime_status() {
                    Ok(GfxPower::Suspended) => dgpu.suspended = true,
                    Ok(GfxPower::Active) => {
                        dgpu.temp = device.get_temp().unwrap_or(-1.0);
                        dgpu.freq_mhz = device.get_freq_mhz().unwrap_or(-1.0);
                        dgpu.usage_pct = device.get_usage_pct().unwrap_or(-1.0);
                    }
                    _ => {}
                }
            } else {
                igpu_temp = device.get_temp().unwrap_or(-1.0);
                igpu_usage = device.get_usage_pct().unwrap_or(-1.0);
            }
        }
    }

    let telemetry = SystemTelemetry {
        cpu,
        dgpu,
        igpu_temp,
        igpu_usage,
        ram_usage_pct: get_ram_usage_pct(),
        fan_rpms: FanTelemetry {
            cpu: cpu_fan,
            gpu: gpu_fan,
            // The hwmon read returns 0 when the fan is missing
            mid: (mid_fan > 0).then_some(mid_fan),
        },
    };

    (telemetry, tick)
}

pub fn get_system_info() -> SystemInfoData {
    let (igpu_name, dgpu_name) = get_gpu_names();

    let mut has_igpu = false;
    let mut has_dgpu = false;
    if let Ok(devices) = Device::find() {
        for device in &devices {
            if device.is_dgpu() {
                has_dgpu = true;
            } else {
                has_igpu = true;
            }
        }
    }

    SystemInfoData {
        cpu_name: get_cpu_model(),
        igpu_name,
        dgpu_name,
        has_igpu,
        has_dgpu,
    }
}
