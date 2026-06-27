use std::sync::Arc;
use std::time::Duration;

use config_traits::StdConfig;
use log::{info, warn};
use rog_platform::backlight::{Backlight, BacklightType};
use tokio::sync::Mutex;
use zbus::fdo::Error as FdoErr;
use zbus::object_server::SignalEmitter;
use zbus::{interface, Connection};

use crate::config::Config;
use crate::error::RogError;
use crate::ASUS_ZBUS_PATH;

#[derive(Clone)]
pub struct CtrlBacklight {
    backlights: Vec<Backlight>,
    config: Arc<Mutex<Config>>,
}

impl CtrlBacklight {
    pub fn new(config: Arc<Mutex<Config>>) -> Result<Self, RogError> {
        let mut backlights = Vec::new();

        if let Ok(primary) = Backlight::new(BacklightType::Primary) {
            info!("Found primary display backlight");
            backlights.push(primary);
        }

        if let Ok(screenpad) = Backlight::new(BacklightType::Screenpad) {
            info!("Found screenpad backlight");
            backlights.push(screenpad);
        }

        if backlights.is_empty() {
            return Err(RogError::MissingFunction("No backlights found".into()));
        }

        Ok(Self { backlights, config })
    }

    fn get_backlight(&self, device_type: &BacklightType) -> Option<&Backlight> {
        self.backlights
            .iter()
            .find(|b| b.device_type() == device_type)
    }

    async fn set_brightness_with_sync(
        &self,
        device_type: &BacklightType,
        level: i32,
    ) -> Result<(), FdoErr> {
        let sync = self
            .config
            .lock()
            .await
            .screenpad_sync_primary
            .unwrap_or_default();

        // If sync is enabled and we're setting screenpad brightness, set primary first
        if sync && *device_type == BacklightType::Screenpad {
            if let Some(primary) = self.get_backlight(&BacklightType::Primary) {
                if let Ok(primary_max) = primary.get_max_brightness() {
                    let primary_scaled = level * primary_max / 100;
                    let _ = primary.set_brightness(primary_scaled);
                }
            }
        }

        if let Some(backlight) = self.get_backlight(device_type) {
            let max = backlight.get_max_brightness().map_err(|e| {
                warn!("Failed to get max brightness: {}", e);
                FdoErr::Failed(format!("Failed to get max brightness: {}", e))
            })?;

            let gamma = self.config.lock().await.screenpad_gamma.unwrap_or(1.0);
            let scaled = if *device_type == BacklightType::Screenpad {
                // Apply non-linear scaling with the configurable gamma value only for Screenpad
                let normalized_level = level as f32 / 100.0;
                let gamma_corrected = normalized_level.powf(gamma);
                (gamma_corrected * max as f32) as i32
            } else {
                // Linear scaling for other devices
                level * max / 100
            };

            backlight.set_brightness(scaled).map_err(|e| {
                warn!("Failed to set brightness: {}", e);
                FdoErr::Failed(format!("Failed to set brightness: {}", e))
            })?;

            // If sync is enabled and we're setting primary brightness, set screenpad
            // afterward
            if sync && *device_type == BacklightType::Primary {
                for other in self
                    .backlights
                    .iter()
                    .filter(|b| b.device_type() != device_type)
                {
                    if let Ok(other_max) = other.get_max_brightness() {
                        let other_scaled = if other.device_type() == &BacklightType::Screenpad {
                            // Apply gamma only to Screenpad
                            let normalized_level = level as f32 / 100.0;
                            let gamma_corrected = normalized_level.powf(gamma);
                            (gamma_corrected * other_max as f32) as i32
                        } else {
                            // Linear scaling for other devices
                            level * other_max / 100
                        };
                        let _ = other.set_brightness(other_scaled);
                    }
                }
            }

            Ok(())
        } else {
            Err(FdoErr::NotSupported(format!(
                "Backlight {:?} not found",
                device_type
            )))
        }
    }

    async fn get_brightness_percent(&self, device_type: &BacklightType) -> Result<i32, FdoErr> {
        if let Some(backlight) = self.get_backlight(device_type) {
            let brightness = backlight.get_brightness().map_err(|e| {
                warn!("Failed to get brightness: {}", e);
                FdoErr::Failed(format!("Failed to get brightness: {}", e))
            })?;

            let max = backlight.get_max_brightness().map_err(|e| {
                warn!("Failed to get max brightness: {}", e);
                FdoErr::Failed(format!("Failed to get max brightness: {}", e))
            })?;

            if *device_type == BacklightType::Screenpad {
                let gamma = self.config.lock().await.screenpad_gamma.unwrap_or(1.0);
                let normalized = brightness as f32 / max as f32;
                let corrected = normalized.powf(1.0 / gamma);
                Ok((corrected * 100.0).round() as i32)
            } else {
                Ok(brightness * 100 / max)
            }
        } else {
            Err(FdoErr::NotSupported(format!(
                "Backlight {:?} not found",
                device_type
            )))
        }
    }

