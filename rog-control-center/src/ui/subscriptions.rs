//! Here are defined the subscriptions, they must be minimal, most of the lifting should be in a helper function

use tokio::sync::mpsc::UnboundedSender;

use crate::{state::Event, ui::helpers::types::BatteryInfo};
use std::time::Duration;

pub async fn subscribe_battery(tx: UnboundedSender<Event>) {
    loop {
        let charge = (rand::random::<u8>() % 101) as u8;
        let info = BatteryInfo {
            health: charge,
            ..Default::default()
        };
        if tx.send(Event::BatteryUpdated(info)).is_err() {
            return;
        };

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
