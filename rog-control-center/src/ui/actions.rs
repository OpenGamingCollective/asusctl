use std::{
    println,
    sync::{Arc, Mutex},
};

use config_traits::StdConfig;
use log::error;
use tokio::sync::watch::Sender;

use crate::{config::Config, state::Action};
pub struct ActionHandler {
    pub tray_tx: Sender<bool>,
    pub config: Arc<Mutex<Config>>,
}
impl ActionHandler {
    pub async fn handle_action(&mut self, action: Action) {
        match action {
            Action::SetBatteryLimit(_) => {}
            Action::SetPlatformProfile(ppd) => {
                println!("Hello from PPD");
            }
            Action::SetTray(b) => {
                match self.config.lock() {
                    Ok(mut c) => {
                        // We got the lock, update the config and update the tray
                        c.enable_tray_icon = b;
                        c.write();
                        let _ = self.tray_tx.send(b);
                    }
                    Err(err) => {
                        error!("Couldn't get config lock: {}", err);
                    }
                }
            }
        }
    }
}
