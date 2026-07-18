use std::fs::create_dir;

use config_traits::{StdConfig, StdConfigLoad1};
use serde::{Deserialize, Serialize};

use crate::{notify::EnabledNotifications, APP_ID};

const CFG_DIR: &str = "rog";
const CFG_FILE_NAME: &str = "rog-control-center.cfg";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub run_in_background: bool,
    pub startup_in_background: bool,
    pub enable_tray_icon: bool,
    #[serde(default)]
    pub enable_autostart: bool,
    #[serde(default)]
    pub enable_global_shortcut: bool,
    pub ac_command: String,
    pub bat_command: String,
    pub dark_mode: bool,
    // intended for use with devices like the ROG Ally
    pub start_fullscreen: bool,
    pub fullscreen_width: u32,
    pub fullscreen_height: u32,
    // This field must be last
    pub notifications: EnabledNotifications,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            run_in_background: true,
            startup_in_background: false,
            enable_tray_icon: true,
            enable_autostart: false,
            enable_global_shortcut: false,
            dark_mode: true,
            start_fullscreen: false,
            fullscreen_width: 1920,
            fullscreen_height: 1080,
            notifications: EnabledNotifications::default(),
            ac_command: String::new(),
            bat_command: String::new(),
        }
    }
}

impl StdConfig for Config {
    fn new() -> Self {
        Config {
            ..Default::default()
        }
    }

    fn file_name(&self) -> String {
        CFG_FILE_NAME.to_owned()
    }

    fn config_dir() -> std::path::PathBuf {
        let mut path = dirs::config_dir().unwrap_or_default();

        path.push(CFG_DIR);
        if !path.exists() {
            create_dir(path.clone())
                .map_err(|e| log::error!("Could not create config dir: {e}"))
                .ok();
            log::info!("Created {path:?}");
        }
        path
    }
}

impl StdConfigLoad1<Config461> for Config {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config461 {
    pub run_in_background: bool,
    pub startup_in_background: bool,
    pub ac_command: String,
    pub bat_command: String,
    pub enable_dgpu_notifications: bool,
    pub dark_mode: bool,
    // This field must be last
    pub enabled_notifications: EnabledNotifications,
}

impl From<Config461> for Config {
    fn from(c: Config461) -> Self {
        Self {
            run_in_background: c.run_in_background,
            startup_in_background: c.startup_in_background,
            enable_tray_icon: true,
            enable_autostart: false,
            enable_global_shortcut: false,
            ac_command: c.ac_command,
            bat_command: c.bat_command,
            dark_mode: true,
            start_fullscreen: false,
            fullscreen_width: 1920,
            fullscreen_height: 1080,
            notifications: c.enabled_notifications,
        }
    }
}

pub fn is_autostart_in_background() -> bool {
    let path = dirs::config_dir().map(|mut p| {
        p.push("autostart");
        p.push(format!("{APP_ID}.desktop"));
        p
    });
    if let Some(path) = path {
        if let Ok(content) = std::fs::read_to_string(path) {
            return content.contains("Exec=rog-control-center --autostart --background")
                || content.contains("Exec=rog-control-center --background");
        }
    }
    false
}

pub fn update_autostart(enable: bool, in_background: bool) {
    update_autostart_with_dir(enable, in_background, None);
}

fn update_autostart_with_dir(
    enable: bool,
    in_background: bool,
    custom_dir: Option<&std::path::Path>,
) {
    let autostart_dir = if let Some(dir) = custom_dir {
        dir.to_path_buf()
    } else {
        match dirs::config_dir() {
            Some(mut p) => {
                p.push("autostart");
                p
            }
            None => {
                log::error!("Could not find config directory for autostart");
                return;
            }
        }
    };

    let desktop_file = autostart_dir.join(format!("{APP_ID}.desktop"));

    if enable {
        if !autostart_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&autostart_dir) {
                log::error!("Failed to create autostart directory: {e}");
                return;
            }
        }

        let exec_cmd = if in_background {
            "rog-control-center --autostart --background"
        } else {
            "rog-control-center --autostart"
        };

        let content = format!(
            "[Desktop Entry]\n\
                       Version=1.0\n\
                       Type=Application\n\
                       Name=ROG Control Center\n\
                       Comment=Make your ASUS ROG Laptop go Brrrrr!\n\
                       Categories=Settings;\n\
                       Icon=rog-control-center\n\
                       Exec={}\n\
                       Terminal=false\n",
            exec_cmd
        );

        if let Err(e) = std::fs::write(&desktop_file, content) {
            log::error!("Failed to write autostart desktop file: {e}");
        } else {
            log::info!("Created autostart entry at {:?}", desktop_file);
        }
    } else if desktop_file.exists() {
        if let Err(e) = std::fs::remove_file(&desktop_file) {
            log::error!("Failed to remove autostart desktop file: {e}");
        } else {
            log::info!("Removed autostart entry at {:?}", desktop_file);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_without_global_shortcut_field_defaults_to_false() {
        let serialized = config_traits::ron::to_string(&Config::default()).unwrap();
        let without_field = serialized.replace("enable_global_shortcut:false,", "");
        assert!(without_field.len() < serialized.len());
        let parsed: Config = config_traits::ron::from_str(&without_field).unwrap();
        assert!(!parsed.enable_global_shortcut);
    }

    #[test]
    fn test_update_autostart() {
        let mut path = std::env::temp_dir();
        path.push(format!("rog-test-autostart-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&path);

        let file_path = path.join(format!("{APP_ID}.desktop"));

        // Test enabling
        update_autostart_with_dir(true, true, Some(&path));
        assert!(file_path.exists());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Name=ROG Control Center"));
        assert!(content.contains("Exec=rog-control-center --autostart --background"));

        // Test disabling
        update_autostart_with_dir(false, false, Some(&path));
        assert!(!file_path.exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(&path);
    }
}
