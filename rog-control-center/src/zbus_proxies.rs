use std::sync::{Arc, Mutex};

use zbus::blocking::proxy::ProxyImpl;
use zbus::blocking::{fdo, Connection};
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

pub fn find_iface<T>(iface_name: &str) -> Result<Vec<T>, Box<dyn std::error::Error>>
where
    T: ProxyImpl<'static> + From<zbus::Proxy<'static>>,
{
    let conn = Connection::system()?;
    let f = fdo::ObjectManagerProxy::new(&conn, "xyz.ljones.Asusd", "/")?;
    let interfaces = f.get_managed_objects()?;
    let mut paths = Vec::new();
    for v in interfaces.iter() {
        // let o: Vec<zbus::names::OwnedInterfaceName> = v.1.keys().map(|e|
        // e.to_owned()).collect(); println!("{}, {:?}", v.0, o);
        for k in v.1.keys() {
            if k.as_str() == iface_name {
                // println!("Found {iface_name} device at {}, {}", v.0, k);
                paths.push(v.0.clone());
            }
        }
    }
    if paths.len() > 1 {
        log::warn!("Multiple asusd interfaces devices found");
    }
    if !paths.is_empty() {
        let mut ctrl = Vec::new();
        paths.sort_by(|a, b| a.cmp(b));
        for path in paths {
            ctrl.push(
                T::builder(&conn)
                    .path(path.clone())?
                    .destination("xyz.ljones.Asusd")?
                    .build()?,
            );
        }
        return Ok(ctrl);
    }

    Err("No Aura interface".into())
}

pub async fn find_iface_async<T>(iface_name: &str) -> Result<Vec<T>, Box<dyn std::error::Error>>
where
    T: zbus::proxy::ProxyImpl<'static> + From<zbus::Proxy<'static>>,
{
    let conn = zbus::Connection::system().await?;
    let f = zbus::fdo::ObjectManagerProxy::new(&conn, "xyz.ljones.Asusd", "/").await?;
    let interfaces = f.get_managed_objects().await?;
    let mut paths = Vec::new();
    for v in interfaces.iter() {
        // let o: Vec<zbus::names::OwnedInterfaceName> = v.1.keys().map(|e|
        // e.to_owned()).collect(); println!("{}, {:?}", v.0, o);
        for k in v.1.keys() {
            if k.as_str() == iface_name {
                // println!("Found {iface_name} device at {}, {}", v.0, k);
                paths.push(v.0.clone());
            }
        }
    }
    if paths.len() > 1 {
        log::warn!("Multiple asusd interfaces devices found");
    }
    if !paths.is_empty() {
        let mut ctrl = Vec::new();
        paths.sort_by(|a, b| a.cmp(b));
        for path in paths {
            ctrl.push(
                T::builder(&conn)
                    .path(path.clone())?
                    .destination("xyz.ljones.Asusd")?
                    .build()
                    .await?,
            );
        }
        return Ok(ctrl);
    }

    Err("No interface".into())
}
