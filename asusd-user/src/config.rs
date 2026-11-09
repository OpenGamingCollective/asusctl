use std::path::PathBuf;
use std::time::Duration;

use config_traits::{StdConfig, StdConfigLoad};
use rog_anime::{ActionLoader, AnimTime, AnimeType, Fade, Sequences as AnimeSequences, Vec2};
use rog_aura::effects::{AdvancedEffects as AuraSequences, Breathe, DoomFlicker, Effect, Static};
use rog_aura::keyboard::LedCode;
use rog_aura::{Colour, Speed};
use serde::{Deserialize, Serialize};

use crate::error::Error;

const ROOT_CONF_DIR: &str = "rog";

fn root_conf_dir() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    dir.push(ROOT_CONF_DIR);
    dir
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigAnime {
    /// Must match the file stem.
    pub name: String,
    /// Ordered list of `ActionLoader` actions.
    pub anime: Vec<ActionLoader>,
}

impl ConfigAnime {
    pub fn create(&self, anime_type: AnimeType) -> Result<AnimeSequences, Error> {
        let mut seq = AnimeSequences::new(anime_type);

        for (idx, action) in self.anime.iter().enumerate() {
            seq.insert(idx, action)?;
        }

        Ok(seq)
    }

    pub fn set_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
}

impl Default for ConfigAnime {
    fn default() -> Self {
        Self {
            name: "anime-default".to_owned(),
            anime: vec![
                ActionLoader::AsusImage {
                    file: "/usr/share/asusd/anime/custom/diagonal-template.png".into(),
                    brightness: 1.0,
                    time: AnimTime::Fade(Fade::new(
                        Duration::from_secs(2),
                        None,
                        Duration::from_secs(2),
                    )),
                },
                ActionLoader::AsusAnimation {
                    file: "/usr/share/asusd/anime/asus/rog/Sunset.gif".into(),
                    brightness: 0.5,
                    time: AnimTime::Fade(Fade::new(
                        Duration::from_secs(6),
                        None,
                        Duration::from_secs(3),
                    )),
                },
                ActionLoader::ImageAnimation {
                    file: "/usr/share/asusd/anime/custom/sonic-run.gif".into(),
                    scale: 0.9,
                    angle: 0.65,
                    translation: Vec2::default(),
                    brightness: 0.5,
                    time: AnimTime::Fade(Fade::new(
                        Duration::from_secs(2),
                        Some(Duration::from_secs(2)),
                        Duration::from_secs(2),
                    )),
                },
                ActionLoader::Image {
                    file: "/usr/share/asusd/anime/custom/rust.png".into(),
                    scale: 1.0,
                    angle: 0.0,
                    translation: Vec2::default(),
                    time: AnimTime::Fade(Fade::new(
                        Duration::from_secs(2),
                        Some(Duration::from_secs(1)),
                        Duration::from_secs(2),
                    )),
                    brightness: 0.6,
                },
                ActionLoader::Pause(Duration::from_secs(1)),
                ActionLoader::ImageAnimation {
                    file: "/usr/share/asusd/anime/custom/sonic-wait.gif".into(),
                    scale: 0.9,
                    angle: 0.0,
                    translation: Vec2::new(3.0, 2.0),
                    brightness: 0.5,
                    time: AnimTime::Count(2),
                },
            ],
        }
    }
}

impl StdConfig for ConfigAnime {
    fn new() -> Self {
        Self::default()
    }

    fn file_name(&self) -> String {
        format!("{}.ron", self.name)
    }

    fn config_dir() -> std::path::PathBuf {
        root_conf_dir()
    }
}

impl StdConfigLoad for ConfigAnime {}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigAura {
    /// Must match the file stem.
    pub name: String,
    /// `AdvancedEffects` value with the ordered effect list and the packet layout.
    pub aura: AuraSequences,
}

impl ConfigAura {
    pub fn set_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
}

impl Default for ConfigAura {
    fn default() -> Self {
        let mut seq = AuraSequences::new(false);
        let mut key = Effect::Breathe(Breathe::new(
            LedCode::W,
            Colour {
                r: 255,
                g: 0,
                b: 20,
            },
            Colour {
                r: 20,
                g: 255,
                b: 0,
            },
            Speed::Low,
        ));

        seq.push(key.clone());
        key.set_led(LedCode::A);
        seq.push(key.clone());
        key.set_led(LedCode::S);
        seq.push(key.clone());
        key.set_led(LedCode::D);
        seq.push(key);

        let key = Effect::Breathe(Breathe::new(
            LedCode::F,
            Colour { r: 255, g: 0, b: 0 },
            Colour { r: 255, g: 0, b: 0 },
            Speed::High,
        ));
        seq.push(key);

        let mut key = Effect::Static(Static::new(LedCode::RCtrl, Colour { r: 0, g: 0, b: 255 }));
        seq.push(key.clone());
        key.set_led(LedCode::LCtrl);
        seq.push(key.clone());
        key.set_led(LedCode::Esc);
        seq.push(key);

        let key = Effect::DoomFlicker(DoomFlicker::new(
            LedCode::N9,
            Colour { r: 0, g: 0, b: 255 },
            80,
            40,
        ));
        seq.push(key);

        Self {
            name: "aura-default".to_owned(),
            aura: seq,
        }
    }
}

impl StdConfig for ConfigAura {
    fn new() -> Self {
        Self::default()
    }

    fn file_name(&self) -> String {
        format!("{}.ron", self.name)
    }

    fn config_dir() -> std::path::PathBuf {
        root_conf_dir()
    }
}

impl StdConfigLoad for ConfigAura {}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ConfigBase {
    /// Name of active anime config file in the user config directory
    pub active_anime: Option<String>,
    /// Name of active aura config file in the user config directory
    pub active_aura: Option<String>,
}

impl StdConfig for ConfigBase {
    fn new() -> Self {
        Self {
            active_anime: Some("anime-default".to_owned()),
            active_aura: Some("aura-default".to_owned()),
        }
    }

    fn file_name(&self) -> String {
        "rog-user.ron".to_owned()
    }

    fn config_dir() -> std::path::PathBuf {
        root_conf_dir()
    }
}

impl StdConfigLoad for ConfigBase {}
