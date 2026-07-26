use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::gpu_pci::{asus_dgpu_disable_exists, asus_gpu_mux_exists};

const WMI_PATH: &str = "/sys/devices/platform/asus-nb-wmi";

/// Detected hardware feature matrix.
///
/// Note: `true` indicates an exposed driver or sysfs interface.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Support for GPU MUX mode switching.
    pub has_gpu_mux: bool,
    /// Support for dGPU disable / dGPU power cutoff.
    pub has_dgpu_disable: bool,
    /// Support for custom PPT (Package Power Tracking) limits.
    pub has_ppt_control: bool,
    /// Support for custom fan curves (CPU/GPU/MID fans).
    pub has_fan_curves: bool,
    /// Support for Panel Overdrive (Fast Response).
    pub has_panel_od: bool,
    /// Support for Mini-LED backlight mode control.
    pub has_mini_led: bool,
    /// Support for AniMe Matrix LED display.
    pub has_anime_matrix: bool,
    /// Support for Slash lighting bar.
    pub has_slash_lighting: bool,
}

impl DeviceCapabilities {
    /// Detect capabilities based on sysfs nodes and platform attributes.
    pub fn detect() -> Self {
        let wmi = Path::new(WMI_PATH);
        Self {
            has_gpu_mux: asus_gpu_mux_exists(),
            has_dgpu_disable: asus_dgpu_disable_exists(),
            has_ppt_control: wmi.join("ppt_pl1_spl").exists() || wmi.join("ppt_pl2_sppt").exists(),
            has_fan_curves: wmi.join("pwm1_auto_point1_pwm").exists()
                || wmi.join("fan_boost_mode").exists(),
            has_panel_od: wmi.join("panel_od").exists(),
            has_mini_led: wmi.join("mini_led_mode").exists(),
            has_anime_matrix: detect_anime_matrix(),
            has_slash_lighting: detect_slash_lighting(),
        }
    }
}

fn detect_anime_matrix() -> bool {
    // Detect USB/HID AniMe matrix presence
    Path::new("/sys/bus/usb/drivers/asus_anime").exists()
}

fn detect_slash_lighting() -> bool {
    // Detect USB/HID Slash lighting presence
    Path::new("/sys/class/leds/asus::slash").exists()
}
