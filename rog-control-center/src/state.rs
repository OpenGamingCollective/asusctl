//! AppState, contains all the datas and the Events

use crate::{
    AttrBool, AttrMinMax,
    helpers::types::{BatteryInfo, SystemTelemetry},
};
use log::info;
use rog_platform::asus_armoury::FirmwareAttribute;
#[derive(Debug, Clone)]
pub enum Event {
    // Hardware update events
    BatteryUpdated(BatteryInfo),
    TelemetryUpdated(SystemTelemetry),
    DmiLoaded(String),

    // Dbus signals
    PlatformProfileSignalled(i32),

    // asusd reachability: false → grey-out + permanent toast
    AsusdState(bool),
    // User pressed the Retry button
    RetryAsusd,

    // System User Action
    UserRequestedPowerProfile(i32),
    UserRequestedPanelOD(AttrBool),
    UserRequestedBootSound(AttrBool),
    UserRequestedScreenAutoBrightness(AttrBool),
    UserRequestedMCUPowerSave(AttrBool),

    UpdatedPowerProfile(i32),
    // asus-armoury update
    UpdatedApuMem(AttrMinMax),
    UpdatedCorePerf(AttrMinMax),
    UpdatedCoreEff(AttrMinMax),
    UpdatedPptPl1Spl(AttrMinMax),
    UpdatedPptPl2Sppt(AttrMinMax),
    UpdatedPptPl3Fppt(AttrMinMax),
    UpdatedPptFppt(AttrMinMax),
    UpdatedPptApuSppt(AttrMinMax),
    UpdatedPptPlatformSppt(AttrMinMax),
    UpdatedNvDynamicBoost(AttrMinMax),
    UpdatedNvTempTarget(AttrMinMax),
    UpdatedDgpuBaseTgp(AttrMinMax),
    UpdatedDgpuTgp(AttrMinMax),
    UpdatedChargeMode(AttrMinMax),
    UpdatedBootSound(AttrBool),
    UpdatedMCUPowerSave(AttrBool),
    UpdatedPanelOD(AttrBool),
    UpdatedPanelHdMode(AttrMinMax),
    UpdatedEgpuConnected(AttrBool),
    UpdatedEgpuEnable(AttrBool),
    UpdatedDgpuDisable(AttrBool),
    UpdatedGpuMuxMode(AttrBool),
    UpdatedMiniLedMode(AttrMinMax),
    UpdatedPendingRebbot(AttrBool),
    // 30
    UpdatedScreenAutoBrightness(AttrBool),

    // System User Action Per Profile
    UserRequestedBatteryLimit(u8),

    // Settings
    UserToggledTray(bool),

    // Window Management
    ToggleWindow,
    ShowWindow,
    HideWindow,
    Quit,
    // Nothing
    None,
}

impl Event {
    /// Convert an asus-armoury firmware attribute into an event
    pub fn firmware_attr_into_event(attr: &FirmwareAttribute, val: AttrMinMax) -> Event {
        match attr {
            FirmwareAttribute::ApuMem => Event::UpdatedApuMem(val),
            FirmwareAttribute::CoresPerformance => Event::UpdatedCorePerf(val),
            FirmwareAttribute::CoresEfficiency => Event::UpdatedCoreEff(val),
            FirmwareAttribute::PptPl1Spl => Event::UpdatedPptPl1Spl(val),
            FirmwareAttribute::PptPl2Sppt => Event::UpdatedPptPl2Sppt(val),
            FirmwareAttribute::PptPl3Fppt => Event::UpdatedPptPl3Fppt(val),
            FirmwareAttribute::PptFppt => Event::UpdatedPptFppt(val),
            FirmwareAttribute::PptApuSppt => Event::UpdatedPptApuSppt(val),
            FirmwareAttribute::PptPlatformSppt => Event::UpdatedPptPlatformSppt(val),
            FirmwareAttribute::NvDynamicBoost => Event::UpdatedNvDynamicBoost(val),
            FirmwareAttribute::NvTempTarget => Event::UpdatedNvTempTarget(val),
            FirmwareAttribute::DgpuBaseTgp => Event::UpdatedDgpuBaseTgp(val),
            FirmwareAttribute::DgpuTgp => Event::UpdatedDgpuTgp(val),
            FirmwareAttribute::ChargeMode => Event::UpdatedChargeMode(val),
            FirmwareAttribute::BootSound => Event::UpdatedBootSound(attr_i32_into_bool(val)),

            FirmwareAttribute::McuPowersave => Event::UpdatedMCUPowerSave(attr_i32_into_bool(val)),
            FirmwareAttribute::PanelOverdrive => Event::UpdatedPanelOD(attr_i32_into_bool(val)),

            FirmwareAttribute::PanelHdMode => Event::UpdatedPanelHdMode(val),
            FirmwareAttribute::EgpuConnected => {
                Event::UpdatedEgpuConnected(attr_i32_into_bool(val))
            }
            FirmwareAttribute::EgpuEnable => Event::UpdatedEgpuEnable(attr_i32_into_bool(val)),
            FirmwareAttribute::DgpuDisable => Event::UpdatedDgpuDisable(attr_i32_into_bool(val)),
            FirmwareAttribute::GpuMuxMode => Event::UpdatedGpuMuxMode(attr_i32_into_bool(val)),
            FirmwareAttribute::MiniLedMode => Event::UpdatedMiniLedMode(val),
            FirmwareAttribute::PendingReboot => {
                Event::UpdatedPendingRebbot(attr_i32_into_bool(val))
            }
            FirmwareAttribute::ScreenAutoBrightness => {
                Event::UpdatedScreenAutoBrightness(attr_i32_into_bool(val))
            }
            // Unknown
            FirmwareAttribute::None => Event::None,
        }
    }
}

