use std::path::Path;

use crate::anime::VirtualAniMeDevice;
use crate::slash::VirtualSlashDevice;
use crate::sysfs_mock::MockSysfs;
use rog_anime::AnimeType;

/// End-to-end simulation harness for executing integration tests against mocked hardware profiles
pub struct TestHarness {
    sysfs: MockSysfs,
    anime: Option<VirtualAniMeDevice>,
    slash: Option<VirtualSlashDevice>,
}

impl TestHarness {
    /// Initialize a full test harness with mock sysfs and best-effort virtual devices
    pub fn new() -> std::io::Result<Self> {
        let sysfs = MockSysfs::new()?;
        let anime = VirtualAniMeDevice::try_create(AnimeType::GA401).ok();
        let slash = VirtualSlashDevice::try_create().ok();

        Ok(Self {
            sysfs,
            anime,
            slash,
        })
    }

    /// Access reference to the mock sysfs environment
    pub fn sysfs(&self) -> &MockSysfs {
        &self.sysfs
    }

    /// Path to the root of the mock sysfs tree
    pub fn sysfs_root(&self) -> &Path {
        self.sysfs.root_path()
    }

    /// Access virtual AniMe Matrix device if active
    pub fn anime(&self) -> Option<&VirtualAniMeDevice> {
        self.anime.as_ref()
    }

    /// Access virtual Slash Lightbar device if active
    pub fn slash(&self) -> Option<&VirtualSlashDevice> {
        self.slash.as_ref()
    }

    /// Explicitly shut down and drop resources managed by the harness
    pub fn shutdown(self) {
        drop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_initialization() {
        let harness = TestHarness::new().expect("Failed to initialize TestHarness");
        assert_eq!(
            harness
                .sysfs()
                .read_platform_profile()
                .expect("Failed to read profile"),
            "balanced\n"
        );
        assert!(harness.sysfs_root().exists());
    }
}
