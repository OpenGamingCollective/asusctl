use crate::cli_opts::XgmLedSubCommand;

pub fn handle_xgm_led(cmd: &XgmLedSubCommand) -> Result<(), Box<dyn std::error::Error>> {
    let xgm_leds = rog_dbus::find_xgm_led_proxies_blocking()?;

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
