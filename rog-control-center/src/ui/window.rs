//! Used to init the GUI
use log::warn;
use rog_dbus::list_iface_blocking;

use crate::MainWindow;

pub fn setup_window() -> MainWindow {
    slint::set_xdg_app_id(crate::APP_ID)
        .map_err(|e| warn!("Couldn't set application ID: {e:?}"))
        .ok();

    let ui = MainWindow::new().expect("Couldn't create main window");

    let _available = list_iface_blocking().unwrap_or_default();

    // OLD CODE, PLEASE KEEP FOR NOW, MIGHT BE USED LATER
    //ui.set_sidebar_items_avilable(
    //    [
    //        true,
    //        available.contains(&"xyz.ljones.Platform".to_string()),
    //        available.contains(&"xyz.ljones.Aura".to_string()),
    //        available.contains(&"xyz.ljones.Anime".to_string()),
    //        available.contains(&"xyz.ljones.Slash".to_string()),
    //        available.contains(&"xyz.ljones.FanCurves".to_string()),
    //        true,
    //        available.contains(&"xyz.ljones.Platform".to_string()),
    //        true,
    //        true,
    //    ]
    //    .into(),
    //);
    ui.set_sidebar_items_avilable(
        [
            true, true, true, true, true, true, true, true, true,
        ]
        .into(),
    );

    ui
}
