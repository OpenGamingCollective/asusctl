pub mod error;
pub mod fan_curve_set;

use error::ProfileError;
use fan_curve_set::CurveData;
use log::debug;
use rog_platform::platform::PlatformProfile;
use serde::{Deserialize, Serialize};
pub use udev::Device;
#[cfg(feature = "dbus")]
use zbus::zvariant::Type;
use zbus::zvariant::{OwnedValue, Value};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Use udev system to find the fan curve path/node which is labelled with
/// "asus_custom_fan_curve" in the kernel
pub fn find_fan_curve_node() -> Result<Device, ProfileError> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("hwmon")?;

    for device in enumerator.scan_devices()? {
        if device.parent_with_subsystem("platform")?.is_some() {
            if let Some(name) = device.attribute_value("name") {
                if name == "asus_custom_fan_curve" {
                    return Ok(device);
                }
            }
        }
    }

    Err(ProfileError::NotSupported)
}

#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "s")
)]
#[derive(Default, Deserialize, Serialize, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum FanCurvePU {
    #[default]
    CPU = 0,
    GPU = 1,
    MID = 2,
}

impl FanCurvePU {
    fn which_fans(device: &Device) -> Vec<Self> {
        let mut fans = Vec::with_capacity(3);
        for fan in [
            Self::CPU,
            Self::GPU,
            Self::MID,
        ] {
            let pwm_num: char = fan.into();
            let pwm_enable = format!("pwm{pwm_num}_enable");
            debug!("Looking for {pwm_enable}");
            for attr in device.attributes() {
                let tmp = attr.name().to_string_lossy();
                if tmp.contains(&pwm_enable) {
                    debug!("Found {pwm_enable}");
                    fans.push(fan);
                }
            }
        }
        fans
    }
}

impl From<FanCurvePU> for &str {
    fn from(pu: FanCurvePU) -> &'static str {
        match pu {
            FanCurvePU::CPU => "cpu",
            FanCurvePU::GPU => "gpu",
            FanCurvePU::MID => "mid",
        }
    }
}

impl From<FanCurvePU> for char {
    fn from(pu: FanCurvePU) -> char {
        match pu {
            FanCurvePU::CPU => '1',
            FanCurvePU::GPU => '2',
            FanCurvePU::MID => '3',
        }
    }
}

impl std::str::FromStr for FanCurvePU {
    type Err = ProfileError;

    fn from_str(fan: &str) -> Result<Self, Self::Err> {
        match fan.to_ascii_lowercase().trim() {
            "cpu" => Ok(FanCurvePU::CPU),
            "gpu" => Ok(FanCurvePU::GPU),
            "mid" => Ok(FanCurvePU::MID),
            _ => Err(ProfileError::ParseProfileName),
        }
    }
}

/// Main purpose of `FanCurves` is to enable restoring state on system boot
#[cfg_attr(feature = "dbus", derive(Type))]
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct FanCurveProfiles {
    pub balanced: Vec<CurveData>,
    pub performance: Vec<CurveData>,
    pub quiet: Vec<CurveData>,
    pub custom: Vec<CurveData>,
}

impl FanCurveProfiles {
    /// Return an array of `FanCurvePU`. An empty array indicates no support for
    /// Curves.
    pub fn supported_fans() -> Result<Vec<FanCurvePU>, ProfileError> {
        let device = find_fan_curve_node()?;
        Ok(FanCurvePU::which_fans(&device))
    }

    pub fn read_from_dev_profile(
        &mut self,
        profile: PlatformProfile,
        device: &Device,
    ) -> Result<(), ProfileError> {
        let fans = Self::supported_fans()?;
        let mut curves = Vec::with_capacity(3);

        for fan in fans {
            let mut curve = CurveData {
                fan,
                ..Default::default()
            };
            debug!("Reading curve for {fan:?}");
            curve.read_from_device(device);
            debug!("Curve: {curve:?}");
            curves.push(curve);
        }

        match profile {
            PlatformProfile::Balanced => self.balanced = curves,
            PlatformProfile::Performance => self.performance = curves,
            PlatformProfile::Quiet | PlatformProfile::LowPower => self.quiet = curves,
            PlatformProfile::Custom => self.custom = curves,
        }
        Ok(())
    }

    /// Reset the stored (self) and device curve to the defaults of the
    /// platform.
    ///
    /// Each `platform_profile` has a different default and the defualt can be
    /// read only for the currently active profile.
    pub fn set_active_curve_to_defaults(
        &mut self,
        profile: PlatformProfile,
        device: &mut Device,
    ) -> Result<(), ProfileError> {
        // Writing "3" to pwm<N>_enable makes the kernel re-fetch factory
        // defaults via the fan_curve_get_factory_default WMI call. On some
        // boards (TUF FX608JMR, BIOS 306) that call blocks ~30s, stalls the EC
        // (input freezes) and returns EIO, so read the curves the driver
        // already reports instead of resetting first.
        self.read_from_dev_profile(profile, device)?;
        Ok(())
    }

