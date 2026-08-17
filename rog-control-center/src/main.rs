use std::env;
use std::sync::{Arc, Mutex};

use config_traits::{StdConfig, StdConfigLoad1};
use dmi_id::DMIID;
use env_logger::Env;
use log::{error, info, warn};

use rog_control_center::cli_options::CliStart;
use rog_control_center::config::Config;
use rog_control_center::error::Result;
use rog_control_center::print_versions;

use rog_control_center::state::Event;
use slint::ComponentHandle;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

fn main() -> Result<()> {
    // Setup Logging
    env_logger::Builder::from_env(
        Env::default().default_filter_or("warn,tracing=error,zbus=error"),
    )
    .format_timestamp(None)
    .init();

    // Parse CLI Arguments
    let cli_parsed: CliStart = argh::from_env();
    if cli_parsed.version {
        print_versions();
        return Ok(());
    }

    // Gamescope Fixes
    if let Ok(gamescope) = env::var("GAMESCOPE_WAYLAND_DISPLAY") {
        if !gamescope.is_empty() {
            unsafe { env::set_var("WAYLAND_DISPLAY", gamescope) };
        } else if let Ok(wayland) = env::var("WAYLAND_DISPLAY") {
            if wayland.is_empty() {
                unsafe { env::set_var("WAYLAND_DISPLAY", "gamescope-0") };
            }
        }
    }

    // Start Tokio Runtime
    let rt = Runtime::new().expect("Unable to create Runtime");
    let _enter = rt.enter();

    // Hardware Info (DMI)
    // TODO: Duplicate of helpers/hardware.rs?
    let dmi = DMIID::new().unwrap_or_default();
    info!(
        "Running on {}, product: {}",
        dmi.board_name, dmi.product_family
    );

    // Config Loading
    let mut config = Config::new().load();

    let _is_rog_ally = {
        #[cfg(feature = "rog_ally")]
        {
            dmi.board_name == "RC71L"
                || dmi.board_name == "RC72L"
                || dmi.product_family == "ROG Ally"
        }
        #[cfg(not(feature = "rog_ally"))]
        {
            false
        }
    };

    #[cfg(feature = "rog_ally")]
    if _is_rog_ally {
        config.notifications.enabled = false;
        config.enable_tray_icon = false;
        config.run_in_background = false;
        config.startup_in_background = false;
        config.start_fullscreen = true;
    }

    if cli_parsed.fullscreen {
        config.start_fullscreen = true;
    } else if cli_parsed.windowed {
        config.start_fullscreen = false;
    }

    let config = Arc::new(Mutex::new(config));

    // Load Translations
    if std::env::var("RUST_TRANSLATIONS").is_ok() {
        slint::init_translations!(env!("ROGCC_TRANSLATIONS_DIR"));
    } else {
        slint::init_translations!("/usr/share/locale/");
    }

    // Create the Slint UI Window
    let ui = rog_control_center::ui::window::setup_window(config.clone());

    // Create the Central Event Channel
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<rog_control_center::state::Event>();

    // Send the dmi product name to the UI
    let _ = event_tx.send(Event::DmiLoaded(dmi.product_name.clone()));

    // Start System Tray
    rog_control_center::tray::init_tray(vec![], config.clone(), event_tx.clone());

    // Start Global Shortcuts
    #[cfg(not(feature = "rog_ally"))]
    let _shortcuts = rog_control_center::shortcuts::start(rt.handle(), event_tx.clone());

    // Bind UI Inputs to the Channel
    rog_control_center::ui::callbacks::bind_ui_events(&ui, event_tx.clone());

    // Start Hardware Subscriptions
    rt.spawn(rog_control_center::ui::subscriptions::subscribe_battery(
        event_tx.clone(),
    ));
    // subscribe_telemetry(event_tx.clone(), true);

    // Start Event Loop
    let ui_weak = ui.as_weak();
    rt.spawn(async move {
        let mut state = rog_control_center::state::AppState::new();
        while let Some(event) = event_rx.recv().await {
            let (_effects, ui_updates) = state.update(event);

            // TODO: Effects

            // Apply UI updates
            if !ui_updates.is_empty() {
                let ui_weak_clone = ui_weak.clone();
                if let Err(err) = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak_clone.upgrade() {
                        for update in ui_updates {
                            rog_control_center::ui::update::apply_ui_update(&ui, update);
                        }
                    }
                }) {
                    warn!("Could not dispatch UI update: {:?}", err);
                };
            }
        }
    });

    // Run the Slint GUI Loop (this blocks until the window is closed)
    let close_config = config.clone();
    ui.window().on_close_requested(move || {
        let background = close_config
            .try_lock()
            .map(|c| c.run_in_background && c.enable_tray_icon)
            .unwrap_or(false);
        if background {
            slint::CloseRequestResponse::HideWindow
        } else {
            let _ = slint::quit_event_loop();
            slint::CloseRequestResponse::HideWindow
        }
    });

    if let Err(e) = slint::run_event_loop_until_quit() {
        error!("Slint event loop error: {e:?}");
    }
    drop(_enter);
    rt.shutdown_background();
    Ok(())
}
