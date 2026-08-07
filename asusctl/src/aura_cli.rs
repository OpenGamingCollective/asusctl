use std::fmt;
use std::str::FromStr;

use argh::FromArgs;
use rog_aura::error::Error;
use rog_aura::{AuraEffect, AuraModeNum, AuraZone, Colour, Direction, Speed};

#[derive(FromArgs, Debug, Clone)]
#[argh(
    subcommand,
    name = "power-tuf",
    description = "aura power (old ROGs and TUF laptops)"
)]
pub struct LedPowerCommand1 {
    #[argh(
        option,
        description = "control if LEDs enabled while awake <true/false>"
    )]
    pub awake: Option<bool>,

    #[argh(
        switch,
        description = "use with awake option; if excluded defaults to false"
    )]
    pub keyboard: bool,

    #[argh(
        switch,
        description = "use with awake option; if excluded defaults to false"
    )]
    pub lightbar: bool,

    #[argh(option, description = "control boot animations <true/false>")]
    pub boot: Option<bool>,

    #[argh(option, description = "control suspend animations <true/false>")]
    pub sleep: Option<bool>,
}

#[derive(FromArgs, Debug, Clone)]
#[argh(subcommand, name = "power", description = "aura power")]
pub struct LedPowerCommand2 {
    #[argh(subcommand)]
    pub command: Option<SetAuraZoneEnabled>,
}

/// Subcommands to enable/disable specific aura zones
#[derive(FromArgs, Debug, Clone)]
#[argh(subcommand)]
pub enum SetAuraZoneEnabled {
    Keyboard(KeyboardPower),
    Logo(LogoPower),
    Lightbar(LightbarPower),
    Lid(LidPower),
    RearGlow(RearGlowPower),
    Ally(AllyPower),
}

/// Keyboard brightness argument helper
#[derive(Debug, Clone)]
pub struct LedBrightness {
    level: Option<u8>,
}

impl LedBrightness {
    pub fn new(level: Option<u8>) -> Self {
        LedBrightness { level }
    }

    pub fn level(&self) -> Option<u8> {
        self.level
    }
}

impl FromStr for LedBrightness {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "off" => Ok(Self::new(Some(0x00))),
            "low" => Ok(Self::new(Some(0x01))),
            "med" => Ok(Self::new(Some(0x02))),
            "high" => Ok(Self::new(Some(0x03))),
            _ => Err(Error::ParseBrightness),
        }
    }
}

impl fmt::Display for LedBrightness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self.level {
            Some(0x00) => "off",
            Some(0x01) => "low",
            Some(0x02) => "med",
            Some(0x03) => "high",
            _ => "unknown",
        };
        write!(f, "{}", s)
    }
}

#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "keyboard",
    description = "set power states for keyboard zone"
)]
pub struct KeyboardPower {
    #[argh(switch, description = "defaults to false if option unused")]
    pub boot: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub awake: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub sleep: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub shutdown: bool,
}

#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "logo",
    description = "set power states for logo zone"
)]
pub struct LogoPower {
    #[argh(switch, description = "defaults to false if option unused")]
    pub boot: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub awake: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub sleep: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub shutdown: bool,
}

#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "lightbar",
    description = "set power states for lightbar zone"
)]
pub struct LightbarPower {
    #[argh(switch, description = "enable power while device is booting")]
    pub boot: bool,
    #[argh(switch, description = "enable power while device is awake")]
    pub awake: bool,
    #[argh(switch, description = "enable power while device is sleeping")]
    pub sleep: bool,
    #[argh(
        switch,
        description = "enable power while device is shutting down or hibernating"
    )]
    pub shutdown: bool,
}

#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "lid",
    description = "set power states for lid zone"
)]
pub struct LidPower {
    #[argh(switch, description = "defaults to false if option unused")]
    pub boot: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub awake: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub sleep: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub shutdown: bool,
}

#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "rear-glow",
    description = "set power states for rear glow zone"
)]
pub struct RearGlowPower {
    #[argh(switch, description = "defaults to false if option unused")]
    pub boot: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub awake: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub sleep: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub shutdown: bool,
}

