use std::sync::{Arc, Mutex};

use log::{debug, error, info};
use rog_aura::keyboard::LaptopAuraPower;
use rog_aura::{AuraDeviceType, PowerZones};
use rog_dbus::zbus_aura::AuraProxy;
use slint::{ComponentHandle, Model, RgbaColor, SharedString};

use crate::config::Config;
use crate::ui::show_toast;
use crate::{
    AuraPageData, MainWindow, PowerZones as SlintPowerZones, set_ui_callbacks, set_ui_props_async,
};

fn decode_hex(s: &str) -> RgbaColor<u8> {
    let s = s.trim_start_matches('#');
    if s.len() < 6 {
        return RgbaColor {
            alpha: 255,
            red: 0,
            green: 0,
            blue: 0,
        };
    }
    let c: Vec<u8> = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(164))
        .collect();
    RgbaColor {
        alpha: 255,
        red: *c.first().unwrap_or(&255),
        green: *c.get(1).unwrap_or(&128),
        blue: *c.get(2).unwrap_or(&32),
    }
}

async fn find_aura_iface() -> Result<AuraProxy<'static>, Box<dyn std::error::Error>> {
    let conn = zbus::Connection::system().await?;
    let mgr = zbus::fdo::ObjectManagerProxy::new(&conn, "xyz.ljones.Asusd", "/").await?;
    let objs = mgr.get_managed_objects().await?;
    let mut paths: Vec<zbus::zvariant::OwnedObjectPath> = objs
        .iter()
        .filter(|(_, ifaces)| ifaces.keys().any(|k| k.as_str() == "xyz.ljones.Aura"))
        .map(|(p, _)| p.clone())
        .collect();
    if paths.len() > 1 {
        log::debug!("Multiple aura devices: {paths:?}");
    }
    let path = paths.pop().ok_or("No Aura interface")?;
    AuraProxy::builder(&conn)
        .path(path)?
        .destination("xyz.ljones.Asusd")?
        .build()
        .await
        .map_err(Into::into)
}

pub async fn prefetch_supported_basic_modes() -> Option<Vec<i32>> {
    let proxy = find_aura_iface().await.ok()?;
    let modes = proxy.supported_basic_modes().await.ok()?;
    Some(modes.iter().map(|n| (*n).into()).collect())
}

