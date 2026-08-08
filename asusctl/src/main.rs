use log::{error, info};
use rog_dbus::list_iface_blocking;
use rog_dbus::zbus_platform::PlatformProxyBlocking;
use rog_platform::platform::Properties;
use zbus::blocking::Connection;

use crate::cli_opts::*;
use crate::platform_cli::{
    check_service, handle_armoury_command, handle_backlight, handle_battery, handle_brightness,
    handle_info, handle_led_mode, handle_led_power1, handle_led_power2, handle_throttle_profile,
    print_info,
};
use crate::slash_cli::{handle_slash_get, handle_slash_list, handle_slash_set, SlashSubCommand};

mod anime_cli;
mod aura_cli;
mod cli_opts;
mod fan_curve_cli;
mod platform_cli;
mod scsi_cli;
mod slash_cli;
mod xgm_led_cli;

fn main() {
    let env = env_logger::Env::default().default_filter_or("info,tracing=error,zbus=error");
    env_logger::Builder::from_env(env)
        .target(env_logger::Target::Stdout)
        .format_timestamp(None)
        .init();

    let parsed: CliStart = argh::from_env();

    let conn = match Connection::system() {
        Ok(c) => c,
        Err(e) => {
            error!("Could not connect to D-Bus system bus: {e}\nIs dbus-daemon running?");
            return;
        }
    };

    if let CliCommand::Scsi(cmd) = &parsed.command {
        if cmd.list {
            if let Err(err) = do_parsed(&parsed, &[], &[], &conn) {
                print_error_help(&*err, &[], &[]);
            }
            return;
        }
    }

    if let Ok(platform_proxy) = PlatformProxyBlocking::new(&conn).map_err(|e| {
        check_service("asusd");
        error!("Error: {e}");
        print_info();
    }) {
        let asusd_version = match platform_proxy.version() {
            Ok(version) => version,
            Err(e) => {
                error!(
                    "Could not get asusd version: {e:?}\nIs asusd.service running? {}",
                    check_service("asusd")
                );
                return;
            }
        };

        let self_version = env!("CARGO_PKG_VERSION");
        if asusd_version != self_version {
            error!("Version mismatch: asusctl = {self_version}, asusd = {asusd_version}");
            return;
        }

        let supported_properties = match platform_proxy.supported_properties() {
            Ok(props) => props,
            Err(e) => {
                error!("Could not get supported properties: {e:?}");
                return;
            }
        };
        let supported_interfaces = match list_iface_blocking() {
            Ok(ifaces) => ifaces,
            Err(e) => {
                error!("Could not get supported interfaces: {e:?}");
                return;
            }
        };

        if let Err(err) = do_parsed(&parsed, &supported_interfaces, &supported_properties, &conn) {
            print_error_help(&*err, &supported_interfaces, &supported_properties);
        }
    }
}

fn print_error_help(
    err: &dyn std::error::Error,
    supported_interfaces: &[String],
    supported_properties: &[Properties],
) {
    check_service("asusd");
    error!("Error: {err}");
    print_info();
    info!("Supported interfaces:\n\n{supported_interfaces:#?}\n");
    info!("Supported properties on Platform:\n\n{supported_properties:#?}\n");
}

fn do_parsed(
    parsed: &CliStart,
    supported_interfaces: &[String],
    supported_properties: &[Properties],
    conn: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    match &parsed.command {
        CliCommand::Aura(a) => match &a.command {
            crate::cli_opts::AuraSubCommand::Effect(mode) => handle_led_mode(mode)?,
            crate::cli_opts::AuraSubCommand::PowerTuf(pow) => handle_led_power1(pow)?,
            crate::cli_opts::AuraSubCommand::Power(pow) => handle_led_power2(pow)?,
        },
        CliCommand::Brightness(cmd) => handle_brightness(cmd)?,
        CliCommand::Profile(cmd) => handle_throttle_profile(conn, supported_properties, cmd)?,
        CliCommand::FanCurve(cmd) => fan_curve_cli::handle_fan_curve(conn, cmd)?,
        CliCommand::Anime(cmd) => anime_cli::handle_anime(cmd)?,
        CliCommand::Slash(cmd) => match &cmd.command {
            SlashSubCommand::Get(_) => handle_slash_get(conn)?,
            SlashSubCommand::Set(cmd) => handle_slash_set(cmd, conn)?,
            SlashSubCommand::List(_) => handle_slash_list(),
        },
        CliCommand::Scsi(cmd) => scsi_cli::handle_scsi(cmd)?,
        CliCommand::Armoury(cmd) => handle_armoury_command(cmd, conn)?,
        CliCommand::Backlight(cmd) => handle_backlight(cmd)?,
        CliCommand::Battery(cmd) => handle_battery(cmd, conn)?,
        CliCommand::XgmLed(cmd) => xgm_led_cli::handle_xgm_led(conn, &cmd.command)?,
        CliCommand::Info(info_opt) => {
            handle_info(info_opt, supported_interfaces, supported_properties)?;
        }
    }

    Ok(())
}
