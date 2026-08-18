use rog_anime::usb::Brightness;
use rog_anime::{Animations, AnimeDataBuffer, DeviceState as AnimeDeviceState};
use zbus::proxy;

#[proxy(
    interface = "xyz.ljones.Anime",
    default_service = "xyz.ljones.Asusd",
    default_path = "/xyz/ljones/aura/anime"
)]
pub trait Anime {
    /// DeviceState method
    fn device_state(&self) -> zbus::Result<AnimeDeviceState>;

    /// RunMainLoop method
    fn run_main_loop(&self, start: bool) -> zbus::Result<()>;

    /// Write method
    fn write(&self, input: &AnimeDataBuffer) -> zbus::Result<()>;

    /// NotifyDeviceState signal
    #[zbus(signal)]
    fn notify_device_state(&self, data: AnimeDeviceState) -> zbus::Result<()>;

    /// Brightness property
    #[zbus(property)]
    fn brightness(&self) -> zbus::Result<Brightness>;
    #[zbus(property)]
    fn set_brightness(&self, value: Brightness) -> zbus::Result<()>;

    /// BuiltinAnimations property
    #[zbus(property)]
    fn builtin_animations(&self) -> zbus::Result<Animations>;
    #[zbus(property)]
    fn set_builtin_animations(&self, value: Animations) -> zbus::Result<()>;

    /// BuiltinsEnabled property
    #[zbus(property)]
    fn builtins_enabled(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_builtins_enabled(&self, value: bool) -> zbus::Result<()>;

    /// EnableDisplay property
    #[zbus(property)]
    fn enable_display(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_enable_display(&self, value: bool) -> zbus::Result<()>;

    /// OffWhenLidClosed property
    #[zbus(property)]
    fn off_when_lid_closed(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_off_when_lid_closed(&self, value: bool) -> zbus::Result<()>;

    /// OffWhenSuspended property
    #[zbus(property)]
    fn off_when_suspended(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_off_when_suspended(&self, value: bool) -> zbus::Result<()>;

    /// OffWhenUnplugged property
    #[zbus(property)]
    fn off_when_unplugged(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_off_when_unplugged(&self, value: bool) -> zbus::Result<()>;
}
