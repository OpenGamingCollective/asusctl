use std::fmt::Display;
use std::path::{Path, PathBuf};

use log::{info, warn};
use serde::{Deserialize, Serialize};
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::error::{PlatformError, Result};
use crate::{
    has_attr, read_attr_string, read_attr_string_array, to_device, watch_attr, write_attr_string,
};

/// The "platform" device provides access to things like:
/// - `dgpu_disable`
/// - `egpu_enable`
/// - `panel_od`
/// - `gpu_mux`
/// - various CPU an GPU tunings
/// - `keyboard_mode`, set keyboard RGB mode and speed
/// - `keyboard_state`, set keyboard power states
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct RogPlatform {
    path: PathBuf,
    pp_path: PathBuf,
    /// `/sys/class/platform-profile`, one directory per profile handler
    pp_class_path: PathBuf,
}

impl RogPlatform {
    has_attr!("platform_profile" pp_path);

    watch_attr!("platform_profile" pp_path);

    has_attr!("platform_profile_choices" pp_path);

    watch_attr!("platform_profile_choices" pp_path);

    /// The active profile as named by the kernel.
    ///
    /// If Quiet was set per handler (see [`Self::set_platform_profile`]) the
    /// legacy interface reads `custom`, this reports `quiet` instead.
    pub fn get_platform_profile(&self) -> Result<String> {
        let profile = read_attr_string(&to_device(&self.pp_path)?, "platform_profile")?;
        if profile.trim() == "custom"
            && handlers_in_low_power(&read_profile_handlers(&self.pp_class_path))
        {
            return Ok(<&str>::from(PlatformProfile::Quiet).to_owned());
        }
        Ok(profile)
    }

    /// Set the profile through the legacy interface. `quiet` or `low-power`
    /// is written to each handler instead when the legacy interface offers
    /// neither but every handler has one of them.
    pub fn set_platform_profile(&self, profile: &str) -> Result<()> {
        let low_power = matches!(
            profile.parse::<PlatformProfile>(),
            Ok(PlatformProfile::Quiet | PlatformProfile::LowPower)
        );
        if low_power && !has_low_power(&self.get_legacy_platform_profile_choices()?) {
            let handlers = read_profile_handlers(&self.pp_class_path);
            if let Some(targets) = low_power_targets(&handlers) {
                info!("Setting low power profile per handler: {targets:?}");
                return write_handler_profiles(&targets);
            }
        }
        write_attr_string(&mut to_device(&self.pp_path)?, "platform_profile", profile)
    }

    /// The profiles the kernel accepts: the legacy choices, plus `Quiet` if
    /// only the per-handler interface can reach it.
    pub fn get_platform_profile_choices(&self) -> Result<Vec<PlatformProfile>> {
        let legacy = self.get_legacy_platform_profile_choices()?;
        Ok(effective_choices(
            legacy,
            &read_profile_handlers(&self.pp_class_path),
        ))
    }

    fn get_legacy_platform_profile_choices(&self) -> Result<Vec<PlatformProfile>> {
        read_attr_string_array(&to_device(&self.pp_path)?, "platform_profile_choices")
    }

    pub fn new() -> Result<Self> {
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("platform").map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;
        enumerator.match_sysname("asus-nb-wmi").map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        if let Some(device) = (enumerator.scan_devices().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("scan_devices failed".into(), err)
        })?)
        .next()
        {
            info!("Found platform support at {:?}", device.sysname());
            return Ok(Self {
                path: device.syspath().to_owned(),
                pp_path: PathBuf::from("/sys/firmware/acpi"),
                pp_class_path: PathBuf::from("/sys/class/platform-profile"),
            });
        }
        Err(PlatformError::MissingFunction(
            "asus-nb-wmi not found".into(),
        ))
    }
}

impl Default for RogPlatform {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            pp_path: PathBuf::new(),
            pp_class_path: PathBuf::new(),
        }
    }
}

/// Names the kernel uses for the low power profile, in order of preference
const LOW_POWER_NAMES: [&str; 2] = [
    "quiet", "low-power",
];

/// A `/sys/class/platform-profile/platform-profile-*` handler.
///
/// The legacy `/sys/firmware/acpi/platform_profile_choices` only lists what
/// every handler offers. On Intel Panther Lake the Intel power slider offers
/// `low-power` and asus-wmi offers `quiet`, so the legacy interface has
/// neither (#387). Each handler still takes its own name directly.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ProfileHandler {
    path: PathBuf,
    choices: Vec<String>,
    profile: String,
}

