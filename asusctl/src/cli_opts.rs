use argh::FromArgs;
use rog_platform::platform::PlatformProfile;

use crate::anime_cli::AnimeCommand;
use crate::aura_cli::{LedBrightness, LedPowerCommand1, LedPowerCommand2, SetAuraBuiltin};
use crate::fan_curve_cli::FanCurveCommand;
use crate::scsi_cli::ScsiCommand;
use crate::slash_cli::SlashCommand;

#[derive(FromArgs, Default, Debug)]
/// asusctl command-line options
pub struct CliStart {
    #[argh(subcommand)]
    pub command: CliCommand,
}

/// Top-level subcommands for asusctl
#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum CliCommand {
    Aura(AuraCommand),
    Brightness(BrightnessCommand),
    Profile(ProfileCommand),
    FanCurve(FanCurveCommand),
    Anime(AnimeCommand),
    Slash(SlashCommand),
    Scsi(ScsiCommand),
    Armoury(ArmouryCommand),
    Backlight(BacklightCommand),
    Battery(BatteryCommand),
    Info(InfoCommand),
    XgmLed(XgmLedCommand),
}

impl Default for CliCommand {
    fn default() -> Self {
        CliCommand::Info(InfoCommand::default())
    }
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "profile", description = "profile management")]
pub struct ProfileCommand {
    #[argh(subcommand)]
    pub command: ProfileSubCommand,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum ProfileSubCommand {
    Next(ProfileNextCommand),
    List(ProfileListCommand),
    Get(ProfileGetCommand),
    Set(ProfileSetCommand),
    Tuning(ProfileTuningCommand),
}

impl Default for ProfileSubCommand {
    fn default() -> Self {
        ProfileSubCommand::List(ProfileListCommand::default())
    }
}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "next",
    description = "toggle to next profile in list"
)]
pub struct ProfileNextCommand {}

#[derive(FromArgs, Debug, Default)]
#[argh(subcommand, name = "list", description = "list available profiles")]
pub struct ProfileListCommand {}

#[derive(FromArgs, Debug, Default)]
#[argh(subcommand, name = "get", description = "get profile")]
pub struct ProfileGetCommand {}

#[derive(FromArgs, Debug, Default)]
#[argh(subcommand, name = "set", description = "set profile")]
pub struct ProfileSetCommand {
    #[argh(positional, description = "profile to set")]
    pub profile: PlatformProfile,

    #[argh(
        switch,
        short = 'a',
        description = "set the profile to use on AC power"
    )]
    pub ac: bool,

    #[argh(
        switch,
        short = 'b',
        description = "set the profile to use on battery power"
    )]
    pub battery: bool,
}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "tuning",
    description = "enable/disable profile tuning"
)]
pub struct ProfileTuningCommand {
    #[argh(
        positional,
        description = "true/false to enable/disable profile tuning respectively. omit to show current state"
    )]
    pub enable: Option<bool>,
}

#[derive(FromArgs, Debug, Default)]
#[argh(subcommand, name = "effect", description = "led mode commands")]
pub struct LedModeCommand {
    #[argh(switch, description = "switch to next aura mode")]
    pub next_mode: bool,

    #[argh(switch, description = "switch to previous aura mode")]
    pub prev_mode: bool,

    #[argh(subcommand)]
    pub command: Option<SetAuraBuiltin>,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "aura", description = "aura device commands")]
pub struct AuraCommand {
    #[argh(subcommand)]
    pub command: AuraSubCommand,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum AuraSubCommand {
    Power(LedPowerCommand2),
    PowerTuf(LedPowerCommand1),
    Effect(LedModeCommand),
}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "armoury",
    description = "armoury / firmware attributes"
)]
pub struct ArmouryCommand {
    #[argh(subcommand)]
    pub command: ArmourySubCommand,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum ArmourySubCommand {
    Set(ArmouryPropertySetCommand),
    Get(ArmouryPropertyGetCommand),
    List(ArmouryPropertyListCommand),
}

impl Default for ArmourySubCommand {
    fn default() -> Self {
        ArmourySubCommand::List(ArmouryPropertyListCommand::default())
    }
}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "set",
    description = "set an asus-armoury firmware-attribute"
)]
pub struct ArmouryPropertySetCommand {
    #[argh(
        positional,
        description = "name of the attribute to set (see asus-armoury list for available properties)"
    )]
    pub property: String,

    #[argh(positional, description = "value to set for the given attribute")]
    pub value: i32,
}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "list",
    description = "list all firmware-attributes supported by asus-armoury"
)]
pub struct ArmouryPropertyListCommand {}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "get",
    description = "get a firmware-attribute from asus-armoury"
)]
pub struct ArmouryPropertyGetCommand {
    #[argh(
        positional,
        description = "name of the property to get (see asus-armoury list for available properties)"
    )]
    pub property: String,
}

