/// The main data conversion for transfering in shortform over dbus or other,
/// or writing directly to the USB device
mod data;
pub use data::*;

pub use crate::error::PlatformError as SlashError;
pub type Result<T> = std::result::Result<T, SlashError>;

/// Provides const methods to create the USB HID control packets
pub mod usb;
