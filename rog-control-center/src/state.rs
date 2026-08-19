//! AppState, contains all the datas and the Events

use crate::ui::helpers::types::{BatteryInfo, SystemTelemetry};

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
    UserRequestedPanelOD(bool),
    UserRequestedBootSound(bool),
    UserRequestedScreenAutoBrightness(bool),
    UserRequestedMCUPowerSave(bool),

    // System User Action Per Profile
    UserRequestedBatteryLimit(u8),

    // Settings
    UserToggledTray(bool),

    // Window Management
    ToggleWindow,
    ShowWindow,
    HideWindow,
    Quit,
}

#[derive(Debug, Clone)]
pub enum Action {
    // System/Home Page
    SetPlatformProfile(i32),
    SetPanelOD(bool),
    SetBootSound(bool),
    SetScreenAutoBrightness(bool),
    SetMCUPowerSave(bool),

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

    // Window Management
    ToggleWindow,
    ShowWindow,
    HideWindow,
    Quit,
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
        }

        (actions, ui_updates)
    }
}
