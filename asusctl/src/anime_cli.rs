use argh::FromArgs;
use rog_anime::AnimeType;
use rog_anime::usb::{AnimAwake, AnimBooting, AnimShutdown, AnimSleeping};

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "anime", description = "anime commands")]
pub struct AnimeCommand {
    #[argh(option, description = "override the display type")]
    pub override_type: Option<AnimeType>,
    #[argh(option, description = "enable/disable the display")]
    pub enable_display: Option<bool>,
    #[argh(
        option,
        description = "enable/disable the builtin run/powersave animation"
    )]
    pub enable_powersave_anim: Option<bool>,
    #[argh(
        option,
        description = "set global base brightness value <off, low, med, high>"
    )]
    pub brightness: Option<rog_anime::usb::Brightness>,
    #[argh(switch, description = "clear the display")]
    pub clear: bool,
    #[argh(
        option,
        description = "turn the anime off when external power is unplugged"
    )]
    pub off_when_unplugged: Option<bool>,
    #[argh(option, description = "turn the anime off when the laptop suspends")]
    pub off_when_suspended: Option<bool>,
    #[argh(option, description = "turn the anime off when the lid is closed")]
    pub off_when_lid_closed: Option<bool>,
    #[argh(option, description = "off with his head!!!")]
    pub off_with_his_head: Option<bool>,
    #[argh(subcommand)]
    pub command: Option<AnimeActions>,
}

/// Anime subcommands (image, gif, builtins, etc.)
#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum AnimeActions {
    Image(AnimeImage),
    PixelImage(AnimeImageDiagonal),
    Gif(AnimeGif),
    PixelGif(AnimeGifDiagonal),
    SetBuiltins(Builtins),
}

#[derive(FromArgs, Debug)]
#[argh(
    subcommand,
    name = "set-builtins",
    description = "change which builtin animations are shown"
)]
pub struct Builtins {
    #[argh(
        option,
        description = "default is used if unspecified, <default:GlitchConstruction, StaticEmergence>"
    )]
    pub boot: AnimBooting,
    #[argh(
        option,
        description = "default is used if unspecified, <default:BinaryBannerScroll, RogLogoGlitch>"
    )]
    pub awake: AnimAwake,
    #[argh(
        option,
        description = "default is used if unspecified, <default:BannerSwipe, Starfield>"
    )]
    pub sleep: AnimSleeping,
    #[argh(
        option,
        description = "default is used if unspecified, <default:GlitchOut, SeeYa>"
    )]
    pub shutdown: AnimShutdown,
    #[argh(option, description = "set/apply the animations <true/false>")]
    pub set: Option<bool>,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "image", description = "display a PNG image")]
pub struct AnimeImage {
    #[argh(option, description = "full path to the png to display")]
    pub path: String,
    #[argh(option, default = "1.0", description = "scale 1.0 == normal")]
    pub scale: f32,
    #[argh(option, default = "0.0", description = "x position (float)")]
    pub x_pos: f32,
    #[argh(option, default = "0.0", description = "y position (float)")]
    pub y_pos: f32,
    #[argh(option, default = "0.0", description = "the angle in radians")]
    pub angle: f32,
    #[argh(option, default = "1.0", description = "brightness 0.0-1.0")]
    pub bright: f32,
}

#[derive(FromArgs, Debug)]
#[argh(
    subcommand,
    name = "pixel-image",
    description = "display a diagonal/pixel-perfect PNG"
)]
pub struct AnimeImageDiagonal {
    #[argh(option, description = "full path to the png to display")]
    pub path: String,
    #[argh(option, default = "1.0", description = "brightness 0.0-1.0")]
    pub bright: f32,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "gif", description = "display an animated GIF")]
pub struct AnimeGif {
    #[argh(option, description = "full path to the gif to display")]
    pub path: String,
    #[argh(option, default = "1.0", description = "scale 1.0 == normal")]
    pub scale: f32,
    #[argh(option, default = "0.0", description = "x position (float)")]
    pub x_pos: f32,
    #[argh(option, default = "0.0", description = "y position (float)")]
    pub y_pos: f32,
    #[argh(option, default = "0.0", description = "the angle in radians")]
    pub angle: f32,
    #[argh(option, default = "1.0", description = "brightness 0.0-1.0")]
    pub bright: f32,
    #[argh(
        option,
        default = "0",
        description = "how many loops to play - 0 is infinite"
    )]
    pub loops: u32,
}

#[derive(FromArgs, Debug)]
#[argh(
    subcommand,
    name = "pixel-gif",
    description = "display an animated diagonal/pixel-perfect GIF"
)]
pub struct AnimeGifDiagonal {
    #[argh(option, description = "full path to the gif to display")]
    pub path: String,
    #[argh(option, default = "1.0", description = "brightness 0.0-1.0")]
    pub bright: f32,
    #[argh(
        option,
        default = "0",
        description = "how many loops to play - 0 is infinite"
    )]
    pub loops: u32,
}