/// Read every handler, sorted by path. Empty if the class is missing or a
/// handler can't be read, which disables the per-handler fallback.
fn read_profile_handlers(class_path: &Path) -> Vec<ProfileHandler> {
    let Ok(entries) = std::fs::read_dir(class_path) else {
        return Vec::new();
    };
    let handlers: Option<Vec<ProfileHandler>> = entries
        .flatten()
        .map(|entry| {
            let path = entry.path();
            let choices = std::fs::read_to_string(path.join("choices")).ok()?;
            let profile = std::fs::read_to_string(path.join("profile")).ok()?;
            Some(ProfileHandler {
                path,
                choices: choices.split_whitespace().map(str::to_owned).collect(),
                profile: profile.trim().to_owned(),
            })
        })
        .collect();
    let mut handlers = handlers.unwrap_or_default();
    handlers.sort();
    handlers
}

/// What to write to each handler for the low power profile: `quiet` where
/// offered, else `low-power`. `None` unless every handler has one of them.
fn low_power_targets(handlers: &[ProfileHandler]) -> Option<Vec<(&Path, &'static str)>> {
    if handlers.is_empty() {
        return None;
    }
    handlers
        .iter()
        .map(|handler| {
            LOW_POWER_NAMES
                .into_iter()
                .find(|name| handler.choices.iter().any(|c| c == name))
                .map(|name| (handler.path.as_path(), name))
        })
        .collect()
}

/// Whether every handler is in its low power profile. The legacy interface
/// reads `custom` then, as the handlers disagree on the name.
fn handlers_in_low_power(handlers: &[ProfileHandler]) -> bool {
    !handlers.is_empty()
        && handlers
            .iter()
            .all(|handler| LOW_POWER_NAMES.contains(&handler.profile.as_str()))
}

fn write_handler_profiles(targets: &[(&Path, &str)]) -> Result<()> {
    for (path, profile) in targets {
        let file = path.join("profile");
        std::fs::write(&file, profile)
            .map_err(|e| PlatformError::Write(file.display().to_string(), e))?;
    }
    Ok(())
}

fn has_low_power(choices: &[PlatformProfile]) -> bool {
    choices.contains(&PlatformProfile::Quiet) || choices.contains(&PlatformProfile::LowPower)
}

/// Add `Quiet` to the legacy choices if only the handlers can reach it
fn effective_choices(
    mut choices: Vec<PlatformProfile>,
    handlers: &[ProfileHandler],
) -> Vec<PlatformProfile> {
    if !has_low_power(&choices) && low_power_targets(handlers).is_some() {
        // The kernel lists choices from low to high power
        choices.insert(0, PlatformProfile::Quiet);
    }
    choices
}

/// Parse a kernel choices list, skipping names `PlatformProfile` can't
/// represent (such as `cool` or `balanced-performance`) instead of turning
/// them into `Balanced`
pub(crate) fn parse_profile_choices(choices: &str) -> Vec<PlatformProfile> {
    choices
        .split_whitespace()
        .filter_map(|name| name.parse().ok())
        .collect()
}

#[repr(u8)]
#[derive(
    Serialize, Deserialize, Default, Type, Value, OwnedValue, Debug, PartialEq, Eq, Clone, Copy,
)]
pub enum GpuMode {
    Optimus = 0,
    Integrated = 1,
    Egpu = 2,
    Vfio = 3,
    Ultimate = 4,
    #[default]
    Error = 254,
    NotSupported = 255,
}

impl From<u8> for GpuMode {
    fn from(v: u8) -> Self {
        match v {
            0 => GpuMode::Optimus,
            1 => GpuMode::Integrated,
            2 => GpuMode::Egpu,
            3 => GpuMode::Vfio,
            4 => GpuMode::Ultimate,
            5 => GpuMode::Error,
            _ => GpuMode::NotSupported,
        }
    }
}

impl From<GpuMode> for u8 {
    fn from(v: GpuMode) -> Self {
        v as u8
    }
}

impl Display for GpuMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuMode::Optimus => write!(f, "Optimus"),
            GpuMode::Integrated => write!(f, "Integrated"),
            GpuMode::Egpu => write!(f, "eGPU"),
            GpuMode::Vfio => write!(f, "VFIO"),
            GpuMode::Ultimate => write!(f, "Ultimate"),
            GpuMode::Error => write!(f, "Error"),
            GpuMode::NotSupported => write!(f, "Not Supported"),
        }
    }
}

