//! `update_and_notify` is responsible for both notifications *and* updating
//! stored statuses about the system state. This is done through either direct,
//! intoify, zbus notifications or similar methods.
//!
//! This module very much functions like a stand-alone app on its own thread.

use std::fmt::Display;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::{debug, error, info, warn};
use notify_rust::{Hint, Notification, Timeout};
use rog_platform::gpu_pci::GfxPower;
use rog_platform::power::AsusPower;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::config::Config;
use crate::error::Result;

const NOTIF_HEADER: &str = "ROG Control";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct EnabledNotifications {
    pub enabled: bool,
    pub receive_notify_gfx: bool,
    pub receive_notify_gfx_status: bool,
}

impl Default for EnabledNotifications {
    fn default() -> Self {
        Self {
            enabled: true,
            receive_notify_gfx: true,
            receive_notify_gfx_status: true,
        }
    }
}

/// Decide what to report for one poll tick of the dGPU status monitor, or
/// `None` to skip the tick.
///
/// Writing dgpu_disable=1 does not remove the device from the PCI bus, so the
/// attribute must win over the runtime status of a still-enumerated dGPU.
///
/// An unknown status from a present device is transitional (runtime PM cycles
/// through "suspending"/"resuming", which parse as unknown) and is skipped.
/// Unknown with no device on the bus is real information — reported, so that
/// listeners fall back instead of keeping a stale state.
fn dgpu_status_for_tick(
    dgpu_disabled: bool,
    runtime_status: Option<GfxPower>,
    mux_discreet: bool,
) -> Option<GfxPower> {
    if dgpu_disabled {
        return Some(GfxPower::AsusDisabled);
    }
    match runtime_status {
        Some(GfxPower::Unknown) => None,
        Some(status) => Some(status),
        // No dGPU on the bus: report the ASUS mux state instead
        None => Some(if mux_discreet {
            GfxPower::AsusMuxDiscreet
        } else {
            GfxPower::Unknown
        }),
    }
}

fn start_dgpu_status_mon(config: Arc<Mutex<Config>>, gpu_status_tx: watch::Sender<GfxPower>) {
    use rog_platform::gpu_pci::{asus_dgpu_disabled, asus_gpu_mux_discreet, Device};

    let find_dgpu = || {
        Device::find()
            .unwrap_or_default()
            .into_iter()
            .find(|d| d.is_dgpu())
    };

    let enabled_notifications_copy = config.clone();
    // Plain old thread is perfectly fine since most of this is potentially blocking
    std::thread::spawn(move || {
        let mut dgpu = find_dgpu();
        match &dgpu {
            Some(dev) => info!(
                "Found dGPU: {}, starting status notifications",
                dev.pci_id()
            ),
            None => warn!("Did not find a dGPU on this system, will keep watching for one"),
        }

        let mut last_status = GfxPower::Unknown;
        let mut ticks: u32 = 0;
        loop {
            std::thread::sleep(Duration::from_millis(1500));
            ticks = ticks.wrapping_add(1);

            let disabled = asus_dgpu_disabled().unwrap_or(false);
            if !disabled {
                // Drop the cached device if it vanished (e.g. unbound/removed
                // from the PCI bus)
                if dgpu.as_ref().is_some_and(|d| !d.dev_path().exists()) {
                    info!("dGPU device is gone, re-detecting");
                    dgpu = None;
                }
                // Re-detection is a full udev scan, so do it sparingly
                if dgpu.is_none() && ticks.is_multiple_of(4) {
                    dgpu = find_dgpu();
                }
            }

            let Some(status) = dgpu_status_for_tick(
                disabled,
                dgpu.as_ref()
                    .map(|dev| dev.get_runtime_status().unwrap_or(GfxPower::Unknown)),
                asus_gpu_mux_discreet().unwrap_or(false),
            ) else {
                continue;
            };

            if status != last_status {
                debug!("dGPU status changed: {:?}", status);
                gpu_status_tx.send_replace(status);
                let notify = enabled_notifications_copy.lock().is_ok_and(|config| {
                    config.notifications.enabled && config.notifications.receive_notify_gfx_status
                });
                if notify {
                    if let Err(e) = do_gpu_status_notif("dGPU status changed:", &status).show() {
                        warn!("Could not show dGPU status notification: {e}");
                    }
                }
            }
            last_status = status;
        }
    });
}

