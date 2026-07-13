use gumdrop::Options;

#[derive(Default, Options)]
pub struct CliStart {
    #[options(help_flag, help = "print help message")]
    pub help: bool,
    #[options(help = "start fullscreen, if used the option is saved")]
    pub fullscreen: bool,
    #[options(help = "fullscreen width")]
    pub width_fullscreen: u32,
    #[options(help = "fullscreen height")]
    pub height_fullscreen: u32,
    #[options(help = "start windowed, if used the option is saved")]
    pub windowed: bool,
    #[options(help = "show program version number")]
    pub version: bool,
    #[options(help = "start in background (UI closed)")]
    pub background: bool,
    #[options(help = "indicate that the app was launched via autostart")]
    pub autostart: bool,
    #[options(
        meta = "",
        help = "set board name for testing, this will make ROGCC show only the keyboard page"
    )]
    pub board_name: Option<String>,
    #[options(
        help = "put ROGCC in layout viewing mode - this is helpful for finding existing layouts \
                that might match your laptop"
    )]
    pub layout_viewing: bool,
}
