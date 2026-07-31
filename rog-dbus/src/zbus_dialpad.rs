//! # D-Bus interface proxy for: `xyz.ljones.Dialpad`

use zbus::proxy;

#[proxy(
    interface = "xyz.ljones.Dialpad",
    default_service = "xyz.ljones.Asusd",
    default_path = "/xyz/ljones/Dialpad"
)]
pub trait Dialpad {
    /// Enable property
    #[zbus(property)]
    fn enabled(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_enabled(&self, value: bool) -> zbus::Result<()>;

    /// Brightness property
    #[zbus(property)]
    fn brightness(&self) -> zbus::Result<u8>;
    #[zbus(property)]
    fn set_brightness(&self, value: u8) -> zbus::Result<()>;

    /// Supported property
    #[zbus(property)]
    fn supported(&self) -> zbus::Result<bool>;
}
