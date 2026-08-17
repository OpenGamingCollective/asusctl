//! Handle UiUpdates

use slint::{ComponentHandle, SharedString};

use crate::{MainWindow, SystemPageData, state::UiUpdate};

pub fn apply_ui_update(ui: &MainWindow, update: UiUpdate) {
    match update {
        UiUpdate::Telemetry(t) => {
            let sys_data = ui.global::<SystemPageData>();
            sys_data.set_cpu_temp_val(t.cpu.temp);
        }
        UiUpdate::ProductName(n) => {
            let sys_data = ui.global::<SystemPageData>();
            sys_data.set_product_name(SharedString::from(n));
        }
        UiUpdate::Battery(b) => {
            let sys_data = ui.global::<SystemPageData>();
            sys_data.set_battery_health(b.health as i32);
        }
        UiUpdate::PlatformProfile(p) => {
            let sys_data = ui.global::<SystemPageData>();
            sys_data.set_platform_profile(p);
        }
        UiUpdate::ShowToast {
            message,
            is_error: _,
        } => {
            crate::ui::toast::show_toast(message.into(), ui.as_weak());
        }
        UiUpdate::ToggleWindow => {
            if ui.window().is_visible() {
                let _ = ui.window().hide();
            } else {
                let _ = ui.window().show();
            }
        }
        UiUpdate::ShowWindow => {
            let _ = ui.window().show();
        }
        UiUpdate::HideWindow => {
            let _ = ui.window().hide();
        }
        UiUpdate::Quit => {
            std::process::exit(0);
        }
    }
}
