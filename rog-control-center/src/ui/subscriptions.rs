//! Here are defined the subscriptions, they must be minimal, most of the lifting should be in a helper function

use rog_platform::cpu::get_ram_usage_pct;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    helpers::{
        hardware::get_cpu_telemetry,
        types::{BatteryInfo, FanTelemetry, GpuTelemetry, SystemTelemetry},
    },
    state::Event,
};
use std::time::Duration;

pub async fn subscribe_battery(tx: UnboundedSender<Event>) {
    loop {
        let charge = (rand::random::<u8>() % 101) as u8;
        let info = BatteryInfo {
            health: charge,
            ..Default::default()
        };
        if tx.send(Event::BatteryUpdated(info)).is_err() {
            return;
        };

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Loop that retrieve system telemetry every 1 sec
pub async fn subscribe_telemetry(tx: UnboundedSender<Event>) {
    loop {
        // CPU
        let cpu = get_cpu_telemetry();

        let ram = get_ram_usage_pct();

        let telemetry = SystemTelemetry {
            cpu,
            dgpu: GpuTelemetry::default(),
            igpu_temp: 0.0,
            igpu_usage: 0.0,
            ram_usage_pct: ram,
            fan_rpms: FanTelemetry::default(),
        };

        if tx.send(Event::TelemetryUpdated(telemetry)).is_err() {
            return;
        }

        tokio::time::sleep(Duration::from_secs(1)).await
    }
}

/// Loop that waits for ppd dbus event
pub async fn subscribe_ppd(_tx: UnboundedSender<Event>) {}
