use std::sync::Arc;

use config_traits::{StdConfig, StdConfigLoad};
use log::{debug, error, info};
use rog_anime::error::AnimeError;
use rog_anime::usb::get_anime_type;
use rog_anime::AnimeType;
use rog_aura::aura_capabilities::{
    parse_firmware_capabilities, parse_hardware_status, parse_power_state_readback,
};
use rog_aura::{AuraDeviceType, AuraModeNum};
use rog_platform::hid_raw::HidRaw;
use rog_platform::keyboard_led::KeyboardBacklight;
use rog_platform::usb_raw::USBRaw;
use rog_scsi::{open_device, ScsiType};
use rog_slash::error::SlashError;
use rog_slash::SlashType;
use tokio::sync::Mutex;

use crate::aura_anime::config::AniMeConfig;
use crate::aura_anime::AniMe;
use crate::aura_laptop::config::AuraConfig;
use crate::aura_laptop::Aura;
use crate::aura_scsi::config::ScsiConfig;
use crate::aura_scsi::ScsiAura;
use crate::aura_slash::config::SlashConfig;
use crate::aura_slash::Slash;
use crate::error::RogError;

pub enum _DeviceHandle {
    /// The AniMe devices require USBRaw as they are not HID devices
    Usb(USBRaw),
    HidRaw(HidRaw),
    LedClass(KeyboardBacklight),
    /// TODO
    MulticolourLed,
    None,
}

#[derive(Clone)]
pub enum DeviceHandle {
    Aura(Aura),
    Slash(Slash),
    /// The AniMe devices require USBRaw as they are not HID devices
    AniMe(AniMe),
    Scsi(ScsiAura),
    Ally(Arc<Mutex<HidRaw>>),
    OldAura(Arc<Mutex<HidRaw>>),
    /// TUF laptops have an aditional set of attributes added to the LED /sysfs/
    TufLedClass(Arc<Mutex<HidRaw>>),
    /// TODO
    MulticolourLed,
    None,
}

impl DeviceHandle {
    /// Try Slash HID. If one exists it is initialsed and returned.
    pub async fn new_slash_hid(
        device: Arc<Mutex<HidRaw>>,
        prod_id: &str,
    ) -> Result<Self, RogError> {
        debug!("Testing for HIDRAW Slash");
        let slash_type = SlashType::from_dmi();
        if matches!(slash_type, SlashType::Unsupported)
            || slash_type
                .prod_id_str()
                .to_lowercase()
                .trim_start_matches("0x")
                != prod_id
        {
            log::info!("Unknown or invalid slash: {prod_id:?}, skipping");
            return Err(RogError::NotFound("No slash device".to_string()));
        }
        info!("Found slash type {slash_type:?}: {prod_id}");

        let mut config = SlashConfig::new().load();
        config.slash_type = slash_type;
        let slash = Slash::new(Some(device), None, Arc::new(Mutex::new(config)));
        slash.do_initialization().await?;
        Ok(Self::Slash(slash))
    }

    /// Try Slash USB. If one exists it is initialsed and returned.
    pub async fn new_slash_usb() -> Result<Self, RogError> {
        debug!("Testing for USB Slash");
        let slash_type = SlashType::from_dmi();
        if matches!(slash_type, SlashType::Unsupported) {
            return Err(RogError::Slash(SlashError::NoDevice));
        }

        if let Ok(usb) = USBRaw::new(slash_type.prod_id()) {
            info!("Found Slash USB {slash_type:?}");

            let mut config = SlashConfig::new().load();
            config.slash_type = slash_type;
            let slash = Slash::new(
                None,
                Some(Arc::new(Mutex::new(usb))),
                Arc::new(Mutex::new(config)),
            );
            slash.do_initialization().await?;
            Ok(Self::Slash(slash))
        } else {
            Err(RogError::NotFound("No slash device found".to_string()))
        }
    }

    /// Try AniMe Matrix HID. If one exists it is initialsed and returned.
    pub async fn maybe_anime_hid(
        _device: Arc<Mutex<HidRaw>>,
        _prod_id: &str,
    ) -> Result<Self, RogError> {
        // TODO: can't use HIDRAW for anime at the moment
        Err(RogError::NotFound(
            "Can't use anime over hidraw yet. Skip.".to_string(),
        ))

        // debug!("Testing for HIDRAW AniMe");
        // let anime_type = AnimeType::from_dmi();
        // dbg!(prod_id);
        // if matches!(anime_type, AnimeType::Unsupported) || prod_id != "193b"
        // {     log::info!("Unknown or invalid AniMe: {prod_id:?},
        // skipping");     return Err(RogError::NotFound("No
        // anime-matrix device".to_string())); }
        // info!("Found AniMe Matrix HIDRAW {anime_type:?}: {prod_id}");

        // let mut config = AniMeConfig::new().load();
        // config.anime_type = anime_type;
        // let mut anime = AniMe::new(Some(device), None,
        // Arc::new(Mutex::new(config))); anime.do_initialization().
        // await?; Ok(Self::AniMe(anime))
    }

