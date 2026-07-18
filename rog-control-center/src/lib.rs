// These lints need to be allowed due to the generated sources
#![allow(clippy::redundant_clone, clippy::cmp_owned)]
slint::include_modules!();

/// Intentionally reexport slint so that GUI consumers don't need to add to
/// `Cargo.toml`
pub use slint;

pub mod cli_options;
pub mod config;
pub mod error;
#[cfg(feature = "mocking")]
pub mod mocking;
pub mod notify;
pub mod shortcuts;
pub mod tray;
pub mod types;
pub mod ui;
pub mod window;
pub mod zbus_proxies;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_ID: &str = "org.opengamingcollective.rog-control-center";
pub const APP_ICON_PATH: &str = "/usr/share/icons/hicolor/512x512/apps/rog-control-center.png";

pub fn print_versions() {
    println!("App and daemon versions:");
    println!("      rog-gui v{}", VERSION);
    println!("        asusd v{}", asusd::VERSION);
    println!("\nComponent crate versions:");
    println!("    rog-anime v{}", rog_anime::VERSION);
    println!("     rog-aura v{}", rog_aura::VERSION);
    println!("     rog-dbus v{}", rog_dbus::VERSION);
    println!(" rog-profiles v{}", rog_profiles::VERSION);
    println!("rog-platform v{}", rog_platform::VERSION);
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Page {
    AppSettings,
    System,
    AuraEffects,
    AnimeMatrix,
    FanCurves,
}
