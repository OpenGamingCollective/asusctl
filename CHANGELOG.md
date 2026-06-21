# Changelog

## [6.3.9]

### Changes

- Added auto-upload hook: thanks @ayush123460
- Added Mini-LED Multi-zone Strong support: thanks @Ghoul4500
- Add aura support for multiple FX608 models: thanks @dmitthedazed
- Add aura support for GA403UM: thans @COOORN
- Add lightbar control to G712L models: thanks @trmdnt
- Fix issue when detachable gear is disconnected
- Fix GPU hybrid/ultimate selection when only gpu_mux_mode is available: thanks @Ghoul4500
- Fix releasing of removed FDs: thanks @DrSAR
- Add support to control the XG Mobile LED
- Add aura support for G615LP: thanks @qwertyman100
- Remove supergfxctl completely
- Improve packaging and README.md: thanks @Owen-sz
- Add proper support for APU memory reservation
- Add aura support for FA608UH: thanks @Igilq
- Change CLI of slash

## [6.3.8]

### Changes

- Added slash control: thanks jijiking
- Disable GPU switching when unsupported
- Fix inotify watcher path: thanks @TheHeroBrine422
- Fix AniMe Matrix USB packet chunking: thanks @luca_pisl
- Start asusd when dmi contains ASUSLaptop: thanks @CongLuanTran
- Fix packet size for aura: thanks @sikuthenomad
- Fix DESTDIR handling in Makefile: thanks @martin-flaska
- Prevent setting custom TDP without a custom fan curve: thanks @Ghoul4500
- Fix `asusd.service` failing 226/NAMESPACE on fresh source installs by creating `/etc/asusd` before namespacing: thanks @scottishwidow
- Add support for 614ji, thanks @lucaszf
- Add support for GA403WR, thanks @ldcanetti94
- Add support for G614PR, thanks @scardracs

## [6.3.7]

### Changes

- Fix flicker of anime matrix when in powersave: thanks @barsikus007
- Add LEDs support for G733QS: thanks @ricoded-o00
- Fix hang at shutdown: thanks @evert
- Load fan profiles at start: thanks @rforced
- Start asusd when dmi is ZenBook: thanks @jason01-95
- Fix G713RS Lightbar support: thanks @Gokhan-Zengin
- Harden systemd profile: thanks @sashok724
- Fix keyboard lagging on newer kernels
- Fix GPU switching via GUI: thanks @evert
- Fix power governor not being set correctly: thanks @sashok724
- Improve make uninstall: thanks @htrex
- Make asusd start on ProArt PX13

## [6.3.6]

### Changes

- Switch GPU works better
- Reworked asus-shutdown to increase compatibility
- Fix a race condition in changing TDP values: thanks @Ghoul4500
- Fix an issue with aura config file: thanks @Ghoul4500
- Add slash support for GA403GM: thanks @dimeskigj
- Add support for G615JHR: thanks @absolutewisp and @Tomo-q3l
- Add aura support for GZ302: thanks @dm_zharov
- Fix README.md spelling: thanks @Bl4ckspell

## [6.3.5]

### Changes
- Fix nv_base_tgp being referenced as dgpu_base_tgp
- Add nv_tgp control slider
- Fix switching to LowPower on unplug
- add g615jm LEDs support: thanks @absolutewisp

## [6.3.4]

### Changes
- Fix ppt_pl3_sppt not being classified as a ppt attribute: thanks @evert
- Fix config not reloading due to mutex being improperly held: thanks @evert
- Fix GX650R missing lightbar: thanks @gaxompe

## [6.3.3]

### Changes
- Fix PPT sliders not updating
- add g614fm LEDs support: thanks @Tohid
- add ga403uv LEDs support: thanks @SPanner0
- add ga403ww LEDs support: thanks @foerderd
- add fx507vv: thanks @goqunatum30

## [6.3.2]

### Changes
- Improve the notification area, @shevchenko0013 strikes again!
- Improve firmware attributes handling
- Very good looking UI restyling from @shevchenko0013
- Added support for GA402NV matrix: thanks @Ghoul4500
- Fixed aura CLI interface
- Fixed LEDs handling in rogcc

## [6.3.1]

### Changes
- Removed a lighting mode that is unavailable in windows to G835L: thanks to @shevchenko0013 again!
- Added translations for Ukranian language, thanks @shevchenko0013!
- Added LEDs definition for G615LR, thanks @btnrv
- Fix improper usage of Quiet when only LowPower is available

## [6.3.0]

### Changed
- Added support for TUF keyboard powerstate control
- Improved AniMe Matrix support thanks to @Seom1177 !
- Fixed a bug with one-shot battery change, thanks @bitr8 !
- Changed the CLI interface of asusctl to be less confusing
- Added support for G835L, thanks to @shevchenko0013 !

## [6.2.0]

### Changed
- Added aura support for FX607V: thanks @jomp16
- Added testing support for G835LW
- Added support for GU605C models slash lighting: thanks @Otters
- Restore fedora: thanks @ali205412
- Add support to G614F models slash lighting

## [6.1.22]

### Changed
- Allow configuration of nv_tgp
- Treat dGPU attributes as power profiles
- Add EXPERTBOOK DMI match to ensure the service is loaded
- Support G815L thanks to @solost !

## [6.1.21]

### Changed
- Kill Fedora: screw your cursed cargo bullshit
- Restore CI building

## [6.1.20]

### Changed
- Addded support for G635L: thanks @luca_pisl !
- Suppress verbose output in applications too, not just daemon

## [6.1.18]

### Changed
- Add aura support for G614FR (ROG Strix G16 2025)
- all notifications now respects the timeout 
- improve udev daemon-starting rule
- reduce log noise

## [v6.1.17]

