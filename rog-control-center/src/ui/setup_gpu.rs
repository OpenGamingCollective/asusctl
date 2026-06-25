use std::sync::Arc;

use log::error;
use rog_dbus::asus_armoury::AsusArmouryProxy;
use rog_dbus::zbus_platform::PlatformProxy;
use rog_dbus::zbus_xgm_led::XgmLedProxy;
use rog_platform::asus_armoury::FirmwareAttribute;
use slint::{ComponentHandle, SharedString, Weak};

use super::show_toast;
use crate::zbus_proxies::find_iface_async;
use crate::{GPUPageData, MainWindow};

/// A selectable GPU mode, independent of which sysfs attributes back it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GpuMode {
    Integrated, // dGPU powered off
    Hybrid,     // iGPU + dGPU (Optimus)
    Ultimate,   // dGPU drives the displays directly (MUX)
}

impl GpuMode {
    fn label(self) -> &'static str {
        match self {
            GpuMode::Integrated => "Integrated",
            GpuMode::Hybrid => "Hybrid",
            GpuMode::Ultimate => "Ultimate",
        }
    }
}

/// GPU switching capabilities exposed by asusd, plus the proxies to drive them.
///
/// `gpu_mux_mode` and `dgpu_disable` are independent firmware attributes: a
/// laptop may expose either, both, or neither. The available modes and whether
/// the control is shown at all are derived from which proxies are present.
struct GpuCaps {
    dgpu: Option<AsusArmouryProxy<'static>>,
    mux: Option<AsusArmouryProxy<'static>>,
}

impl GpuCaps {
    /// Collect the dgpu_disable / gpu_mux_mode proxies from asusd.
    async fn discover() -> Result<Self, Box<dyn std::error::Error>> {
        let attrs = find_iface_async::<AsusArmouryProxy>("xyz.ljones.AsusArmoury").await?;
        let mut caps = GpuCaps {
            dgpu: None,
            mux: None,
        };
        for attr in attrs {
            match attr.name().await {
                Ok(FirmwareAttribute::DgpuDisable) => caps.dgpu = Some(attr),
                Ok(FirmwareAttribute::GpuMuxMode) => caps.mux = Some(attr),
                Ok(_) => {}
                Err(e) => error!("setup_gpu_page: failed to read attribute name: {e:?}"),
            }
        }
        Ok(caps)
    }

    /// Whether any GPU switching is possible on this hardware.
    fn switchable(&self) -> bool {
        self.dgpu.is_some() || self.mux.is_some()
    }

    /// Modes this hardware supports, in display order.
    fn modes(&self) -> Vec<GpuMode> {
        let mut modes = Vec::new();
        if self.dgpu.is_some() {
            modes.push(GpuMode::Integrated);
        }
        if self.switchable() {
            modes.push(GpuMode::Hybrid);
        }
        if self.mux.is_some() {
            modes.push(GpuMode::Ultimate);
        }
        modes
    }

    /// Read the current hardware state and map it to a mode.
    async fn current_mode(&self) -> GpuMode {
        // Absent attributes report their inactive default so they never win below;
        // a read error is logged and falls back to the same default.
        let mux = match &self.mux {
            Some(a) => a.current_value().await.unwrap_or_else(|e| {
                error!("setup_gpu: failed to read gpu_mux_mode: {e:?}");
                1
            }),
            None => 1,
        };
        let dgpu = match &self.dgpu {
            Some(a) => a.current_value().await.unwrap_or_else(|e| {
                error!("setup_gpu: failed to read dgpu_disable: {e:?}");
                0
            }),
            None => 0,
        };
        if mux == 0 {
            GpuMode::Ultimate
        } else if dgpu == 1 {
            GpuMode::Integrated
        } else {
            GpuMode::Hybrid
        }
    }

    /// Apply a target mode, writing only the attributes that exist.
    async fn apply(&self, mode: GpuMode) -> zbus::Result<()> {
        let (dgpu_val, mux_val) = match mode {
            GpuMode::Integrated => (1, 1),
            GpuMode::Hybrid => (0, 1),
            GpuMode::Ultimate => (0, 0),
        };
        if let Some(attr) = &self.dgpu {
            attr.set_current_value(dgpu_val).await?;
        }
        if let Some(attr) = &self.mux {
            attr.set_current_value(mux_val).await?;
        }
        Ok(())
    }
}

/// Index of `mode` within `modes`, defaulting to 0 if not found.
fn index_of(modes: &[GpuMode], mode: GpuMode) -> i32 {
    modes.iter().position(|m| *m == mode).unwrap_or(0) as i32
}

fn set_dropdown_enabled(handle: &Weak<MainWindow>, enabled: bool) {
    handle
        .upgrade_in_event_loop(move |h| {
            h.global::<GPUPageData>().set_gpu_dropdown_enabled(enabled);
        })
        .unwrap_or_else(|e| error!("setup_gpu: failed to set dropdown state: {e:?}"));
}

