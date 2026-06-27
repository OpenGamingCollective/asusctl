use argh::FromArgs;
use rog_platform::platform::PlatformProfile;
use rog_profiles::FanCurvePU;
use rog_profiles::fan_curve_set::CurveData;

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