### Changed
- Fix Makefile
- Share a single HID device

## [v6.1.16]

### Changed
- Expose more properties via rog-control-center
- Add support for a few more models

## [v6.1.15]

### Changed
- Reflect the current asus-armoury status on AC plug connection status change

## [v6.1.14]

### Changed
- Fix formatting
- Attempt to fix tests

## [v6.1.13]

### Changed
- Fix a problem in reloading the service (@evertvorster)
- Add Azerbaijani language (@rashadgasimli)
- Add Ubuntu installation instructions

## [v6.1.12]

### Changed
- Fix an unbounded event loop caused by other processes causing a "modify" event on the screen backlight brightness.

## [v6.1.11]

### Changed
- Fix anime flickering issue when using custom anims (@I-Al-Istannen)
- Include pt_BR translations file (@PabloKiryu)

## Added
- Support for the screenpad brightness on some Laptops. This includes syncing to the primary screen brightness, and a gamma adjustment to set brightness scaling.
  - Add asusctl CLI options
  - Add UI options
  - Add a fake gamma correction (`asusctl backlight --sync-screenpad-brightness`, 1.5 for example sets screenpad low brightness lower than primary, and scales upwards)

### Changed
- asusd: single line fix for profile switching

## [v6.1.9]

### Changed
- ROGCC: better handling of platform profiles

## [v6.1.8]

### Changed
- Testing CI for opensuse RPM build
- ROGCC: Fixes to showing the PPT enablement toggle
- ROGCC: Fixes to how PPT and NV sliders work and enable/disable
- RGOCC: Fix quiet fan-curves availability

## [v6.1.7]

### Changed
- Fix Slash display enable

## [v6.1.6]

### Changed
- Disable skia bindings for UI again. It causes failures in build pipelines and requires extra dependencies.

## [v6.1.5]

### Changed
- Update dependencies
- Fix fan-curve proxy type signatures

## [v6.1.4]

### Changed
- Fix git doing me a dirty

## [v6.1.3]

### Changed
- Many small bugfixes such as for platform profile switching

## [v6.1.2]

### Changed
- Try a slightly different tact to fix charge control slider

## [v6.1.1]

### Changed
- Fix aura data matching
- Fix charge control slider

## [v6.1.0]

### Changed
- Update deps
- Add support for G513RC RGB modes
- Many UI fixes
- Fixes to PPT settings in UI

## [v6.1.0-rc7]

- Refactor PPT handling more:
  1. Per profile, per-ac/dc
  2. Do not apply unless group is enabled
  3. Better reset/disable handling
  4. Selecting a profile defaults PPT to off/disabled
- Bugfix: prevent an AniMe thread async deadlock

## [v6.1.0-rc6]

### Changed
- Two small fixes, one for `low-power` profile name, and one for base gpu tdp
- Move to using platform_profile api only (no throttle_thermal_policy)

## [v6.1.0-rc5]

### Changed
- Per-AC/DC, per-profile tunings enabled (Battery vs AC power +  platform profile)
- Add ability to restore PPT defaults
- Add PPT help dialogue to UI
- Bug fix: correctly set charge limit from UI

## [v6.1.0-rc4]

### Changed
- Bug fix: UI was setting incorrect value for FPPT
- Bug fix: Re-add callbacks for the throttle and epp settings in UI
- Bug fix: Fix UI settigns for AniMe Matrix display
- Bug fix: better handle missing tray (for example gnome)
- Strip out all outdated and unsafe tuning stuff
- Allow each performance profile to have different PPT settings

## [v6.1.0-rc3]

### Changed
- Bug fixes
- Partial support for per-profile CPU tunings (WIP)

## [v6.1.0-rc2]

### Added
- asus-armoury driver support. WIP, will be adjusted/changed further
- More "Slash" display controls

## [v6.1.0-rc1]

### Added
- ROG Arion external driver LED support
- Add GA605W LED layout
- Add GA605 + GU605 Slash support

### Changed
- Fix attribute writes. At some point the kernel API seems to have changed.
- Extremely large refactor of Aura device handling. Should enable easy add of different kinds now.
- Rename CLI args for aura related properties. This will likely change further as more devices are added

## [v6.0.12]

### Changed
- Add Ally X aura config
- Fixes to Ally led power configs
- Fix CLI led modes
- Add FX517Z to aura_support
- Add G614JJ Aura support
- Add G614JZ (2023 Strix G16) Aura support

## [v6.0.11]

### Changed
- Renamed `Strobe` effect to `RainbowCycle` to prevent confusion over what it is
- Ranamed `Rainbow` effect to `RainbowWave`
- Cleaned up serde crate deps
- Fixed AniMe on GA402XZ
- Update some of the G713 device aura features
- Update some of the G513 device aura features

## [v6.0.10]

### Added
- Add the GA401I model to aura_support.

### Changed

- Aura support return a default aura definition instead of nothing
- Minor updates in aura controller to ensure configs are updated if the support file changes
- Don't panic if -ENODEV on fan_curve enable
- Adjust the G513Q support to match what is on the asus website.
- Adjust init sequence of anime to prevent accidental use of Slash as Anime
- Enable notifs and tray icon without supergfx

## [v6.0.9]

### Added

- Add G513RS to laptop DB.
- Add G512LI to laptop DB.

### Changed

- Rename and recreate the default Anime config if cache setup fails

### Fixed

- Nuke the issue of GUI taking 100% of a CPU core

## [v6.0.8]

### Added
- Add G512L laptop DB entry

### Changed
- Add more tests to verify things

### Fix
- asusctl incorrectly assumes fan-curves unsupported. Now fixed.
- try to fix ROGCC using CPU time.

## [v6.0.7]

### Changed

