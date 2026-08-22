pub use asusd::{DBUS_IFACE, DBUS_NAME, DBUS_PATH};
use log::warn;
use std::sync::OnceLock;
use tokio::sync::OnceCell;
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

pub use asus_armoury::{AsusArmouryProxy, AsusArmouryProxyBlocking};
pub use scsi_aura::{ScsiAuraProxy, ScsiAuraProxyBlocking};
pub use zbus_anime::{AnimeProxy, AnimeProxyBlocking};
pub use zbus_aura::{AuraProxy, AuraProxyBlocking};
pub use zbus_backlight::{BacklightProxy, BacklightProxyBlocking};
pub use zbus_fan_curves::{FanCurvesProxy, FanCurvesProxyBlocking};
pub use zbus_platform::{PlatformProxy, PlatformProxyBlocking};
pub use zbus_slash::{SlashProxy, SlashProxyBlocking};
pub use zbus_xgm_led::{XgmLedProxy, XgmLedProxyBlocking};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

static BLOCKING_CONN: OnceLock<zbus::blocking::Connection> = OnceLock::new();
static ASYNC_CONN: OnceCell<zbus::Connection> = OnceCell::const_new();

/// Return a shared process-wide blocking D-Bus system connection.
///
/// Note: The initial `get()` plus `get_or_init()` may briefly create two connections
/// under a race condition, with the losing connection dropped intentionally because
/// `OnceLock::get_or_try_init` is unavailable.
pub fn system_connection_blocking() -> zbus::Result<&'static zbus::blocking::Connection> {
    if let Some(conn) = BLOCKING_CONN.get() {
        return Ok(conn);
    }
    let conn = zbus::blocking::Connection::system()?;
    Ok(BLOCKING_CONN.get_or_init(|| conn))
}

/// Return a shared process-wide async D-Bus system connection.
pub async fn system_connection() -> zbus::Result<&'static zbus::Connection> {
    ASYNC_CONN
        .get_or_try_init(|| async { zbus::Connection::system().await })
        .await
}

/// Compare D-Bus object paths, sorting trailing numeric segments by value (e.g. `/.../0`, `/.../2`, `/.../10`)
/// with standard byte-order comparison as the fallback.
fn cmp_object_paths(
    a: &zbus::zvariant::OwnedObjectPath,
    b: &zbus::zvariant::OwnedObjectPath,
) -> std::cmp::Ordering {
    let a_str = a.as_str();
    let b_str = b.as_str();
    let (prefix_a, seg_a) = a_str.rsplit_once('/').unwrap_or(("", a_str));
    let (prefix_b, seg_b) = b_str.rsplit_once('/').unwrap_or(("", b_str));
    match prefix_a.cmp(prefix_b) {
        std::cmp::Ordering::Equal => {
            if let (Ok(num_a), Ok(num_b)) = (seg_a.parse::<u64>(), seg_b.parse::<u64>()) {
                num_a.cmp(&num_b)
            } else {
                seg_a.cmp(seg_b)
            }
        }
        other => other,
    }
}

pub fn list_iface_blocking() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let conn = system_connection_blocking()?;
    let f = zbus::blocking::fdo::ObjectManagerProxy::new(conn, "xyz.ljones.Asusd", "/")?;
    let interfaces = f.get_managed_objects()?;
    let mut ifaces = Vec::new();
    for ifaces_map in interfaces.values() {
        for k in ifaces_map.keys() {
            ifaces.push(k.to_string());
        }
    }
    ifaces.sort();
    ifaces.dedup();
    Ok(ifaces)
}

pub async fn find_iface_async<T>(iface_name: &str) -> Result<Vec<T>, Box<dyn std::error::Error>>
where
    T: ProxyImpl<'static> + From<zbus::Proxy<'static>>,
{
    let conn = system_connection().await?;
    find_iface_async_with_conn(conn, iface_name).await
}

