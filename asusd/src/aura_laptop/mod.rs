use std::sync::Arc;

use config::AuraConfig;
use config_traits::StdConfig;
use log::{debug, info};
use rog_aura::keyboard::{AuraLaptopUsbPackets, LedUsbPackets};
use rog_aura::usb::{AURA_LAPTOP_LED_APPLY, AURA_LAPTOP_LED_SET};
use rog_aura::{AURA_LAPTOP_LED_MSG_LEN, AuraDeviceType, AuraEffect, LedBrightness, PowerZones};
use rog_platform::DynamicLed;
use rog_platform::error::PlatformError;
use rog_platform::hid_raw::HidRaw;
use rog_platform::keyboard_led::KeyboardBacklight;
use tokio::sync::{Mutex, MutexGuard};

use crate::error::RogError;

pub mod config;
pub mod trait_impls;

#[derive(Debug, Clone)]
pub struct Aura {
    pub dynamic_global: Option<Arc<Mutex<DynamicLed>>>,
    pub dynamic_kbd: Option<Arc<Mutex<DynamicLed>>>,
    pub dynamic_lightbar: Option<Arc<Mutex<DynamicLed>>>,
    pub hid: Option<Arc<Mutex<HidRaw>>>,
    pub backlight: Option<Arc<Mutex<KeyboardBacklight>>>,
    pub config: Arc<Mutex<AuraConfig>>,
}

impl Aura {
    #[must_use]
    pub fn has_dynamic_lighting(&self) -> bool {
        self.dynamic_kbd.is_some()
            || self.dynamic_global.is_some()
            || self.dynamic_lightbar.is_some()
    }

    /// `aura:global`, then keyboard, then lightbar. Callers that need a
    /// different order keep their own list.
    fn dynamic_nodes(&self) -> [Option<&Arc<Mutex<DynamicLed>>>; 3] {
        [
            self.dynamic_global.as_ref(),
            self.dynamic_kbd.as_ref(),
            self.dynamic_lightbar.as_ref(),
        ]
    }

