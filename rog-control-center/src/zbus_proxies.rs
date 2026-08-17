use log::info;

use zbus::blocking::proxy::ProxyImpl;
use zbus::blocking::{Connection, fdo};

pub const ZBUS_PATH: &str = "/xyz/ljones/rogcc";
pub const ZBUS_IFACE: &str = "xyz.ljones.rogcc";

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
        info!("Multiple asusd interfaces devices found");
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
        info!("Multiple asusd interfaces devices found");
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
