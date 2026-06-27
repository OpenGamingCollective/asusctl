//! Tests for the GPU PCI detection and power status module.
//!
//! These tests cover the pure/deterministic parts of `rog_platform::gpu_pci`:
//! enum conversions, label matching, and default values. Hardware-dependent
//! functions (`Device::find`, `get_gpu_power_status`) are tested via integration
//! tests on machines with actual GPUs.

use rog_platform::gpu_pci::{GfxPower, GfxVendor, lscpi_dgpu_check};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// GfxPower – FromStr
// ---------------------------------------------------------------------------

#[test]
fn gfx_power_from_str_active() {
    assert_eq!(GfxPower::from_str("active").unwrap(), GfxPower::Active);
}

#[test]
fn gfx_power_from_str_active_case_insensitive() {
    assert_eq!(GfxPower::from_str("Active").unwrap(), GfxPower::Active);
    assert_eq!(GfxPower::from_str("ACTIVE").unwrap(), GfxPower::Active);
}

#[test]
fn gfx_power_from_str_suspended() {
    assert_eq!(
        GfxPower::from_str("suspended").unwrap(),
        GfxPower::Suspended
    );
}

#[test]
fn gfx_power_from_str_off() {
    assert_eq!(GfxPower::from_str("off").unwrap(), GfxPower::Off);
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
fn gfx_power_from_str_unknown_fallback() {
    assert_eq!(
        GfxPower::from_str("something_weird").unwrap(),
        GfxPower::Unknown
    );
    assert_eq!(GfxPower::from_str("").unwrap(), GfxPower::Unknown);
    assert_eq!(GfxPower::from_str("UNKNOWN").unwrap(), GfxPower::Unknown);
}

#[test]
fn gfx_power_from_str_handles_whitespace() {
    assert_eq!(GfxPower::from_str("  active  ").unwrap(), GfxPower::Active);
    assert_eq!(GfxPower::from_str("\toff\n").unwrap(), GfxPower::Off);
}

// ---------------------------------------------------------------------------
// GfxPower – Display / Into<&str>
// ---------------------------------------------------------------------------

#[test]
fn gfx_power_display_roundtrip() {
    let variants = [
        (GfxPower::Active, "active"),
        (GfxPower::Suspended, "suspended"),
        (GfxPower::Off, "off"),
        (GfxPower::AsusDisabled, "dgpu_disabled"),
        (GfxPower::AsusMuxDiscreet, "asus_mux_discreet"),
        (GfxPower::Unknown, "unknown"),
    ];
    for (variant, expected_str) in variants {
        // Into<&str>
        let s: &str = (&variant).into();
        assert_eq!(s, expected_str, "Into<&str> failed for {variant:?}");

        // Display
        let displayed = format!("{variant}");
        assert_eq!(displayed, expected_str, "Display failed for {variant:?}");

        // Roundtrip: from_str(display) should give back the same variant
        assert_eq!(
            GfxPower::from_str(&displayed).unwrap(),
            variant,
            "Roundtrip failed for {variant:?}"
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
// GfxPower – Copy / Clone / PartialEq
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
// GfxVendor – From<u16>
// ---------------------------------------------------------------------------

#[test]
fn gfx_vendor_from_nvidia() {
    assert_eq!(GfxVendor::from(0x10DEu16), GfxVendor::Nvidia);
}

#[test]
fn gfx_vendor_from_amd() {
    assert_eq!(GfxVendor::from(0x1002u16), GfxVendor::Amd);
}

#[test]
fn gfx_vendor_from_intel() {
    assert_eq!(GfxVendor::from(0x8086u16), GfxVendor::Intel);
}

#[test]
fn gfx_vendor_from_unknown() {
    assert_eq!(GfxVendor::from(0x1234u16), GfxVendor::Unknown);
    assert_eq!(GfxVendor::from(0u16), GfxVendor::Unknown);
}

// ---------------------------------------------------------------------------
// GfxVendor – Display
// ---------------------------------------------------------------------------

#[test]
fn gfx_vendor_display() {
    assert_eq!(format!("{}", GfxVendor::Nvidia), "Nvidia");
    assert_eq!(format!("{}", GfxVendor::Amd), "AMD");
    assert_eq!(format!("{}", GfxVendor::Intel), "Intel");
    assert_eq!(format!("{}", GfxVendor::Unknown), "Unknown");
    assert_eq!(
        format!("{}", GfxVendor::AsusDgpuDisabled),
        "ASUS dGPU disabled"
    );
}

// ---------------------------------------------------------------------------
// GfxVendor – Default
// ---------------------------------------------------------------------------

#[test]
fn gfx_vendor_default_is_unknown() {
    assert_eq!(GfxVendor::default(), GfxVendor::Unknown);
}

// ---------------------------------------------------------------------------
// lscpi_dgpu_check – positive matches
// ---------------------------------------------------------------------------

#[test]
fn lspci_dgpu_check_radeon_rx() {
    assert!(lscpi_dgpu_check("Radeon RX 6800M"));
}

#[test]
fn lspci_dgpu_check_amd_ati() {
    assert!(lscpi_dgpu_check("AMD/ATI Navi 22"));
}

#[test]
fn lspci_dgpu_check_geforce() {
    assert!(lscpi_dgpu_check("GeForce RTX 3080"));
}

#[test]
fn lspci_dgpu_check_geforce_lowercase_f() {
    assert!(lscpi_dgpu_check("Geforce GTX 1660"));
}

#[test]
fn lspci_dgpu_check_quadro() {
    assert!(lscpi_dgpu_check("Quadro T1000"));
}

#[test]
fn lspci_dgpu_check_t1200() {
    assert!(lscpi_dgpu_check("T1200"));
}

// ---------------------------------------------------------------------------
// lscpi_dgpu_check – negative matches
// ---------------------------------------------------------------------------

#[test]
fn lspci_dgpu_check_intel_igpu() {
    assert!(!lscpi_dgpu_check("Intel Corporation UHD Graphics 630"));
}

#[test]
fn lspci_dgpu_check_empty_string() {
    assert!(!lscpi_dgpu_check(""));
}

#[test]
fn lspci_dgpu_check_unrelated_device() {
    assert!(!lscpi_dgpu_check("Realtek RTL8111/8168/8411"));
}

#[test]
fn lspci_dgpu_check_partial_match_not_enough() {
    // "Radeon" alone should not match (the pattern requires "Radeon RX" or "AMD/ATI")
    assert!(!lscpi_dgpu_check("Radeon Pro W6600"));
}

// ---------------------------------------------------------------------------
// GfxPower – serialization (serde)
// ---------------------------------------------------------------------------

#[test]
fn gfx_power_serde_roundtrip() {
    let variants = [
        GfxPower::Active,
        GfxPower::Suspended,
        GfxPower::Off,
        GfxPower::AsusDisabled,
        GfxPower::AsusMuxDiscreet,
        GfxPower::Unknown,
    ];
    for variant in variants {
        let json = serde_json::to_string(&variant).expect("serialize");
        let deserialized: GfxPower = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            deserialized, variant,
            "serde roundtrip failed for {variant:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// GfxVendor – serialization (serde)
// ---------------------------------------------------------------------------

#[test]
fn gfx_vendor_serde_roundtrip() {
    let variants = [
        GfxVendor::Nvidia,
        GfxVendor::Amd,
        GfxVendor::Intel,
        GfxVendor::Unknown,
        GfxVendor::AsusDgpuDisabled,
    ];
    for variant in variants {
        let json = serde_json::to_string(&variant).expect("serialize");
        let deserialized: GfxVendor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            deserialized, variant,
            "serde roundtrip failed for {variant:?}"
        );
    }
}
