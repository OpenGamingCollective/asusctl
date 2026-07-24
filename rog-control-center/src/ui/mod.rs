pub mod setup_anime;
pub mod setup_aura;
pub mod setup_fans;
pub mod setup_gpu;
pub mod setup_slash;
pub mod setup_system;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

static TOAST_SEQ: AtomicU64 = AtomicU64::new(0);

use config_traits::StdConfig;
use log::{error, warn};
use rog_dbus::list_iface_blocking;
use slint::{ComponentHandle, SharedString, Weak};

use crate::config::Config;
use crate::shortcuts::{EnableMode, ShortcutHandle, ShortcutStatus};
use crate::ui::setup_anime::setup_anime_page;
use crate::ui::setup_aura::setup_aura_page;
use crate::ui::setup_fans::setup_fan_curve_page;
use crate::ui::setup_slash::setup_slash_page;
use crate::ui::setup_system::{setup_system_page, setup_system_page_callbacks};
use crate::{AppSettingsPageData, GlobalShortcutStatus, MainWindow, Theme};

// this macro sets up:
// - a link from UI callback -> dbus proxy property
// - a link from dbus property signal -> UI state
// conv1 and conv2 are type conversion args
#[macro_export]
macro_rules! set_ui_callbacks {
    ($handle:ident, $data:ident($($conv1: tt)*),$proxy:ident.$proxy_fn:tt($($conv2: tt)*),$success:literal,$failed:literal) => {
        let handle_copy = $handle.as_weak();
        let proxy_copy = $proxy.clone();
        let data = $handle.global::<$data>();
        concat_idents::concat_idents!(on_set = on_cb_, $proxy_fn {
        data.on_set(move |value| {
            let proxy_copy = proxy_copy.clone();
            let handle_copy = handle_copy.clone();
            tokio::spawn(async move {
                concat_idents::concat_idents!(set = set_, $proxy_fn {
                show_toast(
                    format!($success, value).into(),
                    $failed.into(),
                    handle_copy,
                    proxy_copy.set(value $($conv2)*).await,
                );
                });
            });
            });
        });
        let handle_copy = $handle.as_weak();
        let proxy_copy = $proxy.clone();
        concat_idents::concat_idents!(receive = receive_, $proxy_fn, _changed {
        // spawn required since the while let never exits
        tokio::spawn(async move {
            let mut x = proxy_copy.receive().await;
            concat_idents::concat_idents!(set = set_, $proxy_fn {
            use futures_util::StreamExt;
            while let Some(e) = x.next().await {
                if let Ok(out) = e.get().await {
                    handle_copy.upgrade_in_event_loop(move |handle| {
                        handle.global::<$data>().set(out $($conv1)*);
                    }).ok();
                }
            }
            });
        });
        });
    };
}

pub fn show_toast(
    success: SharedString,
    fail: SharedString,
    handle: Weak<MainWindow>,
    result: zbus::Result<()>,
) {
    // bump sequence so that any previously spawned timers won't clear newer toasts
    let seq = TOAST_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    match result {
        Ok(_) => {
            let delayed_handle = handle.clone();
            let delayed_text = success.clone();
            slint::invoke_from_event_loop(move || handle.unwrap().invoke_show_toast(success)).ok();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if TOAST_SEQ.load(Ordering::SeqCst) == seq {
                    slint::invoke_from_event_loop(move || {
                        delayed_handle
                            .unwrap()
                            .invoke_clear_toast_if_matches(delayed_text)
                    })
                    .ok();
                }
            });
        }
        Err(e) => {
            let delayed_handle = handle.clone();
            let delayed_text = fail.clone();
            slint::invoke_from_event_loop(move || {
                log::warn!("{fail}: {e}");
                handle.unwrap().invoke_show_toast(fail)
            })
            .ok();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if TOAST_SEQ.load(Ordering::SeqCst) == seq {
                    slint::invoke_from_event_loop(move || {
                        delayed_handle
                            .unwrap()
                            .invoke_clear_toast_if_matches(delayed_text)
                    })
                    .ok();
                }
            });
        }
    };
}

