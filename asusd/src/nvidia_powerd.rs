use log::{debug, info, warn};
use zbus::Connection;
use zbus::proxy::CacheProperties;
use zbus::zvariant::OwnedObjectPath;

const UNIT: &str = "nvidia-powerd.service";

#[zbus::proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1",
    gen_blocking = false
)]
trait SystemdManager {
    fn get_unit_file_state(&self, name: &str) -> zbus::Result<String>;
    fn get_unit(&self, name: &str) -> zbus::Result<OwnedObjectPath>;
    fn start_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
    fn stop_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
    fn try_restart_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
    fn reset_failed_unit(&self, name: &str) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1",
    gen_blocking = false
)]
trait SystemdUnit {
    #[zbus(property)]
    fn active_state(&self) -> zbus::Result<String>;
}

/// Unit file states where the admin asked for the unit to be started, either
/// persistently or for this boot only. Everything else (`disabled`, `masked`,
/// `static`, `indirect`, ...) means asusd must leave the unit alone.
fn is_enabled_state(state: &str) -> bool {
    matches!(state, "enabled" | "enabled-runtime")
}

/// The systemd manager, if nvidia-powerd is enabled. `None` otherwise, which is
/// normal when the unit isn't installed.
async fn manager_if_enabled(connection: &Connection) -> Option<SystemdManagerProxy<'static>> {
    let manager = match SystemdManagerProxy::builder(connection)
        .cache_properties(CacheProperties::No)
        .build()
        .await
    {
        Ok(manager) => manager,
        Err(e) => {
            warn!("Could not connect to systemd to manage {UNIT}: {e}");
            return None;
        }
    };
    match manager.get_unit_file_state(UNIT).await {
        Ok(state) if is_enabled_state(&state) => Some(manager),
        Ok(state) => {
            debug!("{UNIT} is {state}, leaving it alone");
            None
        }
        Err(e) => {
            debug!("Could not get the unit file state of {UNIT}, leaving it alone: {e}");
            None
        }
    }
}

/// The unit's `ActiveState` (`active`, `inactive`, `failed`, ...), or `None`
/// if the unit isn't loaded or the state can't be read.
async fn active_state(manager: &SystemdManagerProxy<'_>) -> Option<String> {
    let path = manager.get_unit(UNIT).await.ok()?;
    let unit = SystemdUnitProxy::builder(manager.inner().connection())
        .path(path)
        .ok()?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .ok()?;
    unit.active_state().await.ok()
}

/// Clear the unit's start counter, and its failed state if any. NVIDIA's unit
/// sets `StartLimitIntervalSec=infinity` with `StartLimitBurst=5`, so without
/// this the 6th start of a boot is refused, and every start asusd makes counts.
async fn reset_start_limit(manager: &SystemdManagerProxy<'_>) {
    if let Err(e) = manager.reset_failed_unit(UNIT).await {
        debug!("Could not reset the start limit of {UNIT}: {e}");
    }
}

/// Queue a start job for nvidia-powerd if its unit is enabled. Doesn't wait
/// for the job to finish.
pub async fn start_if_enabled(connection: &Connection) {
    let Some(manager) = manager_if_enabled(connection).await else {
        return;
    };
    if active_state(&manager).await.as_deref() == Some("failed") {
        warn!("{UNIT} had failed, clearing its failed state before starting it");
    }
    reset_start_limit(&manager).await;
    info!("Starting {UNIT}");
    if let Err(e) = manager.start_unit(UNIT, "replace").await {
        warn!("Could not start {UNIT}: {e}");
    }
}

/// Queue a stop job for nvidia-powerd if its unit is enabled. Doesn't wait for
/// the job to finish.
pub async fn stop_if_enabled(connection: &Connection) {
    let Some(manager) = manager_if_enabled(connection).await else {
        return;
    };
    info!("Stopping {UNIT}");
    if let Err(e) = manager.stop_unit(UNIT, "replace").await {
        warn!("Could not stop {UNIT}: {e}");
    }
}

/// Queue a restart job for nvidia-powerd if its unit is enabled and it is
/// running, so it picks up new GPU power limits. Doesn't wait for the job to
/// finish.
pub async fn try_restart_if_enabled(connection: &Connection) {
    let Some(manager) = manager_if_enabled(connection).await else {
        return;
    };
    if !matches!(
        active_state(&manager).await.as_deref(),
        Some("active" | "activating")
    ) {
        debug!("{UNIT} isn't running, not restarting it");
        return;
    }
    reset_start_limit(&manager).await;
    info!("Restarting {UNIT}");
    if let Err(e) = manager.try_restart_unit(UNIT, "replace").await {
        warn!("Could not restart {UNIT}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::is_enabled_state;

    #[test]
    fn only_enabled_states_are_managed() {
        for state in [
            "enabled", "enabled-runtime",
        ] {
            assert!(is_enabled_state(state), "{state}");
        }
        for state in [
            "disabled", "masked", "masked-runtime", "static", "indirect", "alias", "linked",
            "linked-runtime", "generated", "transient", "bad", "",
        ] {
            assert!(!is_enabled_state(state), "{state}");
        }
    }
}
