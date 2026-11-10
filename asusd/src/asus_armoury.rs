use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex, MutexGuard};

use config_traits::StdConfig;
use log::{debug, error, info, warn};
use rog_platform::asus_armoury::{
    AttrValue, Attribute, FirmwareAttribute, FirmwareAttributeType, FirmwareAttributes,
};
use rog_platform::platform::{PlatformProfile, RogPlatform};
use rog_platform::power::AsusPower;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Type, Value};
use zbus::{Connection, fdo, interface};

use crate::config::Config;
use crate::error::RogError;
use crate::{ASUS_ZBUS_PATH, Reloadable};

const MOD_NAME: &str = "asus_armoury";

/// Result of updating one GPU attribute in the shared deferred queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeferredGpuQueueUpdate {
    /// Attribute already matches firmware; only that key was removed.
    DroppedRedundant,
    /// Attribute differs from firmware; value was queued for later apply.
    Queued,
}

/// Update a single GPU attribute in the deferred apply queue.
///
/// When `already_active`, only `attr` is removed. Other pending GPU attributes
/// must stay so Hybrid→Integrated (`dgpu_disable=1` then no-op `gpu_mux_mode=1`)
/// cannot drop the disable request (#459).
fn update_deferred_gpu_queue(
    queue: &mut HashMap<FirmwareAttribute, i32>,
    attr: FirmwareAttribute,
    value: i32,
    already_active: bool,
) -> DeferredGpuQueueUpdate {
    if already_active {
        queue.remove(&attr);
        DeferredGpuQueueUpdate::DroppedRedundant
    } else {
        queue.insert(attr, value);
        DeferredGpuQueueUpdate::Queued
    }
}

/// Re-queue a lasting desired GPU value when firmware drifted and no in-flight
/// pending override exists for that attribute (#357).
fn requeue_desired_gpu_if_drifted(
    queue: &mut HashMap<FirmwareAttribute, i32>,
    attr: FirmwareAttribute,
    desired: i32,
    firmware_matches: bool,
) -> bool {
    if firmware_matches || queue.contains_key(&attr) {
        return false;
    }
    queue.insert(attr, desired);
    true
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, Type, Value, OwnedValue)]
pub struct PossibleValues {
    strings: Vec<String>,
    nums: Vec<i32>,
}

fn dbus_path_for_attr(attr_name: &str) -> OwnedObjectPath {
    ObjectPath::from_str_unchecked(&format!("{ASUS_ZBUS_PATH}/{MOD_NAME}/{attr_name}")).into()
}

#[derive(Clone)]
pub struct AsusArmouryAttribute {
    attr: Attribute,
    /// Shared daemon config. Still `tokio::sync::Mutex` because the rest of
    /// asusd shares this type; never hold it across `.await` or sysfs I/O.
    config: Arc<Mutex<Config>>,
    /// Deferred GPU queue is sync state: lock, copy/mutate, drop before any
    /// `.await` or config lock (governance: no async mutex on hardware queues).
    queued_gpu: Arc<StdMutex<HashMap<FirmwareAttribute, i32>>>,
    /// platform control required here for access to PPD or Throttle profile
    platform: RogPlatform,
    power: AsusPower,
    logged_read_error: Arc<AtomicBool>,
}

