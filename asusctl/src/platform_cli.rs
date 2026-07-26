use std::process::Command;

use dmi_id::DMIID;
use log::{info, warn};
use rog_aura::keyboard::{AuraPowerState, LaptopAuraPower};
use rog_aura::{AuraEffect, PowerZones};
use rog_dbus::asus_armoury::AsusArmouryProxyBlocking;
use rog_dbus::zbus_aura::AuraProxyBlocking;
use rog_dbus::zbus_backlight::BacklightProxyBlocking;
use rog_dbus::zbus_platform::PlatformProxyBlocking;
use rog_platform::platform::{PlatformProfile, Properties};
use rog_profiles::error::ProfileError;
use zbus::blocking::Connection;

use crate::aura_cli::{LedPowerCommand1, LedPowerCommand2};
use crate::cli_opts::{
    ArmouryCommand, ArmourySubCommand, BacklightCommand, BatteryCommand, BatterySubCommand,
    BrightnessCommand, BrightnessSubCommand, InfoCommand, LedModeCommand, ProfileCommand,
};

pub fn check_service(name: &str) -> bool {
    if name != "asusd" && !check_systemd_unit_enabled(name) {
        warn!(
            "{} is not enabled, enable it with `systemctl enable {}`",
            name, name
        );
        true
    } else if !check_systemd_unit_active(name) {
        warn!(
            "{} is not running, start it with `systemctl start {}`",
            name, name
        );
        true
    } else {
        false
    }
}

pub fn check_systemd_unit_active(name: &str) -> bool {
    if let Ok(out) = Command::new("systemctl")
        .arg("is-active")
        .arg(name)
        .output()
    {
        let buf = String::from_utf8_lossy(&out.stdout);
        return !buf.contains("inactive") && !buf.contains("failed");
    }
    false
}

pub fn check_systemd_unit_enabled(name: &str) -> bool {
    if let Ok(out) = Command::new("systemctl")
        .arg("is-enabled")
        .arg(name)
        .output()
    {
        let buf = String::from_utf8_lossy(&out.stdout);
        return buf.contains("enabled") || buf.contains("linked");
    }
    false
}

pub fn print_info() {
    let dmi = DMIID::new().unwrap_or_default();
    let board_name = dmi.board_name;
    let prod_family = dmi.product_family;
    println!("Software version: {}", env!("CARGO_PKG_VERSION"));
    println!("  Product family: {}", prod_family.trim());
    println!("      Board name: {}", board_name.trim());
}

pub use rog_dbus::find_iface_blocking;

pub fn handle_info(
    info_opt: &InfoCommand,
    supported_interfaces: &[String],
    supported_properties: &[Properties],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("asusctl v{}", env!("CARGO_PKG_VERSION"));
    println!();
    print_info();
    println!();

    if info_opt.show_supported {
        println!("Supported Core Functions:\n{:#?}", supported_interfaces);
        println!(
            "Supported Platform Properties:\n{:#?}",
            supported_properties
        );
        if let Ok(aura) = find_iface_blocking::<AuraProxyBlocking>("xyz.ljones.Aura") {
            if let Some(first_aura) = aura.first() {
                let bright = first_aura.supported_brightness()?;
                let modes = first_aura.supported_basic_modes()?;
                let zones = first_aura.supported_basic_zones()?;
                let power = first_aura.supported_power_zones()?;
                println!("Supported Keyboard Brightness:\n{:#?}", bright);
                println!("Supported Aura Modes:\n{:#?}", modes);
                println!("Supported Aura Zones:\n{:#?}", zones);
                println!("Supported Aura Power Zones:\n{:#?}", power);
            } else {
                warn!("No aura interface found");
            }
        } else {
            warn!("No aura interface found");
        }
    }

    Ok(())
}

pub fn handle_battery(
    cmd: &BatteryCommand,
    conn: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    match &cmd.command {
        BatterySubCommand::Limit(l) => {
            let proxy = PlatformProxyBlocking::new(conn)?;
            proxy.set_charge_control_end_threshold(l.limit)?;
        }
        BatterySubCommand::OneShot(o) => {
            let proxy = PlatformProxyBlocking::new(conn)?;
            if let Some(p) = o.percent {
                proxy.set_charge_control_end_threshold(p)?;
            }
            proxy.one_shot_full_charge()?;
        }
        BatterySubCommand::Info(_) => {
            let proxy = PlatformProxyBlocking::new(conn)?;
            let limit = proxy.charge_control_end_threshold()?;
            println!("Current battery charge limit: {}%", limit);
        }
    }

    Ok(())
}

