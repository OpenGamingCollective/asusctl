use crate::{AsusTray, state::Event};
use log::warn;
use tokio::sync::mpsc::UnboundedSender;

pub fn setup_tray(tx: UnboundedSender<Event>) -> AsusTray {
    let tray = AsusTray::new().expect("Couldn't create tray");

    let tx_clone = tx.clone();
    tray.on_quit_window(move || {
        let _ = tx_clone.send(Event::Quit);
    });

    if let Err(err) = tray.show() {
        warn!("Couldn't show tray: {:?}", err);
    };
    tray
}
