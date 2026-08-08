use argh::FromArgs;
use log::warn;
use rog_anime::usb::{get_anime_type, AnimAwake, AnimBooting, AnimShutdown, AnimSleeping};
use rog_anime::AnimeType;
use rog_dbus::find_iface_blocking;

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

pub fn handle_anime(cmd: &AnimeCommand) -> Result<(), Box<dyn std::error::Error>> {
    if cmd.command.is_none()
        && cmd.enable_display.is_none()
        && cmd.enable_powersave_anim.is_none()
        && cmd.brightness.is_none()
        && cmd.off_when_lid_closed.is_none()
        && cmd.off_when_suspended.is_none()
        && cmd.off_when_unplugged.is_none()
        && !cmd.clear
    {
        warn!("Missing arg or command; run 'asusctl anime --help' for usage");
        return Ok(());
    }

    if let Some(action) = cmd.command.as_ref() {
        match action {
            AnimeActions::Image(image) if image.path.is_empty() => {
                warn!("Missing arg or command; run 'asusctl anime image --help' for usage");
                return Ok(());
            }
            AnimeActions::Image(image) => verify_brightness(image.bright)?,
            AnimeActions::PixelImage(image) if image.path.is_empty() => {
                warn!("Missing arg or command; run 'asusctl anime pixel-image --help' for usage");
                return Ok(());
            }
            AnimeActions::PixelImage(image) => verify_brightness(image.bright)?,
            AnimeActions::Gif(gif) if gif.path.is_empty() => {
                warn!("Missing arg or command; run 'asusctl anime gif --help' for usage");
                return Ok(());
            }
            AnimeActions::Gif(gif) => verify_brightness(gif.bright)?,
            AnimeActions::PixelGif(gif) if gif.path.is_empty() => {
                warn!("Missing arg or command; run 'asusctl anime pixel-gif --help' for usage");
                return Ok(());
            }
            AnimeActions::PixelGif(gif) => verify_brightness(gif.bright)?,
            AnimeActions::SetBuiltins(builtins) if builtins.set.is_none() => {
                warn!("Missing arg; run 'asusctl anime set-builtins --help' for usage");
                return Ok(());
            }
            _ => {}
        }
    }

    let animes =
        find_iface_blocking::<rog_dbus::zbus_anime::AnimeProxyBlocking>("xyz.ljones.Anime")?;

    let mut anime_type = get_anime_type();
    if let Some(model) = cmd.override_type {
        anime_type = model;
    } else if let AnimeType::Unsupported = anime_type {
        warn!("Anime display type is Unsupported; consider specifying --override-type");
    }

    for proxy in &animes {
        if let Some(enable) = cmd.enable_display {
            proxy.set_enable_display(enable)?;
        }
        if let Some(enable) = cmd.enable_powersave_anim {
            proxy.set_builtins_enabled(enable)?;
        }
        if let Some(bright) = cmd.brightness {
            proxy.set_brightness(bright)?;
        }
        if let Some(enable) = cmd.off_when_lid_closed {
            proxy.set_off_when_lid_closed(enable)?;
        }
        if let Some(enable) = cmd.off_when_suspended {
            proxy.set_off_when_suspended(enable)?;
        }
        if let Some(enable) = cmd.off_when_unplugged {
            proxy.set_off_when_unplugged(enable)?;
        }

        if cmd.clear {
            let data = vec![255u8; anime_type.data_length()];
            let tmp = rog_anime::AnimeDataBuffer::from_vec(anime_type, data)?;
            proxy.write(tmp)?;
        }

        if let Some(action) = cmd.command.as_ref() {
            match action {
                AnimeActions::Image(image) => {
                    let matrix = rog_anime::AnimeImage::from_png(
                        std::path::Path::new(&image.path),
                        image.scale,
                        image.angle,
                        rog_anime::Vec2::new(image.x_pos, image.y_pos),
                        image.bright,
                        anime_type,
                    )?;

                    proxy.write(<rog_anime::AnimeDataBuffer>::try_from(&matrix)?)?;
                }
                AnimeActions::PixelImage(image) => {
                    let matrix = rog_anime::AnimeDiagonal::from_png(
                        std::path::Path::new(&image.path),
                        image.bright,
                        anime_type,
                    )?;

                    proxy.write(matrix.into_data_buffer(anime_type)?)?;
                }
                AnimeActions::SetBuiltins(builtins) => {
                    if builtins.set == Some(true) {
                        proxy.set_builtin_animations(rog_anime::Animations {
                            boot: builtins.boot,
                            awake: builtins.awake,
                            sleep: builtins.sleep,
                            shutdown: builtins.shutdown,
                        })?;
                    }
                }
                AnimeActions::Gif(_) | AnimeActions::PixelGif(_) => {}
            }
        }
    }

    if let Some(action) = cmd.command.as_ref() {
        match action {
            AnimeActions::Gif(gif) => {
                let matrix = rog_anime::AnimeGif::from_gif(
                    std::path::Path::new(&gif.path),
                    gif.scale,
                    gif.angle,
                    rog_anime::Vec2::new(gif.x_pos, gif.y_pos),
                    rog_anime::AnimTime::Count(1),
                    gif.bright,
                    anime_type,
                )?;

                play_gif_animation(&animes, &matrix, gif.loops)?;
            }
            AnimeActions::PixelGif(gif) => {
                let matrix = rog_anime::AnimeGif::from_diagonal_gif(
                    std::path::Path::new(&gif.path),
                    rog_anime::AnimTime::Count(1),
                    gif.bright,
                    anime_type,
                )?;

                play_gif_animation(&animes, &matrix, gif.loops)?;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Helper to determine the playback iteration strategy.
/// Returns `None` for infinite loops (`loops == 0`), or `Some(count)` for finite playback.
fn compute_loop_plan(loops: u32) -> Option<u32> {
    if loops == 0 {
        None
    } else {
        Some(loops)
    }
}

/// Play GIF animation frames across all proxies. `loops == 0` means infinite playback until interrupted.
fn play_gif_animation(
    proxies: &[rog_dbus::zbus_anime::AnimeProxyBlocking],
    matrix: &rog_anime::AnimeGif,
    loops: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut remaining = compute_loop_plan(loops);

    loop {
        for frame in matrix.frames() {
            for proxy in proxies {
                proxy.write(frame.frame().clone())?;
            }
            std::thread::sleep(frame.delay());
        }
        match remaining {
            None => continue,
            Some(1) => break,
            Some(ref mut count) => *count -= 1,
        }
    }
    Ok(())
}

fn verify_brightness(brightness: f32) -> Result<(), Box<dyn std::error::Error>> {
    if !(0.0..=1.0).contains(&brightness) {
        return Err(format!(
            "Brightness must be between 0.0 and 1.0 (inclusive), was {brightness}"
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_loop_plan_infinite() {
        assert_eq!(compute_loop_plan(0), None);
    }

    #[test]
    fn test_compute_loop_plan_finite() {
        assert_eq!(compute_loop_plan(1), Some(1));
        assert_eq!(compute_loop_plan(5), Some(5));
    }

    #[test]
    fn test_verify_brightness_valid() {
        assert!(verify_brightness(0.0).is_ok());
        assert!(verify_brightness(0.5).is_ok());
        assert!(verify_brightness(1.0).is_ok());
    }

    #[test]
    fn test_verify_brightness_invalid() {
        assert!(verify_brightness(-0.1).is_err());
        assert!(verify_brightness(1.1).is_err());
        assert!(verify_brightness(f32::NAN).is_err());
        assert!(verify_brightness(f32::INFINITY).is_err());
        assert!(verify_brightness(f32::NEG_INFINITY).is_err());
    }

    #[test]
    fn test_loop_iteration_count() {
        assert_eq!(
            compute_loop_plan(0),
            None,
            "0 loops must plan for infinite playback"
        );
        assert_eq!(
            compute_loop_plan(1),
            Some(1),
            "1 loop must plan for 1 iteration"
        );
        assert_eq!(
            compute_loop_plan(5),
            Some(5),
            "5 loops must plan for 5 iterations"
        );
    }
}
