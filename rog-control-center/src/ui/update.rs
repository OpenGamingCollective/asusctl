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

/// raw firmware attribute value for the Slint property setter
macro_rules! firmware_attr_value {
    (MinMax, $val:ident) => {
        $val
    };
    (Bool, $val:ident) => {
        attr_i32_into_bool($val)
    };
}

/// List of asus-armoury firmware attributes exposed to the UI
macro_rules! armoury_attrs {
    ($apply:ident($($args:tt)*)) => {
        $apply!($($args)*
            (ApuMem, apu_mem, MinMax),
            (CoresPerformance, cores_performance, MinMax),
            (CoresEfficiency, cores_efficiency, MinMax),
            (PptPl1Spl, ppt_pl1_spl, MinMax),
            (PptPl2Sppt, ppt_pl2_sppt, MinMax),
            (PptPl3Fppt, ppt_pl3_fppt, MinMax),
            (PptFppt, ppt_fppt, MinMax),
            (PptApuSppt, ppt_apu_sppt, MinMax),
            (PptPlatformSppt, ppt_platform_sppt, MinMax),
            (NvDynamicBoost, nv_dynamic_boost, MinMax),
            (NvTempTarget, nv_temp_target, MinMax),
            (DgpuBaseTgp, dgpu_base_tgp, MinMax),
            (DgpuTgp, dgpu_tgp, MinMax),
            (ChargeMode, charge_mode, MinMax),
            (BootSound, boot_sound, Bool),
            (McuPowersave, mcu_powersave, Bool),
            (PanelOverdrive, panel_overdrive, Bool),
            (PanelHdMode, panel_hd_mode, MinMax),
            (EgpuConnected, egpu_connected, Bool),
            (EgpuEnable, egpu_enable, Bool),
            (DgpuDisable, dgpu_disable, Bool),
            (GpuMuxMode, gpu_mux_mode, Bool),
            (MiniLedMode, mini_led_mode, MinMax),
            (PendingReboot, pending_reboot, Bool),
            (ScreenAutoBrightness, screen_auto_brightness, Bool),
        );
    };
}
pub(crate) use armoury_attrs;

/// Generate the `UiUpdate::FirmwareAttr` setter arms from the attribute table
macro_rules! firmware_attr_setters {
    ($dev_data:ident, $attr:ident, $val:ident; $( ($variant:ident, $prop:ident, $kind:ident) ),+ $(,)?) => {
        match $attr {
            $(
                FirmwareAttribute::$variant => {
                    concat_idents::concat_idents!(setter = set_, $prop {
                        $dev_data.setter(firmware_attr_value!($kind, $val));
                    });
                }
            )+
            // Others
            _ => {}
        }
    };
}

pub fn apply_ui_update(ui: &MainWindow, update: UiUpdate) {
    match update {
        UiUpdate::Telemetry(t) => {
            let sys_data = ui.global::<TelemetryData>();
            // CPU
            sys_data.set_cpu_temp_val(t.cpu.temp);
            sys_data.set_cpu_freq_mhz(t.cpu.freq_mhz);
            sys_data.set_cpu_usage_val(t.cpu.usage_pct);
            // dGPU
            sys_data.set_gpu_temp_val(t.dgpu.temp);
            sys_data.set_gpu_freq_mhz(t.dgpu.freq_mhz);
            sys_data.set_gpu_usage_val(t.dgpu.usage_pct);
            sys_data.set_dgpu_suspended(t.dgpu.suspended);
            // iGPU
            sys_data.set_igpu_temp_val(t.igpu_temp);
            sys_data.set_igpu_usage_val(t.igpu_usage);
            // RAM
            sys_data.set_ram_usage_val(t.ram_usage_pct);
            // Fans (-1 rpm = sensor absent)
            sys_data.set_cpu_fan_rpm(t.fan_rpms.cpu);
            sys_data.set_gpu_fan_rpm(t.fan_rpms.gpu);
            sys_data.set_mid_fan_rpm(t.fan_rpms.mid.unwrap_or(-1));
        }
        UiUpdate::ProductName(n) => {
            let sys_data = ui.global::<SystemInfo>();
            sys_data.set_product_name(SharedString::from(n));
        }
        UiUpdate::SystemInfo(d) => {
            let sys_data = ui.global::<SystemInfo>();
            sys_data.set_cpu_name(SharedString::from(d.cpu_name));
            sys_data.set_igpu_name(SharedString::from(d.igpu_name));
            sys_data.set_dgpu_name(SharedString::from(d.dgpu_name));
            sys_data.set_has_igpu(d.has_igpu);
            sys_data.set_has_dgpu(d.has_dgpu);
        }
        UiUpdate::PlatformProfile(p) => {
            let sys_data = ui.global::<PowerData>();
            sys_data.set_platform_profile(p);
        }
        UiUpdate::FirmwareAttr(attr, v) => {
            let dev_data = ui.global::<AsusArmouryData>();
            armoury_attrs!(firmware_attr_setters(dev_data, attr, v;));
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
