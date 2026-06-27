use log::warn;

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Clone)]
pub struct DMIID {
    pub id_model: String,
    pub dmi_family: String,
    pub dmi_vendor: String,
    pub board_name: String,
    pub board_vendor: String,
    pub bios_date: String,
    pub bios_release: String,
    pub bios_vendor: String,
    pub bios_version: String,
    pub product_family: String,
    pub product_name: String,
}

impl DMIID {
    pub fn new() -> Result<Self, String> {
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{err}");
            format!("dmi enumerator failed: {err}")
        })?;

        enumerator.match_subsystem("dmi").map_err(|err| {
            warn!("{err}");
            format!("dmi match_subsystem failed: {err}")
        })?;

        let mut result = enumerator.scan_devices().map_err(|err| {
            warn!("{err}");
            format!("dmi scan_devices failed: {err}")
        })?;

        if let Some(device) = (result).next() {
            return Ok(Self {
                id_model: device
                    .property_value("ID_MODEL")
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("Unknown".to_string()),
                dmi_family: device
                    .property_value("DMI_FAMILY")
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("Unknown".to_string()),
                dmi_vendor: device
                    .property_value("DMI_VENDOR")
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("Unknown".to_string()),
                board_name: device
                    .attribute_value("board_name")
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("Unknown".to_string()),
                board_vendor: device
                    .attribute_value("board_vendor")
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("Unknown".to_string()),
                bios_date: device
                    .attribute_value("bios_date")
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("Unknown".to_string()),
                bios_release: device
                    .attribute_value("bios_release")
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("Unknown".to_string()),
                bios_vendor: device
                    .attribute_value("bios_vendor")
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("Unknown".to_string()),
                bios_version: device
                    .attribute_value("bios_version")
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("Unknown".to_string()),
                product_family: device
                    .attribute_value("product_family")
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("Unknown".to_string()),
                product_name: device
                    .attribute_value("product_name")
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("Unknown".to_string()),
            });
        }
        Err("dmi not found".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Does not run in docker images"]
    fn dmi_sysfs_properties_not_unknown() {
        let dmi = DMIID::new().unwrap();

        assert_ne!(dmi.id_model, "Unknown".to_string());
        dbg!(dmi.id_model);
        assert_ne!(dmi.dmi_family, "Unknown".to_string());
        dbg!(dmi.dmi_family);
        assert_ne!(dmi.dmi_vendor, "Unknown".to_string());
        dbg!(dmi.dmi_vendor);
        assert_ne!(dmi.board_name, "Unknown".to_string());
        dbg!(dmi.board_name);
        assert_ne!(dmi.board_vendor, "Unknown".to_string());
        dbg!(dmi.board_vendor);
        assert_ne!(dmi.product_family, "Unknown".to_string());
        dbg!(dmi.product_family);
        assert_ne!(dmi.product_name, "Unknown".to_string());
        dbg!(dmi.product_name);
    }
}