    pub async fn maybe_anime_usb() -> Result<Self, RogError> {
        debug!("Testing for USB AniMe");
        let anime_type = get_anime_type();
        if matches!(anime_type, AnimeType::Unsupported) {
            info!("No Anime Matrix capable laptop found");
            return Err(RogError::Anime(AnimeError::NoDevice));
        }

        if let Ok(usb) = USBRaw::new(0x193b) {
            info!("Found AniMe Matrix USB {anime_type:?}");

            let mut config = AniMeConfig::new().load();
            config.anime_type = anime_type;
            let mut anime = AniMe::new(
                None,
                Some(Arc::new(Mutex::new(usb))),
                Arc::new(Mutex::new(config)),
            );
            anime.do_initialization().await?;
            Ok(Self::AniMe(anime))
        } else {
            Err(RogError::NotFound(
                "No AnimeMatrix device found".to_string(),
            ))
        }
    }

    pub async fn maybe_scsi(dev_node: &str, prod_id: &str) -> Result<Self, RogError> {
        debug!("Testing for SCSI");
        let prod_id = ScsiType::from(prod_id);
        if prod_id == ScsiType::Unsupported {
            log::info!("Unknown or invalid SCSI: {prod_id:?}, skipping");
            return Err(RogError::NotFound("No SCSI device".to_string()));
        }
        info!("Found SCSI device {prod_id:?} on {dev_node}");

        let mut config = ScsiConfig::new().load();
        config.dev_type = AuraDeviceType::ScsiExtDisk;
        let dev = Arc::new(Mutex::new(open_device(dev_node)?));
        let scsi = ScsiAura::new(dev, Arc::new(Mutex::new(config)));
        scsi.do_initialization().await?;
        Ok(Self::Scsi(scsi))
    }

    pub async fn maybe_laptop_aura(
        device: Option<Arc<Mutex<HidRaw>>>,
        prod_id: &str,
    ) -> Result<Self, RogError> {
        debug!("Testing for laptop aura");
        let aura_type = AuraDeviceType::from(prod_id);
        if !matches!(
            aura_type,
            AuraDeviceType::LaptopKeyboard2021
                | AuraDeviceType::LaptopKeyboardPre2021
                | AuraDeviceType::LaptopKeyboardTuf
                | AuraDeviceType::Ally
        ) {
            log::info!("Unknown or invalid laptop aura: {prod_id:?}, skipping");
            return Err(RogError::NotFound("No laptop aura device".to_string()));
        }
        info!("Found laptop aura type {prod_id:?}");

        let backlight = KeyboardBacklight::new()
            .map_err(|e| error!("Keyboard backlight error: {e:?}"))
            .map_or(None, |k| {
                info!("Found sysfs backlight control");
                Some(Arc::new(Mutex::new(k)))
            });

        // Modern 0x19b6/0x1a30 keyboards expose their real mode table and
        // physical controller capabilities through read-only feature
        // reports.  Probe before loading the config so newly discovered modes
        // become ordinary D-Bus modes and retain saved colours where present.
        let mut detected_modes: Option<Vec<AuraModeNum>> = None;
        if aura_type.is_new_laptop() {
            if let Some(hid) = device.as_ref() {
                let hid = hid.lock().await;
                match hid.query_aura_status_report() {
                    Ok(Some(report)) => {
                        if let Some(status) = parse_hardware_status(&report) {
                            info!(
                                "Aura firmware status: type={:#04x} year={:#04x} layout={:#04x} regions={:#04x} features={:#04x} family={:#04x}",
                                status.keyboard_type,
                                status.keyboard_year,
                                status.layout,
                                status.region_bits,
                                status.feature_bits,
                                status.model_family
                            );
                        } else {
                            log::debug!("Aura firmware status response failed validation");
                        }
                    }
                    Ok(None) => log::debug!("Aura firmware status report is unavailable"),
                    Err(error) => log::debug!("Aura firmware status probe failed: {error}"),
                }

                match hid.query_aura_capability_report() {
                    Ok(Some(report)) => {
                        if let Some(capabilities) = parse_firmware_capabilities(&report) {
                            info!(
                                "Aura firmware modes: {:?} (mask={:#04x}{:#04x})",
                                capabilities.modes,
                                capabilities.mode_mask_high,
                                capabilities.mode_mask_low
                            );
                            if let Some(power) = parse_power_state_readback(&report) {
                                info!(
                                    "Aura firmware power readback: keyboard={:#04x} logo={:#04x} lightbar={:#04x} aero={:#04x} vcut={:#04x} bump={:#04x} rearglow={:#04x} ac_dc={:?}",
                                    power.keyboard,
                                    power.logo,
                                    power.lightbar,
                                    power.aero,
                                    power.vcut,
                                    power.bump,
                                    power.rear_glow,
                                    power.awake_ac_dc
                                );
                            }
                            detected_modes = Some(capabilities.modes);
                        } else {
                            log::debug!("Aura firmware capability response failed validation");
                        }
                    }
                    Ok(None) => log::debug!("Aura firmware capability report is unavailable"),
                    Err(error) => log::debug!("Aura firmware capability probe failed: {error}"),
                }
            }
        }

        // Load saved mode, colours, brightness, power from disk; apply on reload
        let mut config =
            AuraConfig::load_and_update_config_with_modes(prod_id, detected_modes.as_deref());
        config.led_type = aura_type;
        let aura = Aura {
            hid: device,
            backlight,
            config: Arc::new(Mutex::new(config)),
        };
        aura.do_initialization().await?;
        Ok(Self::Aura(aura))
    }
}
