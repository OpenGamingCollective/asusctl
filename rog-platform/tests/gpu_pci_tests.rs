//! Tests for the GPU PCI detection and power status module.
//!
//! These tests cover the pure/deterministic parts of `rog_platform::gpu_pci`:
//! enum conversions, label matching, and default values. Hardware-dependent
//! functions (`Device::find`, `get_gpu_power_status`) are tested via integration
//! tests on machines with actual GPUs.

use rog_platform::gpu_pci::{GfxPower, GpuTelemetry, lspci_dgpu_check};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// GpuTelemetry – Default
// ---------------------------------------------------------------------------

#[test]
fn gpu_telemetry_default_values() {
    let telemetry = GpuTelemetry::default();
    assert_eq!(telemetry.igpu_temp, -1.0);
    assert_eq!(telemetry.igpu_usage, -1.0);
    assert_eq!(telemetry.dgpu_temp, -1.0);
    assert_eq!(telemetry.dgpu_usage, -1.0);
    assert_eq!(telemetry.dgpu_freq_mhz, -1.0);
    assert!(!telemetry.dgpu_suspended);
}

// ---------------------------------------------------------------------------
// GfxPower – FromStr
// ---------------------------------------------------------------------------

#[test]
fn gfx_power_from_str_active() {
    assert_eq!(GfxPower::from_str("active").unwrap(), GfxPower::Active);
}

#[test]
fn gfx_power_from_str_active_case_insensitive() {
    assert_eq!(GfxPower::from_str("ACTIVE").unwrap(), GfxPower::Active);
    assert_eq!(GfxPower::from_str("Active").unwrap(), GfxPower::Active);
}

#[test]
fn gfx_power_from_str_suspended() {
    assert_eq!(
        GfxPower::from_str("suspended").unwrap(),
        GfxPower::Suspended
    );
}

#[test]
fn gfx_power_from_str_dgpu_disabled() {
    assert_eq!(
        GfxPower::from_str("dgpu_disabled").unwrap(),
        GfxPower::AsusDisabled
    );
}

#[test]
fn gfx_power_from_str_asus_mux_discreet() {
    assert_eq!(
        GfxPower::from_str("asus_mux_discreet").unwrap(),
        GfxPower::AsusMuxDiscreet
    );
}

#[test]
fn gfx_power_from_str_handles_whitespace() {
    assert_eq!(
        GfxPower::from_str("  suspended\n").unwrap(),
        GfxPower::Suspended
    );
    assert_eq!(GfxPower::from_str("\tactive ").unwrap(), GfxPower::Active);
}

#[test]
fn gfx_power_from_str_unknown_fallback() {
    assert_eq!(
        GfxPower::from_str("auto").unwrap(),
        GfxPower::Unknown,
        "unexpected kernel string should map to Unknown"
    );
    assert_eq!(
        GfxPower::from_str("unsupported").unwrap(),
        GfxPower::Unknown
    );
    assert_eq!(GfxPower::from_str("").unwrap(), GfxPower::Unknown);
    assert_eq!(GfxPower::from_str("garbage").unwrap(), GfxPower::Unknown);
}

// ---------------------------------------------------------------------------
// GfxPower – Display round-trip
// ---------------------------------------------------------------------------

#[test]
fn gfx_power_display_roundtrip() {
    let variants = [
        GfxPower::Active,
        GfxPower::Suspended,
        GfxPower::AsusDisabled,
        GfxPower::AsusMuxDiscreet,
        GfxPower::Unknown,
    ];
    for &variant in &variants {
        let s = variant.to_string();
        let parsed = GfxPower::from_str(&s).unwrap();
        assert_eq!(variant, parsed, "failed round-trip for {variant:?}");
    }
}

// ---------------------------------------------------------------------------
// GfxPower – Serde
// ---------------------------------------------------------------------------

#[test]
fn gfx_power_serde_roundtrip() {
    let variants = [
        GfxPower::Active,
        GfxPower::Suspended,
        GfxPower::AsusDisabled,
        GfxPower::AsusMuxDiscreet,
        GfxPower::Unknown,
    ];
    for &variant in &variants {
        let json = serde_json::to_string(&variant).unwrap();
        let deserialized: GfxPower = serde_json::from_str(&json).unwrap();
        assert_eq!(
            variant, deserialized,
            "serde round-trip failed for {variant:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// GfxPower – Default
// ---------------------------------------------------------------------------

#[test]
fn gfx_power_default_is_unknown() {
    assert_eq!(GfxPower::default(), GfxPower::Unknown);
}

// ---------------------------------------------------------------------------
// GfxPower – Copy / Clone
// ---------------------------------------------------------------------------

#[test]
fn gfx_power_copy_clone() {
    let a = GfxPower::Active;
    let b = a;
    let c = a;
    assert_eq!(a, b);
    assert_eq!(b, c);
}

// ---------------------------------------------------------------------------
// lspci_dgpu_check – positive matches
// ---------------------------------------------------------------------------

#[test]
fn lspci_dgpu_check_radeon_rx() {
    assert!(lspci_dgpu_check("Radeon RX 6800M"));
}

#[test]
fn lspci_dgpu_check_amd_ati() {
    assert!(lspci_dgpu_check("AMD/ATI Navi 22"));
}

#[test]
fn lspci_dgpu_check_geforce() {
    assert!(lspci_dgpu_check("GeForce RTX 3080"));
}

#[test]
fn lspci_dgpu_check_geforce_lowercase_f() {
    assert!(lspci_dgpu_check("Geforce GTX 1660"));
}

#[test]
fn lspci_dgpu_check_quadro() {
    assert!(lspci_dgpu_check("Quadro T1000"));
}

#[test]
fn lspci_dgpu_check_t1200() {
    assert!(lspci_dgpu_check("T1200"));
}

// ---------------------------------------------------------------------------
// lspci_dgpu_check – negative matches
// ---------------------------------------------------------------------------

#[test]
fn lspci_dgpu_check_intel_igpu() {
    assert!(!lspci_dgpu_check("Intel Corporation UHD Graphics 630"));
}

#[test]
fn lspci_dgpu_check_empty_string() {
    assert!(!lspci_dgpu_check(""));
}

#[test]
fn lspci_dgpu_check_unrelated_device() {
    assert!(!lspci_dgpu_check("Realtek RTL8111/8168/8411"));
}

#[test]
fn lspci_dgpu_check_partial_match_not_enough() {
    // "Radeon" alone should not match (the pattern requires "Radeon RX" or "AMD/ATI")
    assert!(!lspci_dgpu_check("Radeon Pro W6600"));
}
