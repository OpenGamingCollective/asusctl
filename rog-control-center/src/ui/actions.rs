use tokio::sync::watch::Sender;

use crate::state::Action;
pub struct ActionHandler {
    pub tray_tx: Sender<bool>,
}
impl ActionHandler {
    pub async fn handle_action(&mut self, action: Action) {
        match action {
            Action::SetBatteryLimit(_) => {}
            Action::SetPlatformProfile(_) => {}
            Action::SetTray(b) => {
                let _ = self.tray_tx.send(b);
            }
        }
    }
}
