# Daemon

## Controller pattern

There are a series of traits in the daemon for use with controller objects. Not all traits are required:

- `Reloadable`, for controllers that need the ability to reload (typically on start)
- `ZbusAdd`, for controllers that have zbus derive. These need to run on the zbus server.
- `CtrlTask`, for controllers that need to run tasks every loop.
- `GetSupported`, see if the hardware/functions this controller requires are supported.

The first 3 trait objects get owned by the daemon methods that required them, which is why an `Arc<Mutex<T>>` is required.

Generally the actual controller object will need to live in its own world as its own struct.
Then for each trait that is required a new struct is required that can have the trait implemented, and that struct would have a reference to the main controller via `Arc<Mutex<T>>`.

### Example

Main controller:

For a very simple controller that doesn't need exclusive access you can clone across threads

```rust
#[derive(Clone)]
pub struct CtrlAnime {
    <things the controller requires>
    config: Arc<Mutex<Config>>,
}

// This is the task trait used for such things as file watches, or logind
// notifications (boot/suspend/shutdown etc)
impl crate::CtrlTask for CtrlAnime {}

// The trait to easily add the controller to Zbus to enable the zbus derived functions
// to be polled, run, react etc.
impl crate::ZbusAdd for CtrlAnime {}

impl CtrlAnime {}
```

 Otherwise, you will need to share the controller via mutex

```rust
pub struct CtrlAnime {
    <things the controller requires>
}
// Like this
#[derive(Clone)]
pub struct CtrlAnimeTask(Arc<Mutex<CtrlAnime>>);

#[derive(Clone)]
pub struct CtrlAnimeZbus(Arc<Mutex<CtrlAnime>>);

impl CtrlAnime {}
```

The task trait:

```rust
// Mutex should always be async mutex
pub struct CtrlAnimeTask(Arc<Mutex<CtrlAnime>>);

impl crate::CtrlTask for CtrlAnimeTask {
    // This will run once only
    async fn create_tasks(&self, signal_ctxt: SignalContext<'static>) -> Result<(), RogError> {
       let lock self.inner.lock().await;
        <some action>
        Ok(())
    }

    // This will run until the notification stream closes (which in most cases will be never)
    async fn create_tasks(&self, signal_ctxt: SignalContext<'static>) -> Result<(), RogError> {
        let inner1 = self.inner.clone();
        let inner2 = self.inner.clone();
        let inner3 = self.inner.clone();
        let inner4 = self.inner.clone();
        // This is a free method on CtrlTask trait
        self.create_sys_event_tasks(
            // Loop is required to try an attempt to get the mutex *without* blocking
            // other threads - it is possible to end up with deadlocks otherwise.
            move || loop {
                if let Some(lock) = inner1.try_lock() {
                    run_action(true, lock, inner1.clone());
                    break;
                }
            },
            move || loop {
                if let Some(lock) = inner2.try_lock() {
                    run_action(false, lock, inner2.clone());
                    break;
                }
            },
            move || loop {
                if let Some(lock) = inner3.try_lock() {
                    run_action(true, lock, inner3.clone());
                    break;
                }
            },
            move || loop {
                if let Some(lock) = inner4.try_lock() {
                    run_action(false, lock, inner4.clone());
                    break;
                }
            },
        )
        .await;
    }
}
```

The reloader trait

```rust
pub struct CtrlAnimeReloader(Arc<Mutex<CtrlAnime>>);

impl crate::Reloadable for CtrlAnimeReloader {
    async fn reload(&mut self) -> Result<(), RogError> {
        let lock = self.inner.lock().await;
        <some action>
        Ok(())
    }
}
```

The Zbus requirements:

```rust
pub struct CtrlAnimeZbus(Arc<Mutex<CtrlAnime>>);

#[async_trait]
impl crate::ZbusAdd for CtrlAnimeZbus {
    fn add_to_server(self, server: &mut zbus::ObjectServer) {
        // This is a provided free helper trait with pre-set body. It will move self in-to.
        Self::add_to_server_helper(self, "/org/asuslinux/Anime", server).await;
    }
}

#[dbus_interface(name = "xyz.ljones.Asusd")]
impl CtrlAnimeZbus {
    async fn <zbus method>() {
       let lock = self.inner.lock().await;
        <some action>
    }
}
```

The controller can then be added to the daemon parts as required.
