use std::sync::Arc;

use config_traits::{StdConfig, StdConfigLoad};
use log::{debug, error, info, warn};
use rog_anime::AnimeType;
use rog_anime::error::AnimeError;
use rog_anime::usb::get_anime_type;
use rog_aura::AuraDeviceType;
use rog_platform::hid_raw::HidRaw;
use rog_platform::keyboard_led::KeyboardBacklight;
use rog_platform::usb_raw::USBRaw;
use rog_platform::{DynamicLed, ScsiLed, SlashLed};
use rog_scsi::ScsiType;
use rog_slash::SlashType;
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
    /// Try Slash sysfs LED. Presence of `asus::slash` is the capability gate;
    /// board DMI lists are not used.
    pub async fn maybe_slash() -> Result<Self, RogError> {
        debug!("Testing for Slash");
        let led = SlashLed::new().map_err(|e| {
            warn!("No Slash sysfs LED found: {e}");
            RogError::NotFound("No slash device found".to_string())
        })?;

        info!("Found Slash sysfs LED at {:?}", led.path());
        let mut config = SlashConfig::new().load();
        config.slash_type = SlashType::Unsupported;
        let slash = Slash::new(led, Arc::new(Mutex::new(config)));
        slash.do_initialization().await?;
        Ok(Self::Slash(slash))
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
        let scsi_type = ScsiType::from(prod_id);
        if scsi_type == ScsiType::Unsupported {
            log::info!("Unknown or invalid SCSI: {scsi_type:?}, skipping");
            return Err(RogError::NotFound("No SCSI device".to_string()));
        }

        let mut last_error = None;
        let mut led = None;
        for _ in 0..20 {
            match ScsiLed::find_for_dev(dev_node) {
                Ok(found) => {
                    led = Some(found);
                    break;
                }
                Err(err @ rog_platform::error::PlatformError::MissingFunction(_)) => {
                    last_error = Some(err);
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                Err(err) => return Err(err.into()),
            }
        }
        let led = led.ok_or_else(|| {
            let err = last_error
                .map(|err| err.to_string())
                .unwrap_or_else(|| "unknown discovery error".into());
            log::warn!("No exact SCSI Dynamic Lighting device found for {dev_node}: {err}");
            RogError::NotFound(format!(
                "Dynamic Lighting registration timed out for {dev_node}"
            ))
        })?;

        info!(
            "Found SCSI Dynamic Lighting device {scsi_type:?} on {:?}",
            led.path()
        );

        let mut config = ScsiConfig::new().load();
        config.dev_type = AuraDeviceType::ScsiExtDisk;
        let scsi = ScsiAura::new(led, Arc::new(Mutex::new(config)));
        scsi.do_initialization().await?;
        Ok(Self::Scsi(scsi))
    }

    pub async fn maybe_laptop_aura(
        hid: Option<Arc<Mutex<HidRaw>>>,
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

        // Check for Dynamic Lighting interface
        let (dynamic_global, dynamic_kbd, dynamic_lightbar) = {
            let global = DynamicLed::find("aura:global")
                .map(|g| {
                    info!("Dynamic Lighting global aggregate detected: aura:global");
                    Arc::new(Mutex::new(g))
                })
                .ok();
            let kbd = DynamicLed::find("aura:keyboard")
                .or_else(|_| DynamicLed::find("asus::kbd_backlight"))
                .map(|k| {
                    info!("Dynamic Lighting keyboard detected: {}", k.name());
                    Arc::new(Mutex::new(k))
                })
                .ok();
            let lb = DynamicLed::find("aura:lightbar")
                .map(|l| {
                    info!("Dynamic Lighting lightbar detected: aura:lightbar");
                    Arc::new(Mutex::new(l))
                })
                .ok();
            (global, kbd, lb)
        };

        let dynamic_available =
            dynamic_global.is_some() || dynamic_kbd.is_some() || dynamic_lightbar.is_some();
        let fallback_available = if matches!(aura_type, AuraDeviceType::LaptopKeyboardTuf) {
            backlight.is_some()
        } else {
            hid.is_some()
        };
        if !dynamic_available && !fallback_available {
            debug!("Neither valid Dynamic Lighting nor device-specific fallback detected");
            return Err(RogError::NotFound(
                "No Dynamic Lighting or legacy Aura control path found".to_string(),
            ));
        }

        // Load saved mode, colours, brightness, power from disk; apply on reload
        let mut config = AuraConfig::load_and_update_config(prod_id);
        config.led_type = aura_type;
        let use_hid = !dynamic_available;
        let aura = Aura {
            dynamic_global,
            dynamic_kbd,
            dynamic_lightbar,
            // One device has exactly one owner: valid Dynamic Lighting nodes win,
            // otherwise retain the matching hidraw handle for released kernels.
            hid: use_hid.then_some(hid).flatten(),
            backlight,
            config: Arc::new(Mutex::new(config)),
        };
        aura.do_initialization().await?;
        Ok(Self::Aura(aura))
    }
}