    pub async fn start_watch_primary(&self) -> Result<(), RogError> {
        if self.get_backlight(&BacklightType::Screenpad).is_none() {
            return Ok(());
        }

        if let Some(sync) = self.config.lock().await.screenpad_sync_primary {
            if !sync {
                return Ok(());
            }
        }

        if let Some(backlight) = self.get_backlight(&BacklightType::Primary) {
            let watch = backlight.monitor_brightness()?;

            let backlights = self.clone();
            tokio::spawn(async move {
                let mut last_level = 0;
                let mut buffer = [0; 32];
                use futures_util::StreamExt;
                if let Ok(mut stream) = watch.into_event_stream(&mut buffer) {
                    loop {
                        let _ = stream.next().await;

                        let sync = backlights.config.lock().await.screenpad_sync_primary;
                        if let Some(sync) = sync {
                            if !sync {
                                continue;
                            }
                        } else if backlights
                            .config
                            .lock()
                            .await
                            .screenpad_sync_primary
                            .is_none()
                        {
                            continue;
                        }

                        let level = backlights
                            .get_brightness_percent(&BacklightType::Primary)
                            .await
                            .unwrap_or(60);
                        if last_level != level {
                            last_level = level;
                            backlights
                                .set_brightness_with_sync(&BacklightType::Screenpad, level)
                                .await
                                .ok();
                        }

                        // other processes cause "MODIFY" event and make this spin 100%, so sleep
                        tokio::time::sleep(Duration::from_millis(300)).await;
                    }
                    // watch
                    //     .into_event_stream(&mut buffer)
                    //     .unwrap()
                    //     .for_each(|_| async {})
                    //     .await;
                }
            });
        }

        Ok(())
    }
}

#[interface(name = "xyz.ljones.Backlight")]
impl CtrlBacklight {
    #[zbus(property)]
    async fn screenpad_sync_with_primary(&self) -> bool {
        self.config
            .lock()
            .await
            .screenpad_sync_primary
            .unwrap_or_default()
    }

    #[zbus(property)]
    async fn set_screenpad_sync_with_primary(&self, sync: bool) -> Result<(), zbus::Error> {
        self.config.lock().await.screenpad_sync_primary = Some(sync);
        self.config.lock().await.write();
        Ok(())
    }

    #[zbus(property)]
    async fn screenpad_gamma(&self) -> String {
        (self.config.lock().await.screenpad_gamma.unwrap_or(1.0)).to_string()
    }

    #[zbus(property)]
    async fn set_screenpad_gamma(&self, value: &str) -> Result<(), zbus::Error> {
        let gamma: f32 = value
            .parse()
            .map_err(|_| FdoErr::Failed("Invalid gamma value, must be a valid number".into()))?;

        if gamma < 0.1 {
            return Err(FdoErr::Failed("Gamma value must be greater than 0".into()).into());
        }
        if gamma > 2.0 {
            return Err(FdoErr::Failed("Gamma value must be 2.0 or less".into()).into());
        }
        self.config.lock().await.screenpad_gamma = Some(gamma);
        self.config.lock().await.write();
        Ok(())
    }

    #[zbus(property)]
    async fn primary_brightness(&self) -> Result<i32, FdoErr> {
        self.get_brightness_percent(&BacklightType::Primary).await
    }

    #[zbus(property)]
    async fn set_primary_brightness(
        &self,
        #[zbus(signal_context)] ctxt: SignalEmitter<'_>,
        level: i32,
    ) -> Result<(), zbus::Error> {
        if level > 100 {
            return Err(FdoErr::Failed("Brightness level must be 0-100".into()).into());
        }

        self.set_brightness_with_sync(&BacklightType::Primary, level)
            .await?;
        self.primary_brightness_changed(&ctxt).await?;

        Ok(())
    }

    #[zbus(property)]
    async fn screenpad_brightness(&self) -> Result<i32, FdoErr> {
        self.get_brightness_percent(&BacklightType::Screenpad).await
    }

    #[zbus(property)]
    async fn set_screenpad_brightness(
        &self,
        // #[zbus(signal_context)] ctxt: SignalEmitter<'_>,
        level: i32,
    ) -> Result<(), zbus::Error> {
        if level > 100 {
            return Err(FdoErr::Failed("Brightness level must be 0-100".into()).into());
        }

        self.set_brightness_with_sync(&BacklightType::Screenpad, level)
            .await?;
        // self.screenpad_brightness_changed(&ctxt).await?;

        Ok(())
    }

    #[zbus(property)]
    async fn screenpad_power(&self) -> Result<bool, FdoErr> {
        if let Some(backlight) = self.get_backlight(&BacklightType::Screenpad) {
            let power = backlight.get_bl_power().map_err(|e| {
                warn!("Failed to get backlight power: {}", e);
                FdoErr::Failed(format!("Failed to get backlight power: {}", e))
            })?;
            Ok(power == 0)
        } else {
            Err(FdoErr::NotSupported("Screenpad backlight not found".into()))
        }
    }

    #[zbus(property)]
    async fn set_screenpad_power(
        &self,
        #[zbus(signal_context)] ctxt: SignalEmitter<'_>,
        power: bool,
    ) -> Result<(), zbus::Error> {
        if let Some(backlight) = self.get_backlight(&BacklightType::Screenpad) {
            backlight
                .set_bl_power(if power { 0 } else { 1 })
                .map_err(|e| {
                    warn!("Failed to set backlight power: {}", e);
                    FdoErr::Failed(format!("Failed to set backlight power: {}", e))
                })?;
            self.screenpad_power_changed(&ctxt).await?;
            Ok(())
        } else {
            Err(FdoErr::NotSupported("Screenpad backlight not found".into()).into())
        }
    }
}

impl crate::ZbusRun for CtrlBacklight {
    async fn add_to_server(self, server: &mut Connection) {
        Self::add_to_server_helper(self, ASUS_ZBUS_PATH, server).await;
    }
}

impl crate::Reloadable for CtrlBacklight {
    async fn reload(&mut self) -> Result<(), RogError> {
        info!("Reloading backlight settings");
        Ok(())
    }
}
