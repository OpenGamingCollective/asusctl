use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zbus::fdo;
use zbus::proxy;
use zbus::zvariant::{OwnedFd, Type};

#[derive(Debug, PartialEq, Eq, Copy, Clone, Type)]
#[zvariant(signature = "s")]
pub enum InhibitType {
    Shutdown,
    Sleep,
    Idle,
    HandlePowerKey,
    HandleSuspendKey,
    HandleHibernateKey,
    HandleLidSwitch,
}

impl InhibitType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Shutdown => "shutdown",
            Self::Sleep => "sleep",
            Self::Idle => "idle",
            Self::HandlePowerKey => "handle-power-key",
            Self::HandleSuspendKey => "handle-suspend-key",
            Self::HandleHibernateKey => "handle-hibernate-key",
            Self::HandleLidSwitch => "handle-lid-switch",
        }
    }
}

impl From<InhibitType> for &'static str {
    fn from(value: InhibitType) -> Self {
        value.as_str()
    }
}

impl FromStr for InhibitType {
    type Err = fdo::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "shutdown" => Ok(Self::Shutdown),
            "sleep" => Ok(Self::Sleep),
            "idle" => Ok(Self::Idle),
            "handle-power-key" => Ok(Self::HandlePowerKey),
            "handle-suspend-key" => Ok(Self::HandleSuspendKey),
            "handle-hibernate-key" => Ok(Self::HandleHibernateKey),
            "handle-lid-switch" => Ok(Self::HandleLidSwitch),
            other => Err(fdo::Error::IOError(format!(
                "{other} is an invalid InhibitType"
            ))),
        }
    }
}

impl fmt::Display for InhibitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for InhibitType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for InhibitType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
pub trait Manager {
    fn inhibit(&self, what: InhibitType, who: &str, why: &str, mode: &str)
    -> zbus::Result<OwnedFd>;

    #[zbus(property)]
    fn lid_closed(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn on_external_power(&self) -> zbus::Result<bool>;

    #[zbus(signal)]
    fn prepare_for_shutdown(&self, start: bool) -> zbus::Result<()>;

    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}
