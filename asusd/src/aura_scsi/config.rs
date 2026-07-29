use std::collections::BTreeMap;

use config_traits::{StdConfig, StdConfigLoad};
use rog_aura::AuraDeviceType;
use rog_platform::scsi::{AuraEffect, AuraMode};
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
                (
                    AuraMode::RainbowCycleBreathe,
                    AuraEffect::default_with_mode(AuraMode::RainbowCycleBreathe),
                ),
                (
                    AuraMode::ChaseFade,
                    AuraEffect::default_with_mode(AuraMode::ChaseFade),
                ),
                (
                    AuraMode::RainbowCycleChaseFade,
                    AuraEffect::default_with_mode(AuraMode::RainbowCycleChaseFade),
                ),
                (
                    AuraMode::Chase,
                    AuraEffect::default_with_mode(AuraMode::Chase),
                ),
                (
                    AuraMode::RainbowCycleChase,
                    AuraEffect::default_with_mode(AuraMode::RainbowCycleChase),
                ),
                (
                    AuraMode::RainbowCycleWave,
                    AuraEffect::default_with_mode(AuraMode::RainbowCycleWave),
                ),
                (
                    AuraMode::RainbowPulseChase,
                    AuraEffect::default_with_mode(AuraMode::RainbowPulseChase),
                ),
                (
                    AuraMode::RandomFlicker,
                    AuraEffect::default_with_mode(AuraMode::RandomFlicker),
                ),
                (
                    AuraMode::DoubleFade,
                    AuraEffect::default_with_mode(AuraMode::DoubleFade),
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
