use config_traits::{StdConfig, StdConfigLoad};
use rog_slash::{DeviceState, SlashMode, SlashType};
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "slash.ron";

/// Config for base system actions for the anime display
#[derive(Deserialize, Serialize, Debug)]
pub struct SlashConfig {
    #[serde(skip)]
    pub slash_type: SlashType,
    pub enabled: bool,
    pub brightness: u8,
    pub display_interval: u8,
    pub display_mode: SlashMode,
    /// When true, a background task renders a battery-percentage fill
    /// pattern via the custom-pattern buffer instead of `display_mode`.
    #[serde(default)]
    pub battery_level_mode: bool,
    pub show_on_boot: bool,
    pub show_on_shutdown: bool,
    pub show_on_sleep: bool,
    pub show_on_battery: bool,
    pub show_battery_warning: bool,
    pub show_on_lid_closed: bool,
}

impl Default for SlashConfig {
    fn default() -> Self {
        SlashConfig {
            enabled: true,
            brightness: 255,
            display_interval: 0,
            display_mode: SlashMode::Bounce,
            battery_level_mode: false,
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
