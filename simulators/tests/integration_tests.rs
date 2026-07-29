use rog_platform::platform::{PlatformProfile, RogPlatform};
use rog_simulators::TestHarness;

#[test]
fn test_rog_platform_integration_with_harness() {
    let harness = TestHarness::new().expect("Failed to initialize TestHarness");

    // Initialize production RogPlatform using mock sysfs root
    let platform = RogPlatform::with_root(harness.sysfs_root())
        .expect("Failed to create RogPlatform with sysfs root");

    // Verify initial platform profile read from production RogPlatform
    let profile = platform
        .get_platform_profile()
        .expect("Failed to get profile");
    assert_eq!(profile.trim(), "balanced");

    // Verify platform profile choices
    let choices = platform
        .get_platform_profile_choices()
        .expect("Failed to get choices");
    assert!(choices.contains(&PlatformProfile::Quiet));
    assert!(choices.contains(&PlatformProfile::Performance));

    // Update platform profile via MockSysfs and verify production code reads it
    harness
        .sysfs()
        .set_platform_profile("performance\n")
        .expect("Failed to set platform profile");

    let new_profile = platform
        .get_platform_profile()
        .expect("Failed to get profile");
    assert_eq!(new_profile.trim(), "performance");
}

#[test]
fn test_environment_variable_sysfs_override() {
    let harness = TestHarness::new().expect("Failed to initialize TestHarness");

    std::env::set_var("ASUS_SYSFS_ROOT", harness.sysfs_root());

    let platform = RogPlatform::new().expect("Failed to instantiate RogPlatform with env var");

    let profile = platform
        .get_platform_profile()
        .expect("Failed to get profile");
    assert_eq!(profile.trim(), "balanced");

    std::env::remove_var("ASUS_SYSFS_ROOT");
}