/// Disable the dropdown, apply `mode`, toast the result, then refresh + re-enable.
fn set_gpu_mode(caps: Arc<GpuCaps>, handle: Weak<MainWindow>, mode: GpuMode) {
    // Called from the slint callback on the event-loop thread, so disable the
    // dropdown synchronously here — a second selection can't slip in before the
    // write is queued.
    if let Some(h) = handle.upgrade() {
        h.global::<GPUPageData>().set_gpu_dropdown_enabled(false);
    }

    tokio::spawn(async move {
        let result = caps.apply(mode).await;
        show_toast(
            SharedString::from("GPU mode change scheduled — reboot required for changes to apply."),
            SharedString::from("Failed to set GPU mode"),
            handle.clone(),
            result,
        );

        // Reflect the (possibly unchanged) hardware state back into the dropdown.
        let new_index = index_of(&caps.modes(), caps.current_mode().await);
        handle
            .upgrade_in_event_loop(move |h| {
                h.global::<GPUPageData>().set_gpu_mode_index(new_index);
            })
            .unwrap_or_else(|e| error!("setup_gpu: failed to refresh mode: {e:?}"));

        set_dropdown_enabled(&handle, true);
    });
}

fn set_apu_mem(proxy: AsusArmouryProxy<'static>, handle: Weak<MainWindow>, value: i32) {
    let p = proxy.clone();
    let w = handle.clone();
    tokio::spawn(async move {
        let result = p.set_current_value(value).await;
        show_toast(
            SharedString::from(
                "Reserved GPU memory updated — reboot required for changes to apply.",
            ),
            SharedString::from("Failed to set reserved GPU memory"),
            w.clone(),
            result,
        );

        // Refresh the dropdown to show the (possibly unchanged) hardware state.
        let new_current = p.current_value().await.unwrap_or(value);
        let choices = p.possible_values().await.unwrap_or_default();
        let new_index = choices.iter().position(|v| *v == new_current).unwrap_or(0) as i32;
        w.upgrade_in_event_loop(move |h| {
            h.global::<GPUPageData>().set_apu_mem_index(new_index);
        })
        .unwrap_or_else(|e| error!("setup_gpu: failed to refresh apu_mem index: {e:?}"));
    });
}

fn apu_mem_val_to_label(value: i32) -> SharedString {
    if value == 0 {
        SharedString::from("AUTO")
    } else {
        SharedString::from(format!("{}G", value))
    }
}

