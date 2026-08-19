//! Handle UiUpdates

use slint::{ComponentHandle, SharedString};

use crate::{MainWindow, PowerData, SystemInfo, TelemetryData, state::UiUpdate};

pub fn apply_ui_update(ui: &MainWindow, update: UiUpdate) {
    match update {
        UiUpdate::Telemetry(t) => {
            let sys_data = ui.global::<TelemetryData>();
            // CPU
            sys_data.set_cpu_temp_val(t.cpu.temp);
            sys_data.set_cpu_freq_mhz(t.cpu.freq_mhz);
            sys_data.set_cpu_usage_val(t.cpu.usage_pct);
            // RAM
            sys_data.set_ram_usage_val(t.ram_usage_pct);
        }
        UiUpdate::ProductName(n) => {
            let sys_data = ui.global::<SystemInfo>();
            sys_data.set_product_name(SharedString::from(n));
        }
        UiUpdate::Battery(b) => {
            let sys_data = ui.global::<TelemetryData>();
            sys_data.set_battery_health(b.health as i32);
        }
        UiUpdate::PlatformProfile(p) => {
            let sys_data = ui.global::<PowerData>();
            sys_data.set_platform_profile(p);
        }
        UiUpdate::ShowToast {
            message,
            toast_type,
        } => {
            crate::ui::toast::show_toast(message.into(), toast_type, ui.as_weak());
        }
        UiUpdate::ShowPermanentToast {
            message,
            toast_type,
        } => {
            crate::ui::toast::show_permanent_toast(message.into(), toast_type, ui.as_weak());
        }
        UiUpdate::AsusdState(running) => {
            let sys = ui.global::<SystemInfo>();
            sys.set_asusd_running(running);
            if running {
                crate::ui::toast::show_toast(
                    "asusd connected".into(),
                    crate::state::ToastType::Info,
                    ui.as_weak(),
                );
            }
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
            let _ = slint::quit_event_loop();
        }
    }
}
