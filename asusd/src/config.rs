use std::collections::HashMap;

use config_traits::{StdConfig, StdConfigLoad2};
use rog_platform::asus_armoury::FirmwareAttribute;
use rog_platform::cpu::CPUEPP;
use rog_platform::platform::PlatformProfile;
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "asusd.ron";

/// Default value for base_charge_control_end_threshold when not present in config.
/// Returns 0 so restore_charge_limit() skips restoration for upgraded configs.
fn default_base_charge_limit() -> u8 {
    0
}

#[derive(Default, Clone, Deserialize, Serialize, PartialEq)]
pub struct Tuning {
    pub enabled: bool,
    pub group: HashMap<FirmwareAttribute, i32>,
}
type Tunings = HashMap<PlatformProfile, Tuning>;

#[derive(Deserialize, Serialize, PartialEq)]
pub struct Config {
    /// Current battery charge limit percentage. Set it with
    /// `asusctl battery limit 80` (the accepted range is 20-100).
    pub charge_control_end_threshold: u8,
    /// Charge limit restored after a one-shot full charge. `0` means "not set"
    /// for a config upgraded from an older version.
    #[serde(default = "default_base_charge_limit")]
    pub base_charge_control_end_threshold: u8,
    /// Stop `nvidia-powerd` while on battery and start it again on AC.
    pub disable_nvidia_powerd_on_battery: bool,
    /// Command run when AC power is connected.
    pub ac_command: String,
    /// Command run when AC power is disconnected.
    pub bat_command: String,
    /// Set the CPU energy-performance preference when the platform profile changes.
    pub platform_profile_linked_epp: bool,
    /// Profile applied when the laptop switches to battery.
    pub platform_profile_on_battery: PlatformProfile,
    /// Apply `platform_profile_on_battery` on the power-source change.
    pub change_platform_profile_on_battery: bool,
    /// Profile applied when AC power is connected.
    pub platform_profile_on_ac: PlatformProfile,
    /// Apply `platform_profile_on_ac` on the power-source change.
    pub change_platform_profile_on_ac: bool,
    /// EPP value used for the Quiet profile.
    pub profile_quiet_epp: CPUEPP,
    /// EPP value used for the Balanced profile.
    pub profile_balanced_epp: CPUEPP,
    /// EPP value used for the Custom profile.
    pub profile_custom_epp: CPUEPP,
    /// EPP value used for the Performance profile.
    pub profile_performance_epp: CPUEPP,
    /// Per-profile PPT tuning groups used on AC power.
    pub ac_profile_tunings: Tunings,
    /// Per-profile PPT tuning groups used on battery.
    pub dc_profile_tunings: Tunings,
    /// Persisted firmware-attribute values applied at startup.
    pub armoury_settings: HashMap<FirmwareAttribute, i32>,
    /// ScreenPad gamma. `None` behaves as `1.0` (linear).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub screenpad_gamma: Option<f32>,
    /// Sync ScreenPad brightness with the primary display. `None` behaves as disabled.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub screenpad_sync_primary: Option<bool>,
    /// Persisted XG Mobile LED state: `None` = never set, `Some(false)` = off,
    /// `Some(true)` = on. Re-applied when the device appears.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub xgm_led_enabled: Option<bool>,
    /// Runtime-only AC/battery state; not written to the file.
    #[serde(skip)]
    pub last_power_plugged: u8,
}

impl Config {
    pub fn select_tunings(&mut self, power_plugged: bool, profile: PlatformProfile) -> &mut Tuning {
        let config = if power_plugged {
            &mut self.ac_profile_tunings
        } else {
            &mut self.dc_profile_tunings
        };
        config.entry(profile).or_insert_with(Tuning::default)
    }

    pub fn select_tunings_ref(
        &self,
        power_plugged: bool,
        profile: PlatformProfile,
    ) -> Option<&Tuning> {
        let config = if power_plugged {
            &self.ac_profile_tunings
        } else {
            &self.dc_profile_tunings
        };
        config.get(&profile)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            charge_control_end_threshold: 100,
            // NOTE: This is intentionally 100 (not 0 like the serde default).
            // New installs get 100 (no limit). Upgraded configs missing this
            // field get 0 via serde, which skips restore_charge_limit().
            base_charge_control_end_threshold: 100,
            disable_nvidia_powerd_on_battery: true,
            ac_command: Default::default(),
            bat_command: Default::default(),
            platform_profile_linked_epp: true,
            platform_profile_on_battery: PlatformProfile::Quiet,
            change_platform_profile_on_battery: true,
            platform_profile_on_ac: PlatformProfile::Performance,
            change_platform_profile_on_ac: true,
            profile_quiet_epp: CPUEPP::Power,
            profile_balanced_epp: CPUEPP::BalancePower,
            profile_performance_epp: CPUEPP::Performance,
            profile_custom_epp: CPUEPP::Performance,
            ac_profile_tunings: HashMap::default(),
            dc_profile_tunings: HashMap::default(),
            armoury_settings: HashMap::default(),
            last_power_plugged: Default::default(),
            screenpad_gamma: Default::default(),
            screenpad_sync_primary: Default::default(),
            xgm_led_enabled: Default::default(),
        }
    }
}

impl StdConfig for Config {
    fn new() -> Self {
        Config {
            charge_control_end_threshold: 100,
            disable_nvidia_powerd_on_battery: true,
            platform_profile_on_battery: PlatformProfile::Quiet,
            platform_profile_on_ac: PlatformProfile::Performance,
            ac_command: String::new(),
            bat_command: String::new(),
            ..Default::default()
        }
    }