pub fn handle_backlight(cmd: &BacklightCommand) -> Result<(), Box<dyn std::error::Error>> {
    if cmd.screenpad_brightness.is_none()
        && cmd.screenpad_gamma.is_none()
        && cmd.sync_screenpad_brightness.is_none()
    {
        let backlights = find_iface_blocking::<BacklightProxyBlocking>("xyz.ljones.Backlight")?;
        for backlight in backlights {
            println!("Current screenpad settings:");
            println!("  Brightness: {}", backlight.screenpad_brightness()?);
            println!("  Gamma: {}", backlight.screenpad_gamma()?);
            println!(
                "  Sync with primary: {}",
                backlight.screenpad_sync_with_primary()?
            );
        }

        return Ok(());
    }

    let backlights = find_iface_blocking::<BacklightProxyBlocking>("xyz.ljones.Backlight")?;
    for backlight in backlights {
        if let Some(brightness) = cmd.screenpad_brightness {
            backlight.set_screenpad_brightness(brightness)?;
        }

        if let Some(gamma) = cmd.screenpad_gamma {
            backlight.set_screenpad_gamma(gamma.to_string().as_str())?;
        }

        if let Some(sync) = cmd.sync_screenpad_brightness {
            backlight.set_screenpad_sync_with_primary(sync)?;
        }
    }

    Ok(())
}

pub fn handle_brightness(cmd: &BrightnessCommand) -> Result<(), Box<dyn std::error::Error>> {
    let Ok(aura_proxies) = find_iface_blocking::<AuraProxyBlocking>("xyz.ljones.Aura") else {
        println!("No aura interface found");
        return Ok(());
    };

    match &cmd.command {
        BrightnessSubCommand::Set(s) => {
            for aura in aura_proxies.iter() {
                if let Some(level) = s.level.level() {
                    aura.set_brightness(rog_aura::LedBrightness::from(level))?;
                } else {
                    let current = aura.brightness()?;
                    println!("Current keyboard led brightness: {current:?}");
                }
            }
        }
        BrightnessSubCommand::Get(_) => {
            for aura in aura_proxies.iter() {
                let level = aura.brightness()?;
                println!("Current keyboard led brightness: {level:?}");
            }

            return Ok(());
        }
        BrightnessSubCommand::Next(_) => {
            for aura in aura_proxies.iter() {
                let brightness = aura.brightness()?;
                aura.set_brightness(brightness.next())?;
            }
        }
        BrightnessSubCommand::Prev(_) => {
            for aura in aura_proxies.iter() {
                let brightness = aura.brightness()?;
                aura.set_brightness(brightness.prev())?;
            }
        }
    }

    Ok(())
}

pub fn handle_led_mode(mode: &LedModeCommand) -> Result<(), Box<dyn std::error::Error>> {
    if mode.command.is_none() && !mode.prev_mode && !mode.next_mode {
        warn!("Missing arg or command; run 'asusctl aura --help' for usage");
        if let Ok(aura) = find_iface_blocking::<AuraProxyBlocking>("xyz.ljones.Aura") {
            if let Some(first_aura) = aura.first() {
                let modes = first_aura.supported_basic_modes()?;
                println!("Available modes:");
                for m in modes {
                    println!("  {:?}", m);
                }
            }
        }
        return Ok(());
    }

    if mode.next_mode && mode.prev_mode {
        warn!("Please specify either next or previous");
        return Ok(());
    }
    let aura = find_iface_blocking::<AuraProxyBlocking>("xyz.ljones.Aura")?;
    if mode.next_mode {
        for aura in aura {
            let mode = aura.led_mode()?;
            let modes = aura.supported_basic_modes()?;
            if let Some(pos) = modes.iter().position(|m| *m == mode) {
                let next_pos = if pos + 1 >= modes.len() { 0 } else { pos + 1 };
                if let Some(&target_mode) = modes.get(next_pos) {
                    aura.set_led_mode(target_mode)?;
                }
            } else if let Some(&first) = modes.first() {
                aura.set_led_mode(first)?;
            }
        }
    } else if mode.prev_mode {
        for aura in aura {
            let mode = aura.led_mode()?;
            let modes = aura.supported_basic_modes()?;
            if let Some(pos) = modes.iter().position(|m| *m == mode) {
                let prev_pos = if pos == 0 {
                    modes.len().saturating_sub(1)
                } else {
                    pos - 1
                };
                if let Some(&target_mode) = modes.get(prev_pos) {
                    aura.set_led_mode(target_mode)?;
                }
            } else if let Some(&last) = modes.last() {
                aura.set_led_mode(last)?;
            }
        }
    } else if let Some(mode) = mode.command.as_ref() {
        for aura in aura {
            aura.set_led_mode_data(<AuraEffect>::from(mode))?;
        }
    }

    Ok(())
}

