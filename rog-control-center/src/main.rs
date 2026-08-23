use std::env;
use std::sync::{Arc, Mutex, OnceLock};

use config_traits::{StdConfig, StdConfigLoad1};
use dmi_id::DMIID;
use env_logger::Env;
use log::{error, info, warn};

use rog_control_center::cli_options::CliStart;
use rog_control_center::config::Config;
use rog_control_center::print_versions;
use rog_control_center::tray::setup_tray;

use rog_control_center::helpers::startup::populate_slint_properties;
use rog_control_center::helpers::zbus_proxies::AsusdInterface;
use rog_control_center::state::Event;
use rog_control_center::ui::actions::ActionHandler;
use rog_control_center::ui::subscriptions::{
    subscribe_armoury, subscribe_ppd, subscribe_telemetry,
};
use rog_control_center::ui::window::setup_window;
use slint::ComponentHandle;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use anyhow::Result;

fn main() -> Result<()> {
    // Setup Logging
    env_logger::Builder::from_env(
        Env::default().default_filter_or("info,tracing=error,zbus=error"),
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
        } else if let Ok(wayland) = env::var("WAYLAND_DISPLAY")
            && wayland.is_empty()
        {
            unsafe { env::set_var("WAYLAND_DISPLAY", "gamescope-0") };
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
    let ui = setup_window();

    // Create the Central Event Channel
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<rog_control_center::state::Event>();

    // Create System Tray
    let _tray = setup_tray(event_tx.clone());

    // Send the dmi product name to the UI
    let _ = event_tx.send(Event::DmiLoaded(dmi.product_name.clone()));

    // Bind UI Inputs to the Channel
    rog_control_center::ui::callbacks::bind_ui_events(&ui, event_tx.clone());

    // subscribe_telemetry(event_tx.clone(), true);

    // Detect asusd and send the result to the GUI
    let asusd = Arc::new(OnceLock::new());
    let asusd_set = asusd.clone();
    let asusd_tx = event_tx.clone();
    // Block here, we want to wait for the asusd interface connection
    rt.block_on(async move {
        match AsusdInterface::build().await {
            Ok(int) if int.present() => {
                let _ = asusd_set.set(int);
                let _ = asusd_tx.send(Event::AsusdState(true));
            }
            Ok(_) => {
                warn!("asusd reachable but no known interfaces found");
                let _ = asusd_tx.send(Event::AsusdState(false));
            }
            Err(err) => {
                warn!("asusd is not available: {err}");
                let _ = asusd_tx.send(Event::AsusdState(false));
            }
        }
    });

    // Start Hardware Subscriptions
    rt.spawn(subscribe_telemetry(event_tx.clone()));
    rt.spawn(subscribe_ppd(event_tx.clone(), asusd.clone()));
    rt.spawn(subscribe_armoury(event_tx.clone(), asusd.clone()));

    // Show the window after the pre-startup is done
    let background_startup = match config.try_lock() {
        Ok(c) => c.startup_in_background,
        Err(_) => false,
    };

    if !background_startup && let Err(e) = ui.window().show() {
        warn!("Couldn't show main window: {e:?}");
    }

    let mut action_handler = ActionHandler {
        config: config.clone(),
        asusd: asusd.clone(),
        event_tx: event_tx.clone(),
    };
    // Start Event Loop
    let ui_weak = ui.as_weak();
    rt.spawn(async move {
        let mut state = rog_control_center::state::AppState::new();
        // Get the current values from asusd
        populate_slint_properties(ui_weak.clone(), asusd.clone()).await;
        while let Some(event) = event_rx.recv().await {
            let (actions, ui_updates) = state.update(event);

            if !actions.is_empty() {
                for action in actions {
                    action_handler.handle_action(action).await;
                }
            }

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
