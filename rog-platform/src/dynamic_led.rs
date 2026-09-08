use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use log::{info, warn};

use crate::error::{PlatformError, Result};
use crate::{attr_num, attr_string, to_device};

/// Dynamic Lighting class device under `/sys/class/leds/`.
///
/// Wraps a kernel `led-class-dynamic` sysfs node exposing effects, palette,
/// speed, direction, power states, direct buffer streaming, and standard
/// brightness attributes.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct DynamicLed {
    path: PathBuf,
}

impl DynamicLed {
    attr_string!("effect", path);

    attr_string!("effect_index", path);

    attr_string!("direction", path);

    attr_string!("direction_index", path);

    attr_string!("effects_palette", path);

    attr_string!("speed_range", path);

    attr_string!("zone_type", path);

    attr_string!("matrix_dimensions", path);

    attr_string!("power_states", path);

    attr_string!("power_states_index", path);

    attr_num!("speed", path, u32);

    attr_num!("max_palette_entries", path, u32);

    attr_num!("led_count", path, u32);

    attr_num!("brightness", path, u8);

    attr_num!("max_brightness", path, u8);

    /// Create a new `DynamicLed` by matching the exact sysfs name (e.g.
    /// `"aura:keyboard"`).
    pub fn new(name: &str) -> Result<Self> {
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("DynamicLed udev enumerator failed: {err}");
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("leds").map_err(|err| {
            warn!("DynamicLed match_subsystem failed: {err}");
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        for device in enumerator.scan_devices().map_err(|err| {
            warn!("DynamicLed scan_devices failed: {err}");
            PlatformError::Udev("scan_devices failed".into(), err)
        })? {
            let sysname = device.sysname().to_string_lossy();
            if sysname == name {
                let syspath = device.syspath();
                if Self::is_dynamic_node(syspath) {
                    info!("Found Dynamic Lighting LED device at {:?}", sysname);
                    return Ok(Self {
                        path: syspath.to_path_buf(),
                    });
                }
            }
        }

        Err(PlatformError::MissingFunction(format!(
            "DynamicLed::new(): no dynamic LED named '{name}' found"
        )))
    }

    /// Helper to find a dynamic LED by name.
    pub fn find(name: &str) -> Result<Self> {
        Self::new(name)
    }

    /// Return the LED name (e.g. "aura:keyboard" or "asus::kbd_backlight").
    pub fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
    }

    /// Check if a dynamic LED is present on the system.
    pub fn is_available(name: &str) -> bool {
        Self::find(name).is_ok()
    }

    /// Return the sysfs path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn from_syspath(path: PathBuf) -> Result<Self> {
        if !Self::is_dynamic_node(&path) {
            return Err(PlatformError::MissingFunction(format!(
                "{} is not a Dynamic Lighting LED node",
                path.display()
            )));
        }
        Ok(Self { path })
    }

    fn is_dynamic_node(path: &Path) -> bool {
        [
            "effect", "effect_index", "zone_type", "led_count",
        ]
        .iter()
        .all(|attr| path.join(attr).exists())
    }

    /// Read and parse space-separated list of supported effects from
    /// `effect_index`.
    pub fn get_supported_effects_list(&self) -> Result<Vec<String>> {
        let raw = self.get_effect_index()?;
        Ok(raw.split_whitespace().map(String::from).collect())
    }

    /// Check if a given effect mode string is supported by the kernel driver.
    pub fn is_effect_supported(&self, effect: &str) -> bool {
        self.supports_effect(effect).unwrap_or(false)
    }

    /// Check effect support while preserving sysfs read errors.
    pub fn supports_effect(&self, effect: &str) -> Result<bool> {
        Ok(self
            .get_supported_effects_list()?
            .iter()
            .any(|candidate| candidate == effect))
    }

    /// Read and parse space-separated list of supported directions from
    /// `direction_index`.
    pub fn get_supported_directions_list(&self) -> Result<Vec<String>> {
        let raw = self.get_direction_index()?;
        Ok(raw.split_whitespace().map(String::from).collect())
    }

    /// Set speed after validating the optional advertised inclusive range.
    pub fn set_supported_speed(&self, speed: u32) -> Result<()> {
        if !self.has_speed() {
            return Err(PlatformError::AttrNotFound("speed".into()));
        }
        if self.has_speed_range() {
            let (min, max) = parse_speed_range(&self.get_speed_range()?)?;
            if !(min..=max).contains(&speed) {
                return Err(PlatformError::InvalidValue);
            }
        }
        self.set_speed(speed)
    }

    /// Set direction after validating the optional advertised values.
    pub fn set_supported_direction(&self, direction: &str) -> Result<()> {
        if !self.has_direction() {
            return Err(PlatformError::AttrNotFound("direction".into()));
        }
        if self.has_direction_index()
            && !self
                .get_supported_directions_list()?
                .iter()
                .any(|candidate| candidate == direction)
        {
            return Err(PlatformError::InvalidValue);
        }
        self.set_direction(direction)
    }

