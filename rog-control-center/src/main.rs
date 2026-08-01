use std::env;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;

use config_traits::{StdConfig, StdConfigLoad1};
use dmi_id::DMIID;
use log::{debug, error, info, warn, LevelFilter};
use rog_control_center::cli_options::CliStart;
use rog_control_center::config::Config;
use rog_control_center::error::{Error, Result};
use rog_control_center::notify::start_notifications;
use rog_control_center::print_versions;
use rog_control_center::shortcuts::EnableMode;
use rog_control_center::slint::ComponentHandle;
use rog_control_center::tray::init_tray;
use rog_control_center::ui::setup_window;
use rog_control_center::window::{WindowCommand, WindowController};
use rog_control_center::zbus_proxies::{
    AppState, ROGCCZbus, ROGCCZbusProxyBlocking, ZBUS_IFACE, ZBUS_PATH,
};
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    // NOTE on the `unsafe` env writes below: this is a plain synchronous main,
    // not `#[tokio::main]`. No Tokio runtime (and therefore no worker threads)
    // exists yet — the multi-thread runtime is only built later via
    // `Runtime::new` + `block_on`. With a single thread there is no concurrent
    // getenv that could race these setenv calls, which is what makes them sound.
    // That guarantee does NOT hold under `#[tokio::main]`, where the runtime and
    // its threads are created before the function body runs.

    // Ensure tracing spans are quiet by default unless user overrides
    if std::env::var_os("RUST_LOG").is_none() {
        // SAFETY: single-threaded, no runtime yet (see note above).
        unsafe {
            std::env::set_var("RUST_LOG", "warn,tracing=error,zbus=error");
        }
    }
    let mut logger = env_logger::Builder::new();
    logger
        .parse_default_env()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .target(env_logger::Target::Stderr)
        .format_timestamp(None)
        .init();

    let cli_parsed: CliStart = argh::from_env();

    if cli_parsed.version {
        print_versions();
        return Ok(());
    }

    // If we're running under gamescope we have to set WAYLAND_DISPLAY for winit to
    // use
    if let Ok(gamescope) = env::var("GAMESCOPE_WAYLAND_DISPLAY") {
        debug!("Gamescope detected");
        if !gamescope.is_empty() {
            debug!("Setting WAYLAND_DISPLAY to {}", gamescope);
            // SAFETY: This runs before any threads are spawned (before Runtime::new),
            // so no concurrent getenv can race with this setenv.
            unsafe {
                env::set_var("WAYLAND_DISPLAY", gamescope);
            }
        }
        // gamescope-0
        else if let Ok(wayland) = env::var("WAYLAND_DISPLAY") {
            debug!("Wayland display detected");
            if wayland.is_empty() {
                debug!("Setting WAYLAND_DISPLAY to gamescope-0");
                // SAFETY: This runs before any threads are spawned (before Runtime::new),
                // so no concurrent getenv can race with this setenv.
                unsafe {
                    env::set_var("WAYLAND_DISPLAY", "gamescope-0");
                }
            }
        }
    }

    // Single-instance guard. Skipped when this binary re-spawned itself for a
    // "Reload Window" — the child carries --no-single-instance so it doesn't
    // see the still-registered parent and exit; the parent quits right after
    // spawning, freeing the name for future reloads.
    let skip_single_instance = cli_parsed.no_single_instance;
    if !skip_single_instance {
        // Try to open a proxy and check for app state first
        let user_con = zbus::blocking::Connection::session()?;
        if let Ok(proxy) = ROGCCZbusProxyBlocking::new(&user_con) {
            if let Ok(state) = proxy.state() {
                info!("App is already running: {state:?}, opening the window");
                // if there is a proxy connection assume the app is already running
                proxy.set_state(AppState::MainWindowShouldOpen)?;
                std::process::exit(0);
            }
        }
    }

    // Apply the configured UI language through env vars before Runtime::new
    // (they're process-global). "Reload Window" restarts the process, which
    // re-reads config.language here so gettext resolves @tr() in the chosen
    // locale at init_translations — no setlocale / unsafe / libc needed.
    let startup_language = Config::new().load().language;
    if !startup_language.is_empty() {
        let locale = format!("{startup_language}.UTF-8");
        // SAFETY: single-threaded, no runtime yet (see note at the top of main).
        unsafe {
            env::set_var("LANG", &locale);
            env::set_var("LC_ALL", &locale);
            env::set_var("LANGUAGE", &startup_language);
        }
    }

    // start tokio
    let rt = Runtime::new().expect("Unable to create Runtime");

    #[cfg(feature = "tokio-debug")]
    console_subscriber::init();

    // Run the async body on the runtime. Everything below this point executes
    // inside `block_on`, so `tokio::spawn` and `.await` are available. main
    // keeps ownership of `rt` and shuts it down after block_on returns.
    rt.block_on(async_main(&rt, cli_parsed))
}

