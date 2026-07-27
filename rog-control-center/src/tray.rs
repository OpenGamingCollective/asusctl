//! A self-contained tray icon with menus.
//!
//! The tray icon color reflects the GPU power status, published by the
//! dGPU status monitor in `notify.rs` (the same source as the
//! "dGPU status changed" notifications).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

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
    rog_green: Icon,
    rog_white: Icon,
    rog_yellow: Icon,
    gpu_integrated: Icon,
}

static ICONS: OnceLock<Icons> = OnceLock::new();

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
        GfxPower::Active | GfxPower::Suspended | GfxPower::Off => "Optimus",
        _ => "Unknown",
    }
}

/// Map GPU power status and mode to the appropriate tray icon and title.
fn map_power_to_icon(power_status: GfxPower, mode: &str, icons: &Icons) -> (Icon, String) {
    let icon = match power_status {
        GfxPower::Suspended => icons.rog_blue.clone(),
        GfxPower::Off => {
            if mode == "Vfio" {
                icons.rog_yellow.clone()
            } else {
                icons.rog_green.clone()
            }
        }
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
        let rog_red = read_icon(&PathBuf::from("asus_notif_red.png"));

        let tray_init = AsusTray {
            current_title: TRAY_LABEL.to_string(),
            current_icon: rog_red.clone(),
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
        let rog_blue = read_icon(&PathBuf::from("asus_notif_blue.png"));
        let rog_green = read_icon(&PathBuf::from("asus_notif_green.png"));
        let rog_white = read_icon(&PathBuf::from("asus_notif_white.png"));
        let rog_yellow = read_icon(&PathBuf::from("asus_notif_yellow.png"));
        let gpu_integrated = read_icon(&PathBuf::from("rog-control-center.png"));
        ICONS.get_or_init(|| Icons {
            rog_blue,
            rog_red: rog_red.clone(),
            rog_green,
            rog_white,
            rog_yellow,
            gpu_integrated,
        });

        info!("Started ROGTray with local GPU status monitor");

        // Set initial state from the channel's current value
        let power = *gpu_status.borrow_and_update();
        if let Some(icons) = ICONS.get() {
            let (icon, title) = map_power_to_icon(power, gpu_mode(power), icons);
            tray.update(|tray: &mut AsusTray| {
                tray.current_icon = icon;
                tray.current_title = title;
            })
            .await;
        }

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
                    if let Some(icons) = ICONS.get() {
                        let (icon, title) = map_power_to_icon(power, gpu_mode(power), icons);
                        tray.update(|tray: &mut AsusTray| {
                            tray.current_icon = icon;
                            tray.current_title = title;
                        })
                        .await;
                    }
                }
                _ = config_check.tick() => {}
            }

            if let Ok(lock) = config.try_lock() {
                if !lock.enable_tray_icon {
                    return;
                }
            }
        }
    });
}
