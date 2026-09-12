//! Loading and applying per-model keyboard shortcut profiles.
//!
//! Pipeline: ModelDetector -> ProfileLoader -> ShortcutApplier

use std::fs;
use std::path::{Path, PathBuf};

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use crate::gnome_shortcuts;

type BoxedError = Box<dyn std::error::Error>;

/// Default value for `KeyboardShortcut::enabled` when the YAML omits it.
///
/// Shipping shortcuts as disabled-by-default is the conservative choice; the
/// community can flip this per-shortcut in the profile files.
fn default_enabled() -> bool {
    false
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardShortcut {
    /// Stable identifier, used to build the dconf path. Keep it kebab-case.
    pub id: String,
    /// Human readable name shown in GNOME Settings.
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Command line executed when the shortcut fires.
    pub command: String,
    /// Key or key combination, e.g. `XF86Launch1` or `<Super>k`.
    pub keybinding: String,
    /// Whether this shortcut ships enabled. Defaults to `false`.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardProfile {
    pub name: String,
    #[serde(default)]
    pub product_code: Option<String>,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub keyboard_shortcuts: Vec<KeyboardShortcut>,
}

impl KeyboardProfile {
    /// Shortcuts that are enabled in this profile.
    pub fn enabled_shortcuts(&self) -> impl Iterator<Item = &KeyboardShortcut> {
        self.keyboard_shortcuts.iter().filter(|s| s.enabled)
    }
}

// ---------------------------------------------------------------------------
// Model detection
// ---------------------------------------------------------------------------

pub struct ModelDetector;

impl ModelDetector {
    /// Candidate model identifiers, most specific first.
    ///
    /// Different ASUS laptops expose their marketing name in different DMI
    /// fields, so we collect all of them and let the loader try each in turn.
    pub fn candidates() -> Vec<String> {
        const DMI_FIELDS: [&str; 3] = [
            "/sys/class/dmi/id/product_name",
            "/sys/class/dmi/id/board_name",
            "/sys/class/dmi/id/product_family",
        ];

        let mut found = Vec::new();
        for field in DMI_FIELDS {
            if let Ok(value) = fs::read_to_string(field) {
                let value = value.trim().to_string();
                if !value.is_empty() && !found.contains(&value) {
                    found.push(value);
                }
            }
        }
        found
    }

    /// Primary model name, for logging and error messages.
    pub fn detect() -> Result<String, BoxedError> {
        Self::candidates()
            .into_iter()
            .next()
            .ok_or_else(|| "could not read any DMI identifier from /sys/class/dmi/id".into())
    }
}

// ---------------------------------------------------------------------------
// Profile loading
// ---------------------------------------------------------------------------

pub struct ProfileLoader;

impl ProfileLoader {
    /// Directories searched for profile files, in priority order.
    ///
    /// The old code used a path relative to the current working directory,
    /// which only resolved when running from the repository root.
    fn search_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // 1. Explicit override, handy for testing.
        if let Ok(dir) = std::env::var("ROGCC_PROFILES_DIR") {
            dirs.push(PathBuf::from(dir));
        }

        // 2. Per-user profiles, so a user can add their own model.
        if let Some(config) = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        {
            dirs.push(config.join("rog-control-center/profiles"));
        }

        // 3. System-wide install locations.
        dirs.push(PathBuf::from("/usr/share/rog-control-center/profiles"));
        dirs.push(PathBuf::from("/usr/local/share/rog-control-center/profiles"));

        // 4. Development checkout, relative to cwd and to the binary.
        dirs.push(PathBuf::from("rog-control-center/resources/profiles"));
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                dirs.push(dir.join("../resources/profiles"));
            }
        }

        dirs
    }

    /// Turn a model name into a profile file name.
    fn file_name(model: &str) -> String {
        format!("{}.yaml", model.replace(' ', "_"))
    }

    /// Load the profile for an exact model name.
    pub fn load(model: &str) -> Result<KeyboardProfile, BoxedError> {
        let file_name = Self::file_name(model);

        for dir in Self::search_dirs() {
            let candidate = dir.join(&file_name);
            debug!("looking for profile at {}", candidate.display());
            if candidate.is_file() {
                return Self::load_from_path(&candidate);
            }
        }

        Err(format!("no profile named {file_name} found in any search directory").into())
    }

    /// Try every DMI identifier until one matches a shipped profile.
    pub fn load_for_this_machine() -> Result<KeyboardProfile, BoxedError> {
        let candidates = ModelDetector::candidates();

        for model in &candidates {
            match Self::load(model) {
                Ok(profile) => {
                    info!("matched profile for DMI identifier '{model}'");
                    return Ok(profile);
                }
                Err(e) => debug!("no profile for '{model}': {e}"),
            }
        }

        Err(format!(
            "no profile matched any of: {}",
            candidates.join(", ")
        )
        .into())
    }

    /// Load a profile from an explicit path.
    pub fn load_from_path(path: &Path) -> Result<KeyboardProfile, BoxedError> {
        let content = fs::read_to_string(path)?;
        let profile: KeyboardProfile = serde_yaml::from_str(&content)?;
        info!("loaded profile '{}' from {}", profile.name, path.display());
        Ok(profile)
    }
}

// ---------------------------------------------------------------------------
// Applying
// ---------------------------------------------------------------------------

/// Outcome of applying a set of shortcuts.
#[derive(Debug, Default)]
pub struct ApplyReport {
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: Vec<(String, String)>,
}

pub struct ShortcutApplier;

impl ShortcutApplier {
    /// Apply every enabled shortcut, removing the ones that are disabled.
    ///
    /// Removing disabled entries matters: if a user flips `enabled: false` and
    /// re-runs setup, the stale shortcut must disappear rather than linger.
    pub fn apply(shortcuts: &[KeyboardShortcut]) -> Result<ApplyReport, BoxedError> {
        if !gnome_shortcuts::is_available() {
            return Err(
                "GNOME settings-daemon schemas not found - this command only supports GNOME"
                    .into(),
            );
        }

        let mut report = ApplyReport::default();

        for shortcut in shortcuts {
            if !shortcut.enabled {
                debug!("shortcut '{}' is disabled, removing if present", shortcut.id);
                if let Err(e) = gnome_shortcuts::remove(&shortcut.id) {
                    warn!("could not remove '{}': {e}", shortcut.id);
                }
                report.skipped.push(shortcut.name.clone());
                continue;
            }

            match gnome_shortcuts::apply(
                &shortcut.id,
                &shortcut.name,
                &shortcut.command,
                &shortcut.keybinding,
            ) {
                Ok(()) => {
                    info!("applied '{}' -> {}", shortcut.name, shortcut.keybinding);
                    report.applied.push(shortcut.name.clone());
                }
                Err(e) => {
                    warn!("failed to apply '{}': {e}", shortcut.id);
                    report.failed.push((shortcut.name.clone(), e.to_string()));
                }
            }
        }

        Ok(report)
    }

    /// Remove every shortcut this application owns.
    pub fn remove_all() -> Result<usize, BoxedError> {
        gnome_shortcuts::remove_all_owned()
    }
}