- Add a config option to set if throttle policy is changed on ac/bat change (UI only)
- Allow X11 GUI. This is *not* supported. Please see readme.
- Fixes to some GUI widget layouts and sizing
- Do a backup HID raw write fro brightness if the read-back value does not match. This is a temporary solve for some G14 and G16 until the kernel patch is ready.
- Reimplement the older 0x1866 MCU keyboard control power bits plus UI control for it. If you had a keyboard affected by Lightbar issues and it is older than a couple of years this should help. If not, please file a bug.

## [v6.0.6]

### Added
- Add GX650R laptop to aura DB

### Changed
- Further tweaks to aura init
- More logging
- Fix TUF laptop led power
- Sanitize the dbus path for aura devices (remove `.` chars)
- Even more error handling (gracefully)
- Better checking for dbus interfaces
- Remove dbus `supported_interfaces`
- dbus ObjectManager is now at root `/`

## [v6.0.5]

### Changed

- Better more robust error handling in ROGCC
- Try to better handle pre-2021 laptops with lightbar
- Add more logging to try and trace the charge_control_end_threshold issue
- Make `kbd_brightness` optional to work around issues on some laptops that seem to lack it. Likely this will all need a refactor *again* if work proceeds in kernel for better RGB support.

## [v6.0.4]

### Changed

- Remove some `todo()` in rogcc
- Add missing `#[zbus(property)]` derive to Slash dbus properties
- Match G533Q laptop explicitly to the 0x8166 PID

## [v6.0.3]

### NOTE

- Xorg is not supported any longer. All major desktops and distros are Wayland.

### Changed

- Add a check to prevent non-TUF laptops with screwed up method return from TUF acpi methods from trying to add a TUF aura device without actually being a TUF laptop.
- Make the G834JZ entry in aura db generic for all G834J

## [v6.0.2]

### Changed

- Clean up code
- Don't panic if no aura device in ROGCC
- Try to prevent an errorenous tuf keyboard detection

## [v6.0.1]

### Added

- Support the G713P keyboard

### Changed

- Update all `Cargo.toml` to use workspace deps and remove unused features
- Update dependencies where possible
- Switch the UI to use fluent-dark instead of cosmic-dark due to extremely long compile time

## [v6.0.0]

### Important note

