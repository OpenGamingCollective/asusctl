//! `aura_lamparray` — Microsoft HID LampArray (Usage Page 0x59) backend.
//!
//! This module is a sibling of `aura_slash`, `aura_anime`, `aura_scsi` and
//! deliberately kept separate from `aura_laptop`. The latter drives the ASUS
//! proprietary Aura HID protocol used by USB/asus-wmi keyboards; LampArray
//! is a standards-based feature-report protocol exposed by I2C-HID
//! controllers on newer ASUS TUF laptops (e.g. FA608WV / ITE5570). The two
//! share nothing at the wire level — collapsing them into one struct with
//! an `is_lamparray` flag turned out to be a source of coupling bugs
//! (config-lock deadlocks, double-push races on brightness change).
//!
//! Public surface:
//!   * [`LampArray`] — owning struct, holds the `HidRaw` node, config, and
//!     the currently-running dynamic-effect task handle.
//!   * [`LampArrayZbus`] — zbus interface at `/xyz/ljones/aura/lamparray_<pid>`.
//!
//! The passive chip does not do animations on-die: the host must push
//! `LampRangeUpdate` (report 0x45) frames at ~30 FPS from a tokio task.
//! That task is created via [`LampArray::spawn_effect`] and cancelled by
//! [`LampArray::stop_effect_task`] on every new effect write, so two
//! loops can never race the hid lock.
use std::sync::Arc;

use log::info;
use rog_aura::{AuraEffect, AuraModeNum, LedBrightness};
use rog_platform::hid_raw::HidRaw;
use tokio::runtime::Handle;
use tokio::sync::{Mutex, MutexGuard};
use tokio::task::JoinHandle;

use crate::aura_laptop::config::AuraConfig;
use crate::error::RogError;

pub mod effects;
pub mod trait_impls;

use effects::spawn_effect_task;

#[derive(Debug, Clone)]
pub struct LampArray {
    /// Underlying hidraw node. Feature reports are pushed through
    /// `HidRaw::set_feature_report` / `get_feature_report`.
    pub hid: Arc<Mutex<HidRaw>>,
    /// Shared with the zbus interface. Owns brightness, current mode and the
    /// stored builtin effects (colour1/colour2/speed per mode).
    pub config: Arc<Mutex<AuraConfig>>,
    /// Handle for the currently-running LampArray dynamic-effect task.
    /// Aborted on every effect write so we never accumulate loops.
    pub effect_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Tokio runtime handle captured at construction. The dynamic-effect
    /// task must be spawned via `Handle::spawn` because methods on this
    /// struct are invoked from the zbus executor thread, which is not a
    /// Tokio runtime thread — bare `tokio::spawn()` would panic there.
    pub runtime_handle: Handle,
}

impl LampArray {
    pub fn new(hid: Arc<Mutex<HidRaw>>, config: Arc<Mutex<AuraConfig>>) -> Self {
        info!("LampArray constructed with runtime_handle captured");
        Self {
            hid,
            config,
            effect_task: Arc::new(Mutex::new(None)),
            runtime_handle: Handle::current(),
        }
    }

    pub async fn do_initialization(&self) -> Result<(), RogError> {
        let hid = self.hid.lock().await;
        if let Err(e) = hid.set_use_leds_uapi(false) {
            log::warn!("Failed to disable kernel LampArray LED UAPI: {e:?}");
        }
        hid.set_feature_report(&[
            0x46, 0x01,
        ])?;
        hid.set_feature_report(&[
            0x46, 0x00,
        ])?;
        Ok(())
    }

