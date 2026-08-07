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
use crate::zbus_proxies::AppState;
use crate::{AppSettingsPageData, GlobalShortcutStatus, MainWindow};

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
            slint::invoke_from_event_loop(move || {
                if let Some(h) = handle.upgrade() {
                    h.invoke_show_toast(success);
                }
            })
            .ok();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if TOAST_SEQ.load(Ordering::SeqCst) == seq {
                    slint::invoke_from_event_loop(move || {
                        if let Some(h) = delayed_handle.upgrade() {
                            h.invoke_clear_toast_if_matches(delayed_text);
                        }
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
                if let Some(h) = handle.upgrade() {
                    h.invoke_show_toast(fail);
                }
            })
            .ok();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if TOAST_SEQ.load(Ordering::SeqCst) == seq {
                    slint::invoke_from_event_loop(move || {
                        if let Some(h) = delayed_handle.upgrade() {
                            h.invoke_clear_toast_if_matches(delayed_text);
                        }
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
    app_state: Arc<Mutex<AppState>>,
    is_tuf: bool,
    shortcuts: Option<ShortcutHandle>,
) -> MainWindow {
    slint::set_xdg_app_id(crate::APP_ID)
        .map_err(|e| warn!("Couldn't set application ID: {e:?}"))
        .ok();
    let ui = MainWindow::new().expect("Couldn't create main window");
    // propagate TUF flag to the UI so the sidebar can swap logo branding
    ui.set_is_tuf(is_tuf);
    ui.set_app_version(env!("CARGO_PKG_VERSION").into());
    if let Err(e) = ui.window().show() {
        warn!("Couldn't show main window: {e:?}");
    }

    let available = list_iface_blocking().unwrap_or_default();
    ui.set_sidebar_items_avilable(
        [
            // Needs to match the order of slint sidebar items
            true,                                                   // Home (landing page, degrades gracefully)
            available.contains(&"xyz.ljones.Platform".to_string()), // System Tuning (power limits)
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

    setup_app_settings_page(&ui, config.clone(), shortcuts);
    if available.contains(&"xyz.ljones.Platform".to_string()) {
        setup_system_page(&ui, config.clone(), app_state.clone());
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

/// Locale codes that have a translation on disk: the source `translations/`
/// tree (every subdir is ours) plus any installed under `/usr/share/locale`
/// that actually ships our catalog. Sorted + deduped, with an "en" fallback so
/// the picker is never empty. The locale *list* is automatic; the native
/// display names live in `language_display_name` (add a line there when a new
/// translation lands — until then it shows the raw code).
fn available_languages() -> Vec<SharedString> {
    let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // English is the source language (no .mo needed) — always present, so a
    // fresh config never lands on a translation by default.
    set.insert("en".to_owned());
    // Dev builds: scan the source tree translations dir (harmless on installed
    // builds — the dir won't exist, so read_dir returns Err and is skipped).
    let dev = concat!(env!("CARGO_MANIFEST_DIR"), "/translations");
    for dir in [
        dev, "/usr/share/locale",
    ] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                // /usr/share/locale holds every app's locales, so only count
                // dirs carrying our catalog; the source tree is all ours.
                let ours = dir == dev || path.join("LC_MESSAGES/rog-control-center.mo").exists();
                if ours {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        set.insert(name.to_string());
                    }
                }
            }
        }
    }
    let mut v: Vec<SharedString> = set.into_iter().map(SharedString::from).collect();
    if v.is_empty() {
        v.push(SharedString::from("en"));
    }
    v
}

/// Native display name for a locale code — language pickers conventionally
/// show each language in its own tongue. Unknown codes fall back to themselves
/// so a newly added translation still shows up (as its code) until named here.
fn language_display_name(code: &str) -> SharedString {
    let name = match code {
        "en" => "English",
        "zh_CN" => "简体中文",
        "fr" => "Français",
        "it" => "Italiano",
        "ru" => "Русский",
        "tr" => "Türkçe",
        "uk_UA" => "Українська",
        "pt_BR" => "Português (Brasil)",
        "az" => "Azərbaycanca",
        other => other,
    };
    SharedString::from(name)
}

pub fn setup_app_settings_page(
    ui: &MainWindow,
    config: Arc<Mutex<Config>>,
    shortcuts: Option<ShortcutHandle>,
) {
    let config_copy = config.clone();
    let global = ui.global::<AppSettingsPageData>();
    global.on_set_run_in_background(move |enable| match config_copy.lock() {
        Ok(mut lock) => {
            lock.run_in_background = enable;
            lock.write();
        }
        Err(err) => error!("Could not save setting: {err}"),
    });
    let config_copy = config.clone();
    global.on_set_startup_in_background(move |enable| match config_copy.lock() {
        Ok(mut lock) => {
            lock.startup_in_background = enable;
            lock.write();
        }
        Err(err) => error!("Could not save setting: {err}"),
    });
    let config_copy = config.clone();
    global.on_set_enable_tray_icon(move |enable| match config_copy.lock() {
        Ok(mut lock) => {
            lock.enable_tray_icon = enable;
            lock.write();
        }
        Err(err) => error!("Could not save setting: {err}"),
    });
    let config_copy = config.clone();
    global.on_set_enable_dgpu_notifications(move |enable| match config_copy.lock() {
        Ok(mut lock) => {
            lock.notifications.enabled = enable;
            lock.write();
        }
        Err(err) => error!("Could not save setting: {err}"),
    });
    let config_copy = config.clone();
    global.on_set_enable_autostart(move |enable| match config_copy.lock() {
        Ok(mut lock) => {
            lock.enable_autostart = enable;
            let in_bg = super::config::is_autostart_in_background();
            lock.write();
            super::config::update_autostart(enable, in_bg);
        }
        Err(err) => error!("Could not save setting: {err}"),
    });
    let config_copy = config.clone();
    global.on_set_autostart_in_background(move |enable| match config_copy.lock() {
        Ok(lock) => {
            let autostart = lock.enable_autostart;
            super::config::update_autostart(autostart, enable);
        }
        Err(err) => error!("Could not read setting: {err}"),
    });

    match config.lock() {
        Ok(lock) => {
            global.set_run_in_background(lock.run_in_background);
            global.set_startup_in_background(lock.startup_in_background);
            global.set_enable_tray_icon(lock.enable_tray_icon);
            global.set_enable_dgpu_notifications(lock.notifications.enabled);
            global.set_enable_autostart(lock.enable_autostart);
            global.set_autostart_in_background(super::config::is_autostart_in_background());
        }
        Err(err) => error!("Could not read config: {err}"),
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

    // Discover shipped translations at startup so the picker lists every
    // language without a hardcoded array.
    let codes = available_languages();
    let configured_language = match config.lock() {
        Ok(lock) => lock.language.clone(),
        Err(err) => {
            error!("Could not read config: {err}");
            String::default()
        }
    };
    // When no language is explicitly configured, main.rs leaves the system
    // locale in place and gettext renders whatever LANG/LC_ALL says. Mirror
    // that locale here so the picker doesn't show "English" while the UI is
    // actually (e.g.) Chinese on a fresh install.
    let effective_language = if configured_language.is_empty() {
        std::env::var("LANGUAGE")
            .ok()
            .and_then(|s| s.split(':').next().map(str::to_string))
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("LC_ALL").ok().filter(|s| !s.is_empty()))
            .or_else(|| {
                std::env::var("LANG")
                    .ok()
                    .filter(|s| !s.is_empty() && s != "C" && s != "POSIX")
            })
            .map(|raw| {
                let base = raw.split('.').next().unwrap_or(&raw);
                base.replace("_Hans", "").replace("_Hant", "")
            })
            .unwrap_or_else(|| "en".to_string())
    } else {
        configured_language
    };
    // Match the effective language; fall back to "en" (source language), then
    // index 0 — so a stale config like the old "en_US" still lands on English.
    let current_idx = codes
        .iter()
        .position(|l| l.as_str() == effective_language.as_str())
        .or_else(|| codes.iter().position(|l| l.as_str() == "en"))
        .unwrap_or_else(|| {
            log::warn!("No matching language found in available list; defaulting to index 0");
            0
        }) as i32;
    // The picker shows each language in its own name (standard for language
    // selectors); the raw code is what gets persisted, so keep both in lockstep.
    let display: Vec<SharedString> = codes
        .iter()
        .map(|c| language_display_name(c.as_str()))
        .collect();
    global.set_available_languages(slint::ModelRc::new(slint::VecModel::from(display)));
    global.set_current_language(current_idx);

    let config_copy = config.clone();
    global.on_cb_change_language(move |index: i32| {
        if let Some(code) = codes.get(index as usize) {
            match config_copy.lock() {
                Ok(mut lock) => {
                    lock.language = code.to_string();
                    lock.write();
                    log::info!("Language changed to {code}; reload to apply");
                }
                Err(err) => error!("Could not save language setting: {err}"),
            }
        }
    });

    // Reload Window: spawn a fresh instance flagged to skip the single-instance
    // guard (--no-single-instance), then quit this one. spawn+quit is
    // reliable where exec() was not: the old DBus name is released on quit and
    // the new image never races the check. The child re-reads config.language
    // and re-resolves @tr() in the chosen locale.
    global.on_cb_reload_window(move || {
        let exe = match std::env::current_exe() {
            Ok(e) => e,
            Err(e) => {
                log::error!("reload: cannot resolve current exe: {e}");
                return;
            }
        };
        log::info!("reload: spawning {:?}", exe);
        match std::process::Command::new(exe)
            .arg("--no-single-instance")
            .spawn()
        {
            Ok(_) => {
                slint::quit_event_loop()
                    .unwrap_or_else(|e| log::error!("reload: quit_event_loop: {e}"));
            }
            Err(e) => log::error!("reload: spawn failed: {e}"),
        }
    });
}