async fn async_main(rt: &Runtime, cli_parsed: CliStart) -> Result<()> {
    // version checks
    let self_version = env!("CARGO_PKG_VERSION");
    let zbus_con = zbus::blocking::Connection::system()?;
    let platform_proxy = rog_dbus::zbus_platform::PlatformProxyBlocking::new(&zbus_con)?;
    let asusd_version = match platform_proxy.version() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Could not get asusd version: {e:?}\nIs asusd.service running?");
            std::process::exit(1);
        }
    };
    if asusd_version != self_version {
        println!("Version mismatch: asusctl = {self_version}, asusd = {asusd_version}");
        // return Ok(());
    }


    let (conn, app_state) = {
        let mut last_err: Option<zbus::Error> = None;
        let mut connection = None;
        let mut shared_state = None;
        for attempt in 0..5u32 {
            let state_zbus = ROGCCZbus::new();
            let cloned = state_zbus.clone_state();

            // Build the connection, catching any registration errors (e.g.
            // NameTaken when the child races the parent during reload).
            let build_result: zbus::Result<_> = async {
                zbus::connection::Builder::session()?
                    .name(ZBUS_IFACE)?
                    .serve_at(ZBUS_PATH, state_zbus)?
                    .build()
                    .await
            }
            .await;

            match build_result {
                Ok(c) => {
                    connection = Some(c);
                    shared_state = Some(cloned);
                    break;
                }
                Err(e) => {
                    warn!(
                        "D-Bus name registration attempt {} failed: {e}",
                        attempt + 1
                    );
                    last_err = Some(e);
                    if attempt < 4 {
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }
                }
            }
        }
        match (connection, shared_state) {
            (Some(c), Some(s)) => (c, s),
            _ => {
                // Unreachable in practice — the loop always sets last_err on
                // failure — but avoid a panicking expect: fall back to a plain
                // error variant if the invariant ever breaks.
                return Err(
                    last_err.map(Error::from).unwrap_or(Error::DbusConnectionFailed),
                )
            }
        }
    };

    let dmi = DMIID::new().unwrap_or_default();
    let board_name = dmi.board_name;
    let prod_family = dmi.product_family;
    info!("Running on {board_name}, product: {prod_family}");

    let supported_properties = platform_proxy.supported_properties().unwrap_or_default();

    // Startup
    let mut config = Config::new().load();
    if cli_parsed.fullscreen {
        config.start_fullscreen = true;
        if cli_parsed.width_fullscreen != 0 {
            config.fullscreen_width = cli_parsed.width_fullscreen;
        }
        if cli_parsed.height_fullscreen != 0 {
            config.fullscreen_height = cli_parsed.height_fullscreen;
        }
    } else if cli_parsed.windowed {
        config.start_fullscreen = false;
    }

    let is_rog_ally = {
        #[cfg(feature = "rog_ally")]
        {
            board_name == "RC71L" || board_name == "RC72L" || prod_family == "ROG Ally"
        }
        #[cfg(not(feature = "rog_ally"))]
        {
            false
        }
    };

    #[cfg(feature = "rog_ally")]
    if is_rog_ally {
        config.notifications.enabled = false;
        config.enable_tray_icon = false;
        config.run_in_background = false;
        config.startup_in_background = false;
        config.start_fullscreen = true;
        config.enable_autostart = false;
        config.enable_global_shortcut = false;
    }

    config.write();

    let enable_tray_icon = config.enable_tray_icon;
    let startup_in_background = if cli_parsed.autostart {
        cli_parsed.background
    } else if std::env::var_os("ROGCC_RELOAD_SHOW_WINDOW").is_some() {
        // Spawned by "Reload Window": the user expects the window to appear
        // even if the config normally starts hidden in the background.
        false
    } else {
        cli_parsed.background || config.startup_in_background
    };
    let config = Arc::new(Mutex::new(config));

    // The LANG/LC_ALL env was applied before Runtime::new (see above);
    // gettext picks them up at init_translations below.

    // GPU power status channel: written by the dGPU status monitor in
    // notify.rs, read by the tray to color its icon
    let (gpu_status_tx, gpu_status_rx) =
        tokio::sync::watch::channel(rog_platform::gpu_pci::get_gpu_power_status());

    start_notifications(config.clone(), rt, gpu_status_tx)?;

    if !startup_in_background {
        if let Ok(mut app_state) = app_state.lock() {
            *app_state = AppState::MainWindowShouldOpen;
        }
    }

    if std::env::var("RUST_TRANSLATIONS").is_ok() {
        log::debug!("Using build-time translations from OUT_DIR");
        slint::init_translations!(env!("ROGCC_TRANSLATIONS_DIR"));
    } else {
        log::debug!("Using system-installed translations");
        slint::init_translations!("/usr/share/locale/");
    }

    // Prefetch supported Aura modes once at startup and move into the
    // spawned UI thread so the UI uses a stable, immutable list.
    let prefetched_supported: std::sync::Arc<Option<Vec<i32>>> = std::sync::Arc::new(
        rog_control_center::ui::setup_aura::prefetch_supported_basic_modes().await,
    );

    let window = WindowController::new(
        config.clone(),
        prefetched_supported.clone(),
        app_state.clone(),
    );

    let shortcut_service = if is_rog_ally {
        None
    } else {
        let rt_handle = rt.handle();
        let service =
            rog_control_center::shortcuts::start(rt_handle, conn.clone(), window.clone());
        let handle = service.handle();
        window.set_shortcuts(handle.clone());
        if config.lock().is_ok_and(|c| c.enable_global_shortcut) {
            let restore = handle.clone();
            rt.spawn(async move {
                restore.enable(EnableMode::Restore).await;
            });
        }
        Some(service)
    };

    if enable_tray_icon {
        init_tray(
            supported_properties,
            config.clone(),
            window.clone(),
            gpu_status_rx,
        );
    }

    let shortcuts = shortcut_service.as_ref().map(|service| service.handle());
    thread::spawn(move || {
        let mut state = AppState::StartingUp;
        loop {
            if is_rog_ally {
                let config_copy_2 = config.clone();
                let newui = setup_window(
                    config.clone(),
                    prefetched_supported.clone(),
                    app_state.clone(),
                    None,
                );
                newui.window().on_close_requested(move || {
                    exit(0);
                });

                let ui_copy = newui.as_weak();
                newui
                    .window()
                    .set_rendering_notifier(move |s, _| {
                        if let slint::RenderingState::BeforeRendering = s {
                            let config = config_copy_2.clone();
                            ui_copy
                                .upgrade_in_event_loop(move |w| {
                                    let fullscreen =
                                        config.lock().is_ok_and(|c| c.start_fullscreen);
                                    if fullscreen && !w.window().is_fullscreen() {
                                        w.window().set_fullscreen(fullscreen);
                                    }
                                })
                                .ok();
                        }
                    })
                    .ok();

                continue;
            }

            // save as a var, don't hold the lock the entire time or deadlocks happen
            if let Ok(app_state) = app_state.lock() {
                state = *app_state;
            }

            // This sleep is required to give the event loop time to react
            sleep(Duration::from_millis(300));
            if state == AppState::MainWindowShouldOpen {
                window.request(WindowCommand::Show);
            } else if state == AppState::QuitApp {
                window.request(WindowCommand::Quit);
                break;
            } else if state != AppState::MainWindowOpen {
                if let Ok(config) = config.lock() {
                    let shortcut_alive = shortcuts
                        .as_ref()
                        .is_some_and(|s| s.status().keeps_alive(config.enable_global_shortcut));
                    if !config.run_in_background && !shortcut_alive {
                        window.request(WindowCommand::Quit);
                        break;
                    }
                }
            }
        }
    });

    if let Err(e) = slint::run_event_loop_until_quit() {
        error!("Slint event loop error: {e:?}");
    }
    // Shut the shortcut service down only after the Slint loop (and its portal
    // session) has fully stopped. The runtime itself is dropped in main once
    // block_on returns.
    if let Some(service) = shortcut_service {
        service.shutdown().await;
    }
    Ok(())
}
