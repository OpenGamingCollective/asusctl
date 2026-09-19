//! Handle UiUpdates

use log::info;
use rog_platform::asus_armoury::FirmwareAttribute;
use slint::{ComponentHandle, SharedString};

use crate::{
    AsusArmouryData, AttrBool, AttrMinMax, MainWindow, PowerData, SystemInfo, TelemetryData,
    state::UiUpdate,
};

/// Convert a raw firmware attribute value into a bool
fn attr_i32_into_bool(val: AttrMinMax) -> AttrBool {
    AttrBool {
        current: val.current == 1.0,
        supported: val.supported,
    }
}

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
        UiUpdate::PlatformProfile(p) => {
            let sys_data = ui.global::<PowerData>();
            sys_data.set_platform_profile(p);
        }
        UiUpdate::FirmwareAttr(attr, v) => {
            let dev_data = ui.global::<AsusArmouryData>();
            match attr {
                FirmwareAttribute::ApuMem => dev_data.set_apu_mem(v),
                FirmwareAttribute::CoresPerformance => dev_data.set_cores_performance(v),
                FirmwareAttribute::CoresEfficiency => dev_data.set_cores_efficiency(v),
                FirmwareAttribute::PptPl1Spl => dev_data.set_ppt_pl1_spl(v),
                FirmwareAttribute::PptPl2Sppt => dev_data.set_ppt_pl2_sppt(v),
                FirmwareAttribute::PptPl3Fppt => dev_data.set_ppt_pl3_fppt(v),
                FirmwareAttribute::PptFppt => dev_data.set_ppt_fppt(v),
                FirmwareAttribute::PptApuSppt => dev_data.set_ppt_apu_sppt(v),
                FirmwareAttribute::PptPlatformSppt => dev_data.set_ppt_platform_sppt(v),
                FirmwareAttribute::NvDynamicBoost => dev_data.set_nv_dynamic_boost(v),
                FirmwareAttribute::NvTempTarget => dev_data.set_nv_temp_target(v),
                FirmwareAttribute::DgpuBaseTgp => dev_data.set_dgpu_base_tgp(v),
                FirmwareAttribute::DgpuTgp => dev_data.set_dgpu_tgp(v),
                FirmwareAttribute::ChargeMode => dev_data.set_charge_mode(v),
                FirmwareAttribute::BootSound => dev_data.set_boot_sound(attr_i32_into_bool(v)),
                FirmwareAttribute::McuPowersave => {
                    dev_data.set_mcu_powersave(attr_i32_into_bool(v))
                }
                FirmwareAttribute::PanelOverdrive => {
                    dev_data.set_panel_overdrive(attr_i32_into_bool(v))
                }
                FirmwareAttribute::PanelHdMode => dev_data.set_panel_hd_mode(v),
                FirmwareAttribute::EgpuConnected => {
                    dev_data.set_egpu_connected(attr_i32_into_bool(v))
                }
                FirmwareAttribute::EgpuEnable => dev_data.set_egpu_enable(attr_i32_into_bool(v)),
                FirmwareAttribute::DgpuDisable => dev_data.set_dgpu_disable(attr_i32_into_bool(v)),
                FirmwareAttribute::GpuMuxMode => dev_data.set_gpu_mux_mode(attr_i32_into_bool(v)),
                FirmwareAttribute::MiniLedMode => dev_data.set_mini_led_mode(v),
                FirmwareAttribute::PendingReboot => {
                    dev_data.set_pending_reboot(attr_i32_into_bool(v))
                }
                FirmwareAttribute::ScreenAutoBrightness => {
                    dev_data.set_screen_auto_brightness(attr_i32_into_bool(v))
                }
                FirmwareAttribute::None => {}
            }
        }
        UiUpdate::PPT(b) => {
            let armoury_data = ui.global::<AsusArmouryData>();
            let current = armoury_data.get_ppt_enabled();
            armoury_data.set_ppt_enabled(AttrBool {
                current: b,
                supported: current.supported,
            });
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
        _ => {
            info!("uiupdate not implemented yet: {:?}", update);
        }
    }
}
