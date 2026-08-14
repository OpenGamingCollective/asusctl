use zbus::proxy;

#[proxy(
    interface = "xyz.ljones.Slash",
    default_service = "xyz.ljones.Asusd",
    default_path = "/xyz/ljones"
)]
pub trait Slash {
    /// EnableDisplay property
    #[zbus(property)]
    fn enabled(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_enabled(&self, value: bool) -> zbus::Result<()>;

    /// Brightness property
    #[zbus(property)]
    fn brightness(&self) -> zbus::Result<u8>;
    #[zbus(property)]
    fn set_brightness(&self, value: u8) -> zbus::Result<()>;

    /// Interval property
    #[zbus(property)]
    fn interval(&self) -> zbus::Result<u8>;
    #[zbus(property)]
    fn set_interval(&self, value: u8) -> zbus::Result<()>;

    /// Slash modes property
    #[zbus(property)]
    fn mode(&self) -> zbus::Result<u8>;
    #[zbus(property)]
    fn set_mode(&self, value: u8) -> zbus::Result<()>;

    /// ShowBatteryWarning property
    #[zbus(property)]
    fn show_battery_warning(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_show_battery_warning(&self, value: bool) -> zbus::Result<()>;

    /// ShowOnBattery property
    #[zbus(property)]
    fn show_on_battery(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_show_on_battery(&self, value: bool) -> zbus::Result<()>;

    /// ShowOnBoot property
    #[zbus(property)]
    fn show_on_boot(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_show_on_boot(&self, value: bool) -> zbus::Result<()>;

    /// ShowOnShutdown property
    #[zbus(property)]
    fn show_on_shutdown(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_show_on_shutdown(&self, value: bool) -> zbus::Result<()>;

    /// ShowOnSleep property
    #[zbus(property)]
    fn show_on_sleep(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_show_on_sleep(&self, value: bool) -> zbus::Result<()>;

    /// ShowOnLidClosed property
    #[zbus(property)]
    fn show_on_lid_closed(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_show_on_lid_closed(&self, value: bool) -> zbus::Result<()>;
}
