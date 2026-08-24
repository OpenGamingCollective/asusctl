//! XDG `GlobalShortcuts` portal integration for toggling the main window.
//!
//! Invariants:
//! - Host registration must precede portal use and occur once per connection.
//! - Signal streams are subscribed before List/Bind so early signals are not
//!   missed.
//! - A session permits one bind attempt; cancellation may persist an empty
//!   trigger. KDE may persist denied shortcuts with an empty trigger.
//! - All session signals are drained by a single select loop; a zbus
//!   subscription buffers at most 64 undelivered messages.
//! - The portal session is closed on the bus before the actor task returns,
//!   and the runtime is stopped only after the actor finishes (main.rs).

use ashpd::AppID;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut, Shortcut};
use futures_util::StreamExt;
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use crate::APP_ID;
use crate::window::{WeakWindowController, WindowCommand, WindowController};

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
    #[must_use]
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

impl Assignment {
    fn status(self) -> ShortcutStatus {
        match self {
            Assignment::Assigned => ShortcutStatus::Listening,
            _ => ShortcutStatus::Unassigned,
        }
    }
}

fn classify(entry: Option<(&str, &str)>) -> Assignment {
    match entry {
        None => Assignment::Missing,
        Some((_, trigger)) if trigger.trim().is_empty() => Assignment::Unassigned,
        Some(_) => Assignment::Assigned,
    }
}

fn classify_bound(entry: Option<(&str, &str)>) -> Assignment {
    match entry {
        None => Assignment::Missing,
        Some(_) => Assignment::Assigned,
    }
}

fn shortcut_entry(shortcuts: &[Shortcut]) -> Option<(&str, &str)> {
    shortcuts
        .iter()
        .find(|s| s.id() == SHORTCUT_ID)
        .map(|s| (s.id(), s.trigger_description()))
}

fn assignment(shortcuts: &[Shortcut]) -> Assignment {
    classify(shortcut_entry(shortcuts))
}