pub async fn find_iface_async_with_conn<T>(
    conn: &zbus::Connection,
    iface_name: &str,
) -> Result<Vec<T>, Box<dyn std::error::Error>>
where
    T: ProxyImpl<'static> + From<zbus::Proxy<'static>>,
{
    let f = zbus::fdo::ObjectManagerProxy::new(conn, "xyz.ljones.Asusd", "/").await?;
    let interfaces = f.get_managed_objects().await?;
    let mut paths: Vec<_> = interfaces
        .iter()
        .filter(|(_, ifaces)| ifaces.contains_key(iface_name))
        .map(|(path, _)| path.clone())
        .collect();

    if paths.is_empty() {
        return Err(format!("Did not find {iface_name}").into());
    }

    if paths.len() > 1 {
        warn!("Multiple asusd interfaces devices found for {iface_name}");
    }

    paths.sort_by(cmp_object_paths);
    let mut ctrl = Vec::with_capacity(paths.len());
    for path in paths {
        ctrl.push(
            T::builder(conn)
                .path(path)?
                .destination("xyz.ljones.Asusd")?
                .build()
                .await?,
        );
    }
    Ok(ctrl)
}

pub fn find_iface_blocking<T>(iface_name: &str) -> Result<Vec<T>, Box<dyn std::error::Error>>
where
    T: zbus::blocking::proxy::ProxyImpl<'static> + From<zbus::Proxy<'static>>,
{
    let conn = system_connection_blocking()?;
    find_iface_blocking_with_conn(conn, iface_name)
}

pub fn find_iface_blocking_with_conn<T>(
    conn: &zbus::blocking::Connection,
    iface_name: &str,
) -> Result<Vec<T>, Box<dyn std::error::Error>>
where
    T: zbus::blocking::proxy::ProxyImpl<'static> + From<zbus::Proxy<'static>>,
{
    let f = zbus::blocking::fdo::ObjectManagerProxy::new(conn, "xyz.ljones.Asusd", "/")?;
    let interfaces = f.get_managed_objects()?;
    let mut paths: Vec<_> = interfaces
        .iter()
        .filter(|(_, ifaces)| ifaces.contains_key(iface_name))
        .map(|(path, _)| path.clone())
        .collect();

    if paths.is_empty() {
        return Err(format!("Did not find {iface_name}").into());
    }

    if paths.len() > 1 {
        warn!("Multiple asusd interfaces devices found for {iface_name}");
    }

    paths.sort_by(cmp_object_paths);
    let mut ctrl = Vec::with_capacity(paths.len());
    for path in paths {
        ctrl.push(
            T::builder(conn)
                .path(path)?
                .destination("xyz.ljones.Asusd")?
                .build()?,
        );
    }
    Ok(ctrl)
}

/// Obtain a PlatformProxy instance using the shared async system connection.
pub async fn platform_proxy() -> zbus::Result<PlatformProxy<'static>> {
    let conn = system_connection().await?;
    PlatformProxy::new(conn).await
}

/// Obtain a PlatformProxyBlocking instance using the shared blocking system connection.
pub fn platform_proxy_blocking() -> zbus::Result<PlatformProxyBlocking<'static>> {
    let conn = system_connection_blocking()?;
    PlatformProxyBlocking::new(conn)
}

/// Obtain a FanCurvesProxy instance using the shared async system connection.
pub async fn fan_curves_proxy() -> zbus::Result<FanCurvesProxy<'static>> {
    let conn = system_connection().await?;
    FanCurvesProxy::new(conn).await
}

/// Obtain a FanCurvesProxyBlocking instance using the shared blocking system connection.
pub fn fan_curves_proxy_blocking() -> zbus::Result<FanCurvesProxyBlocking<'static>> {
    let conn = system_connection_blocking()?;
    FanCurvesProxyBlocking::new(conn)
}

/// Obtain a BacklightProxy instance using the shared async system connection.
pub async fn backlight_proxy() -> zbus::Result<BacklightProxy<'static>> {
    let conn = system_connection().await?;
    BacklightProxy::new(conn).await
}

/// Obtain a BacklightProxyBlocking instance using the shared blocking system connection.
pub fn backlight_proxy_blocking() -> zbus::Result<BacklightProxyBlocking<'static>> {
    let conn = system_connection_blocking()?;
    BacklightProxyBlocking::new(conn)
}

/// Discover all AsusArmoury devices (async).
pub async fn find_armoury_proxies()
-> Result<Vec<AsusArmouryProxy<'static>>, Box<dyn std::error::Error>> {
    find_iface_async("xyz.ljones.AsusArmoury").await
}

/// Discover all AsusArmoury devices (blocking).
pub fn find_armoury_proxies_blocking()
-> Result<Vec<AsusArmouryProxyBlocking<'static>>, Box<dyn std::error::Error>> {
    find_iface_blocking("xyz.ljones.AsusArmoury")
}