#[repr(u32)]
#[derive(
    Deserialize,
    Serialize,
    Default,
    Type,
    Value,
    OwnedValue,
    Debug,
    PartialEq,
    Eq,
    Ord,
    PartialOrd,
    Hash,
    Clone,
    Copy,
)]
#[zvariant(signature = "u")]
/// `platform_profile` in asus_wmi
pub enum PlatformProfile {
    #[default]
    Balanced = 0,
    Performance = 1,
    Quiet = 2,
    LowPower = 3,
    Custom = 4,
}

impl PlatformProfile {
    /// The profile after `current` in the kernel's `choices`, wrapping
    /// around. The kernel lists them from low to high power, and only
    /// profiles it offers are picked, so cycling can't get stuck on one it
    /// rejects. From `Custom` or a profile not offered, go to `Balanced`.
    pub fn next(current: Self, choices: &[Self]) -> Self {
        match choices.iter().position(|p| *p == current) {
            Some(i) => choices[(i + 1) % choices.len()],
            // Custom, or not offered by the kernel
            None if choices.contains(&Self::Balanced) => Self::Balanced,
            None => choices.first().copied().unwrap_or_default(),
        }
    }

    /// `Quiet` and `LowPower` are the same semantic profile; which name the
    /// kernel exposes depends on the registered platform_profile handlers.
    /// Substitute the equivalent name when the requested one is unavailable.
    pub fn resolve_alias(self, choices: &[Self]) -> Self {
        if choices.contains(&self) {
            return self;
        }
        let alias = match self {
            Self::Quiet => Self::LowPower,
            Self::LowPower => Self::Quiet,
            _ => return self,
        };
        if choices.contains(&alias) {
            info!("Profile {self} is not exposed by the kernel, using {alias} instead");
            alias
        } else {
            self
        }
    }
}

impl From<i32> for PlatformProfile {
    fn from(num: i32) -> Self {
        match num {
            0 => Self::Balanced,
            1 => Self::Performance,
            2 => Self::Quiet,
            3 => Self::LowPower,
            4 => Self::Custom,
            _ => {
                warn!("Unknown number for PlatformProfile: {}", num);
                Self::Balanced
            }
        }
    }
}

impl From<PlatformProfile> for i32 {
    fn from(p: PlatformProfile) -> Self {
        p as i32
    }
}

impl From<&PlatformProfile> for &str {
    fn from(profile: &PlatformProfile) -> &'static str {
        match profile {
            PlatformProfile::Balanced => "balanced",
            PlatformProfile::Performance => "performance",
            PlatformProfile::Quiet => "quiet",
            PlatformProfile::LowPower => "low-power",
            PlatformProfile::Custom => "custom",
        }
    }
}

impl From<PlatformProfile> for &str {
    fn from(profile: PlatformProfile) -> &'static str {
        <&str>::from(&profile)
    }
}

impl From<String> for PlatformProfile {
    fn from(profile: String) -> Self {
        Self::from(profile.as_str())
    }
}

impl Display for PlatformProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", <&str>::from(self))
    }
}

impl std::str::FromStr for PlatformProfile {
    type Err = PlatformError;

    fn from_str(profile: &str) -> Result<Self> {
        match profile
            .to_ascii_lowercase()
            .trim()
            .replace(|c| !char::is_alphabetic(c), "")
            .as_str()
        {
            "balanced" => Ok(PlatformProfile::Balanced),
            "performance" => Ok(PlatformProfile::Performance),
            "quiet" => Ok(PlatformProfile::Quiet),
            "lowpower" => Ok(PlatformProfile::LowPower),
            "custom" => Ok(PlatformProfile::Custom),
            _ => Err(PlatformError::NotSupported),
        }
    }
}

impl From<&str> for PlatformProfile {
    fn from(profile: &str) -> Self {
        match profile
            .to_ascii_lowercase()
            .trim()
            .replace(|c| !char::is_alphabetic(c), "")
            .as_str()
        {
            "balanced" => PlatformProfile::Balanced,
            "performance" => PlatformProfile::Performance,
            "quiet" => PlatformProfile::Quiet,
            "lowpower" => PlatformProfile::LowPower,
            "custom" => PlatformProfile::Custom,
            _ => {
                warn!("{profile} is unknown, using ThrottlePolicy::Balanced");
                PlatformProfile::Balanced
            }
        }
    }
}

