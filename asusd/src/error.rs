use config_traits::ron;
use rog_anime::error::AnimeError;
use rog_platform::error::PlatformError;
use rog_profiles::error::ProfileError;
use rog_slash::error::SlashError;

#[derive(thiserror::Error, Debug)]
pub enum RogError {
    #[error("Parse gfx vendor error")]
    ParseVendor,

    #[error("Parse LED error")]
    ParseLed,

    #[error("Profile does not exist {0}")]
    MissingProfile(String),

    #[error("udev {0}: {1}")]
    Udev(String, #[source] std::io::Error),

    #[error("Path {0}: {1}")]
    Path(String, #[source] std::io::Error),

    #[error("Read {0}: {1}")]
    Read(String, #[source] std::io::Error),

    #[error("Write {0}: {1}")]
    Write(String, #[source] std::io::Error),

    #[error("Not supported")]
    NotSupported,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Task error: {0}")]
    DoTask(String),

    #[error("Missing functionality: {0}")]
    MissingFunction(String),

    #[error(
        "Led node at {0} is missing, please check you have the required patch or dkms \
         module installed: {1}"
    )]
    MissingLedBrightNode(String, #[source] std::io::Error),

    #[error("Reload error: {0}")]
    ReloadFail(String),

    #[error("Profile error: {0}")]
    Profiles(#[source] ProfileError),

    #[error("Initiramfs error: {0}")]
    Initramfs(String),

    #[error("Modprobe error: {0}")]
    Modprobe(String),

    #[error("std::io error: {0}")]
    Io(#[source] std::io::Error),

    #[error("Zbus error: {0}")]
    Zbus(#[source] zbus::Error),

    #[error("Invalid charging limit, not in range 20-100%: {0}")]
    ChargeLimit(u8),

    #[error("Aura effect not supported")]
    AuraEffectNotSupported,

    #[error("No supported Aura keyboard")]
    NoAuraKeyboard,

    #[error("No Aura keyboard node found")]
    NoAuraNode,

    #[error("AniMe Matrix error: {0}")]
    Anime(#[source] AnimeError),

    #[error("Slash error: {0}")]
    Slash(#[source] SlashError),

    #[error("Asus Platform error: {0}")]
    Platform(#[source] PlatformError),

    #[error("systemd unit action {0} failed")]
    SystemdUnitAction(String),

    #[error("Timed out waiting for systemd unit change {0} state")]
    SystemdUnitWaitTimeout(String),

    #[error("Command exec error: {0}: {1}")]
    Command(String, #[source] std::io::Error),

    #[error("Parse config error: {0}")]
    ParseRon(#[source] ron::Error),
}

impl From<ProfileError> for RogError {
    fn from(err: ProfileError) -> Self {
        RogError::Profiles(err)
    }
}

impl From<AnimeError> for RogError {
    fn from(err: AnimeError) -> Self {
        RogError::Anime(err)
    }
}

impl From<SlashError> for RogError {
    fn from(err: SlashError) -> Self {
        RogError::Slash(err)
    }
}

impl From<PlatformError> for RogError {
    fn from(err: PlatformError) -> Self {
        RogError::Platform(err)
    }
}

impl From<zbus::Error> for RogError {
    fn from(err: zbus::Error) -> Self {
        RogError::Zbus(err)
    }
}

impl From<std::io::Error> for RogError {
    fn from(err: std::io::Error) -> Self {
        RogError::Io(err)
    }
}

impl From<ron::Error> for RogError {
    fn from(err: ron::Error) -> Self {
        RogError::ParseRon(err)
    }
}

impl From<RogError> for zbus::fdo::Error {
    fn from(err: RogError) -> Self {
        zbus::fdo::Error::Failed(err.to_string())
    }
}

impl From<RogError> for zbus::Error {
    fn from(err: RogError) -> Self {
        zbus::Error::Failure(err.to_string())
    }
}
