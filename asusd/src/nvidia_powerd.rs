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
    fn start_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
    fn stop_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
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

/// Queue a start job for nvidia-powerd if its unit is enabled. Doesn't wait
/// for the job to finish.
pub async fn start_if_enabled(connection: &Connection) {
    let Some(manager) = manager_if_enabled(connection).await else {
        return;
    };
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