    /// Node whose `effect_index` describes the basic modes. Unified topology
    /// advertises them on `aura:global`; otherwise the keyboard node does.
    async fn effect_source(&self) -> Option<MutexGuard<'_, DynamicLed>> {
        if let Some(global) = &self.dynamic_global {
            Some(global.lock().await)
        } else if let Some(kbd) = &self.dynamic_kbd {
            Some(kbd.lock().await)
        } else {
            None
        }
    }

    async fn set_asus_topology(&self, mode: &str) -> Result<(), RogError> {
        for led in self.dynamic_nodes().into_iter().flatten() {
            let led = led.lock().await;
            if led.has_asus_aura_mode() {
                led.set_asus_aura_mode(mode)?;
                return Ok(());
            }
        }
        Err(RogError::MissingFunction(
            "ASUS Dynamic Lighting topology control is unavailable".into(),
        ))
    }

    /// Initialise the device if required.
    pub async fn do_initialization(&self) -> Result<(), RogError> {
        // With a chassis lightbar, prefer split so keyboard and lightbar are
        // independently writable (matches kernel auto→split). Unified remains
        // available when userspace wants a single global effect.
        if self.dynamic_lightbar.is_some() {
            self.set_asus_topology("split").await?;
        }
        Ok(())
    }

    pub async fn lock_config(&self) -> MutexGuard<'_, AuraConfig> {
        self.config.lock().await
    }

    /// Will lock the internal config and update. If anything else has locked
    /// this in scope then a deadlock can occur.
    pub async fn update_config(&self) -> Result<(), RogError> {
        let bright = self.get_brightness().await;
        let mut config = self.config.lock().await;
        let bright = bright.unwrap_or(config.brightness);
        config.read();
        config.brightness = bright;
        config.write();
        Ok(())
    }

    pub async fn write_current_config_mode(&self, config: &mut AuraConfig) -> Result<(), RogError> {
        if config.multizone_on {
            let mode = config.current_mode;
            let mut create = false;
            // There is no multizone config for this mode so create one here
            // using the colours of rainbow if it exists, or first available
            // mode, or random
            if config.multizone.is_none() {
                create = true;
            } else if let Some(multizones) = config.multizone.as_ref()
                && !multizones.contains_key(&mode)
            {
                create = true;
            }
            if create {
                info!("No user-set config for zone founding, attempting a default");
                config.create_multizone_default()?;
            }

            if let Some(multizones) = config.multizone.as_mut()
                && let Some(set) = multizones.get(&mode)
            {
                for mode in set.clone() {
                    self.write_effect_and_apply(config.led_type, &mode).await?;
                }
            }
        } else {
            let mode = config.current_mode;
            if let Some(effect) = config.builtins.get(&mode).cloned() {
                self.write_effect_and_apply(config.led_type, &effect)
                    .await?;
            }
        }

        Ok(())
    }

    /// Write the AuraEffect to the device. Will lock `backlight` or `hid`.
    ///
    /// If per-key or software-mode is active it must be marked as disabled in
    /// config.
    pub async fn write_effect_and_apply(
        &self,
        dev_type: AuraDeviceType,
        mode: &AuraEffect,
    ) -> Result<(), RogError> {
        // Priority: Dynamic Lighting sysfs interface.
        // Key1–4 map to the whole keyboard node; BarLeft/BarRight to lightbar.
        // Legacy Logo / other zones remain unsupported under DL.
        if self.has_dynamic_lighting() && !supports_dynamic_zone(mode.zone) {
            return Err(RogError::MissingFunction(
                "Dynamic Lighting exposes keyboard/lightbar topology, not legacy Aura subzones"
                    .into(),
            ));
        }
        if self.has_dynamic_lighting()
            && let Some(eff_str) = mode.mode.to_dynamic_effect_str()
        {
            let speed = mode.speed.to_dynamic_speed();
            let dir_str = mode.direction.to_dynamic_direction_str();
            let palette = mode.to_dynamic_palette();

            let apply_to_led = |led: &DynamicLed| -> Result<bool, RogError> {
                if !led.supports_effect(eff_str)? {
                    return Ok(false);
                }
                if led.has_speed() && map_dl_result(led.set_supported_speed(speed))?.is_none() {
                    return Ok(false);
                }
                if led.has_direction()
                    && map_dl_result(led.set_supported_direction(dir_str))?.is_none()
                {
                    return Ok(false);
                }
                if led.has_effects_palette()
                    && !palette.is_empty()
                    && map_dl_result(led.set_palette_colors(&palette))?.is_none()
                {
                    return Ok(false);
                }
                Ok(map_dl_result(led.set_effect(eff_str))?.is_some())
            };

            match mode.zone {
                rog_aura::AuraZone::BarLeft | rog_aura::AuraZone::BarRight => {
                    self.set_asus_topology("split").await?;
                    if let Some(lb) = &self.dynamic_lightbar {
                        let led = lb.lock().await;
                        if apply_to_led(&led)? {
                            return Ok(());
                        }
                    }
                }
                rog_aura::AuraZone::None => {
                    // Whole-device: only unified topology uses aura:global.
                    // Otherwise (split / auto→split) fan out to keyboard and
                    // lightbar so both zones get the same effect.
                    let unified = if let Some(global) = &self.dynamic_global {
                        let led = global.lock().await;
                        led.has_asus_aura_mode()
                            && led.get_asus_aura_mode().ok().as_deref() == Some("unified")
                    } else {
                        false
                    };
                    if unified && let Some(global) = &self.dynamic_global {
                        let global_led = global.lock().await;
                        if apply_to_led(&global_led)? {
                            return Ok(());
                        }
                    }
                    if let Some(kbd) = &self.dynamic_kbd {
                        let kbd_led = kbd.lock().await;
                        let mut any = apply_to_led(&kbd_led)?;
                        if let Some(lb) = &self.dynamic_lightbar {
                            let lb_led = lb.lock().await;
                            any |= apply_to_led(&lb_led)?;
                        }
                        if any {
                            return Ok(());
                        }
                    }
                }
                // Key1–4: whole keyboard node under DL (no legacy 4-zone split).
                _ => {
                    if self.dynamic_lightbar.is_some() {
                        self.set_asus_topology("split").await?;
                    }
                    if let Some(kbd) = &self.dynamic_kbd {
                        let kbd_led = kbd.lock().await;
                        if apply_to_led(&kbd_led)? {
                            return Ok(());
                        }
                    }
                }
            }
        }

        // When Dynamic Lighting is active, do not fall back to raw hidraw or TUF
        // platform
        if self.has_dynamic_lighting() {
            return Err(RogError::MissingFunction(
                "Dynamic lighting mode or zone not supported by kernel".to_string(),
            ));
        }

        // Fallback is selected only when no valid Dynamic Lighting node exists.
        if matches!(dev_type, AuraDeviceType::LaptopKeyboardTuf)
            && let Some(platform) = &self.backlight
        {
            let buf = [
                1, mode.mode as u8, mode.colour1.r, mode.colour1.g, mode.colour1.b,
                mode.speed as u8,
            ];
            platform.lock().await.set_kbd_rgb_mode(&buf)?;
            return Ok(());
        } else if let Some(hid_raw) = &self.hid {
            let bytes: [u8; AURA_LAPTOP_LED_MSG_LEN] = mode.into();
            let hid_raw = hid_raw.lock().await;
            hid_raw.write_bytes(&pad_hid_report(&bytes))?;
            hid_raw.write_bytes(&pad_hid_report(&AURA_LAPTOP_LED_SET))?;
            hid_raw.write_bytes(&pad_hid_report(&AURA_LAPTOP_LED_APPLY))?;
            return Ok(());
        }

        Err(RogError::NoAuraKeyboard)
    }

    /// Nodes that hold live brightness for the current ASUS topology.
    /// Kernel `auto` resolves to split, so keyboard/lightbar are canonical.
    async fn topology_brightness_slots(&self) -> [Option<&Arc<Mutex<DynamicLed>>>; 3] {
        let mut mode = None;
        for slot in self.dynamic_nodes().into_iter().flatten() {
            let led = slot.lock().await;
            if led.has_asus_aura_mode() {
                mode = led.get_asus_aura_mode().ok();
                break;
            }
        }
        match mode.as_deref() {
            Some("unified") => [
                self.dynamic_global.as_ref(),
                None,
                None,
            ],
            Some("split") | Some("auto") => [
                self.dynamic_kbd.as_ref(),
                self.dynamic_lightbar.as_ref(),
                None,
            ],
            _ => [
                self.dynamic_kbd.as_ref(),
                self.dynamic_lightbar.as_ref(),
                self.dynamic_global.as_ref(),
            ],
        }
    }

    /// Read the current brightness as a 4-step [`LedBrightness`] level.
    ///
    /// Dynamic Lighting nodes may advertise `max_brightness > 3`; those values
    /// are scaled back to Off/Low/Med/High. LED class reads do not return
    /// `-EBUSY`, so inactive nodes can still look readable — follow `aura_mode`
    /// (kernel `auto` is split) instead of trusting the first node that replies.
    pub async fn get_brightness(&self) -> Result<LedBrightness, RogError> {
        for slot in self.topology_brightness_slots().await.into_iter().flatten() {
            let led = slot.lock().await;
            let Some(max) = map_dl_result(led.get_max_brightness())? else {
                continue;
            };
            let Some(value) = map_dl_result(led.get_brightness())? else {
                continue;
            };
            return Ok(LedBrightness::from_scaled(value, max));
        }
        if let Some(backlight) = &self.backlight {
            let value = backlight.lock().await.get_brightness()?;
            return Ok(value.into());
        }
        Err(RogError::MissingFunction(
            "No LED backlight control available".to_string(),
        ))
    }

    /// Set keyboard/Aura brightness from a 4-step [`LedBrightness`] level.
    ///
    /// When Dynamic Lighting nodes are present the value is scaled to each
    /// *active* node's `max_brightness`. Under ASUS `aura_mode=unified` only
    /// `aura:global` accepts writes (`keyboard`/`lightbar` return `-EBUSY`);
    /// under `split` the reverse is true. Inactive-node errors are skipped so
    /// Fn-key brightness works in either topology. The legacy
    /// `KeyboardBacklight` (0..=3) is also updated when present.
    pub async fn set_brightness(&self, brightness: LedBrightness) -> Result<(), RogError> {
        let mut applied = false;
        for led in self.dynamic_nodes().into_iter().flatten() {
            let led = led.lock().await;
            let max = led.get_max_brightness()?;
            if map_dl_result(led.set_brightness(brightness.to_scaled(max)))?.is_some() {
                applied = true;
            }
        }

        if let Some(backlight) = &self.backlight {
            match backlight.lock().await.set_brightness(brightness.into()) {
                Ok(()) => applied = true,
                Err(e) if applied => {
                    debug!("WMI kbd backlight brightness sync failed: {e}");
                }
                Err(e) => return Err(e.into()),
            }
        }

        if applied {
            Ok(())
        } else {
            Err(RogError::MissingFunction(
                "No LED backlight control available".to_string(),
            ))
        }
    }

    /// Set combination state for boot animation/sleep animation/all leds/keys
    /// leds/side leds LED active
    pub async fn set_power_states(&self, config: &AuraConfig) -> Result<(), RogError> {
        if self.has_dynamic_lighting() {
            let mut requested = Vec::new();
            for state in &config.enabled.states {
                if state.boot {
                    requested.push("boot");
                }
                if state.awake {
                    requested.push("awake");
                }
                if state.sleep {
                    requested.push("sleep");
                }
                if state.shutdown {
                    requested.push("shutdown");
                }
            }
            let mut applied = false;
            let mut saw_power_states = false;
            let mut unsupported_request = false;
            for led in self.dynamic_nodes().into_iter().flatten() {
                let led = led.lock().await;
                if !led.has_power_states() {
                    continue;
                }
                saw_power_states = true;
                let states = filter_power_states(&requested, &led.get_power_states_index()?);
                if states.is_empty() && !requested.is_empty() {
                    unsupported_request = true;
                    continue;
                }
                if map_dl_result(led.set_power_states(&states.join(" ")))?.is_some() {
                    applied = true;
                }
            }
            if applied {
                return Ok(());
            }
            if !saw_power_states {
                return Err(RogError::MissingFunction(
                    "Dynamic Lighting node does not expose power_states".into(),
                ));
            }
            if unsupported_request {
                return Err(RogError::MissingFunction(
                    "No requested power state is supported by this Dynamic Lighting node".into(),
                ));
            }
            return Err(RogError::MissingFunction(
                "No active Dynamic Lighting node accepted power_states".into(),
            ));
        }

        if matches!(config.led_type, rog_aura::AuraDeviceType::LaptopKeyboardTuf)
            && let Some(backlight) = &self.backlight
        {
            // TODO: tuf bool array
            let buf = config.enabled.to_bytes(config.led_type);
            backlight.lock().await.set_kbd_rgb_state(&buf)?;
        } else if let Some(hid_raw) = &self.hid {
            let hid_raw = hid_raw.lock().await;
            if let Some(p) = config.enabled.states.first()
                && p.zone == PowerZones::Ally
            {
                hid_raw.write_bytes(&[
                    0x5d,
                    0xd1,
                    0x09,
                    0x01,
                    p.new_to_byte() as u8,
                    0,
                    0,
                ])?;
                return Ok(());
            }
            let bytes = config.enabled.to_bytes(config.led_type);
            hid_raw.write_bytes(&[
                0x5d, 0xbd, 0x01, bytes[0], bytes[1], bytes[2], bytes[3],
            ])?;
        }
        Ok(())
    }

    /// Write an effect block. This is for per-key, but can be repurposed to
    /// write the raw factory mode packets - when doing this it is expected that
    /// only the first `Vec` (`effect[0]`) is valid.
    pub async fn write_effect_block(
        &self,
        config: &mut AuraConfig,
        effect: &AuraLaptopUsbPackets,
    ) -> Result<(), RogError> {
        if config.brightness == LedBrightness::Off {
            config.brightness = LedBrightness::Med;
            config.write();
        }

        if matches!(config.led_type, rog_aura::AuraDeviceType::LaptopKeyboardTuf)
            && !self.has_dynamic_lighting()
            && let Some(tuf) = &self.backlight
        {
            for row in effect.iter() {
                let rgb = row.get(9..12).ok_or(PlatformError::InvalidValue)?;
                let [r, g, b] = rgb else {
                    return Err(PlatformError::InvalidValue.into());
                };
                tuf.lock().await.set_kbd_rgb_mode(&[
                    0, 0, *r, *g, *b, 0,
                ])?;
            }
            return Ok(());
        }

        let dynamic_leds = [
            self.dynamic_kbd.as_ref(),
            self.dynamic_global.as_ref(),
        ];
        for maybe_led in dynamic_leds.into_iter().flatten() {
            let dynamic = maybe_led.lock().await;
            let led_count = dynamic.get_led_count()? as usize;
            let rgb_buf = direct_rgb_payload(effect, led_count)?;
            if !rgb_buf.is_empty() {
                if map_dl_result(dynamic.write_direct(&rgb_buf))?.is_none() {
                    debug!(
                        "Dynamic lighting node '{}' busy (-EBUSY), trying alternate",
                        dynamic.name()
                    );
                    continue;
                }
                config.per_key_mode_active = true;
                return Ok(());
            }
        }
        if self.has_dynamic_lighting() {
            return Err(RogError::MissingFunction(
                "No active Dynamic Lighting node accepted the direct frame".into(),
            ));
        }

        if let Some(hid_raw) = &self.hid {
            let first = effect.first().ok_or(PlatformError::InvalidValue)?;
            let packet_type = *first.get(1).ok_or(PlatformError::InvalidValue)?;
            if effect.iter().any(|row| {
                row.first() != Some(&0x5d)
                    || row.len() > 64
                    || (packet_type == 0xbc && row.len() != 64)
            }) {
                return Err(PlatformError::InvalidValue.into());
            }
            let hid_raw = hid_raw.lock().await;
            if packet_type != 0xbc {
                config.per_key_mode_active = false;
                hid_raw.write_bytes(first)?;
                hid_raw.write_bytes(&AURA_LAPTOP_LED_SET)?;
            } else {
                if !config.per_key_mode_active {
                    hid_raw.write_bytes(&LedUsbPackets::get_init_msg())?;
                    config.per_key_mode_active = true;
                }
                for row in effect {
                    hid_raw.write_bytes(row)?;
                }
            }
            return Ok(());
        }

        Err(RogError::NoAuraKeyboard)
    }

    pub async fn fix_ally_power(&mut self) -> Result<(), RogError> {
        let needs_fix = {
            let config = self.config.lock().await;
            config.led_type == AuraDeviceType::Ally && config.ally_fix.is_none()
        };
        if needs_fix && let Some(hid_raw) = &self.hid {
            hid_raw.lock().await.write_bytes(&[
                0x5d, 0xbd, 0x01, 0xff, 0xff, 0xff, 0xff,
            ])?;
            let mut config = self.config.lock().await;
            config.ally_fix = Some(true);
            config.write();
        }
        Ok(())
    }
}

