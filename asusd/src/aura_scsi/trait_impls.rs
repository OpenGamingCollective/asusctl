use std::collections::BTreeMap;

use config_traits::StdConfig;
use log::error;
use rog_aura::AuraDeviceType;
use rog_scsi::{AuraEffect, AuraMode};
use zbus::fdo::Error as ZbErr;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, interface};

use super::ScsiAura;
use crate::error::RogError;

#[derive(Clone)]
pub struct ScsiZbus(ScsiAura);

impl ScsiZbus {
    pub fn new(scsi: ScsiAura) -> Self {
        Self(scsi)
    }

    pub async fn start_tasks(
        self,
        connection: &Connection,
        path: OwnedObjectPath,
    ) -> Result<(), RogError> {
        connection
            .object_server()
            .at(path.clone(), self)
            .await
            .map_err(|e| error!("Couldn't add server at path: {path}, {e:?}"))
            .ok();
        Ok(())
    }
}

#[interface(name = "xyz.ljones.ScsiAura")]
impl ScsiZbus {
    /// Return the device type for this Aura keyboard
    #[zbus(property)]
    async fn device_type(&self) -> AuraDeviceType {
        self.0.config.lock().await.dev_type
    }

    /// Get enabled or not
    #[zbus(property)]
    async fn enabled(&self) -> bool {
        let lock = self.0.lock_config().await;
        lock.enabled
    }

    /// Set enabled true or false
    #[zbus(property)]
    async fn set_enabled(&self, enabled: bool) {
        let mut config = self.0.lock_config().await;
        config.enabled = enabled;
        config.write();
    }

    #[zbus(property)]
    async fn led_mode(&self) -> u8 {
        let config = self.0.lock_config().await;
        config.current_mode as u8
    }

    #[zbus(property)]
    async fn set_led_mode(&self, mode: AuraMode) -> Result<(), zbus::Error> {
        let mut config = self.0.lock_config().await;
        if let Some(effect) = config.get_effect(mode) {
            self.0
                .write_effect(effect)
                .await
                .map_err(|e| zbus::Error::Failure(format!("{e:?}")))?;
        } else {
            return Err(zbus::Error::Failure("Mode data does not exist".to_string()));
        }
        config.current_mode = mode;
        config.write();
        Ok(())
    }

    /// The current mode data
    #[zbus(property)]
    async fn led_mode_data(&self) -> Result<AuraEffect, ZbErr> {
        // entirely possible to deadlock here, so use try instead of lock()
        if let Ok(config) = self.0.config.try_lock() {
            let mode = config.current_mode;
            match config.modes.get(&mode) {
                Some(effect) => Ok(effect.clone()),
                None => Err(ZbErr::Failed("Could not get the current effect".into())),
            }
        } else {
            Err(ZbErr::Failed("Aura control couldn't lock self".to_string()))
        }
    }

    /// Set an Aura effect if the effect mode or zone is supported.
    ///
    /// On success the aura config file is read to refresh cached values, then
    /// the effect is stored and config written to disk.
    #[zbus(property)]
    async fn set_led_mode_data(&mut self, effect: AuraEffect) -> Result<(), ZbErr> {
        self.0.write_effect(&effect).await?;

        let mut config = self.0.config.lock().await;
        config.save_effect(effect);
        config.write();
        Ok(())
    }

    /// Get the data set for every mode available
    async fn all_mode_data(&self) -> BTreeMap<AuraMode, AuraEffect> {
        let config = self.0.config.lock().await;
        config.modes.clone()
    }
}
