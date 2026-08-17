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

    // User Action
    UserRequestedProfile(i32),
    UserRequestedBatteryLimit(u8),

    // Window Management
    ToggleWindow,
    ShowWindow,
    HideWindow,
    Quit,
}

pub enum Action {
    SetPlatformProfile(i32),
    SetBatteryLimit(u8),
}

pub enum UiUpdate {
    Telemetry(SystemTelemetry),
    Battery(BatteryInfo),
    ProductName(String),
    PlatformProfile(i32),
    ShowToast { message: String, is_error: bool },

    // Window Management
    ToggleWindow,
    ShowWindow,
    HideWindow,
    Quit,
}

#[derive(Default)]
pub struct AppState {
    pub battery: Option<BatteryInfo>,
    pub telemetry: SystemTelemetry,
    pub product_name: String,
    pub active_profile: i32,
}
impl AppState {
    pub fn new() -> Self {
        Self::default()
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
            Event::UserRequestedProfile(requested_profile) => {
                // This one does not directly change the profile
                actions.push(Action::SetPlatformProfile(requested_profile));
            }
            Event::UserRequestedBatteryLimit(requested_limit) => {
                actions.push(Action::SetBatteryLimit(requested_limit));
            }
            Event::ToggleWindow => ui_updates.push(UiUpdate::ToggleWindow),
            Event::ShowWindow => ui_updates.push(UiUpdate::ShowWindow),
            Event::HideWindow => ui_updates.push(UiUpdate::HideWindow),
            Event::Quit => {
                std::process::exit(0);
            }
        }

        (actions, ui_updates)
    }
}