- The kernel patches from [here](https://lore.kernel.org/platform-driver-x86/20240404001652.86207-1-luke@ljones.dev/) are required. The ppt settings _will_ still apply without the patches but will be called a fail due to the read-back not being implemented (solved with kernel patch). These patches have been upstreamed for kernel 6.10
- Z13 devices will need these Z13 devices will need [these](https://lore.kernel.org/linux-input/20240416090402.31057-1-luke@ljones.dev/T/#t)

### Changed

- Upgrade to zbus 4.0.1
- Switch UI over to slint
- Add ability to start rog-control-center fullscreen with a width and height. This should be useful for devices like the ROG Ally.
- Many small changes due to requirements of slint UI

### Added

- Support GA402N keyboard
- Support GL553V keyboard
- Support GU605M keyboard
- Support Z13 lightbar (with kernel patch)
- Resupport the TUF keyboard

### BREAKING

- The aura dbus interface, and well pretty much all dbus interfaces have been changed. The Aura interface in particular works differently to begin implementing _multiple_ aura device support, including _hot-plug_ of devices (USB Aura keybords and others).
- All dbus interfaces except Aura are now in the `/org/asuslinux/` path
- Aura dbus now appear under `/org/asuslinux/<device>` and there may be multiple devices. To find these device you use the `ObjectManager` interface under the `/org/asuslinux` path.

## [v5.0.8]

### Changed

- Reintroduce persisting dark/light mode in config file
- Added ability to change what EPP is linked with each throttle profile
- Don't change EPP or thermal profile if the battery/ac state hasn't actually changed on resume
- Re-implement the `asusctl -s` command (not fully)
- Add more docs to some parts of code, and dbus interfaces
- Reload asusd.ron if changed. Does not notify any dbus listeners (yet)
- Fix the broken pipe error
- Remove the use of bytes in zbus signatures (another cause of broken pipe)

### Added

- Support for G614J LED modes

## [v5.0.7]

### Changed

- Fix to suspend process in anime thread to let custom anims run on wake.
- Fix to reload the fan curves correctly on boot.
- Add new config option `platform_policy_linked_epp` to set if energy_performance_preference should be paired with platform_profile/throttle_thermal_policy
- Small fixes to rog-control-center

## [v5.0.6]

- Revert egui update due to a lot of issues arising from window closing.

## [v5.0.5]

- Resync. A release was made that was missing some commits.

## [v5.0.4]

### Changed

- Added G834JZ led config
- Fix in ROGCC to apply the actual effect changed
- Re-enable all fan curves (available) in ROGCC
- Update smithay-client-toolkit

## [v5.0.3]

### Changed

- Fix and error in platform ppt value gets
- Fix to asusctl CLI where an incorrect enum variant was used in throttle check
- Turn some error messages in to warning or info to prevent confusion
- Re-add the keyboard power settings in rogcc
- Add two new aura dbus properties for providing some basic info on aura modes/power

## [v5.0.2]

### Changed

- Fan-curves: nuke a few async deadlocks
- Anime: force power/wakeup disabled to prevent idiotic random wakes

## [v5.0.1]

### Changed

- Fix setting next fan profile
- Fix the assud.service
- Fix dbus signature of some power setting types for some keyboards

## [v5.0.0]

### Added

- Gnome 45 plugin
- Support for G513RW LED modes
- Support Rog Ally LED modes (basic)
- Add on_lid_closed and on_external_power_changed events for running certain tasks
- Anime dbus: add:
  - SetOffWhenUnplugged, also add asusctl CLI option
  - SetOffWhenSuspended, also add asusctl CLI option
  - SetOffWhenLidClosed, also add asusctl CLI option
- Anime: add brightness_on_battery config option
- Platform: add `post_animation_sound`, kernel 6.7+ requires patch
- Add changing of CPU energy perfromance preference in relation to throttle*thermal_policy. This means that the CPU correctly behaves according to throttle_thermal_policy (and platform profile use is \_removed*)
- Add setting of throttle_thermal_policy on power plug/unplug

### Changed

- asusd: remove set_image_brightness for anime
- asusd: refactor how certain things like display enable/builtins are toggled
- Refactor sleep/shutdown tasks
- rog-control-center: ensure brightness slider works correctly
- Update `smithay-client-toolkit` for fix to issue #407
- Remove the "sleep" animations from Anime to stop preventing the display-off
- Anime:
  - Ensure display is off when lid is closed and option is set
  - Ensure display is off when on battery and option is set
  - Ensure builtin animations run instead of custom animations if option is set

### Breaking

- DBUS stuff. Again. All of it.

## [v4.7.2]

### Added

- Support for G733PZ LED modes
- Support for G713RC LED modes

### Changed

- Fix loading of fan curves from stored settings

## [v4.7.1]

### Changed

- Fixes to asusctl CLI tool to show fan curves
- Fixes to asusd to ensure fan curve defaults are loaded if the config file fails
- Further refine the asusctl CLI for fan-curve control
- Fixes to AniMe detection
- Fixes to aura config creation/loading

### Added

- Support for GV601V LED modes

## [v4.7.0]

### Added

- Support for FX507Z LED modes
- Support for GL503V LED modes
- Support for G733C LED modes
- Support for GV601VI LED modes
- Support for FX505G LED modes
- Support for GA402X LED modes
- Support for G634J LED modes (layout is in progress)
- Support the Rear Glow on some laptops
- Added field to aura_support to determine which LED power zones are supported. This will need folks to contribute data.
- Support M16 matrix display
  - Custom images
  - Pixel gifs
  - Power options
  - Builtin animations
- In-progress simulators for GA402, GU604 animatrix, optional build and takes a single arg
- Add `model_override` option to anime config, this is handy for forcing a model for "Unknown" anime, and for simulators
- Add `mini_led_mode` support to asusd and zbus crates (requires kernel patch https://lkml.org/lkml/2023/6/19/1264)
- Add `mini_led_mode` toggle to rog-control-center GUI, tray, notifications
- Add generation of typescript types from the rust types used via dbus using typeshare
- Add generation of introspection XML from asusd dbus
- Add a reworked gnome extension to the main repo under `desktop-extensions/gnome/`. This was done to better keep the extension in sync with work done on asusd, especially around breaking dbus
- Add support for the mid fan custom curves on some laptops

### Changed

- Move FX506HC to FX506H in arua DB to catch full series of this range
- Move FX506LH to FX506L in arua DB to catch full series of this range
- Move G513I\* to G513I in arua DB to catch full series of this range
- Remove notification handle tracking limit, fixes KDE issue with profile notif
- Rename daemon and daemon-user crates to asusd and asusd-user to not be confusing in workspace naming
- Prevent the multiple notifications from a profile change from occuring (too many functions with side effects!)
- Apply keyboard brightness when setting a mode
- Update GL503 led config
- Arua LED power control has been heavily refactored for 0x19b6+ devices
- Rog Control Center:
  - Added option to enable/disable system tray
  - Added button to fully quit app (exits from background)
  - Moved application settings to new page
  - Aura LED power refactor is now taken advantage of in RCC, exposing all settings

### BREAKING

- All Anime related DBUS methods/notifs are changed
- All dbus interfaces that handled an enum have now been forced to use the enum as String type, not uint or similar, this unfortunately breaks a heap of stuff but has the benefit of allowing asusctl to use crates to generate a typescript (or other) binding to the types being used by zbus for the proxies. The implication here is that there will be an eventual tighter integration with the gnome extension and maybe KDE also.

## [v4.6.2]

- Fix rog-control-center not reopening if `startup_in_background` is set

## [v4.6.1]

### Added

- Support for G733Z LED modes
- Support for GU604V LED modes
- Support for GX650P LED modes
- Support for GV604I LED modes
- Support for FX516P LED modes (this laptop still has further issues, will require a patched kernel when patch is ready)
- Add device code for the Z13 ACRNM keyboard (requires kernel patch, in progress)
- Support for GV301VIC LED modes
- Add device code for the plain Z13 keyboard (requires kernel patch, in progress)
- Support for GV301V LED modes

### Changed

- Adjustments to Anime system events thread
- Add "sleep" animetion config options to anime config
- rog-control-center dark/light mode persistency
- Adjustments to keyboard detection
- Better support of using supergfxctl when available (tray icon and menu)
- Check supergfx version before enabling use in tray (require 5.1.0+)
- Update allowed Aura modes on asusd restart if changed
- Set tray icon for dgpu to "On" if in Vfio mode to prevent confusion
- Add support for Logout/Reboot in notification for KDE

## [v4.6.0]

### Added

- Support for GL703GE keyboard layout
- Support for G533Z modes and keyboard layout

### Changed

- Better handling of `/etc/asusd` not existing
- Better handling of non-existant config files
- Move all config handling to generic traits for better consistency
- Re-parse all configs to RON format
- Move fan-curve config to own config file
- Added option to set `disable_nvidia_powerd_on_battery`
- Add short log entry to throttle_thermal_policy change detection
- ROGCC: Don't notify user if changing to same mux mode
- ROGCC: Add CLI opt for loading a keyboard layout for testing, with live-reload on file change
- ROGCC: Add CLI opt for viewing all layout files + filenames to help find a layout matching your laptop
  - Both of these options would hopefully be temporary and replaced with a "wizard" GUI helper
- Fix profile controller not detecting if platform_profile is changed
- Fix remove the leftover initial config writes on `new()` for some controllers to prevent resetting settings on startup
  - refactor the loading of systemd curve defaults and config file

### BREAKING

- Rename aura dbus method from `per_key_raw` to `direct_addressing_raw` and add doc comment
- Changes to aura.conf:
- Changes to asusd-ledmodes.toml:
  - Rename `standard` to `basic_modes`
  - Rename `multizone` to `basic_zones`
  - Raname `per_key` to `advanced` and change type from `bool` to `AdvancedAuraType`
  - Removed `prod_family`
  - Split all entries to `board_name` (separating `board_names`) (now a huge file)
  - removed `asusd-ledmodes.toml` in favour of `aura_support.ron` due to an unsupported type in toml
- Rename and adjust `LedSupportedFunctions` to closely match the above

## [v4.5.8]

### Changed

- Fix incorrect stop/start order of nvidia-powerd on AC plug/unplug

## [v4.5.7]

### Changed

- ROGCC: Don't notify user if changing to same mux mode
-

## [v4.5.7]

### Changed

- ROGCC: Don't notify user if changing to same mux mode
- asusd: don't block on systemd-unit change: removes all shoddy external command calls in favour of async dbus calls

## [v4.5.6]

### Changed

- Fix tasks not always running correctly on boot/sleep/wake/shutdown by finishing the move to async
- Change how the profile/fan change task monitors changes due to TUF laptops behaving slightly different
- ROGCC: Better handle the use of GPU MUX without supergfxd
- ROGCC: Track if reboot required when not using supergfxd
- Add env var for logging levels to daemon and gui (`RUST_LOG=<error|warn|info|debug|trace>`)
- ROGCC: Very basic support for running a command on AC/Battery switching, this is in config at `~/.config/rog/rog-control-center.cfg`, and for now must be edited by hand and ROGCC restarted (run ROGCC in BG to use effectively)
  - Run ROGCC from terminal to see errors of the AC/Battery command
  - Support for editing via ROGCC GUI will come in future
  - This is ideal for userspace tasks
- asusd: Very basic support for running a command on AC/Battery switching, this is in config at `/etc/asusd/asusd.conf`. A restart of asusd is not required if edited.
  - This is ideal for tasks that require root access (BE SAFE!)
- The above AC/Battery commands are probably best set to run a script for more complex tasks
- asusd: check if nvidia-powerd enabled before toggling

## [v4.5.5]

### Changed

- remove an unwrap() causing panic on main ROGCC thread

## [v4.5.4]

### Changed

- ROGCC:: Allow ROGCC to run without supergfxd
- ROGCC: Tray/notifs now reads dGPU status directly via supergfx crate (supergfxd not required)
- Add rust-toolchain to force minimum rust version

## [v4.5.3]

### Changed

- Adjust how fan graph in ROGCC works, deny incorrect graphs
- Fix to apply the fan curve change in ROGCC to the correct profile
- Support for G713RS LED modes (Author: Peter Ivanov)
- Support for G713RM LED modes (Author: maxbachmann)
- Fix VivoBook detection
- Update dependencies to get latest winit crate (fixes various small issues)

## [v4.5.2]

### Changed

- Update dependencies and bump version

## [v4.5.1]

### Added

- Support for FA506IE LED modes (Author: Herohtar)

### Changed

- Add a basic system tray with dGPU status and gpu mode switch actions
- Fixup some notifications in ROGCC
- Add config options for notifications for ROGCC
- Share states with tray process in ROGCC
- Share tates with tray process in ROGCC

## [v4.5.0]

### Added

- intofy watches on:
  - `charge_control_end_threshold`
  - `panel_od`
  - `gpu_mux_mode`
  - `platform_profile`
  - keyboard brightness
  - These allow for updating any associated config and sending dbus notifications.
- New dbus methods
  - `DgpuDisable`
  - `SetDgpuDisable`
  - `NotifyDgpuDisable`
  - `EgpuEnable`
  - `SetEgpuEnable`
  - `NotifyEgpuEnable`
  - `MainsOnline` (This is AC, check if plugged in or not)
  - `NotifyMainsOnline`
- `nvidia-powerd.service` will now enable or disable depending on the AC power state
  and on resume/boot (hybrid boot). It has been proven that this nvidia daemon can be
  problematic when on battery, not allowing the dgpu to suspend within decent time and
  sometimes blocking it completely.
- Notification to rog-control-center of dGPU state change

### Changed

- Use loops to ensure that mutex is gained for LED changes.
- asusctl now uses tokio for async runtime. This helps simplify some code.
- Properly fix notifs used in rog-control-center

### Breaking

- DBUS: all charge control methods renamed to:
  - `ChargeControlEndThreshold`
  - `SetChargeControlEndThreshold`
  - `NotifyChargeControlEndThreshold`
- DBUS: all panel overdrive methods renamed to:
  - `PanelOd` (from PanelOverdrive)
  - `SetPanelOd`
  - `NotifyPanelOd`
  - Path `/org/asuslinux/Charge` changed to `/org/asuslinux/Power`

## [v4.4.0] - 2022-08-29

### Added

- Support for per-key config has been added to `asusd-user`. At the moment it is
  basic with only a few effects done. Please see the manual for more information.
- Support for unzoned and per-zone effects on some laptops. As above.
- Added three effects to use with Zoned or Per-Key:
  - Static, Breathe, Flicker. More to come.
- Support for G713RS LED modes
- Support for TUF laptop RGB (kernel patches required, these are submitted upstream)

### Changed

- Create new rog-platform crate to manage all i/o in a universal way
  - kbd-led handling (requires kernel patches, TUF specific)
  - platform handling (asus-nb-wmi)
  - power (basic, can be extended in future)
  - hidraw
  - usbraw
- Refactor how ROGCC handles IPC for background open, run-in-bg
- Refactor daemon task creation to be simpler (for development)
- Rename dpu_only to gpu_mux. Update all related messages and info.

### Breaking

- DBUS: rename path `/org/asuslinux/RogBios` to `/org/asuslinux/Platform`
- DBUS: renamed `dedicated_graphic_mode` to `gpu_mux_mode` (`GpuMuxMode`)
- DBUS: renamed `set_dedicated_graphic_mode` to `set_gpu_mux_mode` (`SetGpuMuxMode`)
  - The methods above take an enum: 0 = Discrete, 1 = Optimus

## [4.3.4] - 2022-08-03

### Bugfix

- ROGCC: Remove power setting from correct array

## [4.3.3] - 2022-08-02

### Added

- `rog-control-center` has now been moved in to the main workspace due to
  the heavy dependencies on most of the rog crates
- Preliminary support of TUF RGB keyboards + power states
- Support for G713RW LED modes (Author: jarvis2709)
- Support for G713IC LED modes

### Changed

- The udev rules have been changed to make asusd load with all gamer variants when asus-nb-wmi is loaded
  - TUF, ROG, Zephyrus, Strix

## [4.3.0] - 2022-07-21

### Added

- Clear command for anime `asusctl anime --clear` will clear the display
- Re-added support for LED power states on `0x1866` type keyboards

### Changed

- Make rog-anime more error tolerent. Remove various asserts and return errors instead
- Return error if a pixel-gif is larger than the anime-display dimensions
- Both Anime and Aura dbus interfaces are changed a little
  - Aura power has changed, all power related settings are now in one method
  - Anime methods will now return an error (if errored)
  - /org/asuslinux/Led renamed to /org/asuslinux/Aura

## [4.2.1] - 2022-07-18

### Added

- Add panel overdrive support (autodetects if supported)
- Add detection of dgpu_disable and egpu_enable for diagnostic

### Changed

- Fixed save and restore of multizone LED settings
- Create defaults for multizone

## [4.2.0] - 2022-07-16

### Added

- Support for GA402 Anime Matrix display (Author: I-Al-Istannen & Luke Jones)
- Support for power-config of all LED zones. See `asusctrl led-power --help` (Author: Luke Jones, With much help from: @MNS26)
- Full support for multizone LED <logo, keyboard, lightbar> (Author: Luke Jones, With much help from: @MNS26)
- Add ability to load extra data from `/etc/asusd/asusd-user-ledmodes.toml` for LED support if file exits
- Support for G513IM LED modes
- Support for GX703HS LED modes

### Changed

- Dbus interface for Aura config has been changed, all power control is done with `SetLedsEnabled` and `SetLedsDisabled`
- Data for anime-matrix now requires passing the laptop model as enum
- Extra unit tests for anime stuff to help verify things

### Added

- Support for GA503R LED modes

### Changed

- Refactor LED and AniMe tasks
- Reload keyboard brightness on resume from sleep/hiber

## [4.1.1] - 2022-06-21

### Changed

- Fixes to anime matrix system thread cancelation

## [4.1.0] - 2022-06-20

### Changed

- Huge refactor to use zbus 2.2 + zvariant 3.0 in system-daemon.
- Daemons with tasks now use `smol` for async ops.
- Fixes to fan-curve settings from CLI (Author: Armas Span)
- Add brightness to anime zbus notification
- Adjust how threads in AniMe matrix controller work
- Use proper power-state packet for keyboard LED's (Author: Martin Piffault)

### Added

- Support for GA402R LED modes
- Support for GU502LV LED modes
- Support for G512 LED modes
- Support for G513IC LED modes (Author: dada513)
- Support for G513QM LED modes (Author: Martin Piffault)
- Add side-LED toggle support (Author: Martin Piffault)
- Support reloading keyboard mode on wake (from sleep/hiber)
- Support reloading charge-level on wake (from sleep/hiber)
- Support running AniMe animation blocks on wake/sleep and boot/shutdown events

# [4.0.7] - 2021-12-19

### Changed

- Fix incorrect power-profile validation
- Update asusd-ledmodes.toml to support Asus Rog Strix G15 G513QE (@LordVicky)
- Update patch notes and links

# [4.0.6] - 2021-11-01

### Changed

- Fix CLI for bios toggles

### Added

- Extra commands for AniMe: pixel-image, gif, pixel-gif

# [4.0.5] - 2021-10-27

### Changed

- Convert fan curve percentage to 0-255 expected by kernel driver only if '%' char is used, otherwise the expected range for fan power is 0-255
- Use correct error in daemon for invalid charging limit
- Enforce charging limit values in range 20-100

### Added

- LED modes for G513QR

# [4.0.4] - 2021-10-02

### Changed

- Add missing Profile commands
- Spawn tasks on individual threads to prevent blocking
- Don't force fan-curve default on reload
- Begin obsoleting the graphics switch command in favour of supergfxctl
- Slim down the notification daemon to pure ASUS notifications

# [4.0.3] - 2021-09-16

### Changed

- Don't show fan-curve warning if fan-curve available
- Add G713QR to Strix led-modes
- Fix part of CLI fan-curve control

# [4.0.2] - 2021-09-14

### Changed

- Backup old configs to \*-old if parse fails
- Prevent some types of crashes related to unpatched kernels
- Add better help for graphics errors
- Add better help for asusctl general errors
- Implement fan-curve dbus API
- Implement partial fan-curve control via CLI tool
  - Set fan curve for profile + fan gpu/cpu

# [4.0.1] - 2021-09-11

### Changed

- Fix asusd-ledmodes.toml

# [4.0.0] - 2021-09-10

### Added

- AniMe:
  - Support 8bit RGB, RGBA, 16bit Greyscalw, RGB, RGBA
  - add `AsusImage` type for slanted-template pixel-perfect images
  - `BREAKING:` plain `Image` with time period is changed and old anime configs break as a result (sorry)
- LED:
  - By popular request LED prev/next cycle is added
  - Add led modes for GX551Q

### BREAKING CHANGES

- Graphics control:
  - graphics control is pulled out of asusd and moved to new package; https://gitlab.com/asus-linux/supergfxctl
- Proflies:
  - profiles now depend on power-profile-daemon plus kernel patches for support of platform_profile
    - if your system supports fan-curves you will also require upcoming kernel patches for this
  - profiles are now moved to a new file
  - fan-curves are only partially completed due to this release needing to be done sooner

# [3.7.2] - 2021-08-02

### Added

- Enable multizone support on Strix 513IH
- Add G513QY ledmodes

### Changed

- Fix missing CLI command help for some supported options
- Fix incorrectly selecting profile by name, where the active profile was being copied to the selected profile
- Add `asusd` version back to `asusctl -v` report
- Fix various clippy warnings

# [3.7.1] - 2021-06-11

### Changed

- Refine graphics mode switching:
  - Disallow switching to compute or vfio mode unless existing mode is "Integrated"

# [3.7.0] - 2021-06-06

### Changed

- Set PM to auto for Nvidia always
- Extra info output for gfx dev scan
- Extra info in log for G-Sync to help prevent user confusion around gfx switching
- Add GA503Q led modes
- Added ability to fade in/out gifs and images for anime. This does break anime configs. See manual for details.
- Added task to CtrlLed to set the keyboard LED brightness on wake from suspend
  - requires a kernel patch which will be upstreamed and in fedora rog kernel
- Make gfx change from nvidia to vfio/compute also force-change to integrated _then_
  to requested mode
- Fix invalid gfx status when switching from some modes
- Fix copy over of serde skipped config values on config reload

# [3.6.1] - 2021-05-25

### Changed

- Bugfix: write correct fan modes for profiles
- Bugfix: apply created profiles

# [3.6.1] - 2021-05-25

### Changed

- Bugfix for cycling through profiles

# [3.6.0] - 2021-05-24

### Changed

- Add GX550L led modes
- Don't save compute/vfio modes. Option in config for this is removed.
- Store a temporary non-serialised option in config for if compute/vfio is active
  for informational purposes only (will not apply on boot)
- Save state for LEDs enabled + sleep animation enabled
- Save state for AnimMe enabled + boot animation enabled
- Add extra config options and dbus methods
- Add power state signals for anime and led
- Refactor to use channels for dbus signal handler send/recv
- Split out profiles independant parts to a rog-profiles crate
- Cleanup dependencies
- Fix some dbus Supported issues

# [3.5.2] - 2021-05-15

### Changed

- Bugfix: prevent the hang on compute/integrated mode change

# [3.5.1] - 2021-04-25

### Changed

- Anime:
  - Fix using multiple configs

# [3.5.0] - 2021-04-25

### Changed

- Keyboard:
  - Split out all aura functionality that isn't dependent on the daemon in to a
    new crate `rog-aura` (incomplete)
  - Keyboard LED control now includes:
    - Enable/disable LED's while laptop is awake
    - Enable/disable LED animation while laptop is suspended and AC plugged in
  - Properly reload the last used keyboard mode on boot
- Graphics:
  - Correctly enable compute mode for nvidia plus no-reboot or logout if switching
    from vfio/integrated/compute.
  - Add asusd config option to not save compute/vfio mode switch.
- Anime:
  - Enable basic multiple user anime configs (asusd-user must still be restarted)
- Profiles:
  - Enable dbus methods for freq min/max, fan curve, fan preset, CPU turbo enable.
    These options will apply to the active profile if no profile name is specified.

# [3.4.1] - 2021-04-11

### Changed

- Fix anime init sequence

# [3.4.0] - 2021-04-11

### Changed

- Revert zbus to 1.9.1
- Use enum to show power states, and catch missing pci path for nvidia.
- Partial user-daemon for anime/per-key done, `asusd-user`. Includes asusd-user systemd unit.
- user-daemon provides dbus emthods to insert anime actions, remove from index, set leds on/off
  - Config file is stored in `~/.config/rog/rog-user.cfg`
- AniMe display parts split out to individual crate in preparation for publishing
  on crates.io

# [3.3.0] - 2021-04-3

### Changed

- Add ledmodes for G733QS
- Add ledmodes for GA401Q
- Default to vfio disabled in configuration. Will now hard-error if enabled and
  the kernel modules are builtin. To enable vfio switching `"gfx_vfio_enable": false,`
  must be changed to `true` in `/etc/asusd/asusd.conf`

# [3.2.4] - 2021-03-24

### Changed

- Ignore vfio-builtin error if switching to integrated

# [3.2.3] - 2021-03-24

### Changed

- Better handling of session tracking

### Added

- List all profile data
- Get active profile name
- Get active profile data

# [3.2.2] - 2021-03-23

### Changed

- Fix brightness control, again, for non-RGB keyboards

# [3.2.1] - 2021-03-21

### Changed

- Fix brightness control
- Large cleanup of code relating to LED controls

# [3.2.0] - 2021-03-21

### Changed

- Refactor keyboard LED handling
- Added --list for profiles (Thanks @aqez)
- Added --remove for profiles (Thanks @aqez)
- Added a graphics mode: vfio. This attaches Nvidia devices to vfio module.

### Broken

- Per-key LED modes, which need thinking about how to go ahead with for future

# [3.1.7] - 2021-03-11

### Changed

- Refactor many parts of daemon
- Switch out session monitoring to logind-zbus

# [3.1.6] - 2021-03-11

### Changed

- Graphics switching will now wait until all users logged out before switching

### Changed

- Further tweaks to gfx switching
- More logging on gfx switching
- Filter bios help according to supported modes
- Prevent gfx mode switching if in dedicated/G-Sync mode

# [3.1.4] - 2021-03-10

### Changed

- Notify through dbus if user changes profile manually
- Better help on CLI, show help only for supported items
- Bugfix to gfx switcher

# [3.1.3] - 2021-03-10

### Changed

- Hotfix: gracefully handle removing modules in use caused by display-manager not
  fully shutdown at the time of trying to remove modules. It will now retry every
  250ms per module

# [3.1.2] - 2021-03-10

### Changed

- Test and create /etc/X11/xorg.conf.d/ if it doesn't exist
- Hotfix to better report module issues

# [3.1.1] - 2021-03-10

### Changed

- Add missing nvidia module nvidia_uvm to gfx ctrl list

# [3.1.0] - 2021-03-09

### Added

- GU502LU led-modes

### Changed

- Graphics switching is now rebootless, the daemon will now restart the
  display-manager to switch modes instead. Caveats are:
  - There is no confirmation from the daemon, the program issuing the command
    must confirm the request.
  - systemd only
- Laptops with dedicated Nvidia mode:
  - You still must reboot for the bios to switch modes
  - On boot if dedicated mode is active then asusd will update the required configs
    to put display-manager in nvidia mode

# [3.0.0] - 2021-02-22

### Added

- G531GD led modes

# [3.0.0] - 2021-02-14

### Changed

- Write set+apply after each array in multizone
- Remove misc bad logic
- Use same code path as 0x1866 device to configure led support for 0x1854 device
- Remove duplicate code
- Set correct speeds for multizone
- Remove dbus crate in favour of zbus. This removes the external dbus lib requirement.
- Huge internal refactor
- BREAKING CHANGE: Anime code refactor. DBUS method names have changed
- Cleanup fan and cpu control + configs

# [2.2.2] - 2021-01-31

### Changed

- Fix for dedicated gfx capable laptops in integrated mode
- Fix for 0x1854 device

# [2.2.1] - 2021-01-27

### Added

- Add ROG Zephyrus M15 LED config

### Changed

- Bugfixes
- Fix reboot/restartx status for GFX switching
- Update readme
- Change CLI arg tag for fan modes
- Make dracut include the nvidia modules in initramfs

# [2.2.0] - 2021-01-26

### Added

- Dbus command to fetch all supported functions of the laptop. That is, all the
  functions that asusd supports for the currently running laptop.
- Bios setting toggles for:
  - Dedicated gfx toggle (support depends on the laptop)
  - Bios boot POST sound toggle

### Changed

- added config option for dedicated gfx mode on laptops with it to enable
  switching directly to dedicated using `asusctl graphics -m nvidia`

# [2.1.2] - 2021-01-10

### Changed

- Adjust gfx controller to assume that the graphics driver is loaded if the
  mode is set for nvidia/hybrid

# [2.1.1] - 2021-01-09

### Changed

- Updates to dependencies

# [2.1.0] - 2020-10-25

### Added

- Option to turn off AniMe display (@asere)

### Changed

- Change option -k to show current LED bright (@asere)
- Correctly disable GFX control via config
- Panic and exit if config can't be parsed
- Add DBUS method to toggle to next fan/thermal profile
- Add DBUS method to toggle to next/prev Aura mode

# [2.0.5] - 2020-09-29

### Changed

- Bugfixes

# [2.0.4] - 2020-09-24

### Changed

- Better and more verbose error handling and logging in many places.
- Fix timeout for client waiting on reply for graphics switching

# [2.0.2] - 2020-09-21

### Changed

- graphics options via CLI are now a command block:
  - `asusctl graphics`
  - -m Mode <nvidia, hybrid, compute, integrated>
  - -g Get current mode
  - -f Force reboot or restart display manager without confirmation

# [2.0.0] - 2020-09-21

### Changed

- Code refactor to spawn less tasks. Main loop will run only as fast as
  it receives events
- No-longer using tokio or async, reducing resource use

### Added

- A basic user daemon has been added for user notifications over dbus (XDG spec)
- Added a user systemd service for notifications (asus-notify)
- Graphics mode handling <iGPU only, dGPU only, or hybrid>, see asusctl --help

### BREAKING CHANGES

- asusd.conf has changed slightly and will overwrite old configs
- All DBUS methods/signals/paths etc, are all updated and changed

# [1.1.2] - 2020-09-10

### Changed

- Bump rog-fan-curve to new versiont o support GA401IV

# [1.1.1] - 2020-09-10

### Changed

- Correction to AMD turbo setting

# [1.1.0] - 2020-09-10

### Changed

- Uses string instead of debug print for some errors
- Add interface num arg for LED controller (should help support
  older laptops better)
- Some slightly better error messages
- Fix an idiotic mistake in `for i in 0..2.. if i > 0` -\_-
- Remove "unsupported" warning on laptop ctrl
- Silence warning about AniMe not existing
- Adjust the turbo-toggle CLI arg
- Version bump for new release with fancurves

## [1.0.2] - 2020-08-13

### Changed

- Bugfixes to led brightness watcher
- Bufixes to await/async tasks

## [1.0.1] - 2020-08-13

- Fix small deadlock with awaits

## [1.0.0]
