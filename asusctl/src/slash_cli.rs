use argh::FromArgs;
use rog_dbus::zbus_slash::SlashProxyBlocking;
use rog_slash::SlashMode;
use zbus::blocking::Connection;

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "slash", description = "slash ledbar commands")]
pub struct SlashCommand {
    #[argh(subcommand)]
    pub command: SlashSubCommand,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum SlashSubCommand {
    Get(SlashGetCommand),
    Set(SlashSetCommand),
    List(SlashListCommand),
}

impl Default for SlashSubCommand {
    fn default() -> Self {
        SlashSubCommand::Get(SlashGetCommand::default())
    }
}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "get",
    description = "get the current state of the slash ledbar"
)]
pub struct SlashGetCommand {}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "set", description = "set slash ledbar options")]
pub struct SlashSetCommand {
    #[argh(switch, description = "enable the Slash Ledbar")]
    pub enable: bool,
    #[argh(switch, description = "disable the Slash Ledbar")]
    pub disable: bool,
    #[argh(option, short = 'l', description = "set brightness value <0-255>")]
    pub brightness: Option<u8>,
    #[argh(option, description = "set interval value <0-5>")]
    pub interval: Option<u8>,
    #[argh(option, description = "set SlashMode (use 'list' for options)")]
    pub mode: Option<SlashMode>,

    #[argh(option, short = 'B', description = "show the animation on boot")]
    pub show_on_boot: Option<bool>,
    #[argh(option, short = 'S', description = "show the animation on shutdown")]
    pub show_on_shutdown: Option<bool>,
    #[argh(option, short = 's', description = "show the animation on sleep")]
    pub show_on_sleep: Option<bool>,
    #[argh(option, short = 'b', description = "show the animation on battery")]
    pub show_on_battery: Option<bool>,
    #[argh(
        option,
        short = 'w',
        description = "show the low-battery warning animation"
    )]
    pub show_battery_warning: Option<bool>,
}

#[derive(FromArgs, Debug, Default)]
#[argh(subcommand, name = "list", description = "list available animations")]
pub struct SlashListCommand {}

pub fn handle_slash_set(cmd: &SlashSetCommand) -> Result<(), Box<dyn std::error::Error>> {
    if cmd.brightness.is_none()
        && cmd.interval.is_none()
        && cmd.show_on_boot.is_none()
        && cmd.show_on_shutdown.is_none()
        && cmd.show_on_sleep.is_none()
        && cmd.show_on_battery.is_none()
        && cmd.show_battery_warning.is_none()
        && cmd.mode.is_none()
        && !cmd.enable
        && !cmd.disable
    {
        println!("Missing arg; run 'asusctl slash set --help' for usage");
    }

    let conn = Connection::system()?;
    let proxy = SlashProxyBlocking::new(&conn)
        .map_err(|e| format!("Failed to connect to Slash interface: {e}"))?;

    if cmd.enable {
        proxy.set_enabled(true)?;
    }
    if cmd.disable {
        proxy.set_enabled(false)?;
    }
    if let Some(brightness) = cmd.brightness {
        proxy.set_brightness(brightness)?;
    }
    if let Some(interval) = cmd.interval {
        proxy.set_interval(interval)?;
    }
    if let Some(slash_mode) = cmd.mode {
        proxy.set_mode(slash_mode)?;
    }
    if let Some(show) = cmd.show_on_boot {
        proxy.set_show_on_boot(show)?;
    }
    if let Some(show) = cmd.show_on_shutdown {
        proxy.set_show_on_shutdown(show)?;
    }
    if let Some(show) = cmd.show_on_sleep {
        proxy.set_show_on_sleep(show)?;
    }
    if let Some(show) = cmd.show_on_battery {
        proxy.set_show_on_battery(show)?;
    }
    if let Some(show) = cmd.show_battery_warning {
        proxy.set_show_battery_warning(show)?;
    }

    Ok(())
}

pub fn handle_slash_get() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::system()?;
    let proxy = SlashProxyBlocking::new(&conn)
        .map_err(|e| format!("Failed to connect to Slash interface: {e}"))?;

    let enabled = proxy.enabled()?;
    let brightness = proxy.brightness()?;
    let interval = proxy.interval()?;
    let mode = proxy.mode()?;
    let show_on_boot = proxy.show_on_boot()?;
    let show_on_shutdown = proxy.show_on_shutdown()?;
    let show_on_sleep = proxy.show_on_sleep()?;
    let show_on_battery = proxy.show_on_battery()?;
    let show_battery_warning = proxy.show_battery_warning()?;

    println!(
        "Slash LED: {}",
        if enabled { "enabled" } else { "disabled" }
    );
    println!("Brightness: {}", brightness);
    println!("Interval: {}", interval);
    println!("Mode: {}", mode);
    println!("Show on boot: {}", show_on_boot);
    println!("Show on shutdown: {}", show_on_shutdown);
    println!("Show on sleep: {}", show_on_sleep);
    println!("Show on battery: {}", show_on_battery);
    println!("Show battery warning: {}", show_battery_warning);

    Ok(())
}

pub fn handle_slash_list() {
    let res = SlashMode::list();
    for p in &res {
        println!("{p}");
    }
}
