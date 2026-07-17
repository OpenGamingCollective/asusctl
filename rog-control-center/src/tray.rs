//! A self-contained tray icon with menus.
//!
//! The tray icon color reflects the GPU power status, sourced from asusd's
//! D-Bus interface (`xyz.ljones.Gpu`).

use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use ksni::{Icon, TrayMethods};
use log::{info, warn};
use rog_platform::platform::Properties;

use crate::config::Config;
use crate::window::{WindowCommand, WindowController};
use crate::zbus_proxies::GpuStatusProxyBlocking;

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
    let mut file = OpenOptions::new()
        .read(true)
        .open(&path)
        .unwrap_or_else(|_| panic!("Missing icon: {:?}", path));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();

    let mut img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .expect("icon not found")
        .to_rgba8();
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

/// Map GPU power status and mode to the appropriate tray icon and title.
fn map_power_to_icon(power_status: &str, mode: &str, icons: &Icons) -> (Icon, String) {
    let icon = match power_status {
        "suspended" => icons.rog_blue.clone(),
        "off" => {
            if mode == "Vfio" {
                icons.rog_yellow.clone()
            } else {
                icons.rog_green.clone()
            }
        }
        "dgpu_disabled" => icons.rog_white.clone(),
        "asus_mux_discreet" | "active" => icons.rog_red.clone(),
        _ => icons.gpu_integrated.clone(),
    };

    let title = format!("ROG: gpu mode = {mode}, gpu power = {power_status}");
    (icon, title)
}

/// Start the tray and route its window actions through `WindowController`.
pub fn init_tray(
    _supported_properties: Vec<Properties>,
    config: Arc<Mutex<Config>>,
    window: WindowController,
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

        // Connect to asusd's GPU interface on the system bus
        let sys_con = zbus::blocking::Connection::system().unwrap();
        let gpu_proxy = match GpuStatusProxyBlocking::new(&sys_con) {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    "Could not connect to asusd GPU interface: {e}. \
                     Is asusd running?"
                );
                let icons = ICONS.get().unwrap();
                tray.update(|tray: &mut AsusTray| {
                    tray.current_icon = icons.rog_red.clone();
                    tray.current_title = "ROG: GPU status unavailable".to_string();
                })
                .await;
                return;
            }
        };

        info!("Started ROGTray with asusd GPU interface");

        // Read initial state
        let mut last_power = String::new();
        if let Ok(power) = gpu_proxy.power_status() {
            let mode = gpu_proxy.mode().unwrap_or_default();
            if let Some(icons) = ICONS.get() {
                let (icon, title) = map_power_to_icon(&power, &mode, icons);
                tray.update(|tray: &mut AsusTray| {
                    tray.current_icon = icon;
                    tray.current_title = title;
                })
                .await;
            }
            last_power = power;
        }

        // Poll loop: check GPU power status periodically and update tray icon.
        // This runs alongside the async tray event loop.
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            if let Ok(lock) = config.try_lock() {
                if !lock.enable_tray_icon {
                    return;
                }
            }

            if let Ok(power) = gpu_proxy.power_status() {
                let mode = gpu_proxy.mode().unwrap_or_default();
                if power != last_power {
                    if let Some(icons) = ICONS.get() {
                        let (icon, title) = map_power_to_icon(&power, &mode, icons);
                        tray.update(|tray: &mut AsusTray| {
                            tray.current_icon = icon;
                            tray.current_title = title;
                        })
                        .await;
                    }
                    last_power = power;
                }
            }
        }
    });
}
