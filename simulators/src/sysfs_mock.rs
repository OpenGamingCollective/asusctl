use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

/// Mock virtual sysfs tree for simulating ASUS laptop hardware features
pub struct MockSysfs {
    dir: TempDir,
}

impl MockSysfs {
    /// Create a new mock sysfs environment initialized with typical ASUS ROG hardware nodes
    pub fn new() -> std::io::Result<Self> {
        let dir = TempDir::new()?;
        let sysfs = Self { dir };

        sysfs.init_platform_profile("balanced\n", "quiet balanced performance\n")?;
        sysfs.init_power_supply(1, 80)?;
        sysfs.init_asus_wmi()?;

        Ok(sysfs)
    }

    pub fn root_path(&self) -> &Path {
        self.dir.path()
    }

    fn init_platform_profile(&self, current: &str, choices: &str) -> std::io::Result<()> {
        let profile_dir = self.dir.path().join("firmware/acpi");
        fs::create_dir_all(&profile_dir)?;

        let mut f_curr = File::create(profile_dir.join("platform_profile"))?;
        f_curr.write_all(current.as_bytes())?;

        let mut f_choices = File::create(profile_dir.join("platform_profile_choices"))?;
        f_choices.write_all(choices.as_bytes())?;

        Ok(())
    }

    fn init_power_supply(&self, online: u8, charge_limit: u8) -> std::io::Result<()> {
        let bat_dir = self.dir.path().join("class/power_supply/BAT0");
        let ac_dir = self.dir.path().join("class/power_supply/ADP1");
        fs::create_dir_all(&bat_dir)?;
        fs::create_dir_all(&ac_dir)?;

        let mut f_lim = File::create(bat_dir.join("charge_control_end_threshold"))?;
        f_lim.write_all(charge_limit.to_string().as_bytes())?;

        let mut f_ac = File::create(ac_dir.join("online"))?;
        f_ac.write_all(online.to_string().as_bytes())?;

        Ok(())
    }

    fn init_asus_wmi(&self) -> std::io::Result<()> {
        let wmi_dir = self.dir.path().join("devices/platform/asus-wmi");
        fs::create_dir_all(&wmi_dir)?;

        let nodes = [
            ("panel_od", "1\n"),
            ("dgpu_disable", "0\n"),
            ("egpu_enable", "0\n"),
            ("ppt_pl1_spl", "35\n"),
            ("ppt_pl2_sppt", "65\n"),
            ("ppt_fppt", "80\n"),
        ];

        for (node, val) in nodes {
            let mut f = File::create(wmi_dir.join(node))?;
            f.write_all(val.as_bytes())?;
        }

        Ok(())
    }

    pub fn set_platform_profile(&self, profile: &str) -> std::io::Result<()> {
        let path = self.dir.path().join("firmware/acpi/platform_profile");
        fs::write(path, profile.as_bytes())
    }

    pub fn read_platform_profile(&self) -> std::io::Result<String> {
        let path = self.dir.path().join("firmware/acpi/platform_profile");
        fs::read_to_string(path)
    }

    pub fn set_charge_limit(&self, limit: u8) -> std::io::Result<()> {
        let path = self
            .dir
            .path()
            .join("class/power_supply/BAT0/charge_control_end_threshold");
        fs::write(path, limit.to_string().as_bytes())
    }

    pub fn read_charge_limit(&self) -> std::io::Result<u8> {
        let path = self
            .dir
            .path()
            .join("class/power_supply/BAT0/charge_control_end_threshold");
        let content = fs::read_to_string(path)?;
        content
            .trim()
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sysfs_mock_creation_and_updates() {
        let mock = MockSysfs::new().expect("Failed to create MockSysfs");

        assert_eq!(
            mock.read_platform_profile()
                .expect("Failed to read profile"),
            "balanced\n"
        );
        assert_eq!(mock.read_charge_limit().expect("Failed to read limit"), 80);

        mock.set_platform_profile("performance\n")
            .expect("Failed to update profile");
        assert_eq!(
            mock.read_platform_profile()
                .expect("Failed to read profile"),
            "performance\n"
        );

        mock.set_charge_limit(60)
            .expect("Failed to update charge limit");
        assert_eq!(mock.read_charge_limit().expect("Failed to read limit"), 60);
    }
}
