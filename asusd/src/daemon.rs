use std::env;
use std::error::Error;
use std::sync::Arc;

use ::zbus::Connection;
use asusd::asus_armoury::{start_attributes_zbus, ArmouryAttributeRegistry};
use asusd::aura_manager::DeviceManager;
use asusd::config::Config;
use asusd::ctrl_backlight::CtrlBacklight;
use asusd::ctrl_fancurves::CtrlFanCurveZbus;
use asusd::ctrl_platform::CtrlPlatform;
use asusd::ctrl_xgm_led::CtrlXgmLed;
use asusd::{print_board_info, start_tasks, CtrlTask, Reloadable, ZbusRun, DBUS_NAME};
use config_traits::{StdConfig, StdConfigLoad2};
use log::{error, info, warn};
use rog_platform::asus_armoury::FirmwareAttributes;
use rog_platform::platform::RogPlatform;
use rog_platform::power::AsusPower;
use tokio::sync::Mutex;
use zbus::fdo::ObjectManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting asusd daemon...");

    // console_subscriber::init();
    let mut logger = env_logger::Builder::new();
    logger
        .parse_default_env()
        .target(env_logger::Target::Stdout)
        .format_timestamp(None)
        .filter_level(log::LevelFilter::Debug)
        .init();

    let is_service = match env::var_os("IS_SERVICE") {
        Some(val) => val == "1",
        None => true,
    };

    if !is_service {
        println!("asusd schould be only run from the right systemd service");
        println!(
            "do not run in your terminal, if you need an logs please use journalctl -b -u asusd"
        );
        println!("asusd will now exit");
        return Ok(());
    }

    info!("       daemon v{}", asusd::VERSION);
    info!("    rog-anime v{}", rog_anime::VERSION);
    info!("    rog-slash v{}", rog_slash::VERSION);
    info!("     rog-aura v{}", rog_aura::VERSION);
    info!(" rog-profiles v{}", rog_profiles::VERSION);
    info!("rog-platform v{}", rog_platform::VERSION);

    start_daemon().await?;
    Ok(())
}

/// The actual main loop for the daemon
async fn start_daemon() -> Result<(), Box<dyn Error>> {
    // let supported = SupportedFunctions::get_supported();
    print_board_info();
    // println!("{:?}", supported.supported_functions());

    // Start zbus server
    let mut server = Connection::system().await?;
    if let Err(e) = server.object_server().at("/", ObjectManager).await {
        error!("Failed to register ObjectManager at root '/': {e:?}");
    }

    let config = Config::new().load();
    let cfg_path = config.file_path();
    let config = Arc::new(Mutex::new(config));

    // supported.add_to_server(&mut connection).await;
    let platform = RogPlatform::new()?; // TODO: maybe needs async mutex?
    let power = AsusPower::new()?; // TODO: maybe needs async mutex?
    let attributes = FirmwareAttributes::new();
    let armoury_registry = match start_attributes_zbus(
        &server,
        platform.clone(),
        power.clone(),
        attributes.clone(),
        config.clone(),
    )
    .await
    {
        Ok(registry) => {
            info!("attribute on zbus initialized");
            registry
        }
        Err(e) => {
            error!("Failed to initialize firmware attributes over zbus: {e:?}");
            ArmouryAttributeRegistry::default()
        }
    };

    let fan_curve_config = match CtrlFanCurveZbus::new() {
        Ok(mut ctrl) => {
            info!("FanCurves: found supported fancurves");
            ctrl.set_platform_refs(config.clone(), power.clone());
            let fc_config = Some(ctrl.config());
            ctrl.reload()
                .await
                .unwrap_or_else(|err| warn!("FanCurves reload: {}", err));
            ctrl.add_to_server(&mut server).await;
            info!("FanCurves: initialized");
            fc_config
        }
        Err(err) => {
            error!("FanCurves: {}", err);
            None
        }
    };

    match CtrlBacklight::new(config.clone()) {
        Ok(backlight) => {
            info!("Backlight: found supported backlight");
            backlight.start_watch_primary().await?;
            backlight.add_to_server(&mut server).await;
            info!("Backlight: initialized");
        }
        Err(err) => {
            error!("Backlight: {}", err);
        }
    }

    match CtrlPlatform::new(
        platform,
        power,
        attributes,
        config.clone(),
        &cfg_path,
        CtrlPlatform::signal_context(&server)?,
        server.clone(),
        armoury_registry,
        fan_curve_config,
    ) {
        Ok(ctrl) => {
            info!("CtrlPlatform: initialized");
            let sig_ctx = CtrlPlatform::signal_context(&server)?;
            start_tasks(ctrl, &mut server, sig_ctx).await?;
            info!("CtrlPlatform: tasks started");
        }
        Err(err) => {
            error!("CtrlPlatform: {}", err);
        }
    }

    let _ = DeviceManager::new(server.clone()).await?;

    info!("DeviceManager initialized");

    // XG Mobile LED (non-fatal if not attached)
    match CtrlXgmLed::try_new(config.clone()) {
        Ok(Some(ctrl)) => {
            info!("XG Mobile LED: found and initialized");
            ctrl.add_to_server(&mut server).await;
        }
        Ok(None) => info!("XG Mobile LED: not present"),
        Err(e) => error!("XG Mobile LED: {e}"),
    }

    // Request dbus name after finishing initalizing all functions
    server.request_name(DBUS_NAME).await?;

    info!("Startup success on dbus name {DBUS_NAME}: begining dbus server loop");
    loop {
        // This is just a blocker to idle and ensure the reator reacts
        server.executor().tick().await;
    }
}
