use std::sync::{Arc, Mutex, OnceLock};

use log::{debug, warn};
use tokio::sync::mpsc::UnboundedSender;

use crate::{config::Config, helpers::zbus_proxies::AsusdInterface, state::Event};
use rog_dbus::zbus_platform::PlatformProxy;
use rog_platform::asus_armoury::FirmwareAttribute;
pub struct ActionHandler {
    pub config: Arc<Mutex<Config>>,
    pub asusd: Arc<OnceLock<AsusdInterface>>,
    pub event_tx: UnboundedSender<Event>,
}
impl ActionHandler {
    pub async fn handle(&mut self, event: &Event) {
        debug!("handling event: {:?}", event);
        match event {
            // Re-probe asusd (Retry button)
            Event::RetryAsusd => match AsusdInterface::build().await {
                Ok(int) if int.present() => {
                    let _ = self.asusd.set(int);
                    let _ = self.event_tx.send(Event::AsusdState(true));
                }
                Ok(_) => {
                    warn!("asusd reachable but no known interfaces found");
                    let _ = self.event_tx.send(Event::AsusdState(false));
                }
                Err(err) => {
                    warn!("asusd retry failed: {err}");
                    let _ = self.event_tx.send(Event::AsusdState(false));
                }
            },
            // asusd is down
            _ if self.asusd.get().is_none() => {
                warn!("asusd unavailable, ignoring event {event:?}");
            }
            // System
            Event::UserRequestedPowerProfile(ppd) => {
                if let Some(platform_proxy) = self.platform()
                    && let Err(err) = platform_proxy.set_platform_profile((*ppd).into()).await
                {
                    warn!("failed to set platform profile: {}", err);
                };
            }
            Event::UserRequestedAttr(attr, value) => {
                self.set_attribute(*attr, *value).await;
            }
            Event::UserEnabledPpt(b) => {
                if let Some(platform_proxy) = self.platform() {
                    match platform_proxy.set_enable_ppt_group(*b).await {
                        Ok(()) => {
                            let _ = self.event_tx.send(Event::UpdatedPptEnabled(*b));
                        }
                        Err(err) => warn!("failed to set ppt_group: {}", err),
                    };
                }
            }
            Event::UserRequestedBatteryLimit(_) | Event::UserToggledTray(_) => {
                warn!("Action not implemented: {:?}", event);
            }
            _ => {}
        }
    }

    fn platform(&self) -> Option<&PlatformProxy<'static>> {
        self.asusd.get().and_then(|i| i.platform.as_ref())
    }

    /// Write a single armory firmware attribute via D-Bus
    // Skips with a warning when the attribute is unsupported.
    async fn set_attribute(&self, attr: FirmwareAttribute, value: i32) {
        let proxy = self.asusd.get().and_then(|i| i.attribute(attr));
        match proxy {
            Some(p) => {
                if let Err(e) = p.set_current_value(value).await {
                    warn!(
                        "could not set {value} on attribute {}: {e}",
                        <&str>::from(attr)
                    );
                }
            }
            None => {
                warn!(
                    "attribute {} not supported by this device",
                    <&str>::from(attr)
                );
            }
        }
    }
}
