use crate::cli_opts::{DialpadCommand, DialpadSubCommand};
use log::info;
use rog_dbus::zbus_dialpad::DialpadProxyBlocking;

pub fn handle_dialpad(cmd: &DialpadCommand) -> Result<(), Box<dyn std::error::Error>> {
    let proxy = DialpadProxyBlocking::new(&zbus::blocking::Connection::system()?)
        .map_err(|e| format!("Failed to connect to DialPad interface: {e}"))?;

    match &cmd.command {
        DialpadSubCommand::Get(_) => {
            let supported = proxy.supported().unwrap_or(true);
            let enabled = proxy.enabled()?;
            let brightness = proxy.brightness()?;
            info!(
                "DialPad Supported: {}",
                if supported { "YES" } else { "NO" }
            );
            info!("DialPad Enabled: {}", if enabled { "YES" } else { "NO" });
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
            info!("DialPad brightness set to {}", cmd.value);
        }
    }

    Ok(())
}