pub fn handle_led_power1(power: &LedPowerCommand1) -> Result<(), Box<dyn std::error::Error>> {
    let aura = find_iface_blocking::<AuraProxyBlocking>("xyz.ljones.Aura")?;
    for aura in aura {
        let dev_type = aura.device_type()?;
        if !dev_type.is_old_laptop() && !dev_type.is_tuf_laptop() {
            warn!("This option applies only to keyboards 2021+");
        }

        if power.awake.is_none()
            && power.sleep.is_none()
            && power.boot.is_none()
            && !power.keyboard
            && !power.lightbar
        {
            warn!("Missing arg or command; run 'asusctl aura power-tuf --help' for usage");
            return Ok(());
        }

        if dev_type.is_old_laptop() || dev_type.is_tuf_laptop() {
            handle_led_power_1_do_1866(&aura, power)?;
            return Ok(());
        }
    }

    warn!("These options are for keyboards of product ID 0x1866 or TUF only");
    Ok(())
}

fn handle_led_power_1_do_1866(
    aura: &AuraProxyBlocking,
    power: &LedPowerCommand1,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut states = Vec::new();
    if power.keyboard {
        states.push(AuraPowerState {
            zone: PowerZones::Keyboard,
            boot: power.boot.unwrap_or_default(),
            awake: power.awake.unwrap_or_default(),
            sleep: power.sleep.unwrap_or_default(),
            shutdown: false,
        });
    }
    if power.lightbar {
        states.push(AuraPowerState {
            zone: PowerZones::Lightbar,
            boot: power.boot.unwrap_or_default(),
            awake: power.awake.unwrap_or_default(),
            sleep: power.sleep.unwrap_or_default(),
            shutdown: false,
        });
    }

    let states = LaptopAuraPower { states };
    aura.set_led_power(states)?;
    Ok(())
}

pub fn handle_led_power2(power: &LedPowerCommand2) -> Result<(), Box<dyn std::error::Error>> {
    let aura = find_iface_blocking::<AuraProxyBlocking>("xyz.ljones.Aura")?;
    for aura in aura {
        let dev_type = aura.device_type()?;
        if !dev_type.is_new_laptop() {
            warn!("This option applies only to keyboards 2021+");
            continue;
        }

        if power.command.is_none() {
            warn!("Missing arg or command; run 'asusctl aura power --help' for usage");
            println!("Commands available");
            return Ok(());
        }

        if let Some(_pow) = power.command.as_ref() {
            let mut states = aura.led_power()?;
            let mut set =
                |zone: PowerZones, boot_v: bool, awake_v: bool, sleep_v: bool, shutdown_v: bool| {
                    for state in states.states.iter_mut() {
                        if state.zone == zone {
                            state.boot = boot_v;
                            state.awake = awake_v;
                            state.sleep = sleep_v;
                            state.shutdown = shutdown_v;
                            break;
                        }
                    }
                };

            if let Some(cmd) = &power.command {
                match cmd {
                    crate::aura_cli::SetAuraZoneEnabled::Keyboard(k) => {
                        set(PowerZones::Keyboard, k.boot, k.awake, k.sleep, k.shutdown)
                    }
                    crate::aura_cli::SetAuraZoneEnabled::Logo(l) => {
                        set(PowerZones::Logo, l.boot, l.awake, l.sleep, l.shutdown)
                    }
                    crate::aura_cli::SetAuraZoneEnabled::Lightbar(l) => {
                        set(PowerZones::Lightbar, l.boot, l.awake, l.sleep, l.shutdown)
                    }
                    crate::aura_cli::SetAuraZoneEnabled::Lid(l) => {
                        set(PowerZones::Lid, l.boot, l.awake, l.sleep, l.shutdown)
                    }
                    crate::aura_cli::SetAuraZoneEnabled::RearGlow(r) => {
                        set(PowerZones::RearGlow, r.boot, r.awake, r.sleep, r.shutdown)
                    }
                    crate::aura_cli::SetAuraZoneEnabled::Ally(r) => {
                        set(PowerZones::Ally, r.boot, r.awake, r.sleep, r.shutdown)
                    }
                }
            }

            aura.set_led_power(states)?;
        }
    }

    Ok(())
}

pub fn handle_throttle_profile(
    conn: &Connection,
    supported: &[Properties],
    cmd: &ProfileCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    if !supported.contains(&Properties::ThrottlePolicy) {
        warn!("Profiles not supported by either this kernel or by the laptop.");
        return Err(ProfileError::NotSupported.into());
    }

    let proxy = PlatformProxyBlocking::new(conn)?;
    let current = proxy.platform_profile()?;
    let choices = proxy.platform_profile_choices()?;

    match &cmd.command {
        crate::cli_opts::ProfileSubCommand::Next(_) => {
            proxy.set_platform_profile(PlatformProfile::next(current, &choices))?;
        }
        crate::cli_opts::ProfileSubCommand::Set(s) => {
            if !s.ac && !s.battery {
                proxy.set_platform_profile(s.profile)?;
            } else {
                if s.ac {
                    proxy.set_platform_profile_on_ac(s.profile)?;
                }
                if s.battery {
                    proxy.set_platform_profile_on_battery(s.profile)?;
                }
            }
        }
        crate::cli_opts::ProfileSubCommand::List(_) => {
            for p in &choices {
                println!("{:?}", p);
            }
        }
        crate::cli_opts::ProfileSubCommand::Get(_) => {
            println!("Active profile: {current:?}");
            println!();
            println!("AC profile {:?}", proxy.platform_profile_on_ac()?);
            println!("Battery profile {:?}", proxy.platform_profile_on_battery()?);
        }
    }

    Ok(())
}