pub fn setup_window(
    config: Arc<Mutex<Config>>,
    prefetched_supported: std::sync::Arc<Option<Vec<i32>>>,
    is_tuf: bool,
    shortcuts: Option<ShortcutHandle>,
) -> MainWindow {
    slint::set_xdg_app_id(crate::APP_ID)
        .map_err(|e| warn!("Couldn't set application ID: {e:?}"))
        .ok();
    let ui = MainWindow::new()
        .map_err(|e| warn!("Couldn't create main window: {e:?}"))
        .unwrap();
    // propagate TUF flag to the UI so the sidebar can swap logo branding
    ui.set_is_tuf(is_tuf);
    ui.window()
        .show()
        .map_err(|e| warn!("Couldn't show main window: {e:?}"))
        .unwrap();

    let available = list_iface_blocking().unwrap_or_default();
    ui.set_sidebar_items_avilable(
        [
            // Needs to match the order of slint sidebar items
            available.contains(&"xyz.ljones.Platform".to_string()),
            available.contains(&"xyz.ljones.Aura".to_string()),
            available.contains(&"xyz.ljones.Anime".to_string()),
            available.contains(&"xyz.ljones.Slash".to_string()),
            available.contains(&"xyz.ljones.FanCurves".to_string()),
            true,                                                   // GPU Configuration
            available.contains(&"xyz.ljones.Platform".to_string()), // Battery Info
            true,                                                   // App Settings
            true,                                                   // About
        ]
        .into(),
    );

    ui.on_exit_app(move || {
        slint::quit_event_loop().unwrap();
    });

    setup_app_settings_page(&ui, config.clone(), shortcuts);
    if available.contains(&"xyz.ljones.Platform".to_string()) {
        setup_system_page(&ui, config.clone());
        setup_system_page_callbacks(&ui, config.clone());
    }
    if available.contains(&"xyz.ljones.Aura".to_string()) {
        setup_aura_page(&ui, config.clone(), prefetched_supported.as_ref().clone());
    }
    if available.contains(&"xyz.ljones.Anime".to_string()) {
        setup_anime_page(&ui, config.clone());
    }
    if available.contains(&"xyz.ljones.Slash".to_string()) {
        setup_slash_page(&ui, config.clone());
    }
    if available.contains(&"xyz.ljones.FanCurves".to_string()) {
        setup_fan_curve_page(&ui, config.clone());
    }

    // Populate GPU page choices and callbacks
    setup_gpu::setup_gpu_page(&ui);

    ui
}

fn ui_shortcut_status(status: ShortcutStatus) -> GlobalShortcutStatus {
    match status {
        ShortcutStatus::Disabled => GlobalShortcutStatus::Disabled,
        ShortcutStatus::Starting => GlobalShortcutStatus::Starting,
        ShortcutStatus::Unassigned => GlobalShortcutStatus::Unassigned,
        ShortcutStatus::Listening => GlobalShortcutStatus::Listening,
        ShortcutStatus::Unavailable => GlobalShortcutStatus::Unavailable,
    }
}

fn parse_color_hex(hex: &str) -> Option<slint::Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(slint::Color::from_argb_u8(255, r, g, b))
    } else {
        None
    }
}

