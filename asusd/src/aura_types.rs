use std::sync::Arc;

use config_traits::{StdConfig, StdConfigLoad};
use log::{debug, error, info, warn};
use rog_anime::AnimeType;
use rog_anime::error::AnimeError;
use rog_anime::usb::get_anime_type;
use rog_aura::AuraDeviceType;
use rog_platform::DynamicLed;
use rog_platform::SlashLed;
use rog_platform::XboxLogoLed;
use rog_platform::hid_raw::HidRaw;
use rog_platform::keyboard_led::KeyboardBacklight;
use rog_platform::usb_raw::USBRaw;
use rog_scsi::{ScsiType, open_device};
use rog_slash::SlashType;
use rog_slash::error::SlashError;
use tokio::sync::Mutex;

use crate::aura_anime::AniMe;
use crate::aura_anime::config::AniMeConfig;
use crate::aura_laptop::Aura;
use crate::aura_laptop::config::AuraConfig;
use crate::aura_scsi::ScsiAura;
use crate::aura_scsi::config::ScsiConfig;
use crate::aura_slash::Slash;
use crate::aura_slash::config::SlashConfig;
use crate::error::RogError;

pub enum _DeviceHandle {
    /// The AniMe devices require USBRaw as they are not HID devices
    Usb(USBRaw),
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
    /// TODO
    MulticolourLed,
    None,
}

impl DeviceHandle {
    /// Prefer `asus::slash` Dynamic Lighting when the kernel exposes it.
    pub async fn maybe_slash() -> Result<Self, RogError> {
        debug!("Testing for Slash sysfs LED");
        let led = SlashLed::new().map_err(|e| {
            warn!("No Slash sysfs LED found: {e}");
            RogError::NotFound("No slash device found".to_string())
        })?;

        info!("Found Slash sysfs LED at {:?}", led.path());
        let mut config = SlashConfig::new().load();
        // Packet layout unused under sysfs; keep DMI type for UI/logging.
        config.slash_type = SlashType::from_dmi();
        let slash = Slash::new(Some(led), None, None, Arc::new(Mutex::new(config)));
        slash.do_initialization().await?;
        Ok(Self::Slash(slash))
    }

