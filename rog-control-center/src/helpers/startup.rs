//! Used to get the current values at startup

use std::{
    rc::Rc,
    sync::{Arc, OnceLock},
};

use log::warn;

use rog_platform::platform::PlatformProfile;
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::{AsusArmouryData, MainWindow, PowerData, helpers::zbus_proxies::AsusdInterface};

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
        } else {
            warn!("asus-armoury module may not be loader");
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                let armoury = ui.global::<AsusArmouryData>();
                armoury.set_asus_armoury_loaded(false);
            });
        }

        // Platform profile, populate index, the current selected mode is handled by the platform subscription
        if let Some(platform_proxy) = &asusd_proxy.platform {
            let available_platform_profiles = platform_proxy
                .platform_profile_choices()
                .await
                .unwrap_or_default();
            let mut indexes: Vec<i32> = Vec::new();
            for profile in [
                PlatformProfile::LowPower,
                PlatformProfile::Quiet,
                PlatformProfile::Balanced,
                PlatformProfile::Performance,
            ] {
                if available_platform_profiles.contains(&profile) {
                    indexes.push(profile as i32);
                }
            }
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                let power_data = ui.global::<PowerData>();
                power_data
                    .set_platform_profile_indexes(ModelRc::from(Rc::new(VecModel::from(indexes))));
            });
        }
    }
}