/// Discover all Aura devices (async).
pub async fn find_aura_proxies() -> Result<Vec<AuraProxy<'static>>, Box<dyn std::error::Error>> {
    find_iface_async("xyz.ljones.Aura").await
}

/// Discover all Aura devices (blocking).
pub fn find_aura_proxies_blocking()
-> Result<Vec<AuraProxyBlocking<'static>>, Box<dyn std::error::Error>> {
    find_iface_blocking("xyz.ljones.Aura")
}

/// Discover all Slash lighting devices (async).
pub async fn find_slash_proxies() -> Result<Vec<SlashProxy<'static>>, Box<dyn std::error::Error>> {
    find_iface_async("xyz.ljones.Slash").await
}

/// Discover all Slash lighting devices (blocking).
pub fn find_slash_proxies_blocking()
-> Result<Vec<SlashProxyBlocking<'static>>, Box<dyn std::error::Error>> {
    find_iface_blocking("xyz.ljones.Slash")
}

/// Discover all AniMe Matrix devices (async).
pub async fn find_anime_proxies() -> Result<Vec<AnimeProxy<'static>>, Box<dyn std::error::Error>> {
    find_iface_async("xyz.ljones.Anime").await
}

/// Discover all AniMe Matrix devices (blocking).
pub fn find_anime_proxies_blocking()
-> Result<Vec<AnimeProxyBlocking<'static>>, Box<dyn std::error::Error>> {
    find_iface_blocking("xyz.ljones.Anime")
}

/// Discover all XG Mobile LED devices (async).
pub async fn find_xgm_led_proxies() -> Result<Vec<XgmLedProxy<'static>>, Box<dyn std::error::Error>>
{
    find_iface_async("xyz.ljones.XgmLed").await
}

/// Discover all XG Mobile LED devices (blocking).
pub fn find_xgm_led_proxies_blocking()
-> Result<Vec<XgmLedProxyBlocking<'static>>, Box<dyn std::error::Error>> {
    find_iface_blocking("xyz.ljones.XgmLed")
}

/// Discover all SCSI Aura devices (async).
pub async fn find_scsi_aura_proxies()
-> Result<Vec<ScsiAuraProxy<'static>>, Box<dyn std::error::Error>> {
    find_iface_async("xyz.ljones.ScsiAura").await
}

/// Discover all SCSI Aura devices (blocking).
pub fn find_scsi_aura_proxies_blocking()
-> Result<Vec<ScsiAuraProxyBlocking<'static>>, Box<dyn std::error::Error>> {
    find_iface_blocking("xyz.ljones.ScsiAura")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmp_object_paths() {
        use zbus::zvariant::OwnedObjectPath;

        let path0: OwnedObjectPath = "/xyz/ljones/Aura/0".try_into().expect("valid object path");
        let path2: OwnedObjectPath = "/xyz/ljones/Aura/2".try_into().expect("valid object path");
        let path10: OwnedObjectPath = "/xyz/ljones/Aura/10".try_into().expect("valid object path");
        let path_slash: OwnedObjectPath = "/xyz/ljones/Aura/slash"
            .try_into()
            .expect("valid object path");
        let path_tuf: OwnedObjectPath = "/xyz/ljones/Aura/tuf"
            .try_into()
            .expect("valid object path");

        let mut paths = vec![
            path10.clone(),
            path_tuf.clone(),
            path0.clone(),
            path_slash.clone(),
            path2.clone(),
        ];
        paths.sort_by(cmp_object_paths);

        assert_eq!(
            paths,
            vec![
                path0, path2, path10, path_slash, path_tuf
            ]
        );
    }

    #[test]
    fn test_system_connection_blocking_singleton() {
        let conn1 = match system_connection_blocking() {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "Skipping test_system_connection_blocking_singleton: system D-Bus unavailable: {e}"
                );
                return;
            }
        };
        let conn2 = system_connection_blocking().expect("second call must succeed");
        assert!(std::ptr::eq(conn1, conn2));
    }

    #[tokio::test]
    async fn test_system_connection_async_singleton() {
        let conn1 = match system_connection().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "Skipping test_system_connection_async_singleton: system D-Bus unavailable: {e}"
                );
                return;
            }
        };
        let conn2 = system_connection()
            .await
            .expect("second async call must succeed");
        assert!(std::ptr::eq(conn1, conn2));
    }
}
