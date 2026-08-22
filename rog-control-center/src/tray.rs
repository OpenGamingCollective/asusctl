//! A self-contained tray icon with menus.
//!
//! The tray icon color reflects the GPU power status, published by the
//! dGPU status monitor in `notify.rs` (the same source as the
//! "dGPU status changed" notifications).

use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use ksni::{Icon, TrayMethods};
use log::info;
use rog_platform::gpu_pci::GfxPower;
use rog_platform::platform::Properties;
use tokio::sync::watch;

use crate::config::Config;
use crate::window::{WindowCommand, WindowController};

const TRAY_LABEL: &str = "ROG Control Center";
const TRAY_ICON_PATH: &str = "/usr/share/icons/hicolor/512x512/apps/";

struct Icons {
    rog_blue: Icon,
    rog_red: Icon,
    rog_white: Icon,
    gpu_integrated: Icon,
}

static ICONS: LazyLock<Icons> = LazyLock::new(|| Icons {
    rog_blue: read_icon(Path::new("asus_notif_blue.png")),
    rog_red: read_icon(Path::new("asus_notif_red.png")),
    rog_white: read_icon(Path::new("asus_notif_white.png")),
    gpu_integrated: read_icon(Path::new("rog-control-center.png")),
});

fn read_icon(file: &Path) -> Icon {
    let mut path = PathBuf::from(TRAY_ICON_PATH);
    path.push(file);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            log::warn!("Could not read icon {:?}: {e}, using fallback", path);
            return Icon {
                width: 16,
                height: 16,
                data: vec![255; 16 * 16 * 4],
            };
        }
    };

    let mut img = match image::load_from_memory_with_format(&bytes, image::ImageFormat::Png) {
        Ok(i) => i.to_rgba8(),
        Err(e) => {
            log::warn!("Could not decode icon {:?}: {e}, using fallback", path);
            return Icon {
                width: 16,
                height: 16,
                data: vec![255; 16 * 16 * 4],
            };
        }
    };

    for image::Rgba(pixel) in img.pixels_mut() {
        // (╯°□°）╯︵ ┻━┻
        *pixel = u32::from_be_bytes(*pixel).rotate_right(8).to_be_bytes();
    }

    let (width, height) = img.dimensions();
    Icon {
        width: width as i32,
        height: height as i32,
        data: img.into_raw(),
    }
}

struct AsusTray {
    current_title: String,
    current_icon: Icon,
    window: WindowController,
}

impl ksni::Tray for AsusTray {
    fn id(&self) -> String {
        TRAY_LABEL.into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![self.current_icon.clone()]
    }

    fn title(&self) -> String {
        self.current_title.clone()
    }

