use std::time::Duration;

use config_traits::{StdConfig, StdConfigLoad};
use rog_anime::error::AnimeError;
use rog_anime::usb::Brightness;
use rog_anime::{
    ActionData, ActionLoader, AnimTime, Animations, AnimeType, DeviceState, Fade, Vec2,
};
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "anime.ron";

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AniMeConfigCached {
    pub system: Vec<ActionData>,
    pub boot: Vec<ActionData>,
    pub wake: Vec<ActionData>,
    pub shutdown: Vec<ActionData>,
}

impl AniMeConfigCached {
    pub fn init_from_config(
        &mut self,
        config: &AniMeConfig,
        anime_type: AnimeType,
    ) -> Result<(), AnimeError> {
        let mut sys = Vec::with_capacity(config.system.len());
        for ani in &config.system {
            sys.push(ActionData::from_anime_action(anime_type, ani)?);
        }
        self.system = sys;

        let mut boot = Vec::with_capacity(config.boot.len());
        for ani in &config.boot {
            boot.push(ActionData::from_anime_action(anime_type, ani)?);
        }
        self.boot = boot;

        let mut wake = Vec::with_capacity(config.wake.len());
        for ani in &config.wake {
            wake.push(ActionData::from_anime_action(anime_type, ani)?);
        }
        self.wake = wake;

        let mut shutdown = Vec::with_capacity(config.shutdown.len());
        for ani in &config.shutdown {
            shutdown.push(ActionData::from_anime_action(anime_type, ani)?);
        }
        self.shutdown = shutdown;
        Ok(())
    }
}

/// Config for base system actions for the anime display
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AniMeConfig {
    #[serde(skip)]
    /// Runtime-only display type detected from DMI; not written to the file.
    pub anime_type: AnimeType,
    /// Actions played by the `run_main_loop` D-Bus method. This is the base
    /// system sequence used by tools; it is not the boot animation.
    pub system: Vec<ActionLoader>,
    /// Actions played while booting. `Default` is empty; a newly created file
    /// gets one sample action.
    pub boot: Vec<ActionLoader>,
    /// Actions played when waking from suspend. `Default` is empty; a newly
    /// created file gets one sample action.
    pub wake: Vec<ActionLoader>,
    /// Actions played during shutdown. `Default` is empty; a newly created
    /// file gets one sample action.
    pub shutdown: Vec<ActionLoader>,
    // pub brightness: f32,
    /// Enable the display.
    pub display_enabled: bool,
    /// Global brightness: `Off`, `Low`, `Med`, or `High`.
    pub display_brightness: Brightness,
    /// Use the built-in animations instead of the custom `boot`/`wake`/`shutdown` actions.
    pub builtin_anims_enabled: bool,
    /// Turn the display off when external power is unplugged.
    pub off_when_unplugged: bool,
    /// Turn the display off when the laptop suspends.
    pub off_when_suspended: bool,
    /// Turn the display off when the lid is closed.
    pub off_when_lid_closed: bool,
    /// Brightness used on battery power: `Off`, `Low`, `Med`, or `High`.
    pub brightness_on_battery: Brightness,
    /// Built-in animation selection for each power state; see the table below.
    pub builtin_anims: Animations,
}

impl Default for AniMeConfig {
    fn default() -> Self {
        AniMeConfig {
            anime_type: AnimeType::GA402,
            system: Vec::new(),
            boot: Vec::new(),
            wake: Vec::new(),
            shutdown: Vec::new(),
            // brightness: 1.0,
            display_enabled: true,
            display_brightness: Brightness::Med,
            builtin_anims_enabled: true,
            off_when_unplugged: true,
            off_when_suspended: true,
            off_when_lid_closed: true,
            brightness_on_battery: Brightness::Low,
            builtin_anims: Animations::default(),
        }
    }
}

impl StdConfig for AniMeConfig {
    fn new() -> Self {
        Self::create_default()
    }

    fn file_name(&self) -> String {
        CONFIG_FILE.to_owned()
    }

    fn config_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(crate::CONFIG_PATH_BASE)
    }
}

impl StdConfigLoad for AniMeConfig {}

impl From<&AniMeConfig> for DeviceState {
    fn from(config: &AniMeConfig) -> Self {
        DeviceState {
            display_enabled: config.display_enabled,
            display_brightness: config.display_brightness,
            builtin_anims_enabled: config.builtin_anims_enabled,
            builtin_anims: config.builtin_anims,
            off_when_unplugged: config.off_when_unplugged,
            off_when_suspended: config.off_when_suspended,
            off_when_lid_closed: config.off_when_lid_closed,
            brightness_on_battery: config.brightness_on_battery,
        }
    }
}

impl AniMeConfig {
    // fn clamp_config_brightness(mut config: &mut AnimeConfig) {
    //     if config.brightness < 0.0 || config.brightness > 1.0 {
    //         warn!(
    //             "Clamped brightness to [0.0 ; 1.0], was {}",
    //             config.brightness
    //         );
    //         config.brightness = f32::max(0.0, f32::min(1.0, config.brightness));
    //     }
    // }

    fn create_default() -> Self {
        // create a default config here
        AniMeConfig {
            system: vec![],
            boot: vec![
                ActionLoader::ImageAnimation {
                    file: "/usr/share/asusd/anime/custom/sonic-run.gif".into(),
                    scale: 0.9,
                    angle: 0.65,
                    translation: Vec2::default(),
                    brightness: 1.0,
                    time: AnimTime::Fade(Fade::new(
                        Duration::from_secs(2),
                        Some(Duration::from_secs(2)),
                        Duration::from_secs(2),
                    )),
                },
            ],
            wake: vec![
                ActionLoader::ImageAnimation {
                    file: "/usr/share/asusd/anime/custom/sonic-run.gif".into(),
                    scale: 0.9,
                    angle: 0.65,
                    translation: Vec2::default(),
                    brightness: 1.0,
                    time: AnimTime::Fade(Fade::new(
                        Duration::from_secs(2),
                        Some(Duration::from_secs(2)),
                        Duration::from_secs(2),
                    )),
                },
            ],
            shutdown: vec![
                ActionLoader::ImageAnimation {
                    file: "/usr/share/asusd/anime/custom/sonic-wait.gif".into(),
                    scale: 0.9,
                    angle: 0.0,
                    translation: Vec2::new(3.0, 2.0),
                    brightness: 1.0,
                    time: AnimTime::Infinite,
                },
            ],
            ..Default::default()
        }
    }
}
