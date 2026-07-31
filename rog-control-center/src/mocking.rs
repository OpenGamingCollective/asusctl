use std::collections::BTreeMap;

use rog_aura::{AuraEffect, AuraModeNum};
use rog_platform::gpu_pci::GfxPower;
use rog_platform::platform::{GpuMode, PlatformProfile};

use crate::error::Result;

const NOPE: &str = "";

#[derive(Default)]
pub struct DaemonProxyBlocking<'a> {
    _phantom: &'a str,
}

impl<'a> DaemonProxyBlocking<'a> {
    pub fn new(_c: &bool) -> Result<Self> {
        Ok(Self { _phantom: NOPE })
    }

    pub fn power(&self) -> Result<GfxPower> {
        Ok(GfxPower::Suspended)
    }
}

#[derive(Default)]
pub struct RogDbusClientBlocking<'a> {
    _phantom: &'a str,
}

impl<'a> RogDbusClientBlocking<'a> {
    pub fn new() -> Result<(Self, bool)> {
        Ok((Self { _phantom: NOPE }, true))
    }

    pub fn proxies(&self) -> Proxies {
        Proxies
    }
}

pub struct Proxies;
impl Proxies {
    pub fn rog_bios(&self) -> Bios {
        Bios
    }

    pub fn profile(&self) -> Profile {
        Profile
    }

    pub fn led(&self) -> Led {
        Led
    }

    pub fn anime(&self) -> Anime {
        Anime
    }

    pub fn charge(&self) -> Profile {
        Profile
    }

    pub fn supported(&self) -> Supported {
        Supported
    }
}

pub struct Bios;
impl Bios {
    pub fn post_boot_sound(&self) -> Result<i16> {
        Ok(1)
    }

    pub fn gpu_mux_mode(&self) -> Result<GpuMode> {
        Ok(GpuMode::Optimus)
    }

    pub fn panel_od(&self) -> Result<bool> {
        Ok(true)
    }

    pub fn set_post_boot_sound(&self, _b: bool) -> Result<()> {
        Ok(())
    }

    pub fn set_gpu_mux_mode(&self, _b: GpuMode) -> Result<()> {
        Ok(())
    }

    pub fn set_panel_od(&self, _b: bool) -> Result<()> {
        Ok(())
    }
}

pub struct Profile;
impl Profile {
    pub fn profiles(&self) -> Result<Vec<PlatformProfile>> {
        Ok(vec![
            PlatformProfile::Balanced,
            PlatformProfile::Performance,
            PlatformProfile::Quiet,
        ])
    }

    pub fn active_profile(&self) -> Result<PlatformProfile> {
        Ok(PlatformProfile::Performance)
    }

    pub fn enabled_fan_profiles(&self) -> Result<Vec<PlatformProfile>> {
        Ok(vec![
            PlatformProfile::Performance,
            PlatformProfile::Balanced,
        ])
    }

    pub fn set_fan_curve_enabled(&self, _p: PlatformProfile, _b: bool) -> Result<()> {
        Ok(())
    }

    pub fn charge_control_end_threshold(&self) -> Result<u8> {
        Ok(66)
    }

    pub fn set_charge_control_end_threshold(&self, _l: u8) -> Result<()> {
        Ok(())
    }

    pub fn set_active_profile(&self, _p: PlatformProfile) -> Result<()> {
        Ok(())
    }

    pub fn mains_online(&self) -> Result<bool> {
        Ok(true)
    }
}

pub struct Led;
impl Led {
    pub fn led_modes(&self) -> Result<BTreeMap<AuraModeNum, AuraEffect>> {
        let mut data = BTreeMap::new();
        data.insert(AuraModeNum::Static, AuraEffect::default());
        data.insert(AuraModeNum::Star, AuraEffect::default());
        data.insert(AuraModeNum::Highlight, AuraEffect::default());
        data.insert(AuraModeNum::Rain, AuraEffect::default());
        data.insert(AuraModeNum::RainbowCycle, AuraEffect::default());
        data.insert(AuraModeNum::Ripple, AuraEffect::default());
        data.insert(AuraModeNum::Breathe, AuraEffect::default());
        data.insert(AuraModeNum::Comet, AuraEffect::default());
        data.insert(AuraModeNum::Flash, AuraEffect::default());
        data.insert(AuraModeNum::Laser, AuraEffect::default());
        data.insert(AuraModeNum::Pulse, AuraEffect::default());
        Ok(data)
    }

    pub fn led_mode(&self) -> Result<AuraModeNum> {
        Ok(AuraModeNum::RainbowCycle)
    }

    pub fn led_brightness(&self) -> Result<i16> {
        Ok(1)
    }

    pub fn set_led_mode(&self, _a: &AuraEffect) -> Result<()> {
        Ok(())
    }
}

pub struct Anime;
impl Anime {
    pub fn boot_enabled(&self) -> Result<bool> {
        Ok(true)
    }

    pub fn awake_enabled(&self) -> Result<bool> {
        Ok(true)
    }

    pub fn set_on_off(&self, _b: bool) -> Result<()> {
        Ok(())
    }

    pub fn set_boot_on_off(&self, _b: bool) -> Result<()> {
        Ok(())
    }
}

pub struct Supported;
impl Supported {
    pub fn supported_functions(&self) -> Result<bool> {
        Ok(true)
    }
}