    fn status(&self) -> ksni::Status {
        ksni::Status::Active
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Open ROGCC".into(),
                icon_name: "rog-control-center".into(),
                activate: Box::new(move |s: &mut AsusTray| {
                    s.window.request(WindowCommand::Show);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit ROGCC".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|s: &mut AsusTray| {
                    s.window.request(WindowCommand::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Derive the GPU mode from platform sysfs attributes and the power status.
fn gpu_mode(power_status: GfxPower) -> &'static str {
    use rog_platform::gpu_pci::{asus_dgpu_disabled, asus_gpu_mux_discreet};

    gpu_mode_for(
        asus_dgpu_disabled().unwrap_or(false),
        asus_gpu_mux_discreet().unwrap_or(false),
        power_status,
    )
}

fn gpu_mode_for(dgpu_disabled: bool, mux_discreet: bool, power_status: GfxPower) -> &'static str {
    if dgpu_disabled {
        return "Integrated";
    }
    if mux_discreet {
        return "Ultimate";
    }
    // If a dGPU is present, it's in Optimus/hybrid mode
    match power_status {
        GfxPower::Active | GfxPower::Suspended => "Optimus",
        _ => "Unknown",
    }
}

/// Map GPU power status and mode to the appropriate tray icon and title.
fn map_power_to_icon(power_status: GfxPower, mode: &str, icons: &Icons) -> (Icon, String) {
    let icon = match power_status {
        GfxPower::Suspended => icons.rog_blue.clone(),
        GfxPower::AsusDisabled => icons.rog_white.clone(),
        GfxPower::AsusMuxDiscreet | GfxPower::Active => icons.rog_red.clone(),
        GfxPower::Unknown => icons.gpu_integrated.clone(),
    };

    let title = format!("ROG: gpu mode = {mode}, gpu power = {power_status}");
    (icon, title)
}

/// Start the tray and route its window actions through `WindowController`.
pub fn init_tray(
    _supported_properties: Vec<Properties>,
    config: Arc<Mutex<Config>>,
    window: WindowController,
    mut gpu_status: watch::Receiver<GfxPower>,
) {
    tokio::spawn(async move {
        let tray_init = AsusTray {
            current_title: TRAY_LABEL.to_string(),
            current_icon: ICONS.rog_red.clone(),
            window,
        };

        // TODO: return an error to the UI

        let tray = match tray_init.disable_dbus_name(true).spawn().await {
            Ok(t) => t,
            Err(e) => {
                log::error!(
                    "Tray unable to be initialised: {e:?}. Do you have a system tray enabled?"
                );
                return;
            }
        };

        info!("Tray started");
        info!("Started ROGTray with local GPU status monitor");

        // Set initial state from the channel's current value
        let power = *gpu_status.borrow_and_update();
        let (icon, title) = map_power_to_icon(power, gpu_mode(power), &ICONS);
        tray.update(|tray: &mut AsusTray| {
            tray.current_icon = icon;
            tray.current_title = title;
        })
        .await;

        // Update the tray icon whenever the dGPU status monitor publishes a
        // change. The timer wakes the loop even when the GPU status is
        // steady, so disabling the tray icon in the UI takes effect promptly.
        let mut config_check = tokio::time::interval(std::time::Duration::from_secs(2));
        config_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                changed = gpu_status.changed() => {
                    if changed.is_err() {
                        // Monitor is gone; nothing left to watch
                        return;
                    }
                    let power = *gpu_status.borrow_and_update();
                    let (icon, title) = map_power_to_icon(power, gpu_mode(power), &ICONS);
                    tray.update(|tray: &mut AsusTray| {
                        tray.current_icon = icon;
                        tray.current_title = title;
                    })
                    .await;
                }
                _ = config_check.tick() => {}
            }

            if let Ok(lock) = config.try_lock()
                && !lock.enable_tray_icon
            {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(tag: u8) -> Icon {
        Icon {
            width: 1,
            height: 1,
            data: vec![tag],
        }
    }

    const BLUE: u8 = 1;
    const RED: u8 = 2;
    const WHITE: u8 = 3;
    const FALLBACK: u8 = 4;

    fn icons() -> Icons {
        Icons {
            rog_blue: icon(BLUE),
            rog_red: icon(RED),
            rog_white: icon(WHITE),
            gpu_integrated: icon(FALLBACK),
        }
    }

    fn icon_for(power: GfxPower, mode: &str) -> u8 {
        map_power_to_icon(power, mode, &icons()).0.data[0]
    }

    #[test]
    fn suspended_gets_the_blue_icon() {
        assert_eq!(icon_for(GfxPower::Suspended, "Optimus"), BLUE);
    }

    #[test]
    fn active_and_mux_discreet_get_the_red_icon() {
        assert_eq!(icon_for(GfxPower::Active, "Optimus"), RED);
        assert_eq!(icon_for(GfxPower::AsusMuxDiscreet, "Ultimate"), RED);
    }

    #[test]
    fn dgpu_disabled_gets_the_white_icon() {
        assert_eq!(icon_for(GfxPower::AsusDisabled, "Integrated"), WHITE);
    }

    #[test]
    fn unknown_falls_back_to_the_rogcc_icon() {
        assert_eq!(icon_for(GfxPower::Unknown, "Unknown"), FALLBACK);
    }

    #[test]
    fn title_reports_mode_and_power() {
        let (_, title) = map_power_to_icon(GfxPower::Suspended, "Optimus", &icons());
        assert_eq!(title, "ROG: gpu mode = Optimus, gpu power = suspended");
    }

    #[test]
    fn mode_derivation_precedence() {
        // dgpu_disable wins over everything, mux over runtime status
        assert_eq!(gpu_mode_for(true, false, GfxPower::Active), "Integrated");
        assert_eq!(gpu_mode_for(true, true, GfxPower::Active), "Integrated");
        assert_eq!(gpu_mode_for(false, true, GfxPower::Active), "Ultimate");
        assert_eq!(gpu_mode_for(false, false, GfxPower::Active), "Optimus");
        assert_eq!(gpu_mode_for(false, false, GfxPower::Suspended), "Optimus");
        assert_eq!(gpu_mode_for(false, false, GfxPower::Unknown), "Unknown");
    }
}
