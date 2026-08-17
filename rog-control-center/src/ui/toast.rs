use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use slint::{SharedString, Weak};

use crate::MainWindow;

// A counter of toast that appears
static TOAST_SEQ: AtomicU64 = AtomicU64::new(0);

/// Show a toast on the user screen
pub fn show_toast(message: SharedString, handle: Weak<MainWindow>) {
    // Increase the counter
    let current_seq = TOAST_SEQ.fetch_add(1, Ordering::SeqCst) + 1;

    // Copy the handle to be able to use them in a later move
    let delayed_handle = handle.clone();
    let delayed_text = message.clone();

    // Display the toast on the screen
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(h) = handle.upgrade() {
            h.invoke_show_toast(message);
        }
    });

    // Spawn a simple timer to remove the toast after 5 sec
    tokio::spawn(async move {
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