    /// Legacy Slash HID when no `asus::slash` node is present.
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
        let slash = Slash::new(None, Some(device), None, Arc::new(Mutex::new(config)));
        slash.do_initialization().await?;
        Ok(Self::Slash(slash))
    }

    /// Legacy Slash USB when no `asus::slash` node is present.
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
                None,
                Some(Arc::new(Mutex::new(usb))),
                Arc::new(Mutex::new(config)),
            );
            slash.do_initialization().await?;
            Ok(Self::Slash(slash))
        } else {
            Err(RogError::Slash(SlashError::NoDevice))
        }
    }

    /// Prefer `asus::anime` Dynamic Lighting; fall back to USB only when the
    /// kernel node is absent (older kernels without AniMe DL).
    pub async fn maybe_anime() -> Result<Self, RogError> {
        debug!("Testing for AniMe Matrix");
        let mut anime_type = get_anime_type();

        if let Ok(led) = DynamicLed::find("asus::anime") {
            if matches!(anime_type, AnimeType::Unsupported) {
                // Kernel uses a GA402-like layout for unknown AniMe boards.
                anime_type = AnimeType::GA402;
                info!("AniMe DL on unknown board; using GA402 layout");
            }
            info!(
                "Found AniMe Matrix Dynamic Lighting ({anime_type:?}, {})",
                led.name()
            );
            let mut config = AniMeConfig::new().load();
            config.anime_type = anime_type;
            let mut anime = AniMe::new(
                None,
                Some(Arc::new(Mutex::new(led))),
                Arc::new(Mutex::new(config)),
            );
            anime.do_initialization().await?;
            return Ok(Self::AniMe(anime));
        }

        if matches!(anime_type, AnimeType::Unsupported) {
            info!("No Anime Matrix capable laptop found");
            return Err(RogError::Anime(AnimeError::NoDevice));
        }

        if let Ok(usb) = USBRaw::new(0x193b) {
            info!("Found AniMe Matrix USB {anime_type:?}");

            let mut config = AniMeConfig::new().load();
            config.anime_type = anime_type;
            let mut anime = AniMe::new(
                Some(Arc::new(Mutex::new(usb))),
                None,
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

        // Prefer asus::keyboard; on TUF, asus-wmi may register the same DL ABI
        // on asus::kbd_backlight when ASUS_WMI_DEVID_TUF_RGB_MODE* is present.
        let (dynamic_kbd, dynamic_lightbar, dynamic_logo, dl_owns_kbd_backlight) = {
            let kbd = DynamicLed::find("asus::keyboard")
                .or_else(|_| DynamicLed::find("asus::kbd_backlight"))
                .ok();
            let dl_owns_kbd_backlight = kbd
                .as_ref()
                .is_some_and(|k| k.name().contains("kbd_backlight"));
            let kbd = kbd.map(|k| {
                info!("Dynamic Lighting keyboard detected: {}", k.name());
                Arc::new(Mutex::new(k))
            });
            let lb = DynamicLed::find("asus::lightbar")
                .map(|l| {
                    info!("Dynamic Lighting lightbar detected: {}", l.name());
                    Arc::new(Mutex::new(l))
                })
                .ok();
            let logo = DynamicLed::find("asus::logo")
                .map(|l| {
                    info!("Dynamic Lighting logo detected: {}", l.name());
                    Arc::new(Mutex::new(l))
                })
                .ok();
            if kbd.is_some() || lb.is_some() || logo.is_some() {
                (kbd, lb, logo, dl_owns_kbd_backlight)
            } else {
                debug!(
                    "Dynamic Lighting not detected; using legacy hidraw / TUF platform fallback"
                );
                (None, None, None, false)
            }
        };

        // When any keyboard DL node is present it owns brightness. Do not also
        // keep a WMI/ally KeyboardBacklight handle (dual owner on G614-class).
        let backlight = if dynamic_kbd.is_some() {
            if dl_owns_kbd_backlight {
                debug!("Skipping legacy KeyboardBacklight; Dynamic Lighting owns kbd_backlight");
            } else {
                debug!(
                    "Skipping legacy KeyboardBacklight; Dynamic Lighting owns keyboard brightness"
                );
            }
            None
        } else {
            KeyboardBacklight::new()
                .map_err(|e| error!("Keyboard backlight error: {e:?}"))
                .map_or(None, |k| {
                    info!("Found sysfs backlight control");
                    Some(Arc::new(Mutex::new(k)))
                })
        };

        // One owner: when DL validates, drop hidraw so two paths never fight.
        let hid = if dynamic_kbd.is_some() || dynamic_lightbar.is_some() || dynamic_logo.is_some() {
            None
        } else {
            device
        };

        let xbox_logo = XboxLogoLed::new()
            .map(|led| {
                info!("Found Xbox Guide LED at {:?}", led.path());
                Arc::new(Mutex::new(led))
            })
            .ok();

        // Load saved mode, colours, brightness, power from disk; apply on reload
        let mut config = AuraConfig::load_and_update_config(prod_id);
        config.led_type = aura_type;
        if dynamic_kbd.is_none()
            && dynamic_lightbar.is_none()
            && dynamic_logo.is_none()
            && hid.is_none()
            && backlight.is_none()
        {
            return Err(RogError::NotFound(
                "No usable Aura lighting backend".to_string(),
            ));
        }

        let aura = Aura {
            dynamic_kbd,
            dynamic_lightbar,
            dynamic_logo,
            hid,
            backlight,
            xbox_logo,
            config: Arc::new(Mutex::new(config)),
        };
        aura.do_initialization().await?;
        Ok(Self::Aura(aura))
    }
}