pub fn start_notifications(
    config: Arc<Mutex<Config>>,
    rt: &Runtime,
    gpu_status_tx: watch::Sender<GfxPower>,
) -> Result<Vec<JoinHandle<()>>> {
    // Setup the AC/BAT commands that will run on power status change
    let config_copy = config.clone();
    let blocking = rt.spawn_blocking(move || {
        let power = match AsusPower::new() {
            Ok(p) => p,
            Err(e) => {
                error!("AsusPower failed to initialize: {e}");
                return;
            }
        };

        let mut last_state = power.get_online().unwrap_or_default();
        loop {
            if let Ok(p) = power.get_online() {
                let mut ac = String::new();
                let mut bat = String::new();
                if let Ok(config) = config_copy.lock() {
                    ac.clone_from(&config.ac_command);
                    bat.clone_from(&config.bat_command);
                }

                if p == 0 && p != last_state {
                    let prog: Vec<&str> = bat.split_whitespace().collect();
                    if (!prog.is_empty()) && (!prog[0].is_empty()) {
                        let mut cmd = Command::new(prog[0]);

                        for arg in prog.iter().skip(1) {
                            cmd.arg(*arg);
                        }
                        cmd.spawn()
                            .map_err(|e| error!("Battery power command error: {e:?}"))
                            .ok();
                    }
                } else if p != last_state {
                    let prog: Vec<&str> = ac.split_whitespace().collect();
                    if (!prog.is_empty()) && (!prog[0].is_empty()) {
                        let mut cmd = Command::new(prog[0]);

                        for arg in prog.iter().skip(1) {
                            cmd.arg(*arg);
                        }
                        cmd.spawn()
                            .map_err(|e| error!("AC power command error: {e:?}"))
                            .ok();
                    }
                }
                last_state = p;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });

    info!("Attempting to start plain dgpu status monitor");
    start_dgpu_status_mon(config.clone(), gpu_status_tx);

    Ok(vec![blocking])
}

fn base_notification<T>(message: &str, data: &T) -> Notification
where
    T: Display,
{
    let mut notif = Notification::new();
    notif
        .appname(NOTIF_HEADER)
        .summary(&format!("{message} {data}"))
        .timeout(Timeout::Milliseconds(3000))
        .hint(Hint::Category("device".into()));
    notif
}

fn do_gpu_status_notif(message: &str, data: &GfxPower) -> Notification {
    let mut notif = base_notification(message, &<&str>::from(data).to_owned());
    let icon = match data {
        GfxPower::Suspended => "asus_notif_blue",
        GfxPower::AsusDisabled => "asus_notif_white",
        GfxPower::AsusMuxDiscreet | GfxPower::Active => "asus_notif_red",
        GfxPower::Unknown => "gpu-integrated",
    };
    notif.icon(icon);
    notif
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dgpu_disable_wins_over_an_enumerated_dgpu() {
        assert_eq!(
            dgpu_status_for_tick(true, Some(GfxPower::Active), false),
            Some(GfxPower::AsusDisabled)
        );
        assert_eq!(
            dgpu_status_for_tick(true, None, true),
            Some(GfxPower::AsusDisabled)
        );
    }

    #[test]
    fn runtime_status_reported_when_a_dgpu_is_present() {
        assert_eq!(
            dgpu_status_for_tick(false, Some(GfxPower::Suspended), false),
            Some(GfxPower::Suspended)
        );
        assert_eq!(
            dgpu_status_for_tick(false, Some(GfxPower::Active), false),
            Some(GfxPower::Active)
        );
    }

    #[test]
    fn mux_state_reported_when_no_dgpu_is_on_the_bus() {
        assert_eq!(
            dgpu_status_for_tick(false, None, true),
            Some(GfxPower::AsusMuxDiscreet)
        );
    }

    #[test]
    fn transitional_unknown_from_a_present_dgpu_is_skipped() {
        assert_eq!(
            dgpu_status_for_tick(false, Some(GfxPower::Unknown), false),
            None
        );
        assert_eq!(
            dgpu_status_for_tick(false, Some(GfxPower::Unknown), true),
            None
        );
    }

    #[test]
    fn persistent_unknown_with_no_dgpu_is_reported() {
        assert_eq!(
            dgpu_status_for_tick(false, None, false),
            Some(GfxPower::Unknown)
        );
    }
}