    fn file_name(&self) -> String {
        CONFIG_FILE.to_owned()
    }

    fn config_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(crate::CONFIG_PATH_BASE)
    }
}

impl StdConfigLoad2<Config611, Config601> for Config {}

#[derive(Deserialize, Serialize)]
pub struct Config611 {
    pub charge_control_end_threshold: u8,
    #[serde(skip)]
    pub base_charge_control_end_threshold: u8,
    pub disable_nvidia_powerd_on_battery: bool,
    pub ac_command: String,
    pub bat_command: String,
    pub platform_profile_linked_epp: bool,
    pub platform_profile_on_battery: PlatformProfile,
    pub change_platform_profile_on_battery: bool,
    pub platform_profile_on_ac: PlatformProfile,
    pub change_platform_profile_on_ac: bool,
    pub profile_quiet_epp: CPUEPP,
    pub profile_balanced_epp: CPUEPP,
    pub profile_custom_epp: CPUEPP,
    pub profile_performance_epp: CPUEPP,
    pub ac_profile_tunings: Tunings,
    pub dc_profile_tunings: Tunings,
    pub armoury_settings: HashMap<FirmwareAttribute, i32>,
    #[serde(skip)]
    pub last_power_plugged: u8,
}

impl From<Config611> for Config {
    fn from(c: Config611) -> Self {
        let mut config = Self {
            // Restore the base charge limit
            charge_control_end_threshold: c.charge_control_end_threshold,
            base_charge_control_end_threshold: c.charge_control_end_threshold,
            disable_nvidia_powerd_on_battery: c.disable_nvidia_powerd_on_battery,
            ac_command: c.ac_command,
            bat_command: c.bat_command,
            platform_profile_linked_epp: c.platform_profile_linked_epp,
            platform_profile_on_battery: c.platform_profile_on_battery,
            change_platform_profile_on_battery: c.change_platform_profile_on_battery,
            platform_profile_on_ac: c.platform_profile_on_ac,
            change_platform_profile_on_ac: c.change_platform_profile_on_ac,
            profile_quiet_epp: c.profile_quiet_epp,
            profile_balanced_epp: c.profile_balanced_epp,
            profile_performance_epp: c.profile_performance_epp,
            profile_custom_epp: c.profile_performance_epp,
            last_power_plugged: c.last_power_plugged,
            ac_profile_tunings: HashMap::default(),
            dc_profile_tunings: HashMap::default(),
            armoury_settings: HashMap::default(),
            screenpad_gamma: None,
            screenpad_sync_primary: Default::default(),
            xgm_led_enabled: Default::default(),
        };

        config.ac_profile_tunings = c.ac_profile_tunings;
        config.dc_profile_tunings = c.dc_profile_tunings;
        config.armoury_settings = c.armoury_settings;
        config
    }
}

#[derive(Deserialize, Serialize)]
pub struct Config601 {
    pub charge_control_end_threshold: u8,
    #[serde(skip)]
    pub base_charge_control_end_threshold: u8,
    pub panel_od: bool,
    pub boot_sound: bool,
    pub mini_led_mode: bool,
    pub disable_nvidia_powerd_on_battery: bool,
    pub ac_command: String,
    pub bat_command: String,
    pub platform_profile_linked_epp: bool,
    pub platform_profile_on_battery: PlatformProfile,
    pub change_platform_profile_on_battery: bool,
    pub platform_profile_on_ac: PlatformProfile,
    pub change_platform_profile_on_ac: bool,
    pub profile_quiet_epp: CPUEPP,
    pub profile_balanced_epp: CPUEPP,
    pub profile_performance_epp: CPUEPP,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ppt_pl1_spl: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ppt_pl2_sppt: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ppt_pl3_fppt: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ppt_fppt: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ppt_apu_sppt: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ppt_platform_sppt: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub nv_dynamic_boost: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub nv_temp_target: Option<u8>,
    #[serde(skip)]
    pub last_power_plugged: u8,
}

impl From<Config601> for Config {
    fn from(c: Config601) -> Self {
        Self {
            // Restore the base charge limit
            charge_control_end_threshold: c.charge_control_end_threshold,
            base_charge_control_end_threshold: c.charge_control_end_threshold,
            disable_nvidia_powerd_on_battery: c.disable_nvidia_powerd_on_battery,
            ac_command: c.ac_command,
            bat_command: c.bat_command,
            platform_profile_linked_epp: c.platform_profile_linked_epp,
            platform_profile_on_battery: c.platform_profile_on_battery,
            change_platform_profile_on_battery: c.change_platform_profile_on_battery,
            platform_profile_on_ac: c.platform_profile_on_ac,
            change_platform_profile_on_ac: c.change_platform_profile_on_ac,
            profile_quiet_epp: c.profile_quiet_epp,
            profile_balanced_epp: c.profile_balanced_epp,
            profile_performance_epp: c.profile_performance_epp,
            profile_custom_epp: c.profile_performance_epp,
            last_power_plugged: c.last_power_plugged,
            ac_profile_tunings: HashMap::default(),
            dc_profile_tunings: HashMap::default(),
            armoury_settings: HashMap::default(),
            screenpad_gamma: None,
            screenpad_sync_primary: Default::default(),
            xgm_led_enabled: Default::default(),
        }
    }
}
