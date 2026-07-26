use std::sync::{Arc, Mutex};

use zbus::zvariant::{OwnedValue, Type, Value};
use zbus::{interface, proxy};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Type, Value, OwnedValue)]
#[zvariant(signature = "u")]
pub enum AppState {
    MainWindowOpen = 0,
    /// If the app is running, open the main window
    MainWindowShouldOpen = 1,
    MainWindowClosed = 2,
    StartingUp = 3,
    QuitApp = 4,
    LockFailed = 5,
}

pub struct ROGCCZbus {
    state: Arc<Mutex<AppState>>,
}

impl ROGCCZbus {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AppState::StartingUp)),
        }
    }

    pub fn clone_state(&self) -> Arc<Mutex<AppState>> {
        self.state.clone()
    }
}

pub const ZBUS_PATH: &str = "/xyz/ljones/rogcc";
pub const ZBUS_IFACE: &str = "xyz.ljones.rogcc";

#[interface(name = "xyz.ljones.rogcc")]
impl ROGCCZbus {
    /// Return the device type for this Aura keyboard
    #[zbus(property)]
    async fn state(&self) -> AppState {
        if let Ok(lock) = self.state.try_lock() {
            return *lock;
        }
        AppState::LockFailed
    }

    #[zbus(property)]
    async fn set_state(&self, state: AppState) {
        if let Ok(mut lock) = self.state.try_lock() {
            *lock = state;
        }
    }
}

#[proxy(
    interface = "xyz.ljones.rogcc",
    default_service = "xyz.ljones.rogcc",
    default_path = "/xyz/ljones/rogcc"
)]
pub trait ROGCCZbus {
    /// EnableDisplay property
    #[zbus(property)]
    fn state(&self) -> zbus::Result<AppState>;

    #[zbus(property)]
    fn set_state(&self, state: AppState) -> zbus::Result<()>;
}

/// D-Bus proxy for asusd's GPU power status interface (`xyz.ljones.Gpu`).
#[proxy(
    interface = "xyz.ljones.Gpu",
    default_service = "xyz.ljones.Asusd",
    default_path = "/xyz/ljones/Gpu"
)]
pub trait GpuStatus {
    /// Current GPU power status (e.g. "active", "suspended", "off").
    #[zbus(property)]
    fn power_status(&self) -> zbus::Result<String>;

    /// GPU vendor name.
    #[zbus(property)]
    fn vendor(&self) -> zbus::Result<String>;

    /// Current GPU mode (e.g. "Optimus", "Integrated", "Vfio", "Ultimate").
    #[zbus(property)]
    fn mode(&self) -> zbus::Result<String>;
}

pub use rog_dbus::{find_iface_async, find_iface_blocking};