#[derive(FromArgs, Debug, Default)]
#[argh(subcommand, name = "backlight", description = "backlight options")]
pub struct BacklightCommand {
    #[argh(option, description = "set screen brightness <0-100>")]
    pub screenpad_brightness: Option<i32>,

    #[argh(
        option,
        description = "set screenpad gamma brightness 0.5 - 2.2, 1.0 == linear"
    )]
    pub screenpad_gamma: Option<f32>,

    #[argh(
        option,
        description = "set screenpad brightness to sync with primary display"
    )]
    pub sync_screenpad_brightness: Option<bool>,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "battery", description = "battery options")]
pub struct BatteryCommand {
    #[argh(subcommand)]
    pub command: BatterySubCommand,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum BatterySubCommand {
    Limit(BatteryLimitCommand),
    OneShot(BatteryOneShotCommand),
    Info(BatteryInfoCommand),
}

impl Default for BatterySubCommand {
    fn default() -> Self {
        BatterySubCommand::OneShot(BatteryOneShotCommand::default())
    }
}

#[derive(FromArgs, Debug)]
#[argh(
    subcommand,
    name = "limit",
    description = "set battery charge limit <20-100>"
)]
pub struct BatteryLimitCommand {
    #[argh(positional, description = "charge limit percentage 20-100")]
    pub limit: u8,
}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "oneshot",
    description = "one-shot full charge (optional percent)"
)]
pub struct BatteryOneShotCommand {
    #[argh(positional, description = "optional target percent (defaults to 100)")]
    pub percent: Option<u8>,
}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "info",
    description = "show current battery charge limit"
)]
pub struct BatteryInfoCommand {}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "info",
    description = "show program version and system info"
)]
pub struct InfoCommand {
    #[argh(switch, description = "show supported functions of this laptop")]
    pub show_supported: bool,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "leds", description = "keyboard brightness control")]
pub struct BrightnessCommand {
    #[argh(subcommand)]
    pub command: BrightnessSubCommand,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum BrightnessSubCommand {
    Set(BrightnessSetCommand),
    Get(BrightnessGetCommand),
    Next(BrightnessNextCommand),
    Prev(BrightnessPrevCommand),
}

impl Default for BrightnessSubCommand {
    fn default() -> Self {
        BrightnessSubCommand::Get(BrightnessGetCommand::default())
    }
}

#[derive(FromArgs, Debug)]
#[argh(
    subcommand,
    name = "set",
    description = "set keyboard brightness <off, low, med, high>"
)]
pub struct BrightnessSetCommand {
    #[argh(positional, description = "brightness level: off, low, med, high")]
    pub level: LedBrightness,
}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "get",
    description = "get current keyboard brightness"
)]
pub struct BrightnessGetCommand {}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "next",
    description = "toggle to next keyboard brightness"
)]
pub struct BrightnessNextCommand {}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "prev",
    description = "toggle to previous keyboard brightness"
)]
pub struct BrightnessPrevCommand {}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "xgmled", description = "XG Mobile LED control")]
pub struct XgmLedCommand {
    #[argh(subcommand)]
    pub command: XgmLedSubCommand,
}

/// XG Mobile LED subcommand
#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum XgmLedSubCommand {
    Get(XgmLedGetCommand),
    Set(XgmLedSetCommand),
}

impl Default for XgmLedSubCommand {
    fn default() -> Self {
        XgmLedSubCommand::Get(XgmLedGetCommand::default())
    }
}

#[derive(FromArgs, Debug, Default)]
#[argh(
    subcommand,
    name = "get",
    description = "get current XG Mobile LED state"
)]
pub struct XgmLedGetCommand {}

#[derive(FromArgs, Debug)]
#[argh(
    subcommand,
    name = "set",
    description = "set xg mobile led on (1) or off (0)"
)]
pub struct XgmLedSetCommand {
    #[argh(positional, description = "zero for off, one for on")]
    pub value: u8,
}