fn bound_assignment(shortcuts: &[Shortcut]) -> Assignment {
    classify_bound(shortcut_entry(shortcuts))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnableAction {
    Bind,
    Configure,
    Skip,
}

fn enable_action(
    current: Assignment,
    bind_attempted: bool,
    mode: EnableMode,
    portal_version: u32,
) -> EnableAction {
    match (current, bind_attempted) {
        (Assignment::Unassigned, _) => match (mode, portal_version >= 2) {
            (EnableMode::Interactive, true) => EnableAction::Configure,
            _ => EnableAction::Skip,
        },
        (_, false) => EnableAction::Bind,
        (_, true) => EnableAction::Skip,
    }
}

// ashpd 0.13 migration point: this alias and the Portal impl below are the
// only ashpd-dependent shapes in this module.
type PortalSession = ashpd::desktop::Session<GlobalShortcuts>;

/// Portal proxy with the interface version, read once at init time.
struct Portal {
    gs: GlobalShortcuts,
    version: u32,
}

impl Portal {
    // Host registration must be the first portal call.
    async fn connect() -> Option<Self> {
        let app_id = match AppID::try_from(APP_ID) {
            Ok(id) => id,
            Err(err) => {
                error!("Invalid application ID {APP_ID}: {err}");
                return None;
            }
        };
        if let Err(err) = ashpd::register_host_app(app_id).await {
            error!("Host app registration failed: {err}");
            return None;
        }
        match GlobalShortcuts::new().await {
            Ok(gs) => {
                let version = gs.version();
                Some(Portal { gs, version })
            }
            Err(err) => {
                error!("GlobalShortcuts portal unavailable: {err}");
                None
            }
        }
    }

    async fn create_session(&self) -> ashpd::Result<PortalSession> {
        self.gs.create_session(Default::default()).await
    }

    async fn list_assignment(&self, session: &PortalSession) -> ashpd::Result<Assignment> {
        let request = self.gs.list_shortcuts(session, Default::default()).await?;
        Ok(assignment(request.response()?.shortcuts()))
    }

    // A session permits one bind attempt; cancellation may persist an empty trigger.
    async fn bind_shortcut(&self, session: &PortalSession) -> ashpd::Result<Assignment> {
        let shortcut = NewShortcut::new(SHORTCUT_ID, SHORTCUT_DESCRIPTION)
            .preferred_trigger(PREFERRED_TRIGGER);
        info!("Requesting shortcut bind via portal");
        let request = self
            .gs
            .bind_shortcuts(session, &[shortcut], None, Default::default())
            .await?;
        match request.response() {
            Ok(bound) => Ok(bound_assignment(bound.shortcuts())),
            Err(err) => {
                info!("Shortcut bind not completed ({err}), re-reading assignments");
                self.list_assignment(session).await
            }
        }
    }

    async fn configure(&self, session: &PortalSession) -> ashpd::Result<()> {
        self.gs
            .configure_shortcuts(session, None, Default::default())
            .await
    }
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

#[derive(Clone, Debug)]
pub struct ShortcutHandle {
    commands: mpsc::Sender<Command>,
    status: watch::Receiver<ShortcutStatus>,
    configurable: Arc<AtomicBool>,
}

/// Owns the actor task and its shutdown signal.
#[derive(Debug)]
pub struct ShortcutService {
    handle: ShortcutHandle,
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl ShortcutService {
    #[must_use]
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
    #[must_use]
    pub fn status(&self) -> ShortcutStatus {
        *self.status.borrow()
    }

    #[must_use]
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

    /// Whether the portal supports `ConfigureShortcuts`.
    #[must_use]
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

#[must_use]
pub fn start(rt: &tokio::runtime::Handle, window: &WindowController) -> ShortcutService {
    // Capacity 1: callers wait while the actor is busy with a portal dialog.
    let (commands, rx) = mpsc::channel(1);
    let (status, status_rx) = watch::channel(ShortcutStatus::Disabled);
    let (shutdown, shutdown_rx) = watch::channel(false);
    let configurable = Arc::new(AtomicBool::new(false));
    let task = rt.spawn(run(
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
    window: WeakWindowController,
    mut commands: mpsc::Receiver<Command>,
    status: watch::Sender<ShortcutStatus>,
    configurable: Arc<AtomicBool>,
    mut shutdown: watch::Receiver<bool>,
) {
    // Reuse the portal proxy; recreate sessions per enable cycle.
    let mut portal: Option<Portal> = None;
    let actor = Actor {
        window: &window,
        status: &status,
        configurable: &configurable,
    };
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
                actor
                    .enable_session(&mut portal, &mut commands, &mut shutdown, mode, respond)
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

fn set_status(status: &watch::Sender<ShortcutStatus>, new: ShortcutStatus) {
    let old = *status.borrow();
    if old != new {
        debug!("Status {old:?} -> {new:?}");
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

struct Actor<'a> {
    window: &'a WeakWindowController,
    status: &'a watch::Sender<ShortcutStatus>,
    configurable: &'a AtomicBool,
}

impl Actor<'_> {
    async fn enable_session(
        &self,
        portal_slot: &mut Option<Portal>,
        commands: &mut mpsc::Receiver<Command>,
        shutdown: &mut watch::Receiver<bool>,
        mode: EnableMode,
        respond: oneshot::Sender<ShortcutStatus>,
    ) {
        set_status(self.status, ShortcutStatus::Starting);

        if portal_slot.is_none() {
            match Portal::connect().await {
                Some(proxy) => *portal_slot = Some(proxy),
                None => {
                    finish(self.status, respond, ShortcutStatus::Unavailable);
                    return;
                }
            }
        }
        let portal = portal_slot.as_ref().expect("portal proxy just initialized");
        self.configurable
            .store(portal.version >= 2, Ordering::Release);

        let session = match portal.create_session().await {
            Ok(session) => session,
            Err(err) => {
                error!("Could not create global shortcuts session: {err}");
                finish(self.status, respond, ShortcutStatus::Unavailable);
                return;
            }
        };
        info!("Global shortcuts session created");

        let mut session_loop = SessionLoop {
            portal,
            session: &session,
            window: self.window,
            status_tx: self.status,
            mode,
            respond: Some(respond),
            current: Assignment::Missing,
            bind_attempted: false,
            status: ShortcutStatus::Starting,
            closed_observed: false,
        };
        session_loop.run(commands, shutdown).await;
    }
}

/// Borrows the portal session and owns all per-session state.
struct SessionLoop<'a> {
    portal: &'a Portal,
    session: &'a PortalSession,
    window: &'a WeakWindowController,
    status_tx: &'a watch::Sender<ShortcutStatus>,
    mode: EnableMode,
    respond: Option<oneshot::Sender<ShortcutStatus>>,
    current: Assignment,
    bind_attempted: bool,
    status: ShortcutStatus,
    closed_observed: bool,
}

impl SessionLoop<'_> {
    async fn run(
        &mut self,
        commands: &mut mpsc::Receiver<Command>,
        shutdown: &mut watch::Receiver<bool>,
    ) -> ShortcutStatus {
        debug!("Session loop starting (mode={:?})", self.mode);
        let final_status = tokio::select! {
            _ = shutdown_requested(shutdown) => ShortcutStatus::Disabled,
            result = self.run_inner(commands) => result,
        };

        if let Some(respond) = self.respond.take() {
            let _ = respond.send(final_status);
        }
        // Invariant: the session is closed on the bus before this task returns.
        if let Err(err) = self.session.close().await {
            // The portal already tore the session down; a failing Close is expected.
            if self.closed_observed {
                debug!("Session close after Closed signal: {err}");
            } else {
                warn!("Could not close global shortcuts session: {err}");
            }
        }
        set_status(self.status_tx, final_status);
        info!("Global shortcuts session ended ({final_status:?})");
        final_status
    }

    async fn run_inner(&mut self, commands: &mut mpsc::Receiver<Command>) -> ShortcutStatus {
        // Subscribe before List/Bind to avoid missing early signals.
        let mut activated = match self.portal.gs.receive_activated().await {
            Ok(stream) => stream,
            Err(err) => {
                error!("Could not subscribe to Activated: {err}");
                return ShortcutStatus::Unavailable;
            }
        };
        let mut changed = match self.portal.gs.receive_shortcuts_changed().await {
            Ok(stream) => stream,
            Err(err) => {
                error!("Could not subscribe to ShortcutsChanged: {err}");
                return ShortcutStatus::Unavailable;
            }
        };
        let session = self.session;
        let mut closed = match session.receive_closed().await {
            Ok(stream) => stream,
            Err(err) => {
                error!("Could not subscribe to session Closed: {err}");
                return ShortcutStatus::Unavailable;
            }
        };
        debug!("Portal signal subscriptions ready");

        self.current = match self.portal.list_assignment(session).await {
            Ok(found) => found,
            Err(err) => {
                error!("Could not list shortcuts: {err}");
                return ShortcutStatus::Unavailable;
            }
        };
        self.bind_attempted = false;
        debug!("Assignment after list: {:?}", self.current);

        self.status = match self.apply_enable().await {
            Ok(result) => result,
            Err(err) => {
                error!("Enable failed: {err}");
                return ShortcutStatus::Unavailable;
            }
        };
        set_status(self.status_tx, self.status);
        if let Some(respond) = self.respond.take() {
            let _ = respond.send(self.status);
        }
        info!("Global shortcuts status: {:?}", self.status);

        loop {
            tokio::select! {
                command = commands.recv() => {
                    match command {
                        Some(Command::Disable) => {
                            debug!("Command: Disable");
                            break ShortcutStatus::Disabled;
                        }
                        Some(Command::Configure { respond }) => {
                            debug!("Command: Configure");
                            let ok = if self.portal.version >= 2 {
                                match self.portal.configure(self.session).await {
                                    Ok(()) => true,
                                    Err(err) => {
                                        warn!("ConfigureShortcuts failed: {err}");
                                        false
                                    }
                                }
                            } else {
                                warn!(
                                    "ConfigureShortcuts needs portal version 2 (have {})",
                                    self.portal.version
                                );
                                false
                            };
                            let _ = respond.send(ok);
                        }
                        Some(Command::Enable { mode, respond }) => {
                            debug!("Command: Enable({mode:?})");
                            set_status(self.status_tx, ShortcutStatus::Starting);
                            self.mode = mode;
                            match self.apply_enable().await {
                                Ok(result) => {
                                    self.status = result;
                                    finish(self.status_tx, respond, result);
                                }
                                Err(err) => {
                                    error!("Enable failed: {err}");
                                    finish(self.status_tx, respond, ShortcutStatus::Unavailable);
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
                            if let Some(window) = self.window.upgrade() {
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
                            self.current = assignment(update.shortcuts());
                            let new_status = self.current.status();
                            if new_status != self.status {
                                info!("Shortcut assignment changed: {new_status:?}");
                                self.status = new_status;
                                set_status(self.status_tx, new_status);
                            }
                        }
                        None => break ShortcutStatus::Unavailable,
                    }
                }
                _ = closed.next() => {
                    self.closed_observed = true;
                    debug!("Session Closed signal observed");
                    break ShortcutStatus::Unavailable;
                }
            }
        }
    }

    async fn apply_enable(&mut self) -> ashpd::Result<ShortcutStatus> {
        let action = enable_action(
            self.current, self.bind_attempted, self.mode, self.portal.version,
        );
        debug!(
            "Enable action {action:?} (current {:?}, bind_attempted {}, mode {:?}, portal v{})",
            self.current, self.bind_attempted, self.mode, self.portal.version
        );
        match action {
            EnableAction::Bind => {
                self.bind_attempted = true;
                self.current = self.portal.bind_shortcut(self.session).await?;
            }
            EnableAction::Configure => {
                if let Err(err) = self.portal.configure(self.session).await {
                    warn!("Could not open shortcut configuration: {err}");
                }
            }
            EnableAction::Skip => {}
        }
        Ok(self.current.status())
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
    fn bound_shortcut_with_empty_trigger_is_assigned() {
        assert_eq!(
            classify_bound(Some((SHORTCUT_ID, ""))),
            Assignment::Assigned
        );
        assert_eq!(
            classify_bound(Some((SHORTCUT_ID, "   "))),
            Assignment::Assigned
        );
        assert_eq!(
            classify_bound(Some((SHORTCUT_ID, "XF86Launch3"))),
            Assignment::Assigned
        );
    }

    #[test]
    fn bound_response_without_id_is_missing() {
        assert_eq!(classify_bound(None), Assignment::Missing);
    }

    #[test]
    fn enable_action_binds_once_per_session() {
        assert_eq!(
            enable_action(Assignment::Missing, false, EnableMode::Restore, 1),
            EnableAction::Bind
        );
        assert_eq!(
            enable_action(Assignment::Assigned, false, EnableMode::Restore, 1),
            EnableAction::Bind
        );
        assert_eq!(
            enable_action(Assignment::Missing, true, EnableMode::Interactive, 2),
            EnableAction::Skip
        );
        assert_eq!(
            enable_action(Assignment::Assigned, true, EnableMode::Restore, 2),
            EnableAction::Skip
        );
    }

    #[test]
    fn enable_action_configures_only_interactive_unassigned_on_v2() {
        assert_eq!(
            enable_action(Assignment::Unassigned, false, EnableMode::Interactive, 2),
            EnableAction::Configure
        );
        assert_eq!(
            enable_action(Assignment::Unassigned, true, EnableMode::Interactive, 2),
            EnableAction::Configure
        );
        assert_eq!(
            enable_action(Assignment::Unassigned, false, EnableMode::Interactive, 1),
            EnableAction::Skip
        );
        assert_eq!(
            enable_action(Assignment::Unassigned, false, EnableMode::Restore, 2),
            EnableAction::Skip
        );
    }

    #[test]
    fn unassigned_never_binds_directly() {
        for attempted in [
            false, true,
        ] {
            assert_ne!(
                enable_action(
                    Assignment::Unassigned,
                    attempted,
                    EnableMode::Interactive,
                    2
                ),
                EnableAction::Bind
            );
        }
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
