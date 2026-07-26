use argh::FromArgs;
use log::warn;
use rog_platform::platform::PlatformProfile;
use rog_profiles::fan_curve_set::CurveData;
use rog_profiles::FanCurvePU;

#[derive(FromArgs, Debug, Clone)]
#[argh(subcommand, name = "fan-curve", description = "fan curve commands")]
pub struct FanCurveCommand {
    #[argh(switch, description = "get enabled fan profiles")]
    pub get_enabled: bool,

    #[argh(switch, description = "set the active profile's fan curve to default")]
    pub default: bool,

    #[argh(
        option,
        description = "profile to modify fan-curve for. shows data if no options provided"
    )]
    pub mod_profile: Option<PlatformProfile>,

    #[argh(
        option,
        description = "enable or disable <true/false> fan all curves for a profile; --mod_profile required"
    )]
    pub enable_fan_curves: Option<bool>,

    #[argh(
        option,
        description = "enable or disable <true/false> a single fan curve for a profile; --mod_profile and --fan required"
    )]
    pub enable_fan_curve: Option<bool>,

    #[argh(
        option,
        description = "select fan <cpu/gpu/mid> to modify; --mod_profile required"
    )]
    pub fan: Option<FanCurvePU>,

    #[argh(
        option,
        description = "data format = 30c:1%,49c:2%,...; --mod-profile required. If '%' is omitted the fan range is 0-255"
    )]
    pub data: Option<CurveData>,
}

const REQ_MOD_PROFILE_MSG: &str =
    "--enable-fan-curves, --enable-fan-curve, --fan, and --data options require --mod-profile";

pub fn handle_fan_curve(
    conn: &zbus::blocking::Connection,
    cmd: &FanCurveCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let fan_proxy = rog_dbus::zbus_fan_curves::FanCurvesProxyBlocking::new(conn).map_err(|e| {
        warn!("Fan curves unavailable: {e}");
        rog_profiles::error::ProfileError::NotSupported
    })?;

    if !cmd.get_enabled && !cmd.default && cmd.mod_profile.is_none() {
        warn!("Missing arg or command; run 'asusctl fan-curve --help' for usage");
        return Ok(());
    }

    if (cmd.enable_fan_curves.is_some() || cmd.fan.is_some() || cmd.data.is_some())
        && cmd.mod_profile.is_none()
    {
        warn!("{REQ_MOD_PROFILE_MSG}");
        return Ok(());
    }

    let plat_proxy = rog_dbus::zbus_platform::PlatformProxyBlocking::new(conn)?;
    if cmd.get_enabled {
        let profile = plat_proxy.platform_profile()?;
        let curves = fan_proxy.fan_curve_data(profile)?;
        for curve in curves.iter() {
            println!("{}", String::from(curve));
        }
    }

    if cmd.default {
        let active = plat_proxy.platform_profile()?;
        fan_proxy.set_curves_to_defaults(active)?;
    }

    if let Some(profile) = cmd.mod_profile {
        if cmd.enable_fan_curves.is_none() && cmd.data.is_none() {
            let data = fan_proxy.fan_curve_data(profile)?;
            let ron =
                ron::ser::to_string_pretty(&data, ron::ser::PrettyConfig::new().depth_limit(4))?;
            println!("\nFan curves for {:?}\n\n{}", profile, ron);
        }

        if let Some(enabled) = cmd.enable_fan_curves {
            fan_proxy.set_fan_curves_enabled(profile, enabled)?;
        }

        if let Some(enabled) = cmd.enable_fan_curve {
            if let Some(fan) = cmd.fan {
                fan_proxy.set_profile_fan_curve_enabled(profile, fan, enabled)?;
            } else {
                warn!("{REQ_MOD_PROFILE_MSG}");
            }
        }

        if let Some(mut curve) = cmd.data.clone() {
            let fan = cmd.fan.unwrap_or_default();
            curve.set_fan(fan);
            fan_proxy.set_fan_curve(profile, curve)?;
        }
    }

    Ok(())
}
