use crate::{AsusArmouryData, MainWindow, PowerData, state::Event};
use slint::ComponentHandle;
use tokio::sync::mpsc::UnboundedSender;

/// A simple macro used to bind an user action to an event, also handle the copy of tx
macro_rules! bind {
    // Standard binding, to use when type is i32
    ($ui:ident, $tx:ident, $global:ident, $slint_callback:ident, $event_variant:expr) => {
        let tx_clone = $tx.clone();
        $ui.global::<$global>().$slint_callback(move |val| {
            let _ = tx_clone.send($event_variant(val));
        });
    };

    // Binding with a type cast
    ($ui:ident, $tx:ident, $global:ident, $slint_callback:ident, $event_variant:expr, $cast_type:ty) => {
        let tx_clone = $tx.clone();
        $ui.global::<$global>()
            .$slint_callback(move |val| match <$cast_type>::try_from(val) {
                Ok(v) => {
                    let _ = tx_clone.send($event_variant(v));
                }
                Err(_) => {
                    warn!(
                        concat!(
                            stringify!($slint_callback),
                            "received out-of-range value: {}"
                        ),
                        val
                    );
                }
            });
    };
    // Binding with no value, can be used for "Restore to Default"
    ($ui:ident, $tx:ident, $global:ident, $slint_callback:ident => $event:expr) => {
        let tx_clone = $tx.clone();
        $ui.global::<$global>().$slint_callback(move || {
            let _ = tx_clone.send($event);
        });
    };
}

pub fn bind_ui_events(ui: &MainWindow, tx: UnboundedSender<Event>) {
    // Platform Profile

    bind!(
        ui,
        tx,
        PowerData,
        on_cb_platform_profile,
        Event::UserRequestedPowerProfile
    );

    // Home Page
    bind!(
        ui,
        tx,
        AsusArmouryData,
        on_cb_boot_sound,
        Event::UserRequestedBootSound
    );

    bind!(
        ui,
        tx,
        AsusArmouryData,
        on_cb_panel_overdrive,
        Event::UserRequestedPanelOD
    );

    // Retry asusd
    ui.on_retry_asusd(move || {
        let _ = tx.send(Event::RetryAsusd);
    });
}
