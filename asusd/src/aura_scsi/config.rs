use std::collections::BTreeMap;

use config_traits::{StdConfig, StdConfigLoad};
use rog_aura::AuraDeviceType;
use rog_scsi::{AuraEffect, AuraMode};
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "scsi.ron";

/// Config for base system actions for the anime display
#[derive(Deserialize, Serialize, Debug)]
pub struct ScsiConfig {
    #[serde(skip)]
    pub dev_type: AuraDeviceType,
    pub enabled: bool,
    pub current_mode: AuraMode,
    pub modes: BTreeMap<AuraMode, AuraEffect>,
}

impl ScsiConfig {
    pub fn get_effect(&mut self, mode: AuraMode) -> Option<&AuraEffect> {
        self.modes.get(&mode)
    }

    pub fn save_effect(&mut self, effect: AuraEffect) {
        self.current_mode = effect.mode;
        self.modes.insert(*effect.mode(), effect);
    }
}

impl Default for ScsiConfig {
    fn default() -> Self {
        ScsiConfig {
            enabled: true,
            current_mode: AuraMode::Static,
            dev_type: AuraDeviceType::ScsiExtDisk,
            modes: BTreeMap::from([
                (AuraMode::Off, AuraEffect::default_with_mode(AuraMode::Off)),
                (
                    AuraMode::Static,
                    AuraEffect::default_with_mode(AuraMode::Static),
                ),
                (
                    AuraMode::Breathe,
                    AuraEffect::default_with_mode(AuraMode::Breathe),
                ),
                (
                    AuraMode::Flashing,
                    AuraEffect::default_with_mode(AuraMode::Flashing),
                ),
                (
                    AuraMode::RainbowCycle,
                    AuraEffect::default_with_mode(AuraMode::RainbowCycle),
                ),
                (
                    AuraMode::RainbowWave,
                    AuraEffect::default_with_mode(AuraMode::RainbowWave),
                ),
            ]),
        }
    }
}

impl StdConfig for ScsiConfig {
    fn new() -> Self {
        Self::default()
    }

    fn file_name(&self) -> String {
        CONFIG_FILE.to_owned()
    }

    fn config_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(crate::CONFIG_PATH_BASE)
    }
}

impl StdConfigLoad for ScsiConfig {}
