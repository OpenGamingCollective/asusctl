//! XDG GlobalShortcuts portal integration for toggling the main window.
//!
//! Host registration must precede portal use and occur once per connection.
//! KDE may persist denied shortcuts with an empty trigger.

use ashpd::desktop::global_shortcuts::{
    BindShortcutsOptions, ConfigureShortcutsOptions, GlobalShortcuts, ListShortcutsOptions,
    NewShortcut, Shortcut,
};
use ashpd::desktop::CreateSessionOptions;
use ashpd::AppID;
use futures_util::StreamExt;
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use crate::window::{WeakWindowController, WindowCommand, WindowController};
use crate::APP_ID;

const SHORTCUT_ID: &str = "toggle_rog";
const SHORTCUT_DESCRIPTION: &str = "Open/Close ROG Control Center";
// KEY_PROG3 (Armoury Crate) maps to XF86Launch3.
const PREFERRED_TRIGGER: &str = "XF86Launch3";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutStatus {
    Disabled,
    Starting,
    Unassigned,
    Listening,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnableMode {
    Restore,
    Interactive,
}

impl ShortcutStatus {
    /// Whether the app should stay alive while its window is hidden.
    pub fn keeps_alive(self, enabled_in_config: bool) -> bool {
        match self {
            ShortcutStatus::Starting | ShortcutStatus::Listening => true,
            ShortcutStatus::Disabled => enabled_in_config,
            ShortcutStatus::Unassigned | ShortcutStatus::Unavailable => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Assignment {
    Missing,
    Unassigned,
    Assigned,
}

fn classify(entry: Option<(&str, &str)>) -> Assignment {
    match entry {
        None => Assignment::Missing,
        Some((_, trigger)) if trigger.trim().is_empty() => Assignment::Unassigned,
        Some(_) => Assignment::Assigned,
    }
}

fn assignment(shortcuts: &[Shortcut]) -> Assignment {
    classify(
        shortcuts
            .iter()
            .find(|s| s.id() == SHORTCUT_ID)
            .map(|s| (s.id(), s.trigger_description())),
    )
}

enum Command {
    Enable {
        mode: EnableMode,
        respond: oneshot::Sender<ShortcutStatus>,
    },
    Disable,
    Configure {
        respond: oneshot::Sender<bool>,
    },
}

#[derive(Clone)]
pub struct ShortcutHandle {
    commands: mpsc::Sender<Command>,
    status: watch::Receiver<ShortcutStatus>,
    configurable: Arc<AtomicBool>,
}

/// Owns the actor task and its shutdown signal.
pub struct ShortcutService {
    handle: ShortcutHandle,
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl ShortcutService {
    pub fn handle(&self) -> ShortcutHandle {
        self.handle.clone()
    }

    pub async fn shutdown(self) {
        self.shutdown.send_replace(true);
        if let Err(err) = self.task.await {
            error!("Global shortcut actor failed during shutdown: {err}");
        }
    }
}

impl ShortcutHandle {
    pub fn status(&self) -> ShortcutStatus {
        *self.status.borrow()
    }

    pub fn is_listening(&self) -> bool {
        self.status() == ShortcutStatus::Listening
    }

    pub fn status_receiver(&self) -> watch::Receiver<ShortcutStatus> {
        self.status.clone()
    }

    pub async fn enable(&self, mode: EnableMode) -> ShortcutStatus {
        let (respond, result) = oneshot::channel();
        if self
            .commands
            .send(Command::Enable { mode, respond })
            .await
            .is_err()
        {
            return ShortcutStatus::Unavailable;
        }
        result.await.unwrap_or(ShortcutStatus::Unavailable)
    }

    pub async fn disable(&self) {
        let _ = self.commands.send(Command::Disable).await;
    }

    /// Whether the portal supports ConfigureShortcuts.
    pub fn can_configure(&self) -> bool {
        self.configurable.load(Ordering::Acquire)
    }

    pub async fn configure(&self) -> bool {
        let (respond, result) = oneshot::channel();
        if self
            .commands
            .send(Command::Configure { respond })
            .await
            .is_err()
        {
            return false;
        }
        result.await.unwrap_or(false)
    }
}

pub fn start(
    rt: &tokio::runtime::Handle,
    connection: zbus::Connection,
    window: WindowController,
) -> ShortcutService {
    let (commands, rx) = mpsc::channel(1);
    let (status, status_rx) = watch::channel(ShortcutStatus::Disabled);
    let (shutdown, shutdown_rx) = watch::channel(false);
    let configurable = Arc::new(AtomicBool::new(false));
    let task = rt.spawn(run(
        connection,
        window.downgrade(),
        rx,
        status,
        configurable.clone(),
        shutdown_rx,
    ));
    let handle = ShortcutHandle {
        commands,
        status: status_rx,
        configurable,
    };
    ShortcutService {
        handle,
        shutdown,
        task,
    }
}

async fn run(
    connection: zbus::Connection,
    window: WeakWindowController,
    mut commands: mpsc::Receiver<Command>,
    status: watch::Sender<ShortcutStatus>,
    configurable: Arc<AtomicBool>,
    mut shutdown: watch::Receiver<bool>,
) {
    // Reuse the portal proxy; recreate sessions per enable cycle.
    let mut portal: Option<GlobalShortcuts> = None;
    loop {
        let command = tokio::select! {
            _ = shutdown_requested(&mut shutdown) => break,
            command = commands.recv() => command,
        };
        let Some(command) = command else {
            break;
        };
        match command {
            Command::Enable { mode, respond } => {
                enable(
                    &connection, &window, &mut portal, &status, &configurable, &mut commands,
                    &mut shutdown, mode, respond,
                )
                .await;
            }
            Command::Disable => {}
            Command::Configure { respond } => {
                let _ = respond.send(false);
            }
        }
    }
    set_status(&status, ShortcutStatus::Disabled);
}

async fn shutdown_requested(shutdown: &mut watch::Receiver<bool>) {
    if *shutdown.borrow() {
        return;
    }
    while shutdown.changed().await.is_ok() {
        if *shutdown.borrow() {
            return;
        }
    }
}

// Host registration must be the first portal call on this connection.
async fn init_portal(connection: &zbus::Connection) -> Option<GlobalShortcuts> {
    let app_id = match AppID::try_from(APP_ID) {
        Ok(id) => id,
        Err(err) => {
            error!("Invalid application ID {APP_ID}: {err}");
            return None;
        }
    };
    if let Err(err) = ashpd::register_host_app_with_connection(connection.clone(), app_id).await {
        error!("Host app registration failed: {err}");
        return None;
    }
    match GlobalShortcuts::with_connection(connection.clone()).await {
        Ok(proxy) => Some(proxy),
        Err(err) => {
            error!("GlobalShortcuts portal unavailable: {err}");
            None
        }
    }
}

fn set_status(status: &watch::Sender<ShortcutStatus>, new: ShortcutStatus) {
    if *status.borrow() != new {
        let _ = status.send(new);
    }
}

fn finish(
    status: &watch::Sender<ShortcutStatus>,
    respond: oneshot::Sender<ShortcutStatus>,
    result: ShortcutStatus,
) {
    set_status(status, result);
    let _ = respond.send(result);
}

async fn query_assignment(
    gs: &GlobalShortcuts,
    session: &ashpd::desktop::Session<GlobalShortcuts>,
) -> ashpd::Result<Assignment> {
    let request = gs
        .list_shortcuts(session, ListShortcutsOptions::default())
        .await?;
    Ok(assignment(request.response()?.shortcuts()))
}

// A session permits one bind attempt; cancellation may persist an empty trigger.
async fn bind_shortcut(
    gs: &GlobalShortcuts,
    session: &ashpd::desktop::Session<GlobalShortcuts>,
) -> ashpd::Result<Assignment> {
    let shortcut =
        NewShortcut::new(SHORTCUT_ID, SHORTCUT_DESCRIPTION).preferred_trigger(PREFERRED_TRIGGER);
    info!("Requesting shortcut bind via portal");
    let request = gs
        .bind_shortcuts(session, &[shortcut], None, BindShortcutsOptions::default())
        .await?;
    match request.response() {
        Ok(bound) => Ok(assignment(bound.shortcuts())),
        Err(err) => {
            info!("Shortcut bind not completed ({err}), re-reading assignments");
            query_assignment(gs, session).await
        }
    }
}

async fn apply_enable(
    gs: &GlobalShortcuts,
    session: &ashpd::desktop::Session<GlobalShortcuts>,
    current: &mut Assignment,
    bind_attempted: &mut bool,
    mode: EnableMode,
) -> ashpd::Result<ShortcutStatus> {
    if *current != Assignment::Assigned && mode == EnableMode::Interactive {
        match *current {
            Assignment::Missing if !*bind_attempted => {
                // A failed or cancelled bind may still persist state.
                *bind_attempted = true;
                *current = bind_shortcut(gs, session).await?;
            }
            Assignment::Unassigned if gs.version() >= 2 => {
                // KDE requires Configure for an existing empty trigger.
                if let Err(err) = gs
                    .configure_shortcuts(session, None, ConfigureShortcutsOptions::default())
                    .await
                {
                    warn!("Could not open shortcut configuration: {err}");
                }
            }
            _ => {}
        }
    }
    Ok(match current {
        Assignment::Assigned => ShortcutStatus::Listening,
        _ => ShortcutStatus::Unassigned,
    })
}

#[allow(clippy::too_many_arguments)]
async fn enable(
    connection: &zbus::Connection,
    window: &WeakWindowController,
    portal: &mut Option<GlobalShortcuts>,
    status: &watch::Sender<ShortcutStatus>,
    configurable: &Arc<AtomicBool>,
    commands: &mut mpsc::Receiver<Command>,
    shutdown: &mut watch::Receiver<bool>,
    mode: EnableMode,
    respond: oneshot::Sender<ShortcutStatus>,
) {
    set_status(status, ShortcutStatus::Starting);

    if portal.is_none() {
        match init_portal(connection).await {
            Some(proxy) => *portal = Some(proxy),
            None => {
                finish(status, respond, ShortcutStatus::Unavailable);
                return;
            }
        }
    }
    let gs = portal.as_ref().expect("portal proxy just initialized");
    configurable.store(gs.version() >= 2, Ordering::Release);

    let session = match gs.create_session(CreateSessionOptions::default()).await {
        Ok(session) => session,
        Err(err) => {
            error!("Could not create global shortcuts session: {err}");
            finish(status, respond, ShortcutStatus::Unavailable);
            return;
        }
    };
    info!("Global shortcuts session created");

    run_session(
        gs, session, window, status, commands, shutdown, mode, respond,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn run_session(
    gs: &GlobalShortcuts,
    session: ashpd::desktop::Session<GlobalShortcuts>,
    window: &WeakWindowController,
    status: &watch::Sender<ShortcutStatus>,
    commands: &mut mpsc::Receiver<Command>,
    shutdown: &mut watch::Receiver<bool>,
    mode: EnableMode,
    respond: oneshot::Sender<ShortcutStatus>,
) {
    let mut respond = Some(respond);
    let final_status = tokio::select! {
        _ = shutdown_requested(shutdown) => ShortcutStatus::Disabled,
        result = run_session_inner(gs, &session, window, status, commands, mode, &mut respond) => result,
    };

    if let Some(respond) = respond.take() {
        let _ = respond.send(final_status);
    }
    if let Err(err) = session.close().await {
        warn!("Could not close global shortcuts session: {err}");
    }
    set_status(status, final_status);
    info!("Global shortcuts session ended ({final_status:?})");
}

#[allow(clippy::too_many_arguments)]
async fn run_session_inner(
    gs: &GlobalShortcuts,
    session: &ashpd::desktop::Session<GlobalShortcuts>,
    window: &WeakWindowController,
    status: &watch::Sender<ShortcutStatus>,
    commands: &mut mpsc::Receiver<Command>,
    mode: EnableMode,
    respond: &mut Option<oneshot::Sender<ShortcutStatus>>,
) -> ShortcutStatus {
    // Subscribe before List/Bind to avoid missing early signals.
    let mut activated = match gs.receive_activated().await {
        Ok(stream) => stream,
        Err(err) => {
            error!("Could not subscribe to Activated: {err}");
            return ShortcutStatus::Unavailable;
        }
    };
    let mut changed = match gs.receive_shortcuts_changed().await {
        Ok(stream) => stream,
        Err(err) => {
            error!("Could not subscribe to ShortcutsChanged: {err}");
            return ShortcutStatus::Unavailable;
        }
    };
    let mut closed = match session.receive_closed().await {
        Ok(stream) => stream,
        Err(err) => {
            error!("Could not subscribe to session Closed: {err}");
            return ShortcutStatus::Unavailable;
        }
    };

    let mut current = match query_assignment(gs, session).await {
        Ok(found) => found,
        Err(err) => {
            error!("Could not list shortcuts: {err}");
            return ShortcutStatus::Unavailable;
        }
    };
    let mut bind_attempted = false;

    let mut current_status =
        match apply_enable(gs, session, &mut current, &mut bind_attempted, mode).await {
            Ok(result) => result,
            Err(err) => {
                error!("Enable failed: {err}");
                return ShortcutStatus::Unavailable;
            }
        };
    set_status(status, current_status);
    if let Some(respond) = respond.take() {
        let _ = respond.send(current_status);
    }
    info!("Global shortcuts status: {current_status:?}");

    loop {
        tokio::select! {
            command = commands.recv() => {
                match command {
                    Some(Command::Disable) => break ShortcutStatus::Disabled,
                    Some(Command::Configure { respond }) => {
                        let ok = if gs.version() >= 2 {
                            match gs
                                .configure_shortcuts(session, None, ConfigureShortcutsOptions::default())
                                .await
                            {
                                Ok(()) => true,
                                Err(err) => {
                                    warn!("ConfigureShortcuts failed: {err}");
                                    false
                                }
                            }
                        } else {
                            warn!(
                                "ConfigureShortcuts needs portal version 2 (have {})",
                                gs.version()
                            );
                            false
                        };
                        let _ = respond.send(ok);
                    }
                    Some(Command::Enable { mode, respond }) => {
                        set_status(status, ShortcutStatus::Starting);
                        match apply_enable(gs, session, &mut current, &mut bind_attempted, mode).await {
                            Ok(result) => {
                                current_status = result;
                                finish(status, respond, result);
                            }
                            Err(err) => {
                                error!("Enable failed: {err}");
                                finish(status, respond, ShortcutStatus::Unavailable);
                                break ShortcutStatus::Unavailable;
                            }
                        }
                    }
                    None => break ShortcutStatus::Disabled,
                }
            }
            event = activated.next() => {
                match event {
                    Some(active) if active.shortcut_id() == SHORTCUT_ID => {
                        info!("Shortcut activated, toggling window");
                        if let Some(window) = window.upgrade() {
                            window.request(WindowCommand::Toggle);
                        }
                    }
                    Some(_) => {}
                    None => break ShortcutStatus::Unavailable,
                }
            }
            event = changed.next() => {
                match event {
                    Some(update) => {
                        current = assignment(update.shortcuts());
                        let new_status = match current {
                            Assignment::Assigned => ShortcutStatus::Listening,
                            _ => ShortcutStatus::Unassigned,
                        };
                        if new_status != current_status {
                            info!("Shortcut assignment changed: {new_status:?}");
                            current_status = new_status;
                            set_status(status, new_status);
                        }
                    }
                    None => break ShortcutStatus::Unavailable,
                }
            }
            _ = closed.next() => break ShortcutStatus::Unavailable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_missing_when_absent() {
        assert_eq!(classify(None), Assignment::Missing);
    }

    #[test]
    fn classify_unassigned_without_trigger() {
        assert_eq!(classify(Some((SHORTCUT_ID, ""))), Assignment::Unassigned);
        assert_eq!(classify(Some((SHORTCUT_ID, "   "))), Assignment::Unassigned);
    }

    #[test]
    fn classify_assigned_with_trigger() {
        assert_eq!(
            classify(Some((SHORTCUT_ID, "XF86Launch3"))),
            Assignment::Assigned
        );
    }

    #[test]
    fn keeps_alive_during_startup_and_bind() {
        assert!(ShortcutStatus::Starting.keeps_alive(false));
        assert!(ShortcutStatus::Listening.keeps_alive(false));
        assert!(ShortcutStatus::Disabled.keeps_alive(true));
    }

    #[test]
    fn does_not_keep_alive_when_useless() {
        assert!(!ShortcutStatus::Disabled.keeps_alive(false));
        assert!(!ShortcutStatus::Unassigned.keeps_alive(true));
        assert!(!ShortcutStatus::Unavailable.keeps_alive(true));
    }
}
