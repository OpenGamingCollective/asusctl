use argh::FromArgs;

#[derive(Default, FromArgs)]
/// ROG Control Center
pub struct CliStart {
    /// start fullscreen, if used the option is saved
    #[argh(switch)]
    pub fullscreen: bool,
    /// fullscreen width
    #[argh(option, default = "0")]
    pub width_fullscreen: u32,
    /// fullscreen height
    #[argh(option, default = "0")]
    pub height_fullscreen: u32,
    /// start windowed, if used the option is saved
    #[argh(switch)]
    pub windowed: bool,
    /// show program version number
    #[argh(switch)]
    pub version: bool,
    /// start in background (UI closed)
    #[argh(switch)]
    pub background: bool,
    /// indicate that the app was launched via autostart
    #[argh(switch)]
    pub autostart: bool,
    /// set board name for testing, this will make ROGCC show only the keyboard page
    #[argh(option)]
    pub board_name: Option<String>,
    /// put ROGCC in layout viewing mode - this is helpful for finding existing layouts that might match your laptop
    #[argh(switch)]
    pub layout_viewing: bool,
}
