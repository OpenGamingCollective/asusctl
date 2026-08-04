pub use asusd::{DBUS_IFACE, DBUS_NAME, DBUS_PATH};
use zbus::proxy::ProxyImpl;

pub mod asus_armoury;
pub mod scsi_aura;
pub mod zbus_anime;
pub mod zbus_aura;
pub mod zbus_backlight;
pub mod zbus_fan_curves;
pub mod zbus_platform;
pub mod zbus_slash;
pub mod zbus_xgm_led;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn list_iface_blocking() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let conn = zbus::blocking::Connection::system()?;
    let f = zbus::blocking::fdo::ObjectManagerProxy::new(&conn, "xyz.ljones.Asusd", "/")?;
    let interfaces = f.get_managed_objects()?;
    let mut ifaces = Vec::new();
    for v in interfaces.iter() {
        for k in v.1.keys() {
            ifaces.push(k.to_string());
        }
    }
    ifaces.sort();
    ifaces.dedup();
    Ok(ifaces)
}

pub fn has_iface_blocking(iface: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let conn = zbus::blocking::Connection::system()?;
    let f = zbus::blocking::fdo::ObjectManagerProxy::new(&conn, "xyz.ljones.Asusd", "/")?;
    let interfaces = f.get_managed_objects()?;
    for v in interfaces.iter() {
        for k in v.1.keys() {
            if k.as_str() == iface {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub async fn has_iface(iface: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let conn = zbus::Connection::system().await?;
    let f = zbus::fdo::ObjectManagerProxy::new(&conn, "xyz.ljones.Asusd", "/").await?;
    let interfaces = f.get_managed_objects().await?;
    for v in interfaces.iter() {
        for k in v.1.keys() {
            if k.as_str() == iface {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub async fn find_iface_async<T>(iface_name: &str) -> Result<Vec<T>, Box<dyn std::error::Error>>
where
    T: ProxyImpl<'static> + From<zbus::Proxy<'static>>,
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

    Err(format!("Did not find {iface_name}").into())
}

pub fn find_iface_blocking<T>(iface_name: &str) -> Result<Vec<T>, Box<dyn std::error::Error>>
where
    T: zbus::blocking::proxy::ProxyImpl<'static> + From<zbus::Proxy<'static>>,
{
    let conn = zbus::blocking::Connection::system()?;
    let f = zbus::blocking::fdo::ObjectManagerProxy::new(&conn, "xyz.ljones.Asusd", "/")?;
    let interfaces = f.get_managed_objects()?;
    let mut paths = Vec::new();
    for v in interfaces.iter() {
        for k in v.1.keys() {
            if k.as_str() == iface_name {
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

    Err(format!("Did not find {iface_name}").into())
}
