use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use crate::MainWindow;
use crate::state::ToastType;
use log::error;
use slint::{SharedString, Weak};
use tokio::runtime::Handle;

// A counter of toast that appears
static TOAST_SEQ: AtomicU64 = AtomicU64::new(0);

/// Show a persistent toast with a retry button
pub fn show_permanent_toast(
    message: SharedString,
    toast_type: ToastType,
    handle: Weak<MainWindow>,
) {
    let code = toast_type as i32;
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(h) = handle.upgrade() {
            h.invoke_show_permanent_toast(message, code);
        }
    });
}

/// Show a toast on the user screen
pub fn show_toast(message: SharedString, toast_type: ToastType, handle: Weak<MainWindow>) {
    // Increase the counter
    let current_seq = TOAST_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let code = toast_type as i32;

    // Copy the handle to be able to use them in a later move
    let delayed_handle = handle.clone();
    let delayed_text = message.clone();

    // Display the toast on the screen
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(h) = handle.upgrade() {
            h.invoke_show_toast(message, code);
        }
    });

    let handle = match Handle::try_current() {
        Ok(h) => h,
        Err(err) => {
            error!("Cannot obtain handle: {}", err);
            return;
        }
    };

    // Spawn a simple timer to remove the toast after 5 sec
    handle.spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;

        if TOAST_SEQ.load(Ordering::SeqCst) == current_seq {
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(h) = delayed_handle.upgrade() {
                    h.invoke_clear_toast_if_matches(delayed_text);
                }
            });
        }
    });
}
