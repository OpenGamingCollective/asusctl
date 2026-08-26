# Daemon

## Synchronous Controller Architecture

Controllers in the daemon manage hardware features, user configurations, and platform events using a **purely synchronous (`sync`) concurrency and state model**.

In Linux, kernel interactions via sysfs (`/sys`), debugfs, ACPI WMI, and raw device nodes are inherently synchronous and blocking file operations. Managing controller state with standard library synchronous primitives (`std::sync`) ensures predictable execution, eliminates async runtime overhead, and keeps domain logic straightforward.

### Controller Traits

Controllers can implement the standard daemon lifecycle and capability traits:

- `GetSupported`: Checks if the hardware or kernel features required by the controller are supported on the current machine.
- `Reloadable`: For controllers that need the ability to reload state (typically on startup or upon receiving configuration reload signals).
- `CtrlTask`: For background workers handling system lifecycle events (boot, suspend, resume, shutdown) or watching sysfs/udev nodes.
- `ZbusRun`: For exposing controller interfaces to the D-Bus system bus via `zbus`.

Depending on the controller's complexity and concurrency requirements, these traits can be implemented directly on the controller struct or via dedicated wrappers.

### Synchronous Concurrency & State Ownership Models

When sharing state across controller traits, background threads, or D-Bus handlers, choose the concurrency model based on ownership and access patterns:

- **Thread Worker / Actor Pattern (`std::thread` + `std::sync::mpsc`)**: Preferred for controllers that manage sequential hardware I/O (such as Aura RGB USB HID, AniMe Matrix, or event queues). A dedicated OS worker thread retains single-ownership of the physical device handle and processes incoming commands from a `std::sync::mpsc::channel`. This naturally avoids concurrent hardware access, serializes I/O, and eliminates deadlocks by construction.
- **Shared Memory with Standard Synchronous Locks (`Arc<std::sync::RwLock<T>>` / `Arc<std::sync::Mutex<T>>`)**: Suitable for lightweight in-memory state (such as configuration files or cached status). Prefer `std::sync::RwLock` for read-heavy state to allow concurrent readers without lock contention, and `std::sync::Mutex` when writes are frequent.
- **Lock-Free Synchronization with Atomics (`std::sync::atomic`)**: For simple status flags, counters, or mode indicators (e.g. device connection status, active power mode, suspension state), prefer atomic types (`AtomicBool`, `AtomicU32`, `AtomicUsize`) with appropriate memory ordering (`Ordering::Relaxed`, `Ordering::Acquire`, `Ordering::Release`) to eliminate lock contention and mutex allocations entirely.
- **Static & Write-Once Initialization (`std::sync::LazyLock` / `std::sync::OnceLock`)**: For immutable lookup tables, regexes, and device capability maps initialized lazily or once at startup, use `std::sync::LazyLock` or `std::sync::OnceLock` instead of dynamic locking primitives.
- **Fast Critical Sections (Copy-on-Read)**: Keep locked critical sections minimal. Acquire the lock, copy or extract the required value into a local variable, and immediately drop the lock guard before performing hardware I/O or long-running operations.
- **No Spin Locks or Busy-Waiting**: Never use spin locks or polling loops (`loop { try_lock() }`) for shared state or event dispatching. Use blocking synchronization (`std::sync::mpsc::Receiver::recv()`, condition variables, or OS event polling) to avoid wasting CPU cycles.

### Examples

#### 1. Controller with Shared In-Memory State (`Arc<std::sync::RwLock<Config>>`)

For controllers that manage device settings and respond to system events (e.g. `CtrlPlatform`), keep the controller cheaply cloneable by wrapping read-heavy configuration in `std::sync::RwLock` and passing references to underlying platform handles:

```rust
use std::sync::{Arc, RwLock};
use zbus::Connection;
use zbus::object_server::SignalEmitter;
use crate::error::RogError;
use crate::{CtrlTask, Reloadable, ZbusRun};

#[derive(Clone)]
pub struct CtrlPlatform {
    platform: RogPlatform,
    power: AsusPower,
    config: Arc<RwLock<Config>>,
}

// Zbus interface registration
impl ZbusRun for CtrlPlatform {
    async fn add_to_server(self, server: &mut Connection) {
        Self::add_to_server_helper(self, "/xyz/ljones/Platform", server).await;
    }
}

// Synchronous configuration reload handler
impl Reloadable for CtrlPlatform {
    async fn reload(&mut self) -> Result<(), RogError> {
        // Read configuration and extract value immediately to minimize critical section
        let charge_limit = self
            .config
            .read()
            .map_err(|e| RogError::LockError(e.to_string()))?
            .charge_control_end_threshold;

        // Perform hardware sysfs write without holding the lock
        self.power.set_charge_control_end_threshold(charge_limit)?;
        Ok(())
    }
}

// Background task and system lifecycle event handling
impl CtrlTask for CtrlPlatform {
    fn zbus_path() -> &'static str {
        "/xyz/ljones/Platform"
    }

    async fn create_tasks(&self, _signal_ctxt: SignalEmitter<'static>) -> Result<(), RogError> {
        let ctrl = self.clone();

        // Register event handlers for system lifecycle events
        self.create_sys_event_tasks(
            move |sleeping| {
                let ctrl = ctrl.clone();
                async move {
                    if !sleeping {
                        // Re-apply charge limit when resuming from sleep
                        if let Ok(config) = ctrl.config.read() {
                            let limit = config.charge_control_end_threshold;
                            drop(config);
                            ctrl.power.set_charge_control_end_threshold(limit).ok();
                        }
                    }
                }
            },
            move |_shutdown| async move { /* handle shutdown */ },
            move |_lid_closed| async move { /* handle lid switch event */ },
            move |_on_ac| async move { /* handle AC/battery power transition */ },
        )
        .await;

        Ok(())
    }
}
```

#### 2. Synchronous Worker Thread for Hardware I/O (`std::thread` + `std::sync::mpsc`)

For hardware devices that require exclusive, serialized access (e.g. Aura RGB USB HID, AniMe Matrix animations), use a dedicated background OS worker thread with `std::sync::mpsc`:

```rust
use std::sync::mpsc::{self, Sender, SyncSender};
use std::thread;
use zbus::interface;
use crate::error::RogError;

enum DeviceMsg {
    SetBrightness {
        brightness: u8,
        reply: Sender<Result<(), RogError>>,
    },
    SetMode {
        mode: AuraMode,
        reply: Sender<Result<(), RogError>>,
    },
}

#[derive(Clone)]
pub struct CtrlAura {
    tx: SyncSender<DeviceMsg>,
}

impl CtrlAura {
    pub fn new(mut device: AuraDevice) -> Self {
        let (tx, rx) = mpsc::sync_channel::<DeviceMsg>(32);

        // Dedicated OS worker thread retains single-ownership of the physical device
        thread::Builder::new()
            .name("aura-worker".into())
            .spawn(move || {
                // Blocking loop waits for incoming commands without busy-waiting
                while let Ok(msg) = rx.recv() {
                    match msg {
                        DeviceMsg::SetBrightness { brightness, reply } => {
                            let res = device.write_brightness(brightness);
                            let _ = reply.send(res);
                        }
                        DeviceMsg::SetMode { mode, reply } => {
                            let res = device.write_mode(mode);
                            let _ = reply.send(res);
                        }
                    }
                }
            })
            .expect("Failed to spawn aura-worker thread");

        Self { tx }
    }
}

#[interface(name = "xyz.ljones.Aura")]
impl CtrlAura {
    async fn set_brightness(&self, val: u8) -> zbus::fdo::Result<()> {
        let (reply_tx, reply_rx) = mpsc::channel();

        self.tx
            .send(DeviceMsg::SetBrightness {
                brightness: val,
                reply: reply_tx,
            })
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        // Wait for synchronous worker response
        reply_rx
            .recv()
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }
}
```

#### 3. Lock-Free Status & Direct Kernel Sysfs Management (`std::sync::atomic`)

For controllers with simple state flags or high-frequency status queries, combine lock-free atomics with synchronous in-memory caching:

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use zbus::interface;
use crate::error::RogError;

#[derive(Clone)]
pub struct CtrlXgm {
    // Lock-free atomic indicator for high-frequency status queries
    is_active: Arc<AtomicBool>,
    // Synchronous mutex for fast in-memory caching
    cached_name: Arc<Mutex<Option<String>>>,
}

impl CtrlXgm {
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            cached_name: Arc::new(Mutex::new(None)),
        }
    }

    /// Fast lock-free query with zero lock contention
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Acquire)
    }

    /// Fast synchronous cache update without async lock allocation
    pub fn update_cached_name(&self, name: String) {
        if let Ok(mut lock) = self.cached_name.lock() {
            *lock = Some(name);
        }
    }

    /// Synchronous sysfs kernel interaction
    pub fn write_xgm_active(&self, active: bool) -> Result<(), RogError> {
        std::fs::write(
            "/sys/devices/platform/asus-nb-wmi/xgm_active",
            if active { "1" } else { "0" },
        )
        .map_err(|e| RogError::SysfsWrite(e.to_string()))?;

        self.is_active.store(active, Ordering::Release);
        Ok(())
    }
}

#[interface(name = "xyz.ljones.Xgm")]
impl CtrlXgm {
    async fn set_active(&self, active: bool) -> zbus::fdo::Result<()> {
        self.write_xgm_active(active)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }
}
```
