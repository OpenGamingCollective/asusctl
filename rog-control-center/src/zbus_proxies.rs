use std::collections::HashMap;

use anyhow::Result;
use log::info;
use zbus::blocking::proxy::ProxyImpl;
use zbus::blocking::{Connection, fdo};

use rog_dbus::asus_armoury::AsusArmouryProxy;
use rog_dbus::scsi_aura::ScsiAuraProxy;
use rog_dbus::zbus_anime::AnimeProxy;
use rog_dbus::zbus_aura::AuraProxy;
use rog_dbus::zbus_backlight::BacklightProxy;
use rog_dbus::zbus_fan_curves::FanCurvesProxy;
use rog_dbus::zbus_platform::PlatformProxy;
use rog_dbus::zbus_slash::SlashProxy;
use rog_dbus::zbus_xgm_led::XgmLedProxy;
use rog_platform::asus_armoury::FirmwareAttribute;
use zbus::names::OwnedInterfaceName;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

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

pub struct AsusdInterface {
    pub conn: zbus::Connection,
    // One armory attribute object per firmware attribute
    pub armoury: HashMap<FirmwareAttribute, AsusArmouryProxy<'static>>,
    pub scsi_aura: Option<ScsiAuraProxy<'static>>,
    pub anime: Option<AnimeProxy<'static>>,
    pub aura: Option<AuraProxy<'static>>,
    pub backlight: Option<BacklightProxy<'static>>,
    pub fan_curves: Option<FanCurvesProxy<'static>>,
    pub platform: Option<PlatformProxy<'static>>,
    pub slash: Option<SlashProxy<'static>>,
    pub xgm_led: Option<XgmLedProxy<'static>>,
}
impl AsusdInterface {
    pub async fn build() -> Result<Self> {
        let conn = zbus::Connection::system().await?;

        let proxy = zbus::fdo::ObjectManagerProxy::new(&conn, "xyz.ljones.Asusd", "/").await?;

        let objects: HashMap<
            OwnedObjectPath,
            HashMap<OwnedInterfaceName, HashMap<String, OwnedValue>>,
        > = proxy.get_managed_objects().await?;

        let mut armoury: HashMap<FirmwareAttribute, AsusArmouryProxy<'static>> = HashMap::new();
        let mut scsi_aura: Option<ScsiAuraProxy<'static>> = None;
        let mut anime: Option<AnimeProxy<'static>> = None;
        let mut aura: Option<AuraProxy<'static>> = None;
        let mut backlight: Option<BacklightProxy<'static>> = None;
        let mut fan_curves: Option<FanCurvesProxy<'static>> = None;
        let mut platform: Option<PlatformProxy<'static>> = None;
        let mut slash: Option<SlashProxy<'static>> = None;
        let mut xgm_led: Option<XgmLedProxy<'static>> = None;

        for (path, interfaces) in objects {
            for iface in interfaces.keys() {
                match iface.as_str() {
                    "xyz.ljones.Platform" => {
                        platform = Some(
                            PlatformProxy::builder(&conn)
                                .path(path.clone())?
                                .destination("xyz.ljones.Asusd")?
                                .build()
                                .await?,
                        );
                    }
                    "xyz.ljones.FanCurves" => {
                        fan_curves = Some(
                            FanCurvesProxy::builder(&conn)
                                .path(path.clone())?
                                .destination("xyz.ljones.Asusd")?
                                .build()
                                .await?,
                        );
                    }
                    "xyz.ljones.Backlight" => {
                        backlight = Some(
                            BacklightProxy::builder(&conn)
                                .path(path.clone())?
                                .destination("xyz.ljones.Asusd")?
                                .build()
                                .await?,
                        );
                    }
                    "xyz.ljones.Slash" => {
                        slash = Some(
                            SlashProxy::builder(&conn)
                                .path(path.clone())?
                                .destination("xyz.ljones.Asusd")?
                                .build()
                                .await?,
                        );
                    }
                    "xyz.ljones.XgmLed" => {
                        xgm_led = Some(
                            XgmLedProxy::builder(&conn)
                                .path(path.clone())?
                                .destination("xyz.ljones.Asusd")?
                                .build()
                                .await?,
                        );
                    }
                    "xyz.ljones.ScsiAura" => {
                        scsi_aura = Some(
                            ScsiAuraProxy::builder(&conn)
                                .path(path.clone())?
                                .destination("xyz.ljones.Asusd")?
                                .build()
                                .await?,
                        );
                    }
                    "xyz.ljones.Aura" => {
                        aura = Some(
                            AuraProxy::builder(&conn)
                                .path(path.clone())?
                                .destination("xyz.ljones.Asusd")?
                                .build()
                                .await?,
                        );
                    }
                    "xyz.ljones.Anime" => {
                        anime = Some(
                            AnimeProxy::builder(&conn)
                                .path(path.clone())?
                                .destination("xyz.ljones.Asusd")?
                                .build()
                                .await?,
                        );
                    }
                    "xyz.ljones.AsusArmoury" => {
                        // One attribute object per firmware attribute
                        let leaf = path.as_str().rsplit('/').next().unwrap_or("");
                        let attr = FirmwareAttribute::from(leaf);
                        if attr != FirmwareAttribute::None {
                            armoury.insert(
                                attr,
                                AsusArmouryProxy::builder(&conn)
                                    .path(path.clone())?
                                    .destination("xyz.ljones.Asusd")?
                                    .build()
                                    .await?,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            conn,
            armoury,
            scsi_aura,
            anime,
            aura,
            backlight,
            fan_curves,
            platform,
            slash,
            xgm_led,
        })
    }

    /// True when at least one asusd interface was discovered
    pub fn present(&self) -> bool {
        self.platform.is_some()
            || self.fan_curves.is_some()
            || self.backlight.is_some()
            || self.slash.is_some()
            || self.xgm_led.is_some()
            || self.scsi_aura.is_some()
            || self.aura.is_some()
            || self.anime.is_some()
            || !self.armoury.is_empty()
    }

    /// Proxy for a specific armory firmware attribute, if the board exposes it
    pub fn attribute(&self, attr: FirmwareAttribute) -> Option<&AsusArmouryProxy<'static>> {
        self.armoury.get(&attr)
    }
}