    /// Write the curves for the selected profile to the device. If the curve is
    /// in the enabled list it will become active.
    pub fn write_profile_curve_to_platform(
        &mut self,
        profile: PlatformProfile,
        device: &mut Device,
    ) -> Result<(), ProfileError> {
        let fans = match profile {
            PlatformProfile::Balanced => &mut self.balanced,
            PlatformProfile::Performance => &mut self.performance,
            PlatformProfile::Quiet | PlatformProfile::LowPower => &mut self.quiet,
            PlatformProfile::Custom => &mut self.custom,
        };
        for fan in fans.iter().filter(|f| !f.enabled) {
            debug!("write_profile_curve_to_platform: writing profile:{profile}, {fan:?}");
            fan.write_to_device(device)?;
        }
        // Write enabled fans last because the kernel currently resets *all* if one is
        // disabled
        for fan in fans.iter().filter(|f| f.enabled) {
            debug!("write_profile_curve_to_platform: writing profile:{profile}, {fan:?}");
            fan.write_to_device(device)?;
        }
        Ok(())
    }

    pub fn set_profile_curves_enabled(&mut self, profile: PlatformProfile, enabled: bool) {
        match profile {
            PlatformProfile::Balanced => {
                for curve in self.balanced.iter_mut() {
                    curve.enabled = enabled;
                }
            }
            PlatformProfile::Performance => {
                for curve in self.performance.iter_mut() {
                    curve.enabled = enabled;
                }
            }
            PlatformProfile::Quiet | PlatformProfile::LowPower => {
                for curve in self.quiet.iter_mut() {
                    curve.enabled = enabled;
                }
            }
            PlatformProfile::Custom => {
                for curve in self.custom.iter_mut() {
                    curve.enabled = enabled;
                }
            }
        }
    }

    pub fn set_profile_fan_curve_enabled(
        &mut self,
        profile: PlatformProfile,
        fan: FanCurvePU,
        enabled: bool,
    ) {
        match profile {
            PlatformProfile::Balanced => {
                for curve in self.balanced.iter_mut() {
                    if curve.fan == fan {
                        curve.enabled = enabled;
                        break;
                    }
                }
            }
            PlatformProfile::Performance => {
                for curve in self.performance.iter_mut() {
                    if curve.fan == fan {
                        curve.enabled = enabled;
                        break;
                    }
                }
            }
            PlatformProfile::Quiet | PlatformProfile::LowPower => {
                for curve in self.quiet.iter_mut() {
                    if curve.fan == fan {
                        curve.enabled = enabled;
                        break;
                    }
                }
            }
            PlatformProfile::Custom => {
                for curve in self.custom.iter_mut() {
                    if curve.fan == fan {
                        curve.enabled = enabled;
                        break;
                    }
                }
            }
        }
    }

    pub fn get_fan_curves_for(&self, name: PlatformProfile) -> &[CurveData] {
        match name {
            PlatformProfile::Balanced => &self.balanced,
            PlatformProfile::Performance => &self.performance,
            PlatformProfile::Quiet | PlatformProfile::LowPower => &self.quiet,
            PlatformProfile::Custom => &self.custom,
        }
    }

    pub fn get_fan_curve_for(&self, name: &PlatformProfile, pu: FanCurvePU) -> Option<&CurveData> {
        match name {
            PlatformProfile::Balanced => {
                for this_curve in self.balanced.iter() {
                    if this_curve.fan == pu {
                        return Some(this_curve);
                    }
                }
            }
            PlatformProfile::Performance => {
                for this_curve in self.performance.iter() {
                    if this_curve.fan == pu {
                        return Some(this_curve);
                    }
                }
            }
            PlatformProfile::Quiet | PlatformProfile::LowPower => {
                for this_curve in self.quiet.iter() {
                    if this_curve.fan == pu {
                        return Some(this_curve);
                    }
                }
            }
            PlatformProfile::Custom => {
                for this_curve in self.custom.iter() {
                    if this_curve.fan == pu {
                        return Some(this_curve);
                    }
                }
            }
        }
        None
    }

    pub fn save_fan_curve(
        &mut self,
        curve: CurveData,
        profile: PlatformProfile,
    ) -> Result<(), ProfileError> {
        match profile {
            PlatformProfile::Balanced => {
                for this_curve in self.balanced.iter_mut() {
                    if this_curve.fan == curve.fan {
                        *this_curve = curve;
                        break;
                    }
                }
            }
            PlatformProfile::Performance => {
                for this_curve in self.performance.iter_mut() {
                    if this_curve.fan == curve.fan {
                        *this_curve = curve;
                        break;
                    }
                }
            }
            PlatformProfile::Quiet | PlatformProfile::LowPower => {
                for this_curve in self.quiet.iter_mut() {
                    if this_curve.fan == curve.fan {
                        *this_curve = curve;
                        break;
                    }
                }
            }
            PlatformProfile::Custom => {
                for this_curve in self.custom.iter_mut() {
                    if this_curve.fan == curve.fan {
                        *this_curve = curve;
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}