pub fn setup_app_settings_page(
    ui: &MainWindow,
    config: Arc<Mutex<Config>>,
    shortcuts: Option<ShortcutHandle>,
) {
    let config_copy = config.clone();
    let global = ui.global::<AppSettingsPageData>();
    global.on_set_run_in_background(move |enable| {
        if let Ok(mut lock) = config_copy.try_lock() {
            lock.run_in_background = enable;
            lock.write();
        }
    });
    let config_copy = config.clone();
    global.on_set_startup_in_background(move |enable| {
        if let Ok(mut lock) = config_copy.try_lock() {
            lock.startup_in_background = enable;
            lock.write();
        }
    });
    let config_copy = config.clone();
    global.on_set_enable_tray_icon(move |enable| {
        if let Ok(mut lock) = config_copy.try_lock() {
            lock.enable_tray_icon = enable;
            lock.write();
        }
    });
    let config_copy = config.clone();
    global.on_set_enable_dgpu_notifications(move |enable| {
        if let Ok(mut lock) = config_copy.try_lock() {
            lock.notifications.enabled = enable;
            lock.write();
        }
    });
    let config_copy = config.clone();
    global.on_set_enable_autostart(move |enable| {
        if let Ok(mut lock) = config_copy.try_lock() {
            lock.enable_autostart = enable;
            let in_bg = super::config::is_autostart_in_background();
            lock.write();
            super::config::update_autostart(enable, in_bg);
        }
    });
    let config_copy = config.clone();
    global.on_set_autostart_in_background(move |enable| {
        if let Ok(lock) = config_copy.try_lock() {
            let autostart = lock.enable_autostart;
            super::config::update_autostart(autostart, enable);
        }
    });
    let config_copy = config.clone();
    let ui_weak = ui.as_weak();
    global.on_set_theme_preset(move |preset| {
        let preset = preset.clamp(0, 4);
        if let Ok(mut lock) = config_copy.try_lock() {
            lock.theme_preset = preset;
            lock.write();
        }
        if let Some(ui) = ui_weak.upgrade() {
            ui.global::<Theme>().set_theme_preset(preset);
        }
    });
    let config_copy = config.clone();
    let ui_weak = ui.as_weak();
    global.on_set_custom_accent_hex(move |hex| {
        if let Ok(mut lock) = config_copy.try_lock() {
            lock.custom_accent_hex = hex.to_string();
            lock.write();
        }
        if let Some(ui) = ui_weak.upgrade() {
            if let Some(color) = parse_color_hex(&hex) {
                ui.global::<Theme>().set_custom_accent_color(color);
            }
        }
    });
    let config_copy = config.clone();
    let ui_weak = ui.as_weak();
    global.on_set_custom_secondary_hex(move |hex| {
        if let Ok(mut lock) = config_copy.try_lock() {
            lock.custom_secondary_hex = hex.to_string();
            lock.write();
        }
        if let Some(ui) = ui_weak.upgrade() {
            if let Some(color) = parse_color_hex(&hex) {
                ui.global::<Theme>().set_custom_secondary_color(color);
            }
        }
    });

    if let Ok(lock) = config.try_lock() {
        global.set_run_in_background(lock.run_in_background);
        global.set_startup_in_background(lock.startup_in_background);
        global.set_enable_tray_icon(lock.enable_tray_icon);
        global.set_enable_dgpu_notifications(lock.notifications.enabled);
        global.set_enable_autostart(lock.enable_autostart);
        global.set_autostart_in_background(super::config::is_autostart_in_background());
        let preset = lock.theme_preset.clamp(0, 4);
        global.set_theme_preset(preset);
        global.set_custom_accent_hex(lock.custom_accent_hex.clone().into());
        global.set_custom_secondary_hex(lock.custom_secondary_hex.clone().into());
        ui.global::<Theme>().set_theme_preset(preset);
        if let Some(color) = parse_color_hex(&lock.custom_accent_hex) {
            ui.global::<Theme>().set_custom_accent_color(color);
        }
        if let Some(color) = parse_color_hex(&lock.custom_secondary_hex) {
            ui.global::<Theme>().set_custom_secondary_color(color);
        }
    }

    global.set_show_global_shortcut_controls(shortcuts.is_some());
    if let Some(handle) = shortcuts {
        match config.lock() {
            Ok(lock) => global.set_enable_global_shortcut(lock.enable_global_shortcut),
            Err(err) => error!("Could not read config for global shortcut setting: {err}"),
        }

        // Subscribe before reading the current value so no transition is
        // lost between the two.
        let mut statuses = handle.status_receiver();
        let initial_status = *statuses.borrow_and_update();
        global.set_global_shortcut_status(ui_shortcut_status(initial_status));
        global.set_global_shortcut_configurable(handle.can_configure());

        let status_handle = handle.clone();
        let weak = ui.as_weak();
        tokio::spawn(async move {
            while statuses.changed().await.is_ok() {
                let status = *statuses.borrow();
                let configurable = status_handle.can_configure();
                weak.upgrade_in_event_loop(move |ui| {
                    let global = ui.global::<AppSettingsPageData>();
                    global.set_global_shortcut_status(ui_shortcut_status(status));
                    global.set_global_shortcut_configurable(configurable);
                })
                .ok();
            }
        });

        let toggle_handle = handle.clone();
        let config_copy = config.clone();
        let weak = ui.as_weak();
        global.on_set_enable_global_shortcut(move |enable| {
            if enable {
                match config_copy.lock() {
                    Ok(mut lock) => {
                        lock.enable_global_shortcut = true;
                        lock.write();
                    }
                    Err(err) => error!("Could not save global shortcut setting: {err}"),
                }
                let handle = toggle_handle.clone();
                let config = config_copy.clone();
                let weak = weak.clone();
                tokio::spawn(async move {
                    let status = handle.enable(EnableMode::Interactive).await;
                    if status == ShortcutStatus::Unassigned {
                        // The user cancelled the first-time bind dialog, so
                        // the feature can do nothing: revert the intent.
                        match config.lock() {
                            Ok(mut lock) => {
                                lock.enable_global_shortcut = false;
                                lock.write();
                            }
                            Err(err) => {
                                error!("Could not revert global shortcut setting: {err}")
                            }
                        }
                        handle.disable().await;
                        weak.upgrade_in_event_loop(|ui| {
                            ui.global::<AppSettingsPageData>()
                                .set_enable_global_shortcut(false);
                        })
                        .ok();
                    }
                    // `Unavailable` keeps the config: the failure may be
                    // temporary and the next startup will retry the restore.
                });
            } else {
                match config_copy.lock() {
                    Ok(mut lock) => {
                        lock.enable_global_shortcut = false;
                        lock.write();
                    }
                    Err(err) => error!("Could not save global shortcut setting: {err}"),
                }
                let handle = toggle_handle.clone();
                tokio::spawn(async move {
                    handle.disable().await;
                });
            }
        });

        global.on_manage_global_shortcut(move || {
            let handle = handle.clone();
            tokio::spawn(async move {
                match handle.status() {
                    // First use opens the Bind dialog; an existing but
                    // unassigned shortcut opens Configure (portal v2) via
                    // the actor's interactive enable flow.
                    ShortcutStatus::Unassigned => {
                        handle.enable(EnableMode::Interactive).await;
                    }
                    // Reconfigure an active shortcut in the desktop's own
                    // shortcut settings.
                    ShortcutStatus::Listening => {
                        handle.configure().await;
                    }
                    _ => {}
                }
            });
        });
    }
}