impl AsusArmouryAttribute {
    pub fn new(
        attr: Attribute,
        platform: RogPlatform,
        power: AsusPower,
        config: Arc<Mutex<Config>>,
        queued_gpu: Arc<StdMutex<HashMap<FirmwareAttribute, i32>>>,
    ) -> Self {
        Self {
            attr,
            config,
            queued_gpu,
            platform,
            power,
            logged_read_error: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn attribute_name(&self) -> String {
        String::from(self.attr.name())
    }

    fn lock_queued_gpu(&self) -> MutexGuard<'_, HashMap<FirmwareAttribute, i32>> {
        self.queued_gpu
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Persist `pending_gpu` while the caller already holds the config lock.
    /// Callers must drop the sync queue guard before awaiting this lock.
    fn write_pending_gpu(config: &mut Config, queue: &HashMap<FirmwareAttribute, i32>) {
        if config.pending_gpu != *queue {
            config.pending_gpu = queue.clone();
            config.write();
        }
    }

    /// Persist lasting desired GPU mode in `armoury_settings` (#357).
    fn write_desired_gpu(config: &mut Config, attr: FirmwareAttribute, value: i32) -> bool {
        if config.armoury_settings.get(&attr).copied() == Some(value) {
            return false;
        }
        config.armoury_settings.insert(attr, value);
        true
    }

    async fn clear_queued_if_matches(&self, name: FirmwareAttribute, value: i32) {
        let queue_snapshot = {
            let mut queue = self.lock_queued_gpu();
            if queue.get(&name).copied() == Some(value) {
                queue.remove(&name);
            }
            queue.clone()
        };
        let mut config = self.config.lock().await;
        // Successful (or already-active) apply becomes the lasting desired mode.
        let desired_dirty = Self::write_desired_gpu(&mut config, name, value);
        let pending_dirty = config.pending_gpu != queue_snapshot;
        if pending_dirty {
            config.pending_gpu = queue_snapshot;
        }
        if desired_dirty || pending_dirty {
            config.write();
        }
    }

    /// If firmware drifted from lasting desired GPU mode, re-queue for deferred
    /// apply via asus-shutdown. Never sync-writes GPU sysfs from reload (#357).
    async fn ensure_desired_gpu_queued(&self) {
        let name = self.name();
        if name.property_type() != FirmwareAttributeType::Gpu {
            return;
        }
        let desired = {
            let config = self.config.lock().await;
            config.armoury_settings.get(&name).copied()
        };
        let Some(desired) = desired else {
            return;
        };
        let firmware_matches = self.attr.current_value().ok() == Some(AttrValue::Integer(desired));
        let queue_snapshot = {
            let mut queue = self.lock_queued_gpu();
            if !requeue_desired_gpu_if_drifted(&mut queue, name, desired, firmware_matches) {
                return;
            }
            info!(
                "Re-queued desired GPU attribute {} = {desired} after firmware drift",
                <&str>::from(name)
            );
            queue.clone()
        };
        let mut config = self.config.lock().await;
        Self::write_pending_gpu(&mut config, &queue_snapshot);
    }

    fn resolve_i32_value(refreshed: Option<i32>, cached: &AttrValue) -> i32 {
        refreshed
            .or(match cached {
                AttrValue::Integer(i) => Some(*i),
                _ => None,
            })
            .unwrap_or(-1)
    }

    pub async fn emit_limits(&self, connection: &Connection) -> Result<(), RogError> {
        let path = dbus_path_for_attr(self.attr.name());
        let signal = SignalEmitter::new(connection, path)?;
        self.min_value_changed(&signal).await?;
        self.max_value_changed(&signal).await?;
        self.scalar_increment_changed(&signal).await?;
        self.current_value_changed(&signal).await?;
        Ok(())
    }

    pub async fn move_to_zbus(self, connection: &Connection) -> Result<(), RogError> {
        let path = dbus_path_for_attr(self.attr.name());
        connection
            .object_server()
            .at(path.clone(), self)
            .await
            .map_err(|e| error!("Couldn't add server at path: {path}, {e:?}"))
            .ok();
        Ok(())
    }

    async fn watch_and_notify(
        &mut self,
        signal_ctxt: SignalEmitter<'static>,
    ) -> Result<(), RogError> {
        use futures_util::StreamExt;

        let name = self.name();
        macro_rules! watch_value_notify {
            ($attr_str:expr, $fn_prop_changed:ident) => {
                match self.attr.get_watcher() {
                    Ok(watch) => {
                        let name = <&str>::from(name);
                        let ctrl = self.clone();
                        let sig = signal_ctxt.clone();
                        tokio::spawn(async move {
                            let mut buffer = [0; 32];
                            if let Ok(stream) = watch.into_event_stream(&mut buffer) {
                                stream
                                    .for_each(|_| async {
                                        debug!("{} changed", name);
                                        ctrl.$fn_prop_changed(&sig).await.ok();
                                    })
                                    .await;
                            } else {
                                info!(
                                    "inotify event stream failed for {} ({}). You can ignore this \
                                     if unsupported",
                                    name, $attr_str
                                );
                            }
                        });
                    }
                    Err(e) => info!(
                        "inotify watch failed: {}. You can ignore this if your device does not \
                         support the feature",
                        e
                    ),
                }
            };
        }

        // "current_value", "default_value", "min_value", "max_value"
        watch_value_notify!("current_value", current_value_changed);
        watch_value_notify!("default_value", default_value_changed);
        watch_value_notify!("min_value", min_value_changed);
        watch_value_notify!("max_value", max_value_changed);

        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct ArmouryAttributeRegistry {
    attrs: Vec<AsusArmouryAttribute>,
}

impl ArmouryAttributeRegistry {
    pub fn push(&mut self, attr: AsusArmouryAttribute) {
        self.attrs.push(attr);
    }

    pub async fn emit_limits(&self, connection: &Connection) -> Result<(), RogError> {
        let mut last_err: Option<RogError> = None;
        for attr in &self.attrs {
            if let Err(e) = attr.emit_limits(connection).await {
                error!(
                    "Failed to emit updated limits for attribute '{}': {e:?}",
                    attr.attribute_name()
                );
                last_err = Some(e);
            }
        }
        if let Some(err) = last_err {
            Err(err)
        } else {
            Ok(())
        }
    }
}

impl crate::Reloadable for AsusArmouryAttribute {
    async fn reload(&mut self) -> Result<(), RogError> {
        info!("Reloading {}", self.attr.name());
        let attribute: FirmwareAttribute = self.attr.name().into();
        let name = self.attr.name();

        let mut config = self.config.lock().await;
        let apply_value = match attribute.property_type() {
            FirmwareAttributeType::Ppt => {
                let profile: PlatformProfile = self.platform.get_platform_profile()?.into();
                let power_plugged = self
                    .power
                    .get_online()
                    .map_err(|e| {
                        error!("Could not get power status: {e:?}");
                        e
                    })
                    .unwrap_or_default()
                    == 1;

                let apply_value = {
                    config.select_tunings_ref(power_plugged, profile).and_then(
                        |tuning| match tuning.enabled {
                            true => tuning.group.get(&self.name()).copied(),
                            false => None,
                        },
                    )
                };

                apply_value.map_or(AttrValue::None, AttrValue::Integer)
            }
            FirmwareAttributeType::Gpu => {
                // Lasting desired mode lives in armoury_settings and is re-queued
                // into pending_gpu by ensure_desired_gpu_queued (#357). Never
                // sync-write GPU firmware attributes from reload.
                info!("Reload called on GPU attribute {name}: deferred via pending_gpu");
                AttrValue::None
            }
            FirmwareAttributeType::ReadOnly => {
                if config.armoury_settings.remove(&attribute).is_some() {
                    info!("Removed stale persisted read-only attribute {name} from config");
                    config.write();
                }
                info!("Reload called on read-only attribute {name}: doing nothing");
                AttrValue::None
            }
            FirmwareAttributeType::Norestore => {
                if config.armoury_settings.remove(&attribute).is_some() {
                    info!("Removed stale persisted norestore attribute {name} from config");
                    config.write();
                }
                info!("Reload called on norestore attribute {name}: doing nothing");
                AttrValue::None
            }
            _ => {
                info!("Reload called on firmware attribute {name}");
                match config.armoury_settings.get(&attribute) {
                    Some(saved_value) => AttrValue::Integer(*saved_value),
                    None => AttrValue::None,
                }
            }
        };

        match apply_value {
            AttrValue::None => {
                info!(
                    "No saved value for attribute {}: skipping.",
                    self.attr.name()
                );
            }
            _ => {
                info!("Applying value {apply_value:?} to attribute {name}");
                if let Err(e) = self.attr.set_current_value(&apply_value) {
                    if attribute.property_type() == FirmwareAttributeType::Ppt {
                        warn!(
                            "Could not restore PPT setting {name} to {apply_value:?}: {e:?}. Hardware may restrict this setting in current power state."
                        );
                    } else {
                        error!("Could not set {name} value: {e:?}");
                        self.attr.base_path_exists();
                        return Err(e.into());
                    }
                } else {
                    info!("Restored asus-armoury setting {name} to {apply_value:?}");
                }
            }
        }

        Ok(())
    }
}

/// If return is `-1` on a property then there is available value for that
/// property
#[interface(name = "xyz.ljones.AsusArmoury")]
impl AsusArmouryAttribute {
    #[zbus(property)]
    fn name(&self) -> FirmwareAttribute {
        self.attr.name().into()
    }

    #[zbus(property)]
    async fn available_attrs(&self) -> Vec<String> {
        let mut attrs = Vec::new();
        if !matches!(self.attr.default_value(), AttrValue::None) {
            attrs.push("default_value".to_string());
        }
        if !matches!(self.attr.min_value(), AttrValue::None) {
            attrs.push("min_value".to_string());
        }
        if !matches!(self.attr.max_value(), AttrValue::None) {
            attrs.push("max_value".to_string());
        }
        if !matches!(self.attr.scalar_increment(), AttrValue::None) {
            attrs.push("scalar_increment".to_string());
        }
        if !matches!(self.attr.possible_values(), AttrValue::None) {
            attrs.push("possible_values".to_string());
        }
        match self.attr.current_value() {
            Ok(value) => {
                self.logged_read_error.store(false, Ordering::Relaxed);
                if matches!(value, AttrValue::Integer(_)) {
                    attrs.push("current_value".to_string());
                }
            }
            Err(e) => {
                if !self.logged_read_error.swap(true, Ordering::Relaxed) {
                    error!(
                        "Firmware attribute '{}' is not supported or failed to read: {e:?}",
                        self.attr.name()
                    );
                }
            }
        }
        attrs
    }

    /// If return is `-1` then there is no default value
    #[zbus(property)]
    async fn default_value(&self) -> i32 {
        match self.attr.default_value() {
            AttrValue::Integer(i) => *i,
            _ => -1,
        }
    }

    async fn restore_default(&self) -> fdo::Result<()> {
        let name = self.attr.name();
        match self.name().property_type() {
            FirmwareAttributeType::ReadOnly => {
                warn!("Attempted to restore read-only attribute {name}: write discarded");
                Err(fdo::Error::NotSupported(format!(
                    "{name} is read-only and cannot be changed"
                )))
            }
            FirmwareAttributeType::Ppt => {
                let AttrValue::Integer(default_val) = *self.attr.default_value() else {
                    return Err(fdo::Error::NotSupported(format!(
                        "No default value available for {name}"
                    )));
                };

                let profile: PlatformProfile = self.platform.get_platform_profile()?.into();
                let power_plugged = self.power.get_online().unwrap_or_default() == 1;

                let mut config = self.config.lock().await;
                let tuning = config.select_tunings(power_plugged, profile);
                if tuning.enabled {
                    self.attr
                        .set_current_value(&AttrValue::Integer(default_val))
                        .map_err(|e| {
                            error!("Could not set value: {e:?}");
                            e
                        })?;
                }
                tuning.group.insert(self.name(), default_val);
                config.write();
                Ok(())
            }
            _ => {
                self.attr.restore_default()?;
                let attribute = self.name();
                // Drop deferred apply and lasting desired so asus-shutdown cannot
                // undo this restore after a reboot. Drop the sync queue guard
                // before awaiting the config lock.
                let queue_snapshot = {
                    let mut queue = self.lock_queued_gpu();
                    queue.remove(&attribute);
                    queue.clone()
                };
                let mut config = self.config.lock().await;
                let removed_setting = config.armoury_settings.remove(&attribute).is_some();
                let pending_dirty = config.pending_gpu != queue_snapshot;
                if pending_dirty {
                    config.pending_gpu = queue_snapshot;
                }
                if pending_dirty || removed_setting {
                    config.write();
                }
                Ok(())
            }
        }
    }

    #[zbus(property)]
    async fn min_value(&self) -> i32 {
        Self::resolve_i32_value(self.attr.refresh_min_value(), self.attr.min_value())
    }

    #[zbus(property)]
    async fn max_value(&self) -> i32 {
        Self::resolve_i32_value(self.attr.refresh_max_value(), self.attr.max_value())
    }

    #[zbus(property)]
    async fn scalar_increment(&self) -> i32 {
        Self::resolve_i32_value(
            self.attr.refresh_scalar_increment(),
            self.attr.scalar_increment(),
        )
    }

    #[zbus(property)]
    async fn possible_values(&self) -> Vec<i32> {
        match self.attr.possible_values() {
            AttrValue::EnumInt(i) => i.clone(),
            _ => Vec::default(),
        }
    }

    #[zbus(property)]
    async fn current_value(&self) -> fdo::Result<i32> {
        if self.name().property_type() == FirmwareAttributeType::Ppt {
            let profile: PlatformProfile = self.platform.get_platform_profile()?.into();
            let power_plugged = self.power.get_online().unwrap_or_default() == 1;
            let config = self.config.lock().await;
            if let Some(tuning) = config.select_tunings_ref(power_plugged, profile)
                && tuning.enabled
                && let Some(tune) = tuning.group.get(&self.name())
            {
                return Ok(*tune);
            }
            if let Ok(AttrValue::Integer(cur)) = self.attr.current_value() {
                return Ok(cur);
            }
            if let AttrValue::Integer(i) = self.attr.default_value() {
                return Ok(*i);
            }
            return Err(fdo::Error::Failed(
                "Could not read current value".to_string(),
            ));
        }

        // Queued GPU values stay on `queued_gpu_value` / `pending_gpu` so
        // `current_value` always mirrors firmware until shutdown apply.

        if let Ok(value) = self.attr.current_value() {
            self.logged_read_error.store(false, Ordering::Relaxed);
            if let AttrValue::Integer(i) = value {
                return Ok(i);
            }
        }
        Err(fdo::Error::Failed(
            "Could not read current value".to_string(),
        ))
    }

    #[zbus(property)]
    async fn set_current_value(&mut self, value: i32) -> fdo::Result<()> {
        let name = self.attr.name();

        // if read-only, don't even attempt to set or persist value
        if self.name().property_type() == FirmwareAttributeType::ReadOnly {
            warn!("Attempted to set read-only attribute {name}: write discarded");
            return Err(fdo::Error::NotSupported(format!(
                "{name} is read-only and cannot be changed"
            )));
        }

        let (apply_value, ppt_context) = match self.name().property_type() {
            FirmwareAttributeType::Ppt => {
                let profile: PlatformProfile = self.platform.get_platform_profile()?.into();
                let power_plugged = self.power.get_online().unwrap_or_default() == 1;

                let config = self.config.lock().await;
                let tuning_enabled = config
                    .select_tunings_ref(power_plugged, profile)
                    .map(|t| t.enabled)
                    .unwrap_or(false);

                if !tuning_enabled {
                    warn!(
                        "Cannot set PPT property {name}: profile tuning is disabled for current profile and power state"
                    );
                    return Err(fdo::Error::Failed(format!(
                        "Cannot set PPT property {name}: profile tuning is disabled for current profile and power state. Enable tuning first."
                    )));
                }

                debug!("Tuning is enabled: setting value to PPT property {name} = {value}");
                (AttrValue::Integer(value), Some((profile, power_plugged)))
            }
            FirmwareAttributeType::Gpu => {
                let attr_name = self.name();
                let already_active =
                    self.attr.current_value().ok() == Some(AttrValue::Integer(value));
                // Only drop *this* attribute's pending entry. Sibling GPU
                // attributes (e.g. dgpu_disable queued before a no-op
                // gpu_mux_mode write) must remain for shutdown/boot apply (#459).
                let queue_snapshot = {
                    let mut queue = self.lock_queued_gpu();
                    match update_deferred_gpu_queue(&mut queue, attr_name, value, already_active) {
                        DeferredGpuQueueUpdate::DroppedRedundant => info!(
                            "GPU attribute {name} is already {value}, dropping only that deferred entry"
                        ),
                        DeferredGpuQueueUpdate::Queued => {
                            debug!("Queueing GPU attribute {name} = {value} for delayed apply")
                        }
                    }
                    queue.clone()
                };
                let mut config = self.config.lock().await;
                // Persist lasting desired mode immediately so a later boot can
                // re-queue after firmware reset even once pending_gpu clears (#357).
                let desired_dirty = Self::write_desired_gpu(&mut config, attr_name, value);
                let pending_dirty = config.pending_gpu != queue_snapshot;
                if pending_dirty {
                    config.pending_gpu = queue_snapshot;
                }
                if desired_dirty || pending_dirty {
                    config.write();
                }
                return Ok(());
            }
            FirmwareAttributeType::Norestore => {
                debug!("Setting norestore attribute {name} = {value} synchronously");
                self.attr
                    .set_current_value(&AttrValue::Integer(value))
                    .map_err(|e| {
                        error!("Could not set value {value} to attribute {name}: {e:?}");
                        e
                    })?;
                return Ok(());
            }
            _ => (AttrValue::Integer(value), None),
        };

        // Write to sysfs first. If kernel rejects the value (e.g. invalid on battery),
        // we return Err immediately and DO NOT modify in-memory or persisted config.
        self.attr.set_current_value(&apply_value).map_err(|e| {
            error!("Could not set value {value} to attribute {name}: {e:?}");
            e
        })?;

        // Commit to in-memory config and persist to disk ONLY AFTER successful hardware write.
        {
            let mut config = self.config.lock().await;
            if let Some((profile, power_plugged)) = ppt_context {
                let tuning = config.select_tunings(power_plugged, profile);
                tuning.group.insert(self.name(), value);
                debug!("Store tuning config for {name} = {:?}", value);
            } else {
                config.armoury_settings.insert(self.name(), value);
                debug!("Set config for {name} = {value}");
            }
            config.write();
        }

        // When an nv_* attribute (Nvidia TDP/temp) is written, restart
        // nvidia-powerd so it re-reads the new TDP limits from hardware.
        match self.name() {
            FirmwareAttribute::NvDynamicBoost
            | FirmwareAttribute::NvTempTarget
            | FirmwareAttribute::DgpuTgp => {
                let _ = std::process::Command::new("systemctl")
                    .args([
                        "try-restart",
                        "nvidia-powerd.service",
                    ])
                    .output();
            }
            _ => {}
        }

        Ok(())
    }

    /// Returns queued GPU value when present, otherwise `-1`.
    #[zbus(property)]
    async fn queued_gpu_value(&self) -> fdo::Result<i32> {
        if self.name().property_type() != FirmwareAttributeType::Gpu {
            return Ok(-1);
        }
        let queued = self.lock_queued_gpu();
        Ok(queued.get(&self.name()).copied().unwrap_or(-1))
    }

    /// Applies queued GPU value if present and returns whether anything was applied.
    async fn apply_queued_gpu_value(&mut self) -> fdo::Result<bool> {
        if self.name().property_type() != FirmwareAttributeType::Gpu {
            return Ok(false);
        }

        let name = self.name();
        let value = {
            let queue = self.lock_queued_gpu();
            let Some(value) = queue.get(&name).copied() else {
                return Ok(false);
            };
            value
        };

        if self.attr.current_value().ok() == Some(AttrValue::Integer(value)) {
            info!(
                "Queued GPU attribute {} = {value} is already active, skipping redundant write",
                <&str>::from(name)
            );
            self.clear_queued_if_matches(name, value).await;
            return Ok(true);
        }

        // Sysfs write with no queue/config lock held.
        self.attr
            .set_current_value(&AttrValue::Integer(value))
            .map_err(|e| {
                error!(
                    "Could not apply queued GPU attribute {} = {value}: {e:?}",
                    <&str>::from(name)
                );
                e
            })?;

        info!(
            "Applied queued GPU attribute {} = {value}",
            <&str>::from(name)
        );

        // Keep the value queued (and persisted) on failure so the next reboot
        // can finish a partial Ultimate→Hybrid apply.
        self.clear_queued_if_matches(name, value).await;
        Ok(true)
    }
}

pub async fn start_attributes_zbus(
    conn: &Connection,
    platform: RogPlatform,
    power: AsusPower,
    attributes: FirmwareAttributes,
    config: Arc<Mutex<Config>>,
) -> Result<ArmouryAttributeRegistry, RogError> {
    let mut registry = ArmouryAttributeRegistry::default();
    let pending = {
        let config = config.lock().await;
        config.pending_gpu.clone()
    };
    if !pending.is_empty() {
        info!(
            "Restored {} deferred GPU attribute(s) from config for deferred apply",
            pending.len()
        );
    }
    let queued_gpu = Arc::new(StdMutex::new(pending));
    for attr in attributes.attributes() {
        let mut attr = AsusArmouryAttribute::new(
            attr.clone(),
            platform.clone(),
            power.clone(),
            config.clone(),
            queued_gpu.clone(),
        );

        let registry_attr = attr.clone();

        // Re-queue lasting desired GPU mode before reload so asus-shutdown's
        // boot apply can see pending_gpu when firmware reset to Hybrid (#357).
        attr.ensure_desired_gpu_queued().await;

        if let Err(e) = attr.reload().await {
            error!(
                "Skipping attribute '{}' due to reload error: {e:?}",
                attr.attr.name()
            );
            // Stop registering remaining attributes on reload failure to prevent
            // exposing partially initialized or inconsistent hardware state.
            break;
        }

        let attr_name = attr.attribute_name();

        let path = dbus_path_for_attr(attr_name.as_str());
        match zbus::object_server::SignalEmitter::new(conn, path) {
            Ok(sig) => {
                if let Err(e) = attr.watch_and_notify(sig).await {
                    error!("Failed to start watcher for '{}': {e:?}", attr.attr.name());
                }
            }
            Err(e) => {
                error!(
                    "Failed to create SignalEmitter for '{}': {e:?}",
                    attr.attr.name()
                );
            }
        }

        if let Err(e) = attr.move_to_zbus(conn).await {
            error!("Failed to register attribute '{attr_name}' on zbus: {e:?}");
            continue;
        }

        registry.push(registry_attr);
    }
    Ok(registry)
}

pub async fn set_config_or_default(
    attrs: &FirmwareAttributes,
    config: &mut Config,
    power_plugged: bool,
    profile: PlatformProfile,
) {
    let mut changed = false;
    for attr in attrs.attributes().iter() {
        let name: FirmwareAttribute = attr.name().into();
        match name.property_type() {
            FirmwareAttributeType::Ppt => {
                let tuning = config.select_tunings(power_plugged, profile);
                if !tuning.enabled {
                    debug!("Tuning group is not enabled, skipping");
                    continue;
                }

                let target_val = tuning.group.get(&name).copied();
                let applied = match target_val {
                    Some(tune) => attr.set_current_value(&AttrValue::Integer(tune)).is_ok(),
                    None => false,
                };

                let final_val = if applied {
                    target_val
                } else {
                    if target_val.is_some() {
                        warn!(
                            "Failed to set PPT {}: falling back to default/current value.",
                            <&str>::from(name)
                        );
                    }
                    let default = attr.default_value();
                    if attr.set_current_value(default).is_ok() {
                        match default {
                            AttrValue::Integer(i) => Some(*i),
                            _ => None,
                        }
                    } else {
                        match attr.current_value() {
                            Ok(AttrValue::Integer(cur)) => Some(cur),
                            _ => None,
                        }
                    }
                };

                if let Some(val) = final_val {
                    if target_val.is_none() {
                        tuning.group.insert(name, val);
                        info!("Set tuning config for {} = {:?}", <&str>::from(name), val);
                        changed = true;
                    } else if target_val != Some(val) {
                        warn!(
                            "PPT {} is running at {val:?}; keeping stored target {target_val:?}",
                            <&str>::from(name)
                        );
                    }
                }
            }
            FirmwareAttributeType::Gpu => {
                // Lasting desired GPU mode stays in armoury_settings and is
                // re-queued into pending_gpu at Armoury start (#357). Profile/AC
                // changes must not wipe it or sync-write GPU sysfs here.
            }
            FirmwareAttributeType::ReadOnly => {
                if config.armoury_settings.remove(&name).is_some() {
                    info!(
                        "Removed stale persisted read-only attribute {} from config",
                        <&str>::from(name)
                    );
                    changed = true;
                }
                // Never restore or apply read-only attributes
            }
            FirmwareAttributeType::Norestore => {
                if config.armoury_settings.remove(&name).is_some() {
                    info!(
                        "Removed stale persisted norestore attribute {} from config",
                        <&str>::from(name)
                    );
                    changed = true;
                }
                // Never restore or apply norestore attributes
            }
            _ => {
                if let Some(saved_value) = config.armoury_settings.get(&name) {
                    attr.set_current_value(&AttrValue::Integer(*saved_value))
                        .map_err(|e| {
                            error!("Failed to set {}: {e}", <&str>::from(name));
                        })
                        .ok();
                    info!(
                        "Restored armoury setting for {} = {:?}",
                        <&str>::from(name),
                        saved_value
                    );
                }
            }
        }
    }
    if changed {
        config.write();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redundant_mux_write_keeps_queued_dgpu_disable() {
        let mut queue = HashMap::new();
        assert_eq!(
            update_deferred_gpu_queue(&mut queue, FirmwareAttribute::DgpuDisable, 1, false),
            DeferredGpuQueueUpdate::Queued
        );
        assert_eq!(queue.get(&FirmwareAttribute::DgpuDisable), Some(&1));

        assert_eq!(
            update_deferred_gpu_queue(&mut queue, FirmwareAttribute::GpuMuxMode, 1, true),
            DeferredGpuQueueUpdate::DroppedRedundant
        );
        assert_eq!(
            queue.get(&FirmwareAttribute::DgpuDisable),
            Some(&1),
            "sibling dgpu_disable must survive a no-op gpu_mux_mode write"
        );
        assert!(!queue.contains_key(&FirmwareAttribute::GpuMuxMode));
    }

    #[test]
    fn redundant_write_drops_only_that_attribute_pending_value() {
        let mut queue = HashMap::from([
            (FirmwareAttribute::DgpuDisable, 1),
            (FirmwareAttribute::GpuMuxMode, 0),
        ]);
        assert_eq!(
            update_deferred_gpu_queue(&mut queue, FirmwareAttribute::GpuMuxMode, 0, true),
            DeferredGpuQueueUpdate::DroppedRedundant
        );
        assert_eq!(queue.get(&FirmwareAttribute::DgpuDisable), Some(&1));
        assert!(!queue.contains_key(&FirmwareAttribute::GpuMuxMode));
    }

    #[test]
    fn desired_gpu_requeues_when_firmware_drifted() {
        let mut queue = HashMap::new();
        assert!(requeue_desired_gpu_if_drifted(
            &mut queue,
            FirmwareAttribute::DgpuDisable,
            1,
            false,
        ));
        assert_eq!(queue.get(&FirmwareAttribute::DgpuDisable), Some(&1));
    }

    #[test]
    fn desired_gpu_skips_requeue_when_firmware_matches() {
        let mut queue = HashMap::new();
        assert!(!requeue_desired_gpu_if_drifted(
            &mut queue,
            FirmwareAttribute::DgpuDisable,
            1,
            true,
        ));
        assert!(queue.is_empty());
    }

    #[test]
    fn desired_gpu_preserves_in_flight_pending_override() {
        let mut queue = HashMap::from([(FirmwareAttribute::DgpuDisable, 0)]);
        assert!(!requeue_desired_gpu_if_drifted(
            &mut queue,
            FirmwareAttribute::DgpuDisable,
            1,
            false,
        ));
        assert_eq!(
            queue.get(&FirmwareAttribute::DgpuDisable),
            Some(&0),
            "in-flight pending must win over lasting desired"
        );
    }
}