fn direct_rgb_payload(
    packets: &AuraLaptopUsbPackets,
    led_count: usize,
) -> Result<Vec<u8>, PlatformError> {
    let expected = led_count
        .checked_mul(3)
        .ok_or(PlatformError::InvalidValue)?;
    let first = packets.first().ok_or(PlatformError::InvalidValue)?;
    let mut rgb = Vec::with_capacity(expected);
    if packets.len() == 1 && led_count == 4 {
        rgb.extend_from_slice(first.get(9..21).ok_or(PlatformError::InvalidValue)?);
    } else {
        for packet in packets {
            if packet.get(1).copied() != Some(0xbc) {
                return Err(PlatformError::InvalidValue);
            }
            let count = *packet.get(7).ok_or(PlatformError::InvalidValue)? as usize;
            let payload_len = count.checked_mul(3).ok_or(PlatformError::InvalidValue)?;
            let end = 9usize
                .checked_add(payload_len)
                .ok_or(PlatformError::InvalidValue)?;
            rgb.extend_from_slice(packet.get(9..end).ok_or(PlatformError::InvalidValue)?);
        }
    }
    if rgb.len() != expected {
        return Err(PlatformError::InvalidValue);
    }
    Ok(rgb)
}

pub(super) fn supports_dynamic_zone(zone: rog_aura::AuraZone) -> bool {
    matches!(
        zone,
        rog_aura::AuraZone::None
            | rog_aura::AuraZone::Key1
            | rog_aura::AuraZone::Key2
            | rog_aura::AuraZone::Key3
            | rog_aura::AuraZone::Key4
            | rog_aura::AuraZone::BarLeft
            | rog_aura::AuraZone::BarRight
    )
}

