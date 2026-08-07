use std::process::Command;

use dmi_id::DMIID;
use log::{info, warn};
use rog_aura::keyboard::{AuraPowerState, LaptopAuraPower};
use rog_aura::{AuraEffect, PowerZones};
use rog_dbus::asus_armoury::AsusArmouryProxyBlocking;
use rog_dbus::find_iface_blocking;
use rog_dbus::zbus_aura::AuraProxyBlocking;
use rog_dbus::zbus_backlight::BacklightProxyBlocking;
use rog_dbus::zbus_platform::PlatformProxyBlocking;
use rog_platform::asus_armoury::FirmwareAttributeType;
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
        false
    } else if !check_systemd_unit_active(name) {
        warn!(
            "{} is not running, start it with `systemctl start {}`",
            name, name
        );
        false
    } else {
        true
    }
}

pub fn check_systemd_unit_active(name: &str) -> bool {
    if let Ok(out) = Command::new("systemctl")
        .arg("is-active")
        .arg(name)
        .output()
    {
        let buf = String::from_utf8_lossy(&out.stdout);
        return buf.trim() == "active";
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
    info!("Software version: {}", env!("CARGO_PKG_VERSION"));
    info!("  Product family: {}", prod_family.trim());
    info!("      Board name: {}", board_name.trim());
}

pub fn handle_info(
    info_opt: &InfoCommand,
    supported_interfaces: &[String],
    supported_properties: &[Properties],
) -> Result<(), Box<dyn std::error::Error>> {
    info!("asusctl v{}", env!("CARGO_PKG_VERSION"));
    print_info();

    if info_opt.show_supported {
        info!("Supported Core Functions:\n{:#?}", supported_interfaces);
        info!(
            "Supported Platform Properties:\n{:#?}",
            supported_properties
        );
        match find_iface_blocking::<AuraProxyBlocking>("xyz.ljones.Aura") {
            Ok(aura) => {
                if let Some(first_aura) = aura.first() {
                    let bright = first_aura.supported_brightness()?;
                    let modes = first_aura.supported_basic_modes()?;
                    let zones = first_aura.supported_basic_zones()?;
                    let power = first_aura.supported_power_zones()?;
                    info!("Supported Keyboard Brightness:\n{:#?}", bright);
                    info!("Supported Aura Modes:\n{:#?}", modes);
                    info!("Supported Aura Zones:\n{:#?}", zones);
                    info!("Supported Aura Power Zones:\n{:#?}", power);
                } else {
                    warn!("No aura interface found");
                }
            }
            Err(err) => {
                warn!("No aura interface found: {err}");
            }
        }
    }

    Ok(())
}

pub fn handle_battery(
    cmd: &BatteryCommand,
    conn: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let proxy = PlatformProxyBlocking::new(conn)?;
    match &cmd.command {
        BatterySubCommand::Limit(l) => {
            proxy.set_charge_control_end_threshold(l.limit)?;
        }
        BatterySubCommand::OneShot(o) => {
            if let Some(p) = o.percent {
                proxy.set_charge_control_end_threshold(p)?;
            }
            proxy.one_shot_full_charge()?;
        }
        BatterySubCommand::Info(_) => {
            let limit = proxy.charge_control_end_threshold()?;
            info!("Current battery charge limit: {}%", limit);
        }
    }

    Ok(())
}

pub fn handle_backlight(cmd: &BacklightCommand) -> Result<(), Box<dyn std::error::Error>> {
    let backlights = find_iface_blocking::<BacklightProxyBlocking>("xyz.ljones.Backlight")?;

    if cmd.screenpad_brightness.is_none()
        && cmd.screenpad_gamma.is_none()
        && cmd.sync_screenpad_brightness.is_none()
    {
        for backlight in backlights {
            info!("Current screenpad settings:");
            info!("  Brightness: {}", backlight.screenpad_brightness()?);
            info!("  Gamma: {}", backlight.screenpad_gamma()?);
            info!(
                "  Sync with primary: {}",
                backlight.screenpad_sync_with_primary()?
            );
        }

        return Ok(());
    }

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
        warn!("No aura interface found");
        return Ok(());
    };

    match &cmd.command {
        BrightnessSubCommand::Set(s) => {
            for aura in aura_proxies.iter() {
                if let Some(level) = s.level.level() {
                    aura.set_brightness(rog_aura::LedBrightness::from(level))?;
                } else {
                    let current = aura.brightness()?;
                    info!("Current keyboard led brightness: {current:?}");
                }
            }
        }
        BrightnessSubCommand::Get(_) => {
            for aura in aura_proxies.iter() {
                let level = aura.brightness()?;
                info!("Current keyboard led brightness: {level:?}");
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
                info!("Available modes:");
                for m in modes {
                    info!("  {:?}", m);
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
    if power.awake.is_none()
        && power.sleep.is_none()
        && power.boot.is_none()
        && !power.keyboard
        && !power.lightbar
    {
        warn!("Missing arg or command; run 'asusctl aura power-tuf --help' for usage");
        return Ok(());
    }

    let aura = find_iface_blocking::<AuraProxyBlocking>("xyz.ljones.Aura")?;
    for aura in aura {
        let dev_type = aura.device_type()?;
        if dev_type.is_old_laptop() || dev_type.is_tuf_laptop() {
            handle_led_power_1_do_1866(&aura, power)?;
        } else {
            warn!("These options are for keyboards of product ID 0x1866 or TUF only");
        }
    }

    Ok(())
}

fn handle_led_power_1_do_1866(
    aura: &AuraProxyBlocking,
    power: &LedPowerCommand1,
) -> Result<(), Box<dyn std::error::Error>> {
    if !power.keyboard && !power.lightbar {
        warn!("Must specify at least one zone: --keyboard or --lightbar");
        return Ok(());
    }

    let current_power = aura.led_power()?;
    let mut states = Vec::new();

    let target_zones = [
        (power.keyboard, PowerZones::Keyboard),
        (power.lightbar, PowerZones::Lightbar),
    ];

    for (enabled, zone) in target_zones {
        if enabled {
            let mut state = current_power
                .states
                .iter()
                .find(|s| s.zone == zone)
                .copied()
                .unwrap_or_else(|| AuraPowerState {
                    zone,
                    ..Default::default()
                });
            if let Some(boot) = power.boot {
                state.boot = boot;
            }
            if let Some(awake) = power.awake {
                state.awake = awake;
            }
            if let Some(sleep) = power.sleep {
                state.sleep = sleep;
            }
            states.push(state);
        }
    }

    let states = LaptopAuraPower { states };
    aura.set_led_power(states)?;
    Ok(())
}

pub fn handle_led_power2(power: &LedPowerCommand2) -> Result<(), Box<dyn std::error::Error>> {
    let Some(cmd) = &power.command else {
        warn!("Missing arg or command; run 'asusctl aura power --help' for usage");
        return Ok(());
    };

    let aura = find_iface_blocking::<AuraProxyBlocking>("xyz.ljones.Aura")?;
    for aura in aura {
        let dev_type = aura.device_type()?;
        if !dev_type.is_new_laptop() {
            warn!("This option applies only to keyboards 2021+");
            continue;
        }

        let mut states = aura.led_power()?;
        let mut set =
            |zone: PowerZones, boot_v: bool, awake_v: bool, sleep_v: bool, shutdown_v: bool| {
                if let Some(state) = states.states.iter_mut().find(|s| s.zone == zone) {
                    state.boot = boot_v;
                    state.awake = awake_v;
                    state.sleep = sleep_v;
                    state.shutdown = shutdown_v;
                } else {
                    warn!("Zone {zone:?} is not supported by this device");
                }
            };

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

        aura.set_led_power(states)?;
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
                info!("{:?}", p);
            }
        }
        crate::cli_opts::ProfileSubCommand::Get(_) => {
            info!("Active profile: {current:?}");
            info!("AC profile {:?}", proxy.platform_profile_on_ac()?);
            info!("Battery profile {:?}", proxy.platform_profile_on_battery()?);
        }
        crate::cli_opts::ProfileSubCommand::Tuning(t) => match t.enable {
            Some(true) => {
                proxy.set_enable_ppt_group(true)?;
                info!("Profile tuning enabled");
            }
            Some(false) => {
                proxy.set_enable_ppt_group(false)?;
                info!("Profile tuning disabled");
            }
            None => {
                info!("Profile tuning: {}", proxy.enable_ppt_group()?);
            }
        },
    }

    Ok(())
}

pub fn print_firmware_attr(
    attr: &AsusArmouryProxyBlocking,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = attr.name()?;
    info!("{}:", <&str>::from(name));

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
            (Some(min), Some(c), Some(max)) => info!("  current: {min}..[{c}]..{max}"),
            (Some(min), Some(c), None) => info!("  current: {min}..[{c}]"),
            (None, Some(c), Some(max)) => info!("  current: [{c}]..{max}"),
            (None, Some(c), None) => info!("  current: {c}"),
            _ => info!("  current: unavailable"),
        }

        if has_default {
            match attr.default_value().ok() {
                Some(d) => info!("  default: {d}"),
                None => info!("  default: unavailable"),
            }
        }
    } else if has_possible && has_current {
        let c = attr.current_value().ok();
        let v = attr.possible_values().ok();
        if let (Some(c), Some(v)) = (c, v) {
            let mut s = String::from("  current: [");
            for (idx, item) in v.iter().enumerate() {
                if *item == c {
                    s.push_str(&format!("({c})"));
                } else {
                    s.push_str(&item.to_string());
                }
                if idx < v.len() - 1 {
                    s.push(',');
                }
            }
            s.push(']');
            info!("{s}");
            if has_default {
                match attr.default_value().ok() {
                    Some(d) => info!("  default: {d}"),
                    None => info!("  default: unavailable"),
                }
            }
        } else {
            info!("  current: unavailable");
        }
    } else if has_current {
        match attr.current_value().ok() {
            Some(c) => info!("  current: {c}"),
            None => info!("  current: unavailable"),
        }
    } else {
        info!("  unavailable");
    }

    Ok(())
}