// Populate GPU page choices and wire the `cb_set_gpu_mode` callback
pub fn setup_gpu_page(ui: &MainWindow) {
    let handle = ui.as_weak();

    tokio::spawn(async move {
        let caps = match GpuCaps::discover().await {
            Ok(caps) => Arc::new(caps),
            Err(e) => {
                error!("setup_gpu_page: failed to get AsusArmoury proxies: {e:?}");
                return;
            }
        };

        let modes = caps.modes();
        let switchable = caps.switchable();
        let current_index = index_of(&modes, caps.current_mode().await);
        let choices: Vec<SharedString> = modes
            .iter()
            .map(|m| SharedString::from(m.label()))
            .collect();

        let caps_cb = caps.clone();
        let handle_cb = handle.clone();
        if let Err(e) = handle.upgrade_in_event_loop(move |handle| {
            let global = handle.global::<GPUPageData>();
            global.set_gpu_modes(choices.as_slice().into());
            global.set_gpu_switchable(switchable);
            global.set_gpu_dropdown_enabled(switchable);
            global.set_gpu_mode_index(current_index);

            global.on_cb_set_gpu_mode(move |index| {
                let Some(mode) = modes.get(index as usize).copied() else {
                    return;
                };
                set_gpu_mode(caps_cb.clone(), handle_cb.clone(), mode);
            });
        }) {
            error!("setup_gpu_page: upgrade_in_event_loop: {e:?}");
        }

        // --- APU mem ---
        let apu_mem_proxy: Option<AsusArmouryProxy<'static>> = async {
            let Ok(attrs) = find_iface_async::<AsusArmouryProxy>("xyz.ljones.AsusArmoury").await
            else {
                error!("setup_gpu: failed to find AsusArmoury proxies for apu_mem");
                return None;
            };
            for attr in attrs {
                match attr.name().await {
                    Ok(FirmwareAttribute::ApuMem) => return Some(attr),
                    Ok(_) => {}
                    Err(e) => error!("setup_gpu: failed to read attribute name: {e:?}"),
                }
            }
            None
        }
        .await;

        if let Some(proxy) = apu_mem_proxy {
            let possible = proxy.possible_values().await.unwrap_or_default();
            let current = proxy.current_value().await.unwrap_or(0);
            let apu_choices: Vec<SharedString> =
                possible.iter().map(|v| apu_mem_val_to_label(*v)).collect();
            let apu_index = possible.iter().position(|v| *v == current).unwrap_or(0) as i32;

            let proxy_cb = proxy.clone();
            let handle_cb = handle.clone();
            if let Err(e) = handle.upgrade_in_event_loop(move |h| {
                let global = h.global::<GPUPageData>();
                global.set_apu_mem_present(true);
                global.set_apu_mem_choices(apu_choices.as_slice().into());
                global.set_apu_mem_index(apu_index);
                let weak_handle = h.as_weak();
                global.on_cb_set_apu_mem(move |index| {
                    let Some(value) = possible.get(index as usize).copied() else {
                        return;
                    };
                    // Disable the dropdown while applying
                    weak_handle
                        .upgrade_in_event_loop(move |h| {
                            h.global::<GPUPageData>().set_apu_mem_index(index);
                        })
                        .unwrap_or_else(|e| {
                            error!("setup_gpu: failed to set apu_mem index: {e:?}")
                        });
                    set_apu_mem(proxy_cb.clone(), handle_cb.clone(), value);
                });
            }) {
                error!("setup_gpu: failed to wire apu_mem callback: {e:?}");
            }
        }

        // --- XG Mobile LED ---
        let xgm_results: Option<(XgmLedProxy<'static>, bool)> = async {
            let Ok(mut proxies) = find_iface_async::<XgmLedProxy>("xyz.ljones.XgmLed").await else {
                error!("setup_gpu: no XG Mobile LED interface");
                return None;
            };
            let xgm_proxy = proxies.pop()?;
            let enabled = xgm_proxy.xgm_led_enabled().await.unwrap_or(false);
            Some((xgm_proxy, enabled))
        }
        .await;
        if let Some((xgm_proxy, enabled)) = xgm_results {
            let handle_xgm = handle.clone();
            if let Err(e) = handle.upgrade_in_event_loop(move |h| {
                h.global::<GPUPageData>().set_has_xgm_led(true);
                h.global::<GPUPageData>().set_xgm_led_enabled(enabled);
            }) {
                error!("setup_gpu: failed to set XGM LED initial state: {e:?}");
            }

            // Wire callback
            let proxy_cb = xgm_proxy.clone();
            let handle_cb = handle_xgm.clone();
            if let Err(e) = handle_xgm.upgrade_in_event_loop(move |h| {
                h.global::<GPUPageData>()
                    .on_cb_set_xgm_led_enabled(move |checked| {
                        let p = proxy_cb.clone();
                        let w = handle_cb.clone();
                        tokio::spawn(async move {
                            let res = p.set_xgm_led_enabled(checked).await;
                            show_toast(
                                SharedString::from("XG Mobile LED updated"),
                                SharedString::from("Failed to set XG Mobile LED"),
                                w,
                                res,
                            );
                        });
                    });
            }) {
                error!("setup_gpu: failed to wire XGM LED callback: {e:?}");
            }

            // Listen for property changes
            let proxy_stream = xgm_proxy.clone();
            let handle_stream = handle.clone();
            tokio::spawn(async move {
                use futures_util::StreamExt;
                let mut stream = proxy_stream.receive_xgm_led_enabled_changed().await;
                while let Some(e) = stream.next().await {
                    if let Ok(enabled) = e.get().await {
                        handle_stream
                            .upgrade_in_event_loop(move |h| {
                                h.global::<GPUPageData>().set_xgm_led_enabled(enabled);
                            })
                            .ok();
                    }
                }
            });
        }

        // --- Disable nvidia-powerd on battery ---
        let platform_proxy = match zbus::Connection::system().await {
            Ok(conn) => match PlatformProxy::builder(&conn).build().await {
                Ok(p) => p,
                Err(e) => {
                    error!("setup_gpu: failed to create PlatformProxy: {e:?}");
                    return;
                }
            },
            Err(e) => {
                error!("setup_gpu: failed to connect to system bus: {e:?}");
                return;
            }
        };

        // Read initial value
        let initial = platform_proxy
            .disable_nvidia_powerd_on_battery()
            .await
            .unwrap_or(true);
        handle
            .upgrade_in_event_loop(move |h| {
                h.global::<GPUPageData>()
                    .set_disable_nvidia_powerd_on_battery(initial);
            })
            .ok();

        // Wire callback
        let proxy_cb = platform_proxy.clone();
        let handle_cb = handle.clone();
        handle
            .upgrade_in_event_loop(move |h| {
                h.global::<GPUPageData>()
                    .on_cb_disable_nvidia_powerd_on_battery(move |value| {
                        let p = proxy_cb.clone();
                        let w = handle_cb.clone();
                        tokio::spawn(async move {
                            let res = p.set_disable_nvidia_powerd_on_battery(value).await;
                            show_toast(
                                SharedString::from("Updated nvidia-powerd on-battery setting"),
                                SharedString::from(
                                    "Failed to update nvidia-powerd on-battery setting",
                                ),
                                w,
                                res,
                            );
                        });
                    });
            })
            .ok();

        // Listen for external changes
        let proxy_stream = platform_proxy.clone();
        let handle_stream = handle.clone();
        tokio::spawn(async move {
            use futures_util::StreamExt;
            let mut stream = proxy_stream
                .receive_disable_nvidia_powerd_on_battery_changed()
                .await;
            while let Some(e) = stream.next().await {
                if let Ok(value) = e.get().await {
                    handle_stream
                        .upgrade_in_event_loop(move |h| {
                            h.global::<GPUPageData>()
                                .set_disable_nvidia_powerd_on_battery(value);
                        })
                        .ok();
                }
            }
        });
    });
}