pub fn attr_i32_into_bool(val: AttrMinMax) -> AttrBool {
    AttrBool {
        current: val.current == 1.0,
        supported: val.supported,
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    // System/Home Page
    SetPlatformProfile(i32),
    SetPanelOD(AttrBool),
    SetBootSound(AttrBool),
    SetScreenAutoBrightness(AttrBool),
    SetMCUPowerSave(AttrBool),

    SetBatteryLimit(u8),

    // Re-probe asusd
    RetryAsusd,

    // Settings
    SetTray(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
    Info = 0,
    Warn = 1,
    Error = 2,
}

#[derive(Debug, Clone)]
pub enum UiUpdate {
    Telemetry(SystemTelemetry),
    Battery(BatteryInfo),
    ProductName(String),
    PlatformProfile(i32),
    ShowToast {
        message: String,
        toast_type: ToastType,
    },
    // asusd gone/back
    AsusdState(bool),
    // Non-closable toast
    ShowPermanentToast {
        message: String,
        toast_type: ToastType,
    },

    // Home
    BootSound(AttrBool),
    PanelOD(AttrBool),
    ScreenAutoBrightness(bool),
    MCUPowerSave(bool),

    // Window Management
    ToggleWindow,
    ShowWindow,
    HideWindow,
    Quit,

    // Unknown attribute / no event
    None,
}

#[derive(Default)]
pub struct AppState {
    pub asusd_running: bool,
    pub battery: Option<BatteryInfo>,
    pub telemetry: SystemTelemetry,
    pub product_name: String,
    pub active_profile: i32,
}
impl AppState {
    pub fn new() -> Self {
        // Assume asusd is running, changed at boot
        Self {
            asusd_running: true,
            ..Self::default()
        }
    }

    /// Takes an event, update the state if needed and return what should happen
    pub fn update(&mut self, event: Event) -> (Vec<Action>, Vec<UiUpdate>) {
        let mut actions: Vec<Action> = Vec::new();
        let mut ui_updates: Vec<UiUpdate> = Vec::new();

        match event {
            Event::BatteryUpdated(new_battery) => {
                // Only update if the values changed
                if self.battery.as_ref() != Some(&new_battery) {
                    self.battery = Some(new_battery.clone());
                    ui_updates.push(UiUpdate::Battery(new_battery));
                }
            }
            Event::TelemetryUpdated(new_telemetry) => {
                if self.telemetry != new_telemetry {
                    self.telemetry = new_telemetry.clone();
                    ui_updates.push(UiUpdate::Telemetry(new_telemetry));
                }
            }
            Event::DmiLoaded(new_product_name) => {
                self.product_name = new_product_name.clone();
                ui_updates.push(UiUpdate::ProductName(new_product_name));
            }
            Event::PlatformProfileSignalled(new_profile) => {
                if self.active_profile != new_profile {
                    self.active_profile = new_profile;
                    ui_updates.push(UiUpdate::PlatformProfile(new_profile));
                }
            }
            Event::AsusdState(running) => {
                // Only react on a transition, error toast once when it goes down
                if self.asusd_running != running {
                    self.asusd_running = running;
                    ui_updates.push(UiUpdate::AsusdState(running));
                    if !running {
                        ui_updates.push(UiUpdate::ShowPermanentToast {
                            message: "asusd is not running, please retry again".to_string(),
                            toast_type: ToastType::Error,
                        });
                    }
                }
            }
            Event::RetryAsusd => {
                actions.push(Action::RetryAsusd);
            }

            // System User Action
            Event::UserRequestedPowerProfile(requested_profile) => {
                actions.push(Action::SetPlatformProfile(requested_profile));
            }

            Event::UserRequestedPanelOD(b) => {
                actions.push(Action::SetPanelOD(b));
            }

            Event::UserRequestedBootSound(b) => {
                actions.push(Action::SetBootSound(b));
            }

            Event::UserRequestedScreenAutoBrightness(b) => {
                actions.push(Action::SetScreenAutoBrightness(b));
            }

            Event::UserRequestedMCUPowerSave(b) => {
                actions.push(Action::SetMCUPowerSave(b));
            }

            Event::UserRequestedBatteryLimit(requested_limit) => {
                actions.push(Action::SetBatteryLimit(requested_limit));
            }
            Event::UpdatedBootSound(b) => {
                ui_updates.push(UiUpdate::BootSound(b));
            }
            Event::UpdatedPanelOD(b) => {
                ui_updates.push(UiUpdate::PanelOD(b));
            }
            // Config
            Event::UserToggledTray(b) => {
                actions.push(Action::SetTray(b));
            }
            // Window Management
            Event::ToggleWindow => ui_updates.push(UiUpdate::ToggleWindow),
            Event::ShowWindow => ui_updates.push(UiUpdate::ShowWindow),
            Event::HideWindow => ui_updates.push(UiUpdate::HideWindow),
            Event::Quit => {
                ui_updates.push(UiUpdate::Quit);
            }
            _ => {
                info!("Event not implemented yet: {:?}", event);
            }
        }

        (actions, ui_updates)
    }
}