    /// Write one complete RGB frame to the `direct_buffer` binary attribute.
    pub fn write_direct(&self, data: &[u8]) -> Result<()> {
        let led_count = self.get_led_count()? as usize;
        validate_direct_len(led_count, data.len())?;

        let direct_path = self.path.join("direct_buffer");
        let mut file = OpenOptions::new()
            .write(true)
            .open(&direct_path)
            .map_err(|e| PlatformError::IoPath(direct_path.to_string_lossy().into_owned(), e))?;
        file.write_all(data)
            .map_err(|e| PlatformError::IoPath(direct_path.to_string_lossy().into_owned(), e))
    }

    /// Write palette colors as formatted `"#RRGGBB #RRGGBB ..."` string to
    /// `effects_palette`.
    pub fn set_palette_colors(&self, colors: &[(u8, u8, u8)]) -> Result<()> {
        if !self.has_effects_palette() {
            return Err(PlatformError::AttrNotFound("effects_palette".into()));
        }
        if self.has_max_palette_entries() && colors.len() > self.get_max_palette_entries()? as usize
        {
            return Err(PlatformError::InvalidValue);
        }
        let formatted: Vec<String> = colors
            .iter()
            .map(|(r, g, b)| format!("#{r:02x}{g:02x}{b:02x}"))
            .collect();
        let palette_str = formatted.join(" ");
        self.set_effects_palette(&palette_str)
    }

    /// Whether this ASUS node exposes the non-generic topology selector.
    pub fn has_asus_aura_mode(&self) -> bool {
        self.name().starts_with("aura:") && self.path.join("aura_mode").exists()
    }

    /// Read the active ASUS topology mode.
    pub fn get_asus_aura_mode(&self) -> Result<String> {
        parse_active_index(
            &std::fs::read_to_string(self.path.join("aura_mode")).map_err(|e| {
                PlatformError::IoPath(self.path.join("aura_mode").display().to_string(), e)
            })?,
        )
    }

    /// Set the ASUS topology mode (`auto`, `unified`, or `split`).
    pub fn set_asus_aura_mode(&self, mode: &str) -> Result<()> {
        if !matches!(mode, "auto" | "unified" | "split") {
            return Err(PlatformError::InvalidValue);
        }
        let path = self.path.join("aura_mode");
        std::fs::write(&path, mode)
            .map_err(|e| PlatformError::IoPath(path.display().to_string(), e))
    }
}

fn validate_direct_len(led_count: usize, data_len: usize) -> Result<()> {
    let expected = led_count
        .checked_mul(3)
        .ok_or(PlatformError::InvalidValue)?;
    if data_len == expected {
        Ok(())
    } else {
        Err(PlatformError::InvalidValue)
    }
}

fn parse_active_index(raw: &str) -> Result<String> {
    raw.split_whitespace()
        .find_map(|word| word.strip_prefix('[')?.strip_suffix(']'))
        .map(str::to_owned)
        .ok_or(PlatformError::InvalidValue)
}

fn parse_speed_range(raw: &str) -> Result<(u32, u32)> {
    let values: Vec<_> = raw
        .split(|c: char| !c.is_ascii_digit())
        .filter(|value| !value.is_empty())
        .map(str::parse::<u32>)
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| PlatformError::ParseNum)?;
    match values.as_slice() {
        [min, max] if min <= max => Ok((*min, *max)),
        _ => Err(PlatformError::InvalidValue),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[test]
    fn test_parse_active_aura_mode() {
        let sample = "[auto] unified split\n";
        assert_eq!(super::parse_active_index(sample).unwrap(), "auto");

        let sample2 = "auto [unified] split\n";
        assert_eq!(super::parse_active_index(sample2).unwrap(), "unified");
        assert!(super::parse_active_index("auto unified split").is_err());
    }

    #[test]
    fn test_palette_colors_formatting() {
        let colors = [
            (255, 0, 128),
            (0, 255, 64),
        ];
        let formatted: Vec<String> = colors
            .iter()
            .map(|(r, g, b)| format!("#{r:02x}{g:02x}{b:02x}"))
            .collect();
        let palette_str = formatted.join(" ");
        assert_eq!(palette_str, "#ff0080 #00ff40");
    }

    #[test]
    fn validates_exact_direct_frame_size() {
        assert!(super::validate_direct_len(4, 12).is_ok());
        assert!(super::validate_direct_len(4, 11).is_err());
        assert!(super::validate_direct_len(usize::MAX, 0).is_err());
    }

    #[test]
    fn parses_bounded_speed_range() {
        assert_eq!(super::parse_speed_range("0 2\n").unwrap(), (0, 2));
        assert_eq!(super::parse_speed_range("[1-4]").unwrap(), (1, 4));
        assert!(super::parse_speed_range("fast slow").is_err());
        assert!(super::parse_speed_range("4 1").is_err());
    }

    #[test]
    fn validates_mandatory_generic_attributes() {
        let path =
            std::env::temp_dir().join(format!("asusctl-dynamic-led-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).expect("temporary test directory must be creatable");
        for attr in [
            "effect", "effect_index", "zone_type", "led_count",
        ] {
            fs::write(path.join(attr), b"").expect("temporary attribute must be writable");
        }
        assert!(super::DynamicLed::is_dynamic_node(&path));
        fs::remove_file(path.join("zone_type")).expect("test attribute must be removable");
        assert!(!super::DynamicLed::is_dynamic_node(&path));
        fs::remove_dir_all(path).expect("temporary test directory must be removable");
    }
}
