use std::sync::{Arc, Mutex};

use log::{error, info};
use rog_anime::Animations;
use rog_dbus::find_iface_async;
use rog_dbus::zbus_anime::AnimeProxy;
use slint::ComponentHandle;

use crate::config::Config;
use crate::ui::show_toast;
use crate::{AnimePageData, MainWindow, set_ui_callbacks, set_ui_props_async};

pub fn setup_anime_page(ui: &MainWindow, _states: Arc<Mutex<Config>>) {
    let handle = ui.as_weak();
    tokio::spawn(async move {
        let Ok(animes) = find_iface_async::<AnimeProxy>("xyz.ljones.Anime").await else {
            info!("This device appears to have no aura interfaces");
            return;
        };

        for anime in animes {
            set_ui_props_async!(handle, anime, AnimePageData, brightness);
            set_ui_props_async!(handle, anime, AnimePageData, builtins_enabled);
            set_ui_props_async!(handle, anime, AnimePageData, enable_display);
            set_ui_props_async!(handle, anime, AnimePageData, off_when_lid_closed);
            set_ui_props_async!(handle, anime, AnimePageData, off_when_suspended);
            set_ui_props_async!(handle, anime, AnimePageData, off_when_unplugged);

            let builtins = anime.builtin_animations().await.unwrap_or_default();
            handle
                .upgrade_in_event_loop(move |handle| {
                    {
                        let global = handle.global::<AnimePageData>();
                        global.set_boot_anim(builtins.boot as i32);
                        global.set_awake_anim(builtins.awake as i32);
                        global.set_sleep_anim(builtins.sleep as i32);
                        global.set_shutdown_anim(builtins.shutdown as i32);

                        let handle_copy = handle.as_weak();
                        let anime_copy = anime.clone();
                        global.on_cb_builtin_animations(move |boot, awake, sleep, shutdown| {
                            let handle_copy = handle_copy.clone();
                            let anime_copy = anime_copy.clone();
                            tokio::spawn(async move {
                                show_toast(
                                    "Anime builtin animations changed".into(),
                                    "Failed to set Anime builtin animations".into(),
                                    handle_copy,
                                    anime_copy
                                        .set_builtin_animations(Animations {
                                            boot: boot.into(),
                                            awake: awake.into(),
                                            sleep: sleep.into(),
                                            shutdown: shutdown.into(),
                                        })
                                        .await,
                                );
                            });
                        });

                        let handle_copy = handle.as_weak();
                        let anime_copy = anime.clone();
                        tokio::spawn(async move {
                            let mut x = anime_copy.receive_builtin_animations_changed().await;
                            use futures_util::StreamExt;
                            while let Some(e) = x.next().await {
                                if let Ok(out) = e.get().await {
                                    handle_copy
                                        .upgrade_in_event_loop(move |handle| {
                                            handle
                                                .global::<AnimePageData>()
                                                .set_boot_anim(out.boot.into());
                                            handle
                                                .global::<AnimePageData>()
                                                .set_awake_anim(out.awake.into());
                                            handle
                                                .global::<AnimePageData>()
                                                .set_sleep_anim(out.sleep.into());
                                            handle
                                                .global::<AnimePageData>()
                                                .set_shutdown_anim(out.shutdown.into());
                                        })
                                        .ok();
                                }
                            }
                        });
                    }

                    set_ui_callbacks!(handle,
                        AnimePageData(.into()),
                        anime.brightness(.into()),
                        "Anime LED brightness successfully set to {}",
                        "Setting Anime LED brightness failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        AnimePageData(),
                        anime.builtins_enabled(),
                        "Keyboard LED mode successfully set to {}",
                        "Setting keyboard LEDmode failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        AnimePageData(),
                        anime.enable_display(),
                        "Anime display successfully set to {}",
                        "Setting Anime display failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        AnimePageData(),
                        anime.off_when_lid_closed(),
                        "Anime off_when_lid_closed successfully set to {}",
                        "Setting Anime off_when_lid_closed failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        AnimePageData(),
                        anime.off_when_suspended(),
                        "Anime off_when_suspended successfully set to {}",
                        "Setting Anime off_when_suspended failed"
                    );
                    set_ui_callbacks!(
                        handle,
                        AnimePageData(),
                        anime.off_when_unplugged(),
                        "Anime off_when_unplugged successfully set to {}",
                        "Setting Anime off_when_unplugged failed"
                    );
                })
                .map_err(|e| error!("setup_anime_page: upgrade_in_event_loop: {e:?}"))
                .ok();
        }
    });
}
