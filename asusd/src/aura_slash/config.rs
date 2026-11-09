use config_traits::{StdConfig, StdConfigLoad};
use rog_slash::{DeviceState, SlashMode, SlashType};
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "slash.ron";

/// Config for base system actions for the anime display
#[derive(Deserialize, Serialize, Debug)]
pub struct SlashConfig {
    #[serde(skip)]
    /// Runtime-only device type detected from DMI; not written to the file.
    pub slash_type: SlashType,
    /// Enable the Slash LED bar.
    pub enabled: bool,
    /// Brightness, `0-255`.
    pub brightness: u8,
    /// Animation interval. The CLI documents `0-5`; the file stores a `u8`.
    pub display_interval: u8,
    /// Animation mode.
    pub display_mode: SlashMode,
    /// Show the animation on boot.
    pub show_on_boot: bool,
    /// Show the animation on shutdown.
    pub show_on_shutdown: bool,
    /// Show the animation on sleep.
    pub show_on_sleep: bool,
    /// Show the animation on battery power.
    pub show_on_battery: bool,
    /// Show the low-battery warning animation.
    pub show_battery_warning: bool,
    /// Show the animation when the lid is closed.
    pub show_on_lid_closed: bool,
}

impl Default for SlashConfig {
    fn default() -> Self {
        SlashConfig {
            enabled: true,
            brightness: 255,
            display_interval: 0,
            display_mode: SlashMode::Bounce,
            slash_type: SlashType::Unsupported,
            show_on_boot: true,
            show_on_shutdown: true,
            show_on_sleep: true,
            show_on_battery: true,
            show_battery_warning: true,
            show_on_lid_closed: true,
        }
    }
}
impl StdConfig for SlashConfig {
    fn new() -> Self {
        Self::default()
    }

    fn file_name(&self) -> String {
        CONFIG_FILE.to_owned()
    }

    fn config_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(crate::CONFIG_PATH_BASE)
    }
}

impl StdConfigLoad for SlashConfig {}

impl From<&SlashConfig> for DeviceState {
    fn from(config: &SlashConfig) -> Self {
        DeviceState {
            slash_enabled: config.enabled,
            slash_brightness: config.brightness,
            slash_interval: config.display_interval,
            slash_mode: config.display_mode,
        }
    }
}
