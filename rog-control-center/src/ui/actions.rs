use std::sync::{Arc, Mutex, OnceLock};

use log::{debug, warn};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    config::Config,
    helpers::zbus_proxies::AsusdInterface,
    state::{Action, Event},
};
use rog_platform::asus_armoury::FirmwareAttribute;
pub struct ActionHandler {
    pub config: Arc<Mutex<Config>>,
    pub asusd: Arc<OnceLock<AsusdInterface>>,
    pub event_tx: UnboundedSender<Event>,
}
impl ActionHandler {
    pub async fn handle_action(&mut self, action: Action) {
        debug!("handling action: {:?}", action);
        match action {
            // Re-probe asusd (Retry button)
            Action::RetryAsusd => match AsusdInterface::build().await {
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
                warn!("asusd unavailable, ignoring action {action:?}");
            }
            // System
            Action::SetPlatformProfile(ppd) => {
                if let Some(asusd_proxy) = self.asusd.get()
                    && let Some(platform_proxy) = &asusd_proxy.platform
                    && let Err(err) = platform_proxy.set_platform_profile(ppd.into()).await
                {
                    warn!("failed to set platform profile: {}", err);
                };
            }
            Action::SetAttr(attr, value) => {
                self.set_attribute(attr, value).await;
            }
            Action::SetPPTEnabled(b) => {
                if let Some(asusd_proxy) = self.asusd.get()
                    && let Some(platform_proxy) = &asusd_proxy.platform
                {
                    match platform_proxy.set_enable_ppt_group(b).await {
                        Ok(()) => {
                            let _ = self.event_tx.send(Event::UpdatedPptEnabled(b));
                        }
                        Err(err) => warn!("failed to set ppt_group: {}", err),
                    };
                }
            }
            _ => {
                warn!("Action not implemented: {:?}", action);
            }
        }
    }

    /// Write a single armory firmware attribute via D-Bus (panel OD, boot
    /// sound, PPT, …). Skips with a warning when the attribute is unsupported.
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
