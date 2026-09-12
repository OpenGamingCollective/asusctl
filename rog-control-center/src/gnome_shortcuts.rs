//! GNOME backend for custom keyboard shortcuts.
//!
//! A GNOME custom shortcut requires TWO things:
//!
//! 1. The values themselves, under the relocatable schema
//!    `org.gnome.settings-daemon.plugins.media-keys.custom-keybinding`
//!    at `/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/<id>/`
//!    (keys: `name`, `command`, `binding`).
//!
//! 2. That path registered in the `custom-keybindings` list of
//!    `org.gnome.settings-daemon.plugins.media-keys`.
//!
//! Writing only (1) is silently ignored by gnome-settings-daemon: this is the
//! exact reason shortcuts appeared to be written but never worked.
//!
//! Note: the paths stored in the list always have a TRAILING SLASH. The value
//! path and the registered path must match exactly.

use std::process::Command;

type BoxedError = Box<dyn std::error::Error>;

/// Schema holding the list of registered custom shortcuts.
const MEDIA_KEYS_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";

/// Relocatable schema for an individual custom shortcut.
const CUSTOM_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";

/// dconf base path where custom shortcuts live.
const CUSTOM_BASE: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings";

/// Prefix applied to every shortcut we own, so we never touch user-created ones.
const OWNED_PREFIX: &str = "rog-";

// ---------------------------------------------------------------------------
// Low level helpers
// ---------------------------------------------------------------------------

/// Run `gsettings` with the given arguments and return trimmed stdout.
fn gsettings(args: &[&str]) -> Result<String, BoxedError> {
    let out = Command::new("gsettings").args(args).output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("`gsettings {}` failed: {}", args.join(" "), err.trim()).into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Quote a Rust string as a GVariant string literal.
///
/// `gsettings set` parses its value argument as GVariant text, so a bare
/// string with spaces or quotes would be rejected or misparsed.
fn gvariant_string(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// Parse a GVariant string array as printed by `gsettings get`.
///
/// Accepts both `@as []` (empty) and `['/path/a/', '/path/b/']`.
fn parse_path_list(raw: &str) -> Vec<String> {
    let body = raw
        .trim()
        .trim_start_matches("@as")
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');

    body.split(',')
        .map(|item| item.trim().trim_matches('\'').trim_matches('"'))
        .filter(|item| !item.is_empty())
        .map(String::from)
        .collect()
}

/// Render a list of dconf paths back into GVariant text.
fn format_path_list(paths: &[String]) -> String {
    if paths.is_empty() {
        return "@as []".to_string();
    }
    let inner: Vec<String> = paths.iter().map(|p| gvariant_string(p)).collect();
    format!("[{}]", inner.join(", "))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Returns true if this looks like a GNOME session we can configure.
pub fn is_available() -> bool {
    gsettings(&["list-keys", MEDIA_KEYS_SCHEMA]).is_ok()
}

//// Strips a leading `rog-` from the id if the profile author already wrote
/// one, so ids like `rog-control-center` don't become `rog-rog-control-center`.
pub fn path_for(id: &str) -> String {
    let id = id.strip_prefix(OWNED_PREFIX).unwrap_or(id);
    format!("{CUSTOM_BASE}/{OWNED_PREFIX}{id}/")
}

/// Read the list of currently registered custom shortcut paths.
pub fn registered_paths() -> Result<Vec<String>, BoxedError> {
    let raw = gsettings(&["get", MEDIA_KEYS_SCHEMA, "custom-keybindings"])?;
    Ok(parse_path_list(&raw))
}

/// Overwrite the list of registered custom shortcut paths.
fn set_registered_paths(paths: &[String]) -> Result<(), BoxedError> {
    let value = format_path_list(paths);
    gsettings(&["set", MEDIA_KEYS_SCHEMA, "custom-keybindings", &value])?;
    Ok(())
}

/// Create or update a single shortcut, and make sure GNOME knows about it.
///
/// Idempotent: running it twice updates in place instead of duplicating.
pub fn apply(id: &str, name: &str, command: &str, binding: &str) -> Result<(), BoxedError> {
    let path = path_for(id);
    let target = format!("{CUSTOM_SCHEMA}:{path}");

    // Step 1 - write the three values under the relocatable schema.
    gsettings(&["set", &target, "name", &gvariant_string(name)])?;
    gsettings(&["set", &target, "command", &gvariant_string(command)])?;
    gsettings(&["set", &target, "binding", &gvariant_string(binding)])?;

    // Step 2 - register the path so gnome-settings-daemon picks it up.
    let mut paths = registered_paths()?;
    if !paths.iter().any(|p| p == &path) {
        paths.push(path);
        set_registered_paths(&paths)?;
    }

    Ok(())
}

/// Remove one of our shortcuts: unregister it, then wipe its values.
pub fn remove(id: &str) -> Result<(), BoxedError> {
    let path = path_for(id);

    let mut paths = registered_paths()?;
    let before = paths.len();
    paths.retain(|p| p != &path);
    if paths.len() != before {
        set_registered_paths(&paths)?;
    }

    // `dconf reset -f` clears the whole subtree. Failure here is not fatal:
    // the shortcut is already unregistered and therefore inactive.
    let _ = Command::new("dconf").arg("reset").arg("-f").arg(&path).output();

    Ok(())
}

/// Remove every shortcut owned by us, leaving user-created ones untouched.
pub fn remove_all_owned() -> Result<usize, BoxedError> {
    let owned_prefix = format!("{CUSTOM_BASE}/{OWNED_PREFIX}");
    let paths = registered_paths()?;

    let (ours, theirs): (Vec<String>, Vec<String>) =
        paths.into_iter().partition(|p| p.starts_with(&owned_prefix));

    if !ours.is_empty() {
        set_registered_paths(&theirs)?;
        for path in &ours {
            let _ = Command::new("dconf").arg("reset").arg("-f").arg(path).output();
        }
    }

    Ok(ours.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_list() {
        assert!(parse_path_list("@as []").is_empty());
        assert!(parse_path_list("[]").is_empty());
    }

    #[test]
    fn roundtrips_paths() {
        let paths = vec!["/a/b/".to_string(), "/c/d/".to_string()];
        assert_eq!(parse_path_list(&format_path_list(&paths)), paths);
    }

    #[test]
    fn path_has_trailing_slash() {
        assert!(path_for("aura").ends_with('/'));
    }

    #[test]
    fn escapes_quotes() {
        assert_eq!(gvariant_string("it's"), r"'it\'s'");
    }
}