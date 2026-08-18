//! Types used to represent data

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BatteryInfo {
    pub health: u8,
    pub consumption: f32,
    pub status: String,
    pub estimated_time: (i32, i32),
    pub is_charging: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CpuTelemetry {
    pub temp: f32,
    pub freq_mhz: f32,
    pub usage_pct: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GpuTelemetry {
    pub temp: f32,
    pub freq_mhz: f32,
    pub usage_pct: f32,
    pub suspended: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SystemTelemetry {
    pub cpu: CpuTelemetry,
    pub dgpu: GpuTelemetry,
    pub igpu_temp: f32,
    pub igpu_usage: f32,
    pub ram_usage_pct: f32,
    pub fan_rpms: (i32, i32, i32), // (CPU, GPU, Mid)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AttrMinMaxData {
    pub min: i32,
    pub max: i32,
    pub current: f32,
}