#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "ally",
    description = "set power states for ally zone"
)]
pub struct AllyPower {
    #[argh(switch, description = "defaults to false if option unused")]
    pub boot: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub awake: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub sleep: bool,
    #[argh(switch, description = "defaults to false if option unused")]
    pub shutdown: bool,
}

/// Single speed-based effect
#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "rainbow-cycle",
    description = "single speed-based effect"
)]
pub struct SingleSpeed {
    #[argh(option, description = "set the speed: low, med, high")]
    pub speed: Speed,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

/// Single speed effect with direction
#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "rainbow-wave",
    description = "single speed effect with direction"
)]
pub struct SingleSpeedDirection {
    #[argh(option, description = "set the direction: up, down, left, right")]
    pub direction: Direction,

    #[argh(option, description = "set the speed: low, med, high")]
    pub speed: Speed,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

/// Static single-colour effect
#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "static",
    description = "static single-colour effect"
)]
pub struct SingleColour {
    #[argh(option, short = 'c', description = "set the RGB value e.g. ff00ff")]
    pub colour: Colour,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

/// Single-colour effect with speed
#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "highlight",
    description = "single-colour effect with speed"
)]
pub struct SingleColourSpeed {
    #[argh(option, short = 'c', description = "set the RGB value e.g. ff00ff")]
    pub colour: Colour,

    #[argh(option, description = "set the speed: low, med, high")]
    pub speed: Speed,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

/// Two-colour breathing effect
#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "breathe",
    description = "two-colour breathing effect"
)]
pub struct TwoColourSpeed {
    #[argh(option, description = "set the first RGB value e.g. ff00ff")]
    pub colour: Colour,

    #[argh(option, description = "set the second RGB value e.g. ff00ff")]
    pub colour2: Colour,

    #[argh(option, description = "set the speed: low, med, high")]
    pub speed: Speed,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

/// Two-colour star effect (separate subcommand name)
#[derive(FromArgs, Debug, Clone, Default)]
#[argh(subcommand, name = "stars", description = "two-colour star effect")]
pub struct StarsTwoColour {
    #[argh(option, description = "set the first RGB value e.g. ff00ff")]
    pub colour: Colour,

    #[argh(option, description = "set the second RGB value e.g. ff00ff")]
    pub colour2: Colour,

    #[argh(option, description = "set the speed: low, med, high")]
    pub speed: Speed,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

/// Rain effect (single-speed, separate subcommand name)
#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "rain",
    description = "single speed-based rain effect"
)]
pub struct RainSingleSpeed {
    #[argh(option, description = "set the speed: low, med, high")]
    pub speed: Speed,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

/// Laser (single-colour with speed) separate subcommand
#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "laser",
    description = "single-colour effect with speed"
)]
pub struct LaserSingleColourSpeed {
    #[argh(option, short = 'c', description = "set the RGB value e.g. ff00ff")]
    pub colour: Colour,

    #[argh(option, description = "set the speed: low, med, high")]
    pub speed: Speed,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

/// Ripple (single-colour with speed) separate subcommand
#[derive(FromArgs, Debug, Clone, Default)]
#[argh(
    subcommand,
    name = "ripple",
    description = "single-colour effect with speed"
)]
pub struct RippleSingleColourSpeed {
    #[argh(option, short = 'c', description = "set the RGB value e.g. ff00ff")]
    pub colour: Colour,

    #[argh(option, description = "set the speed: low, med, high")]
    pub speed: Speed,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

/// Pulse / Comet / Flash variants (single-colour) separate subcommands
#[derive(FromArgs, Debug, Clone, Default)]
#[argh(subcommand, name = "pulse", description = "single-colour pulse effect")]
pub struct PulseSingleColour {
    #[argh(option, short = 'c', description = "set the RGB value e.g. ff00ff")]
    pub colour: Colour,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

#[derive(FromArgs, Debug, Clone, Default)]
#[argh(subcommand, name = "comet", description = "single-colour comet effect")]
pub struct CometSingleColour {
    #[argh(option, short = 'c', description = "set the RGB value e.g. ff00ff")]
    pub colour: Colour,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

#[derive(FromArgs, Debug, Clone, Default)]
#[argh(subcommand, name = "flash", description = "single-colour flash effect")]
pub struct FlashSingleColour {
    #[argh(option, short = 'c', description = "set the RGB value e.g. ff00ff")]
    pub colour: Colour,