/// CamelCase names of the properties. Intended for use with DBUS
#[repr(u8)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, PartialOrd)]
#[zvariant(signature = "s")]
pub enum Properties {
    ChargeControlEndThreshold,
    DgpuDisable,
    GpuMuxMode,
    PostAnimationSound,
    PanelOd,
    MiniLedMode,
    EgpuEnable,
    ThrottlePolicy,
}

pub fn get_fan_rpms() -> (i32, i32, i32) {
    let mut cpu = 0;
    let mut gpu = 0;
    let mut mid = 0;
    if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(name) = std::fs::read_to_string(path.join("name"))
                && name.trim() == "asus"
            {
                if let Ok(v) = std::fs::read_to_string(path.join("fan1_input")) {
                    cpu = v.trim().parse().unwrap_or(0);
                }
                if let Ok(v) = std::fs::read_to_string(path.join("fan2_input")) {
                    gpu = v.trim().parse().unwrap_or(0);
                }
                if let Ok(v) = std::fs::read_to_string(path.join("fan3_input")) {
                    mid = v.trim().parse().unwrap_or(0);
                }
                break;
            }
        }
    }
    (cpu, gpu, mid)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::platform::{
        PlatformProfile, effective_choices, handlers_in_low_power, low_power_targets,
        parse_profile_choices, read_profile_handlers, write_handler_profiles,
    };

    // asus-wmi only ever exposes these
    const ASUS_WMI: &[PlatformProfile] = &[
        PlatformProfile::Quiet,
        PlatformProfile::Balanced,
        PlatformProfile::Performance,
    ];
    // amd-pmf exposes low-power instead, with quiet as a hidden choice
    const AMD_PMF: &[PlatformProfile] = &[
        PlatformProfile::LowPower,
        PlatformProfile::Balanced,
        PlatformProfile::Performance,
    ];

    #[test]
    fn alias_substitutes_when_name_is_absent() {
        assert_eq!(
            PlatformProfile::LowPower.resolve_alias(ASUS_WMI),
            PlatformProfile::Quiet
        );
        assert_eq!(
            PlatformProfile::Quiet.resolve_alias(AMD_PMF),
            PlatformProfile::LowPower
        );
    }

    #[test]
    fn alias_is_a_no_op_when_name_is_available() {
        assert_eq!(
            PlatformProfile::Quiet.resolve_alias(ASUS_WMI),
            PlatformProfile::Quiet
        );
        assert_eq!(
            PlatformProfile::LowPower.resolve_alias(AMD_PMF),
            PlatformProfile::LowPower
        );
        for profile in [
            PlatformProfile::Balanced,
            PlatformProfile::Performance,
        ] {
            assert_eq!(profile.resolve_alias(ASUS_WMI), profile);
            assert_eq!(profile.resolve_alias(AMD_PMF), profile);
        }
    }

    #[test]
    fn alias_does_not_invent_an_unavailable_profile() {
        // Custom has no equivalent, and neither variant is available here
        assert_eq!(
            PlatformProfile::Custom.resolve_alias(ASUS_WMI),
            PlatformProfile::Custom
        );
        let no_quiet = [
            PlatformProfile::Balanced,
            PlatformProfile::Performance,
        ];
        assert_eq!(
            PlatformProfile::Quiet.resolve_alias(&no_quiet),
            PlatformProfile::Quiet
        );
        assert_eq!(
            PlatformProfile::LowPower.resolve_alias(&no_quiet),
            PlatformProfile::LowPower
        );
    }

    #[test]
    fn choices_skip_names_without_a_profile() {
        let choices = "low-power cool quiet balanced balanced-performance performance\n";
        assert_eq!(
            parse_profile_choices(choices),
            [
                PlatformProfile::LowPower,
                PlatformProfile::Quiet,
                PlatformProfile::Balanced,
                PlatformProfile::Performance,
            ]
        );
    }

    #[test]
    fn next_cycles_through_kernel_choices() {
        assert_eq!(
            PlatformProfile::next(PlatformProfile::Quiet, ASUS_WMI),
            PlatformProfile::Balanced
        );
        assert_eq!(
            PlatformProfile::next(PlatformProfile::Balanced, ASUS_WMI),
            PlatformProfile::Performance
        );
        assert_eq!(
            PlatformProfile::next(PlatformProfile::Performance, ASUS_WMI),
            PlatformProfile::Quiet
        );
        assert_eq!(
            PlatformProfile::next(PlatformProfile::Performance, AMD_PMF),
            PlatformProfile::LowPower
        );
    }

    #[test]
    fn next_skips_profiles_the_kernel_does_not_offer() {
        // Intel Panther Lake without the kernel fix (#387)
        let choices = [
            PlatformProfile::Balanced,
            PlatformProfile::Performance,
        ];
        // Used to be Quiet, which failed to set and left cycling stuck
        assert_eq!(
            PlatformProfile::next(PlatformProfile::Performance, &choices),
            PlatformProfile::Balanced
        );
        assert_eq!(
            PlatformProfile::next(PlatformProfile::Custom, &choices),
            PlatformProfile::Balanced
        );
        assert_eq!(
            PlatformProfile::next(PlatformProfile::Custom, &[]),
            PlatformProfile::Balanced
        );
    }

    /// A fake `/sys/class/platform-profile` with one handler per
    /// `(choices, profile)`, removed on drop
    struct FakeClass(PathBuf);

    impl FakeClass {
        fn new(name: &str, handlers: &[(&str, &str)]) -> Self {
            let root = std::env::temp_dir()
                .join(format!("rog-platform-test-{}-{name}", std::process::id()));
            std::fs::remove_dir_all(&root).ok();
            for (i, (choices, profile)) in handlers.iter().enumerate() {
                let dir = root.join(format!("platform-profile-{i}"));
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(dir.join("choices"), format!("{choices}\n")).unwrap();
                std::fs::write(dir.join("profile"), format!("{profile}\n")).unwrap();
            }
            Self(root)
        }
    }

    impl Drop for FakeClass {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    // Intel power slider and asus-wmi, as on the GU606AW
    const PANTHER_LAKE: &[(&str, &str)] = &[
        ("low-power balanced performance", "balanced"),
        ("quiet balanced performance", "balanced"),
    ];

    #[test]
    fn quiet_is_offered_and_set_per_handler_when_legacy_lacks_it() {
        let class = FakeClass::new("panther-lake", PANTHER_LAKE);
        let handlers = read_profile_handlers(&class.0);
        assert_eq!(handlers.len(), 2);
        assert!(!handlers_in_low_power(&handlers));

        let legacy = parse_profile_choices("balanced performance\n");
        assert_eq!(
            effective_choices(legacy, &handlers),
            [
                PlatformProfile::Quiet,
                PlatformProfile::Balanced,
                PlatformProfile::Performance,
            ]
        );

        let targets = low_power_targets(&handlers).unwrap();
        write_handler_profiles(&targets).unwrap();
        let handlers = read_profile_handlers(&class.0);
        let profiles: Vec<&str> = handlers.iter().map(|h| h.profile.as_str()).collect();
        assert_eq!(
            profiles,
            [
                "low-power", "quiet"
            ]
        );
        // The legacy interface now reads custom, which is really Quiet
        assert!(handlers_in_low_power(&handlers));
    }

    #[test]
    fn legacy_choices_are_kept_when_they_have_quiet() {
        // With quiet as a hidden choice of the Intel power slider
        let class = FakeClass::new("fixed-kernel", PANTHER_LAKE);
        let handlers = read_profile_handlers(&class.0);
        let legacy = parse_profile_choices("quiet balanced performance");
        assert_eq!(effective_choices(legacy.clone(), &handlers), legacy);
        assert_eq!(effective_choices(AMD_PMF.to_vec(), &handlers), AMD_PMF);
    }

    #[test]
    fn no_fallback_unless_every_handler_has_low_power() {
        let class = FakeClass::new(
            "no-low-power",
            &[
                ("balanced performance", "balanced"),
                ("quiet balanced performance", "quiet"),
            ],
        );
        let handlers = read_profile_handlers(&class.0);
        assert!(low_power_targets(&handlers).is_none());
        assert!(!handlers_in_low_power(&handlers));
        let legacy = parse_profile_choices("balanced performance");
        assert_eq!(effective_choices(legacy.clone(), &handlers), legacy);

        let missing = read_profile_handlers(&class.0.join("missing"));
        assert!(missing.is_empty());
        assert!(low_power_targets(&missing).is_none());
        assert!(!handlers_in_low_power(&missing));
    }
}