pub fn handle_armoury_command(
    cmd: &ArmouryCommand,
    conn: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let attrs = find_iface_blocking::<AsusArmouryProxyBlocking>("xyz.ljones.AsusArmoury")
        .map_err(|e| format!("Could not reach asusd armoury interface: {e}"))?;
    match &cmd.command {
        ArmourySubCommand::List(_) => {
            for attr in attrs.iter() {
                print_firmware_attr(attr)?;
            }
            Ok(())
        }
        ArmourySubCommand::Get(g) => {
            let mut found = false;
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
            for attr in attrs.iter() {
                let name = attr.name()?;
                if <&str>::from(name) == s.property {
                    let mut value: i32 = s.value;
                    if value == -1 {
                        info!("Setting to default");
                        value = attr.default_value()?;
                    }
                    attr.set_current_value(value)?;

                    if name.property_type() == FirmwareAttributeType::Ppt {
                        // Only Ok(true) means the value was applied to hardware now
                        match PlatformProxyBlocking::new(conn).and_then(|p| p.enable_ppt_group()) {
                            Ok(true) => {}
                            Ok(false) => info!(
                                "PPT config updated and will be applied when tuning is enabled\n\
                                 See: asusctl profile tuning --help"
                            ),
                            Err(e) => {
                                info!(
                                    "PPT config updated, but tuning state is unknown: {e}\n\
                                    See: asusctl profile tuning --help"
                                )
                            }
                        }
                    }

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
