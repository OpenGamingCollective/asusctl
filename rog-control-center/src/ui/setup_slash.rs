use std::str::FromStr;
use std::sync::{Arc, Mutex};

use log::{error, info};
use rog_dbus::find_iface_async;
use rog_dbus::zbus_slash::SlashProxy;
use rog_slash::SlashMode;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::config::Config;
use crate::ui::show_toast;
use crate::{MainWindow, SlashPageData, set_ui_callbacks, set_ui_props_async};

fn slash_modes() -> Vec<SharedString> {
    SlashMode::list()
        .into_iter()
        .map(SharedString::from)
        .collect()
}

fn slash_mode_to_index(mode: SlashMode) -> i32 {
    SlashMode::list()
        .iter()
        .position(|value| value == &mode.to_string())
        .map(|index| index as i32)
        .unwrap_or_default()
}

fn slash_mode_from_index(index: i32) -> SlashMode {
    let modes = SlashMode::list();
    let selected = modes
        .get(index.max(0) as usize)
        .cloned()
        .unwrap_or_else(|| SlashMode::default().to_string());
    SlashMode::from_str(&selected).unwrap_or_default()
}

pub fn setup_slash_page(ui: &MainWindow, _states: Arc<Mutex<Config>>) {
    let handle = ui.as_weak();
    tokio::spawn(async move {
        let Ok(slashes) = find_iface_async::<SlashProxy>("xyz.ljones.Slash").await else {
            info!("This device appears to have no slash interface");
            return;
        };

        for slash in slashes {
            set_ui_props_async!(handle, slash, SlashPageData, enabled);
            set_ui_props_async!(handle, slash, SlashPageData, brightness);
            set_ui_props_async!(handle, slash, SlashPageData, interval);
            set_ui_props_async!(handle, slash, SlashPageData, show_on_boot);
            set_ui_props_async!(handle, slash, SlashPageData, show_on_shutdown);
            set_ui_props_async!(handle, slash, SlashPageData, show_on_sleep);
            set_ui_props_async!(handle, slash, SlashPageData, show_on_battery);
            set_ui_props_async!(handle, slash, SlashPageData, show_battery_warning);
            set_ui_props_async!(handle, slash, SlashPageData, show_on_lid_closed);

            if let Ok(mode) = slash.mode().await {
                let idx = slash_mode_to_index(mode);
                let choices = slash_modes();
                handle
                    .upgrade_in_event_loop(move |handle| {
                        let global = handle.global::<SlashPageData>();
                        global.set_mode_choices(ModelRc::new(VecModel::from(choices)));
                        global.set_mode(idx);
                    })
                    .ok();
            }

            handle
                .upgrade_in_event_loop(move |handle| {
                    let global = handle.global::<SlashPageData>();
                    if global.get_mode_choices().row_count() == 0 {
                        global.set_mode_choices(ModelRc::new(VecModel::from(slash_modes())));
                    }

                    let handle_copy = handle.as_weak();
                    let slash_copy = slash.clone();
                    global.on_cb_mode(move |index| {
                        let handle_copy = handle_copy.clone();
                        let slash_copy = slash_copy.clone();
                        tokio::spawn(async move {
                            show_toast(
                                format!(
                                    "Slash animation successfully set to {}",
                                    slash_mode_from_index(index)
                                )
                                .into(),
                                "Setting Slash animation failed".into(),
                                handle_copy,
                                slash_copy.set_mode(slash_mode_from_index(index)).await,
                            );
                        });
                    });

                    let handle_copy = handle.as_weak();
                    let slash_copy = slash.clone();
                    tokio::spawn(async move {
                        let mut x = slash_copy.receive_mode_changed().await;
                        use futures_util::StreamExt;
                        while let Some(e) = x.next().await {
                            if let Ok(out) = e.get().await {
                                let idx = slash_mode_to_index(out);
                                handle_copy
                                    .upgrade_in_event_loop(move |handle| {
                                        handle.global::<SlashPageData>().set_mode(idx);
                                    })
                                    .ok();
                            }
                        }
                    });

                    set_ui_callbacks!(
                        handle,
                        SlashPageData(),
                        slash.enabled(),
                        "Slash lighting successfully set to {}",
                        "Setting Slash lighting failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        SlashPageData(.into()),
                        slash.brightness(.try_into().unwrap_or_default()),
                        "Slash brightness successfully set to {}",
                        "Setting Slash brightness failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        SlashPageData(.into()),
                        slash.interval(.try_into().unwrap_or_default()),
                        "Slash interval successfully set to {}",
                        "Setting Slash interval failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        SlashPageData(),
                        slash.show_on_boot(),
                        "Slash boot animation visibility successfully set to {}",
                        "Setting Slash boot animation visibility failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        SlashPageData(),
                        slash.show_on_shutdown(),
                        "Slash shutdown animation visibility successfully set to {}",
                        "Setting Slash shutdown animation visibility failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        SlashPageData(),
                        slash.show_on_sleep(),
                        "Slash sleep animation visibility successfully set to {}",
                        "Setting Slash sleep animation visibility failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        SlashPageData(),
                        slash.show_on_battery(),
                        "Slash battery animation visibility successfully set to {}",
                        "Setting Slash battery animation visibility failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        SlashPageData(),
                        slash.show_battery_warning(),
                        "Slash battery warning successfully set to {}",
                        "Setting Slash battery warning failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        SlashPageData(),
                        slash.show_on_lid_closed(),
                        "Slash lid-closed animation visibility successfully set to {}",
                        "Setting Slash lid-closed animation visibility failed"
                    );
                })
                .map_err(|e| error!("setup_slash_page: upgrade_in_event_loop: {e:?}"))
                .ok();
        }
    });
}