    #[argh(
        option,
        default = "AuraZone::None",
        description = "set the zone for this effect e.g. 0, 1, one, logo, lightbar-left"
    )]
    pub zone: AuraZone,
}

/// Builtin aura effects
#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum SetAuraBuiltin {
    Static(SingleColour),              // 0
    Breathe(TwoColourSpeed),           // 1
    RainbowCycle(SingleSpeed),         // 2
    RainbowWave(SingleSpeedDirection), // 3
    Stars(StarsTwoColour),             // 4
    Rain(RainSingleSpeed),             // 5
    Highlight(SingleColourSpeed),      // 6
    Laser(LaserSingleColourSpeed),     // 7
    Ripple(RippleSingleColourSpeed),   // 8
    Pulse(PulseSingleColour),          // 10
    Comet(CometSingleColour),          // 11
    Flash(FlashSingleColour),          // 12
}

impl Default for SetAuraBuiltin {
    fn default() -> Self {
        SetAuraBuiltin::Static(SingleColour::default())
    }
}

impl From<&SingleColour> for AuraEffect {
    fn from(aura: &SingleColour) -> Self {
        Self {
            colour1: aura.colour,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&SingleSpeed> for AuraEffect {
    fn from(aura: &SingleSpeed) -> Self {
        Self {
            speed: aura.speed,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&SingleColourSpeed> for AuraEffect {
    fn from(aura: &SingleColourSpeed) -> Self {
        Self {
            colour1: aura.colour,
            speed: aura.speed,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&TwoColourSpeed> for AuraEffect {
    fn from(aura: &TwoColourSpeed) -> Self {
        Self {
            colour1: aura.colour,
            colour2: aura.colour2,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&SingleSpeedDirection> for AuraEffect {
    fn from(aura: &SingleSpeedDirection) -> Self {
        Self {
            speed: aura.speed,
            direction: aura.direction,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&StarsTwoColour> for AuraEffect {
    fn from(aura: &StarsTwoColour) -> Self {
        Self {
            colour1: aura.colour,
            colour2: aura.colour2,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&RainSingleSpeed> for AuraEffect {
    fn from(aura: &RainSingleSpeed) -> Self {
        Self {
            speed: aura.speed,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&LaserSingleColourSpeed> for AuraEffect {
    fn from(aura: &LaserSingleColourSpeed) -> Self {
        Self {
            colour1: aura.colour,
            speed: aura.speed,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&RippleSingleColourSpeed> for AuraEffect {
    fn from(aura: &RippleSingleColourSpeed) -> Self {
        Self {
            colour1: aura.colour,
            speed: aura.speed,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&PulseSingleColour> for AuraEffect {
    fn from(aura: &PulseSingleColour) -> Self {
        Self {
            colour1: aura.colour,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&CometSingleColour> for AuraEffect {
    fn from(aura: &CometSingleColour) -> Self {
        Self {
            colour1: aura.colour,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&FlashSingleColour> for AuraEffect {
    fn from(aura: &FlashSingleColour) -> Self {
        Self {
            colour1: aura.colour,
            zone: aura.zone,
            ..Default::default()
        }
    }
}

impl From<&SetAuraBuiltin> for AuraEffect {
    fn from(aura: &SetAuraBuiltin) -> Self {
        match aura {
            SetAuraBuiltin::Static(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::Static;
                data
            }
            SetAuraBuiltin::Breathe(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::Breathe;
                data
            }
            SetAuraBuiltin::RainbowCycle(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::RainbowCycle;
                data
            }
            SetAuraBuiltin::RainbowWave(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::RainbowWave;
                data
            }
            SetAuraBuiltin::Stars(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::Star;
                data
            }
            SetAuraBuiltin::Rain(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::Rain;
                data
            }
            SetAuraBuiltin::Highlight(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::Highlight;
                data
            }
            SetAuraBuiltin::Laser(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::Laser;
                data
            }
            SetAuraBuiltin::Ripple(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::Ripple;
                data
            }
            SetAuraBuiltin::Pulse(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::Pulse;
                data
            }
            SetAuraBuiltin::Comet(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::Comet;
                data
            }
            SetAuraBuiltin::Flash(x) => {
                let mut data: AuraEffect = x.into();
                data.mode = AuraModeNum::Flash;
                data
            }
        }
    }
}
