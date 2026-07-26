use argh::FromArgs;
use log::warn;
use rog_scsi::{AuraMode, Colour, Direction, Speed};

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "scsi", description = "scsi LED commands")]
pub struct ScsiCommand {
    #[argh(option, description = "enable the SCSI drive LEDs")]
    pub enable: Option<bool>,

    #[argh(option, description = "set LED mode (use 'list' for all options)")]
    pub mode: Option<AuraMode>,

    #[argh(
        option,
        description = "set LED mode speed <slowest, slow, med, fast, fastest>"
    )]
    pub speed: Option<Speed>,

    #[argh(option, description = "set LED mode direction <forward, reverse>")]
    pub direction: Option<Direction>,

    #[argh(
        option,
        description = "set LED colours <hex>, specify up to 4 with repeated arg"
    )]
    pub colours: Vec<Colour>,

    #[argh(switch, description = "list available animations")]
    pub list: bool,
}

pub fn handle_scsi(cmd: &ScsiCommand) -> Result<(), Box<dyn std::error::Error>> {
    if !cmd.list && cmd.enable.is_none() && cmd.mode.is_none() && cmd.colours.is_empty() {
        warn!("Missing arg or command; run 'asusctl scsi --help' for usage");
    }

    let scsis = crate::platform_cli::find_iface_blocking::<
        rog_dbus::scsi_aura::ScsiAuraProxyBlocking,
    >("xyz.ljones.ScsiAura")?;

    for scsi in scsis {
        if let Some(enable) = cmd.enable {
            scsi.set_enabled(enable)?;
        }

        if let Some(mode) = cmd.mode {
            scsi.set_led_mode(mode)?;
        }

        let mut mode = scsi.led_mode_data()?;
        let mut do_update = false;
        if !cmd.colours.is_empty() {
            for (count, c) in cmd.colours.iter().enumerate() {
                if count == 0 {
                    mode.colour1 = *c;
                }
                if count == 1 {
                    mode.colour2 = *c;
                }
                if count == 2 {
                    mode.colour3 = *c;
                }
                if count == 3 {
                    mode.colour4 = *c;
                }
            }
            do_update = true;
        }

        if let Some(speed) = cmd.speed {
            mode.speed = speed;
            do_update = true;
        }

        if let Some(dir) = cmd.direction {
            mode.direction = dir;
            do_update = true;
        }

        if do_update {
            scsi.set_led_mode_data(mode.clone())?;
        }

        println!("{mode}");
    }

    if cmd.list {
        let res = AuraMode::list();
        for p in &res {
            println!("{:?}", p);
        }
    }

    Ok(())
}