/// `Ok(None)` when the node is inactive (`-EBUSY`). Other errors propagate.
fn map_dl_result<T>(result: Result<T, PlatformError>) -> Result<Option<T>, RogError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(err) => {
            let err = RogError::from(err);
            if is_dl_node_inactive(&err) {
                Ok(None)
            } else {
                Err(err)
            }
        }
    }
}

/// Kernel returns `-EBUSY` when writing a Dynamic Lighting node that is inactive
/// for the current `aura_mode` (`unified` vs `split`).
fn is_dl_node_inactive(err: &RogError) -> bool {
    let io = match err {
        RogError::Platform(PlatformError::IoPath(_, e)) => Some(e),
        RogError::Platform(PlatformError::Io(e)) => Some(e),
        RogError::Write(_, e) | RogError::Path(_, e) | RogError::Io(e) => Some(e),
        _ => None,
    };
    io.is_some_and(|e| e.kind() == std::io::ErrorKind::ResourceBusy || e.raw_os_error() == Some(16))
}

fn pad_hid_report(bytes: &[u8]) -> [u8; 64] {
    let mut padded = [0u8; 64];
    if let Some(slot) = padded.get_mut(..bytes.len()) {
        slot.copy_from_slice(bytes);
    }
    padded
}

fn filter_power_states<'a>(requested: &[&'a str], supported: &str) -> Vec<&'a str> {
    let supported: std::collections::HashSet<_> = supported.split_whitespace().collect();
    requested
        .iter()
        .copied()
        .filter(|state| supported.contains(state))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};

    use rog_aura::AuraZone;
    use rog_platform::error::PlatformError;

    use super::{
        direct_rgb_payload, filter_power_states, is_dl_node_inactive, supports_dynamic_zone,
    };
    use crate::error::RogError;

    #[test]
    fn direct_payload_requires_exact_size() {
        let mut packet = vec![0; 15];
        packet[1] = 0xbc;
        packet[7] = 2;
        packet[9..15].copy_from_slice(&[
            1, 2, 3, 4, 5, 6,
        ]);
        assert_eq!(
            direct_rgb_payload(&vec![packet.clone()], 2).unwrap().len(),
            6
        );
        assert!(direct_rgb_payload(&vec![packet], 3).is_err());
        assert!(direct_rgb_payload(&Vec::new(), 1).is_err());
    }

    #[test]
    fn power_states_are_intersected_with_capabilities() {
        assert_eq!(
            filter_power_states(
                &[
                    "boot", "awake", "shutdown"
                ],
                "awake sleep"
            ),
            ["awake"]
        );
    }

    #[test]
    fn dynamic_topology_allows_keyboard_and_lightbar_proxies() {
        assert!(supports_dynamic_zone(AuraZone::None));
        assert!(supports_dynamic_zone(AuraZone::Key1));
        assert!(supports_dynamic_zone(AuraZone::BarLeft));
        assert!(!supports_dynamic_zone(AuraZone::Logo));
    }

    #[test]
    fn inactive_dl_node_detects_ebusy() {
        let err = RogError::Platform(PlatformError::IoPath(
            "aura:keyboard/brightness".into(),
            Error::new(ErrorKind::ResourceBusy, "Device or resource busy"),
        ));
        assert!(is_dl_node_inactive(&err));
        let errno = RogError::Platform(PlatformError::IoPath(
            "aura:keyboard/direct_buffer".into(),
            Error::from_raw_os_error(16),
        ));
        assert!(is_dl_node_inactive(&errno));
    }
}