pub fn print_firmware_attr(
    attr: &AsusArmouryProxyBlocking,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = attr.name()?;
    println!("{}:", <&str>::from(name));

    let attrs = attr.available_attrs().unwrap_or_default();

    let has_min = attrs.contains(&"min_value".to_string());
    let has_max = attrs.contains(&"max_value".to_string());
    let has_current = attrs.contains(&"current_value".to_string());
    let has_possible = attrs.contains(&"possible_values".to_string());
    let has_default = attrs.contains(&"default_value".to_string());

    if has_current && (has_min || has_max) {
        let c = attr.current_value().ok();
        let min = if has_min { attr.min_value().ok() } else { None };
        let max = if has_max { attr.max_value().ok() } else { None };
        match (min, c, max) {
            (Some(min), Some(c), Some(max)) => println!("  current: {min}..[{c}]..{max}"),
            (Some(min), Some(c), None) => println!("  current: {min}..[{c}]"),
            (None, Some(c), Some(max)) => println!("  current: [{c}]..{max}"),
            _ => println!("  current: unavailable"),
        }

        if has_default {
            match attr.default_value().ok() {
                Some(d) => println!("  default: {}\n", d),
                None => println!("  default: unavailable\n"),
            }
        } else {
            println!();
        }
    } else if has_possible && has_current {
        let c = attr.current_value().ok();
        let v = attr.possible_values().ok();
        if let (Some(c), Some(v)) = (c, v) {
            for p in v.iter().enumerate() {
                if p.0 == 0 {
                    print!("  current: [");
                }
                if *p.1 == c {
                    print!("({c})");
                } else {
                    print!("{}", p.1);
                }
                if p.0 < v.len() - 1 {
                    print!(",");
                }
                if p.0 == v.len() - 1 {
                    print!("]");
                }
            }
            if has_default {
                match attr.default_value().ok() {
                    Some(d) => println!("  default: {}\n", d),
                    None => println!("  default: unavailable\n"),
                }
            } else {
                println!("\n");
            }
        } else {
            println!("  current: unavailable\n");
        }
    } else if has_current {
        match attr.current_value().ok() {
            Some(c) => println!("  current: {c}\n"),
            None => println!("  current: unavailable\n"),
        }
    } else {
        println!("  unavailable\n");
    }

    Ok(())
}

#[allow(clippy::manual_is_multiple_of, clippy::nonminimal_bool)]
pub fn handle_armoury_command(cmd: &ArmouryCommand) -> Result<(), Box<dyn std::error::Error>> {
    match &cmd.command {
        ArmourySubCommand::List(_) => {
            if let Ok(attrs) =
                find_iface_blocking::<AsusArmouryProxyBlocking>("xyz.ljones.AsusArmoury")
            {
                for attr in attrs.iter() {
                    print_firmware_attr(attr)?;
                }
            }
            Ok(())
        }
        ArmourySubCommand::Get(g) => {
            let mut found = false;
            let attrs = find_iface_blocking::<AsusArmouryProxyBlocking>("xyz.ljones.AsusArmoury")
                .map_err(|e| format!("Could not reach asusd armoury interface: {e}"))?;
            for attr in attrs.iter() {
                let name = attr.name()?;
                if <&str>::from(name) == g.property {
                    print_firmware_attr(attr)?;
                    found = true;
                }
            }
            if !found {
                return Err(format!("Firmware attribute '{}' not found", g.property).into());
            }
            Ok(())
        }
        ArmourySubCommand::Set(s) => {
            let mut found = false;
            let attrs = find_iface_blocking::<AsusArmouryProxyBlocking>("xyz.ljones.AsusArmoury")
                .map_err(|e| format!("Could not reach asusd armoury interface: {e}"))?;
            for attr in attrs.iter() {
                let name = attr.name()?;
                if <&str>::from(name) == s.property {
                    let mut value: i32 = s.value;
                    if value == -1 {
                        info!("Setting to default");
                        value = attr.default_value()?;
                    }
                    attr.set_current_value(value)?;
                    print_firmware_attr(attr)?;
                    found = true;
                }
            }
            if !found {
                return Err(format!("Firmware attribute '{}' not found", s.property).into());
            }
            Ok(())
        }
    }
}
