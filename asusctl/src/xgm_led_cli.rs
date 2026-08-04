use crate::cli_opts::XgmLedSubCommand;
use rog_dbus::find_iface_blocking;
use rog_dbus::zbus_xgm_led::XgmLedProxyBlocking;

pub fn handle_xgm_led(cmd: &XgmLedSubCommand) -> Result<(), Box<dyn std::error::Error>> {
    let xgm_leds = find_iface_blocking::<XgmLedProxyBlocking>("xyz.ljones.XgmLed")?;

    for proxy in &xgm_leds {
        match cmd {
            XgmLedSubCommand::Get(_) => {
                let enabled = proxy.xgm_led_enabled()?;
                println!("XG Mobile LED: {}", if enabled { "ON" } else { "OFF" });
            }
            XgmLedSubCommand::Set(cmd) => {
                let enabled = cmd.value != 0;
                proxy.set_xgm_led_enabled(enabled)?;
                println!(
                    "XG Mobile LED set to {}",
                    if enabled { "ON" } else { "OFF" }
                );
            }
        }
    }

    Ok(())
}
