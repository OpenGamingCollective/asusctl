//! Used to get the current values at startup

use std::{
    eprintln,
    sync::{Arc, OnceLock},
};

use log::warn;
use rog_dbus::asus_armoury::AsusArmouryProxy;

use rog_platform::asus_armoury::FirmwareAttribute;
use slint::ComponentHandle;

use crate::{
    AsusArmouryData, AttrBool, AttrMinMax, MainWindow, PowerData,
    helpers::zbus_proxies::AsusdInterface,
};

/// Get asusd attribute and update the UI to reflect them
pub async fn populate_slint_properties(
    ui_weak: slint::Weak<MainWindow>,
    asusd: Arc<OnceLock<AsusdInterface>>,
) {
    if let Some(asusd_proxy) = asusd.get() {
        // Fetch armoury attributes if not empty
        if asusd_proxy.is_armoury_loaded() {
            // Set asus_armoury_loaded to true
            {
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    let armoury = ui.global::<AsusArmouryData>();
                    armoury.set_asus_armoury_loaded(true);
                });
            }

            for (attr, proxy) in asusd_proxy.armoury.iter() {
                match attr {
                    FirmwareAttribute::BootSound => {
                        if let Ok(value) = proxy.current_value().await {
                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                let armoury = ui.global::<AsusArmouryData>();
                                armoury.set_boot_sound(AttrBool {
                                    current: value == 1,
                                    supported: true,
                                });
                            });
                        };
                    }
                    FirmwareAttribute::PanelOverdrive => {
                        if let Ok(value) = proxy.current_value().await {
                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                let armoury = ui.global::<AsusArmouryData>();
                                armoury.set_panel_overdrive(AttrBool {
                                    current: value == 1,
                                    supported: true,
                                });
                            });
                        };
                    }
                    FirmwareAttribute::PptFppt => {
                        if let Some(attr) = get_min_max_current(proxy).await {
                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                let power = ui.global::<PowerData>();
                                power.set_ppt_fppt(attr);
                            });
                        }
                    }
                    FirmwareAttribute::PptApuSppt => {
                        if let Some(attr) = get_min_max_current(proxy).await {
                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                let power = ui.global::<PowerData>();
                                power.set_ppt_apu_sppt(attr);
                            });
                        }
                    }
                    FirmwareAttribute::PptPlatformSppt => {
                        if let Some(attr) = get_min_max_current(proxy).await {
                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                let power = ui.global::<PowerData>();
                                power.set_ppt_platform_sppt(attr);
                            });
                        }
                    }
                    _ => {
                        let attr: &str = (*attr).into();
                        eprintln!("Unknown asus-armoury attribute: {}", attr);
                    }
                }
            }
        } else {
            warn!("skipping asus-armoury, module may not be loader");
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                let armoury = ui.global::<AsusArmouryData>();
                armoury.set_asus_armoury_loaded(false);
            });
        }
    }
}

pub async fn get_min_max_current(proxy: &AsusArmouryProxy<'_>) -> Option<AttrMinMax> {
    if let (Ok(min), Ok(max), Ok(current)) = (
        proxy.min_value().await,
        proxy.max_value().await,
        proxy.current_value().await,
    ) {
        Some(AttrMinMax {
            min,
            max,
            current: current as f32,
            supported: true,
        })
    } else {
        None
    }
}
