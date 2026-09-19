//! Here are defined the subscriptions, they must be minimal, most of the lifting should be in a helper function

use crate::{
    helpers::{
        hardware::get_system_telemetry,
        types::BatteryInfo,
        zbus_proxies::{AsusdInterface, get_min_max_current},
    },
    state::Event,
};
use futures_util::{Stream, StreamExt, stream::SelectAll};
use rog_dbus::asus_armoury::AsusArmouryProxy;
use rog_platform::{asus_armoury::FirmwareAttribute, cpu::CpuTicks};
use std::{
    collections::HashMap,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::mpsc::UnboundedSender;

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
pub async fn subscribe_telemetry(tx: UnboundedSender<Event>, window_visible: Arc<AtomicBool>) {
    let mut prev_tick: Option<CpuTicks> = None;
    loop {
        if !window_visible.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        let (telemetry, tick) = get_system_telemetry(prev_tick);
        prev_tick = tick;

        if tx.send(Event::TelemetryUpdated(telemetry)).is_err() {
            return;
        }

        tokio::time::sleep(Duration::from_secs(1)).await
    }
}

/// Loop that waits for ppd dbus event
pub async fn subscribe_ppd(tx: UnboundedSender<Event>, asusd: Arc<OnceLock<AsusdInterface>>) {
    loop {
        if let Some(asusd_proxy) = asusd.get()
            && let Some(platform_proxy) = &asusd_proxy.platform
        {
            let mut ppd_stream = platform_proxy.receive_platform_profile_changed().await;
            while let Some(msg) = ppd_stream.next().await {
                if let Ok(new_ppd) = msg.get().await {
                    let _ = tx.send(Event::PlatformProfileSignalled(new_ppd.into()));
                }
            }
        } else {
            tokio::time::sleep(Duration::from_secs(1)).await
        }
    }
}

/// Stream for all armoury attr
async fn armoury_changes(
    armoury: &HashMap<FirmwareAttribute, AsusArmouryProxy<'static>>,
) -> impl Stream<Item = (FirmwareAttribute, AsusArmouryProxy<'static>)> {
    let mut select_all = SelectAll::new();
    for (attr, iface) in armoury {
        let stream: zbus::proxy::PropertyStream<'_, i32> =
            iface.receive_current_value_changed().await;
        select_all.push(stream.map(move |_| (*attr, iface.clone())));
    }
    select_all
}

pub async fn subscribe_armoury(tx: UnboundedSender<Event>, asusd: Arc<OnceLock<AsusdInterface>>) {
    loop {
        let Some(asusd_proxy) = asusd.get() else {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        };

        if asusd_proxy.armoury.is_empty() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        let mut changes = armoury_changes(&asusd_proxy.armoury).await;

        while let Some((attr, iface)) = changes.next().await {
            if let Some(mm) = get_min_max_current(&iface).await {
                let _ = tx.send(Event::FirmwareAttrUpdated(attr, mm));
            }
        }
    }
}
