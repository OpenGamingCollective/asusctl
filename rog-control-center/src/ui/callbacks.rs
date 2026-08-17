use slint::ComponentHandle;
use tokio::sync::mpsc::UnboundedSender;

use crate::{MainWindow, SystemPageData, state::Event};

/// A simple macro used to bind an user action to an event, also handle the copy of tx
macro_rules! bind {
    // Standard binding, to use when type is i32
    ($ui:ident, $tx:ident, $slint_callback:ident, $event_variant:expr) => {
        let tx_clone = $tx.clone();
        $ui.global::<SystemPageData>().$slint_callback(move |val| {
            let _ = tx_clone.send($event_variant(val));
        });
    };

    // Binding with a type cast
    ($ui:ident, $tx:ident, $slint_callback:ident, $event_variant:expr, $cast_type:ty) => {
        let tx_clone = $tx.clone();
        $ui.global::<SystemPageData>().$slint_callback(move |val| {
            let _ = tx_clone.send($event_variant(val as $cast_type));
        });
    };
    // Binding with no value, can be used for "Restore to Default"
    ($ui:ident, $tx:ident, $slint_callback:ident => $event:expr) => {
        let tx_clone = $tx.clone();
        $ui.global::<SystemPageData>().$slint_callback(move || {
            let _ = tx_clone.send($event);
        });
    };
}

pub fn bind_ui_events(ui: &MainWindow, tx: UnboundedSender<Event>) {
    // Platform Profile

    bind!(ui, tx, on_cb_platform_profile, Event::UserRequestedProfile);

    bind!(
        ui,
        tx,
        on_cb_charge_control_end_threshold,
        Event::UserRequestedBatteryLimit,
        u8
    );
}