pub fn setup_aura_page(
    ui: &MainWindow,
    _states: Arc<Mutex<Config>>,
    prefetched_supported: Option<Vec<i32>>,
) {
    let g = ui.global::<AuraPageData>();
    g.on_cb_hex_from_colour(|c| {
        format!("#{:02X}{:02X}{:02X}", c.red(), c.green(), c.blue()).into()
    });
    g.on_cb_hex_to_colour(|s| decode_hex(s.as_str()).into());

    let handle = ui.as_weak();
    tokio::spawn(async move {
        let Ok(aura) = find_aura_iface().await else {
            info!("No aura interfaces");
            return Ok::<(), zbus::Error>(());
        };

        set_ui_props_async!(handle, aura, AuraPageData, brightness);
        set_ui_props_async!(handle, aura, AuraPageData, led_power);
        set_ui_props_async!(handle, aura, AuraPageData, device_type);

        let modes_vec: Vec<i32> = match prefetched_supported {
            Some(p) => p,
            None => aura
                .supported_basic_modes()
                .await
                .ok()
                .map(|m| m.iter().map(|n| (*n).into()).collect())
                .unwrap_or_default(),
        };

        // Restore saved mode, colours, zone, speed, direction from asusd (persisted to disk).
        // Use effect.mode as single source — avoid led_mode() which can fail (try_lock).
        let restore = aura.led_mode_data().await.ok();
        let raw_mode: Option<i32> = restore.as_ref().map(|d| d.mode.into());
        let restore_zone: Option<i32> = restore.as_ref().map(|d| d.zone.into());
        let d_slint = restore.map(|d| d.into());
        handle
            .upgrade_in_event_loop(move |h| {
                let names = h.global::<AuraPageData>().get_mode_names();
                let mut raws = Vec::new();
                let mut mode_names = Vec::new();
                for (i, name) in names.iter().enumerate() {
                    let raw = i as i32;
                    // Skip Direct (13): the basic dropdown has no frame producer.
                    if modes_vec.contains(&raw) && i != 9 && raw != 13 {
                        raws.push(raw);
                        mode_names.push(name.clone());
                    }
                }
                h.global::<AuraPageData>()
                    .set_supported_basic_modes(raws.as_slice().into());
                h.global::<AuraPageData>()
                    .set_available_mode_names(mode_names.as_slice().into());
                if let Some(d) = d_slint {
                    h.global::<AuraPageData>().invoke_update_led_mode_data(d);
                    if let Some(cm) = raw_mode {
                        let idx = raws.iter().position(|&r| r == cm).unwrap_or(0) as i32;
                        h.global::<AuraPageData>().set_current_available_mode(idx);
                    }
                    h.invoke_external_colour_change();
                }
            })
            .map_err(|e| error!("{e}"))
            .ok();

        if let Ok(mut pow3r) = aura.supported_power_zones().await {
            let dev = aura
                .device_type()
                .await
                .unwrap_or(AuraDeviceType::LaptopKeyboard2021);
            handle
                .upgrade_in_event_loop(move |handle| {
                    let names: Vec<SharedString> = handle
                        .global::<AuraPageData>()
                        .get_power_zone_names()
                        .iter()
                        .collect();
                    if dev.is_old_laptop() {
                        pow3r.retain(|z| *z != PowerZones::None);
                        let n: Vec<SharedString> =
                            pow3r.iter().map(|z| names[(*z) as usize].clone()).collect();
                        handle
                            .global::<AuraPageData>()
                            .set_power_zone_names_old(n.as_slice().into());
                    } else {
                        let p: Vec<SlintPowerZones> = pow3r
                            .iter()
                            .filter(|z| **z != PowerZones::None)
                            .map(|z| (*z).into())
                            .collect();
                        handle
                            .global::<AuraPageData>()
                            .set_supported_power_zones(p.as_slice().into());
                    }
                })
                .ok();
        }

        if let Ok(targets) = aura.supported_effect_targets().await {
            handle
                .upgrade_in_event_loop(move |handle| {
                    let labels: Vec<SharedString> = handle
                        .global::<AuraPageData>()
                        .get_zone_names()
                        .iter()
                        .collect();
                    let mut raws: Vec<i32> = Vec::new();
                    let mut names: Vec<SharedString> = Vec::new();
                    for zone in &targets {
                        let raw: i32 = (*zone).into();
                        if let Some(label) = labels.get(raw as usize) {
                            raws.push(raw);
                            names.push(label.clone());
                        }
                    }
                    // Enable when there is a real choice (synced + at least one
                    // chassis node beyond a redundant single-zone keyboard).
                    let enabled = raws.len() > 2
                        || raws.iter().any(|z| {
                            matches!(*z, 5..=7) // Logo / Lightbar / Lightbar Right
                        });
                    let idx = restore_zone
                        .and_then(|z| raws.iter().position(|&r| r == z))
                        .unwrap_or(0) as i32;
                    let g = handle.global::<AuraPageData>();
                    g.set_supported_effect_targets(raws.as_slice().into());
                    g.set_effect_target_names(names.as_slice().into());
                    g.set_effect_targets_enabled(enabled);
                    g.set_effect_target_index(idx);
                })
                .ok();
        }

        let proxy = aura.clone();
        let weak = handle.clone();
        handle
            .upgrade_in_event_loop(move |h| {
                set_ui_callbacks!(h,
                    AuraPageData(.into()),
                    proxy.brightness(.into()),
                    "Brightness set to {}",
                    "Brightness failed"
                );

                let p = proxy.clone();
                let w = weak.clone();
                h.global::<AuraPageData>().on_apply_led_mode_data(move || {
                    let Some(ui) = w.upgrade() else { return };
                    let g = ui.global::<AuraPageData>();
                    let mut slint_effect = g.get_led_mode_data();
                    // Zone dropdown is authoritative for chassis target.
                    slint_effect.zone = g.get_selected_effect_zone();
                    let raw: rog_aura::AuraEffect = slint_effect.into();
                    let pp = p.clone();
                    let t = w.clone();
                    tokio::spawn(async move {
                        let r = pp.set_led_mode_data(raw).await;
                        show_toast("LED mode applied".into(), "LED mode failed".into(), t, r);
                    });
                });

                // Zone dropdown only switches the editing target. Hardware is
                // written when colour / mode / speed / direction apply, using
                // selected_effect_zone (not a stale led_mode_data.zone).
                let w_zone = weak.clone();
                h.global::<AuraPageData>()
                    .on_cb_effect_target_index(move |idx| {
                        let Some(ui) = w_zone.upgrade() else { return };
                        let g = ui.global::<AuraPageData>();
                        let targets: Vec<i32> = g.get_supported_effect_targets().iter().collect();
                        let Some(&zone) = targets.get(idx as usize) else {
                            error!("Effect target index out of range: {idx}");
                            return;
                        };
                        g.set_effect_target_index(idx);
                        let mut effect = g.get_led_mode_data();
                        effect.zone = zone;
                        g.set_led_mode_data(effect);
                        g.set_zone(zone);
                    });

                h.invoke_external_colour_change();
            })
            .ok();

        let weak_power = handle.clone();
        let proxy_power = aura.clone();
        handle
            .upgrade_in_event_loop(|h| {
                h.global::<AuraPageData>().on_cb_led_power(move |power| {
                    let w = weak_power.clone();
                    let p = proxy_power.clone();
                    let pw: LaptopAuraPower = power.into();
                    tokio::spawn(async move {
                        show_toast(
                            "Aura power updated".into(),
                            "Aura power failed".into(),
                            w,
                            p.set_led_power(pw).await,
                        );
                    });
                });
            })
            .map_err(|e| error!("{e}"))
            .ok();

        let stream_handle = handle.clone();
        let aura_stream = aura.clone();
        tokio::spawn(async move {
            use futures_util::StreamExt;
            let mut stream = aura_stream.receive_led_mode_data_changed().await;
            while let Some(e) = stream.next().await {
                if let Ok(out) = e.get().await {
                    let raw: i32 = out.mode.into();
                    let data = out.into();
                    stream_handle
                        .upgrade_in_event_loop(move |h| {
                            let g = h.global::<AuraPageData>();
                            g.invoke_update_led_mode_data(data);
                            let supported: Vec<i32> =
                                g.get_supported_basic_modes().iter().collect();
                            let idx = supported.iter().position(|&x| x == raw).unwrap_or(0) as i32;
                            g.set_current_available_mode(idx);
                            let targets: Vec<i32> =
                                g.get_supported_effect_targets().iter().collect();
                            let zone = g.get_led_mode_data().zone;
                            let zidx = targets.iter().position(|&t| t == zone).unwrap_or(0) as i32;
                            g.set_effect_target_index(zidx);
                            h.invoke_external_colour_change();
                        })
                        .map_err(|e| error!("{e}"))
                        .ok();
                }
            }
        });
        debug!("Aura setup done");
        Ok(())
    });
}