    pub async fn lock_config(&self) -> MutexGuard<'_, AuraConfig> {
        self.config.lock().await
    }

    /// Write the currently active mode from config to the device. Mirrors
    /// `Aura::write_current_config_mode` but for the LampArray path only.
    /// Caller owns the config lock and passes it in — same reason as
    /// `write_effect_locked`.
    pub async fn write_current_config_mode(&self, config: &mut AuraConfig) -> Result<(), RogError> {
        let effect = config
            .builtins
            .get(&config.current_mode)
            .ok_or_else(|| RogError::MissingFunction("Unsupported LampArray effect".into()))?;
        self.write_effect_locked(config, effect).await
    }

    /// LampArray helper — write the current static colour to the whole
    /// keyboard at the requested intensity (0-255). The protocol is the
    /// Microsoft HID LampArray usage page:
    ///   * report 0x46 — "autonomous mode" toggle (we disable so the OS owns)
    ///   * report 0x41 — LampArrayAttributes (read to discover LampCount)
    ///   * report 0x44 — LampMultiUpdate (eight lamp IDs, eight RGBA tuples)
    pub async fn push_rgb_i(&self, r: u8, g: u8, b: u8, intensity: u8) -> Result<(), RogError> {
        let hid = self.hid.lock().await;
        // Disable autonomous so we own the lamp array
        hid.set_feature_report(&[
            0x46, 0x00,
        ])?;
        // Read LampArrayAttributes to discover the lamp count
        let mut attr = vec![0u8; 23];
        attr[0] = 0x41;
        hid.get_feature_report(&mut attr)?;
        let lamp_count = u16::from_le_bytes([
            attr[1], attr[2],
        ]);
        if lamp_count == 0 {
            return Err(RogError::MissingFunction(
                "LampArray reports zero lamps".to_string(),
            ));
        }
        if lamp_count != 1 {
            return Err(RogError::MissingFunction(
                "This backend requires a single-zone LampArray".into(),
            ));
        }
        let payload = single_zone_report(r, g, b, intensity);
        hid.set_feature_report(&payload)?;
        info!(
            "LampArray ready: LampCount={lamp_count} rgb=({r:02x},{g:02x},{b:02x}) i={intensity}"
        );
        Ok(())
    }

    /// Variant for callers that already hold the config lock. Pass the
    /// already-locked config in to avoid the deadlock that re-locking would
    /// cause.
    pub async fn write_effect_locked(
        &self,
        config: &AuraConfig,
        mode: &AuraEffect,
    ) -> Result<(), RogError> {
        // Same rule as `write_effect`: kill any running animation before
        // dispatching so we don't accumulate tasks across reloads.
        self.stop_effect_task().await;
        let intensity = Self::brightness_to_intensity(config.brightness);
        match mode.mode {
            AuraModeNum::Static => {
                let r = mode.colour1.r;
                let g = mode.colour1.g;
                let b = mode.colour1.b;
                info!("lamparray_write_effect_locked: Static, single push");
                info!("lamparray_write_effect_locked: about to push rgb (caller owns config lock)");
                self.push_rgb_i(r, g, b, intensity).await
            }
            _ => {
                info!(
                    "lamparray_write_effect_locked: dynamic mode {:?}, spawning effect task",
                    mode.mode
                );
                self.spawn_effect(mode.clone(), intensity).await
            }
        }
    }

    /// Brightness -> intensity mapping for LampArray. Reuses the colour from
    /// the currently active builtin effect in config so the keyboard keeps
    /// the same hue when the user only changes brightness.
    ///
    /// Uses `try_lock` to avoid the init-time deadlock when a caller higher
    /// in the stack already owns the config lock (see comment on
    /// [`LampArray::write_effect`]).
    pub async fn set_brightness(&self, value: u8) -> Result<(), RogError> {
        let level = match value {
            0 => LedBrightness::Off,
            1 => LedBrightness::Low,
            2 => LedBrightness::Med,
            _ => LedBrightness::High,
        };
        let mut config = self.config.lock().await;
        config.brightness = level;
        self.write_current_config_mode(&mut config).await
    }

    /// Aura power states on LampArray - we collapse the per-zone flags into a
    /// simple on/off: any zone enabled -> full intensity with the saved RGB,
    /// all disabled -> intensity 0.
    pub async fn set_aura_power(&self, config: &AuraConfig) -> Result<(), RogError> {
        let awake = config.enabled.states.iter().any(|state| state.awake);
        if awake && let Some(effect) = config.builtins.get(&config.current_mode) {
            return self.write_effect_locked(config, effect).await;
        }
        self.stop_effect_task().await;
        self.push_rgb_i(0, 0, 0, 0).await
    }

    /// Abort the current LampArray effect task, if any. Safe to call even
    /// when no task is running.
    pub async fn stop_effect_task(&self) {
        let mut slot = self.effect_task.lock().await;
        if let Some(handle) = slot.take() {
            handle.abort();
            info!("lamparray_effect_task: cancelled");
        }
    }

    /// Spawn a tokio task that drives one of the dynamic LampArray effects
    /// (Breathe / RainbowCycle / RainbowWave / Pulse). Delegates the frame
    /// generation to `effects::spawn_effect_task`.
    async fn spawn_effect(&self, mode: AuraEffect, intensity: u8) -> Result<(), RogError> {
        let handle =
            spawn_effect_task(self.hid.clone(), &self.runtime_handle, mode, intensity).await?;
        let mut slot = self.effect_task.lock().await;
        *slot = Some(handle);
        Ok(())
    }

    fn brightness_to_intensity(b: LedBrightness) -> u8 {
        match b {
            LedBrightness::Off => 0,
            LedBrightness::Low => 64,
            LedBrightness::Med => 128,
            LedBrightness::High => 255,
        }
    }
}

/// G-Helper's LampMultiUpdate layout: count, complete flag, eight u16 lamp
/// IDs, then eight RGBA tuples. The FA608WV exposes one lamp (ID zero).
pub(crate) fn single_zone_report(r: u8, g: u8, b: u8, intensity: u8) -> [u8; 51] {
    let mut report = [0u8; 51];
    report[0] = 0x44;
    report[1] = 1;
    report[2] = 1;
    report[19..23].copy_from_slice(&[
        r, g, b, intensity,
    ]);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lamparray_multi_update_matches_hardware_confirmed_white_packet() {
        // This exact 51-byte packet illuminated the FA608WV after the padded
        // autonomous-mode sequence, while short reports produced no light.
        let expected = [
            0x44, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(single_zone_report(255, 255, 255, 255), expected);
    }

    #[test]
    fn lamparray_brightness_preserves_rgb_and_supports_off() {
        assert_eq!(
            &single_zone_report(255, 0, 0, 64)[19..23],
            &[
                255, 0, 0, 64
            ]
        );
        assert_eq!(
            &single_zone_report(0, 0, 255, 0)[19..23],
            &[
                0, 0, 255, 0
            ]
        );
    }
}
