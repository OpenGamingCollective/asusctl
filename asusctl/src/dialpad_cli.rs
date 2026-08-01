use std::str::FromStr;

use log::info;
use rog_dbus::zbus_dialpad::DialpadProxyBlocking;
use rog_platform::dialpad::DialpadMode;

use crate::cli_opts::{DialpadCommand, DialpadSubCommand};

pub fn handle_dialpad(cmd: &DialpadCommand) -> Result<(), Box<dyn std::error::Error>> {
    let connection = zbus::blocking::Connection::system()
        .map_err(|e| format!("Failed to connect to D-Bus system bus: {e}"))?;

    let proxy = DialpadProxyBlocking::new(&connection)
        .map_err(|e| format!("Failed to initialize DialPad D-Bus proxy: {e}"))?;

    if !proxy.supported()? {
        return Err("DialPad is not supported on this device".into());
    }

    match &cmd.command {
        DialpadSubCommand::Get(_) => {
            let enabled = if proxy.enabled().unwrap_or(false) {
                "YES"
            } else {
                "NO"
            };
            let mode = proxy.mode().unwrap_or_else(|_| "UNKNOWN".into());
            let brightness = proxy
                .brightness()
                .map(|b| b.to_string())
                .unwrap_or_else(|_| "UNKNOWN".into());

            info!("DialPad Supported: YES");
            info!("DialPad Enabled: {enabled}");
            info!("DialPad Mode: {mode}");
            info!("DialPad Brightness: {brightness}");
        }
        DialpadSubCommand::On(_) => {
            proxy.set_enabled(true)?;
            info!("DialPad set to enabled");
        }
        DialpadSubCommand::Off(_) => {
            proxy.set_enabled(false)?;
            info!("DialPad set to disabled");
        }
        DialpadSubCommand::Brightness(cmd) => {
            proxy.set_brightness(cmd.value)?;
            let actual = proxy.brightness().unwrap_or(cmd.value);
            info!("DialPad brightness set to {}", actual);
        }
        DialpadSubCommand::Mode(cmd) => {
            let parsed_mode = DialpadMode::from_str(&cmd.mode).map_err(|_| {
                format!(
                    "Invalid mode '{}'. Valid modes: hardware, virtual, auto",
                    cmd.mode
                )
            })?;

            proxy.set_mode(&parsed_mode.to_string())?;
            info!("DialPad mode set to {}", parsed_mode);
        }
    }

    Ok(())
}
