//! A self-contained tray icon with menus.
//!
//! The tray icon color reflects the GPU power status, published by the
//! dGPU status monitor in `notify.rs` (the same source as the
//! "dGPU status changed" notifications).

use std::path::{Path, PathBuf};

use crate::state::Event;
use ksni::{Icon, TrayMethods};
use log::info;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;

const TRAY_LABEL: &str = "ROG Control Center";
//const TRAY_ICON_PATH: &str = "/usr/share/icons/hicolor/512x512/apps/";
const TRAY_ICON_PATH: &str = "/home/luytan/Projects/asusctl/rog-control-center/data";

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
    tx: UnboundedSender<Event>,
    pub status: ksni::Status,
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
        self.status
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Open ROGCC".into(),
                icon_name: "rog-control-center".into(),
                activate: Box::new(move |s: &mut AsusTray| {
                    let _ = s.tx.send(Event::ShowWindow);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit ROGCC".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|s: &mut AsusTray| {
                    let _ = s.tx.send(Event::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Start the tray and route its window actions through `WindowController`.
pub fn init_tray(mut enable_tray_rx: watch::Receiver<bool>, tx: UnboundedSender<Event>) {
    tokio::spawn(async move {
        let tray_init = AsusTray {
            current_title: TRAY_LABEL.to_string(),
            current_icon: read_icon(Path::new("rog-sidebar-logo.png")),
            tx,
            status: if *enable_tray_rx.borrow() {
                ksni::Status::Active
            } else {
                ksni::Status::Passive
            },
        };

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

        loop {
            if enable_tray_rx.changed().await.is_err() {
                break; // sender dropped
            }
            let enabled = *enable_tray_rx.borrow_and_update();
            let _ = tray
                .update(move |t: &mut AsusTray| {
                    t.status = if enabled {
                        ksni::Status::Active
                    } else {
                        ksni::Status::Passive
                    };
                })
                .await;
        }
    });
}
