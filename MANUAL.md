# asusctrl manual

**NOTE:** this manual is in need of an update in some places. If you find issues please file issue reports.

`asusd` is a utility for Linux to control many aspects of various ASUS laptops
but can also be used with non-asus laptops with reduced features.

## Programs Available

- `asusd`: The main system daemon. It is autostarted by a udev rule and systemd unit.
- `asusctl`: The CLI for interacting with the system daemon
- `rog-control-center`: Optional GUI for `asusd`

## `asusd`

`asusd` is the main system-level daemon which will control/load/save various settings in a safe way for the user, along with exposing a _safe_ dbus interface for these interactions. This section covers only the daemon plus the various configuration file options.

The functionality that `asusd` exposes is:

- anime control
- led keyboard control (aura)
- charge limiting
- bios/efivar control
- power profile switching
- fan curves (if supported, this is auto-detected)

each of these will be detailed in sections.

### AniMe control

Controller for the fancy AniMe matrix display on the lid of some machines. This controller is a work in progress.

#### Config options

If you have an AniMe device, system-level options live in `/etc/asusd/anime.ron`:

1. `system: []`: continuous playlist while its loop is running (replaces the old `asusd-user` session daemon). Start it with the D-Bus method `RunMainLoop(true)` on `xyz.ljones.Anime`, and call it again after restarting `asusd` or resuming from suspend.
2. `boot: []`: sequence that plays on system boot (when asusd is loaded)
3. `wake: []`: sequence that plays when waking from suspend
4. `shutdown: []`: sequence that plays when shutdown begins
5. Display / builtin animation toggles and brightness (see the generated defaults on first run)

Each list entry is an `ActionLoader` from `rog-anime` (AsusAnimation, AsusImage, ImageAnimation, Image, Pause). Paths must be absolute.

### Migrating from `asusd-user` (removed)

`asusd-user` and `~/.config/rog/` user playlists are removed. To keep a continuous AniMe sequence:

1. Ensure your account can talk to `asusd` on the system bus. The caller of `RunMainLoop` must be `root` or a member of `adm`, `sudo`, `users`, or `wheel` (see `data/asusd.conf`). Package installation does not add you to those groups. Check with `groups`; if none match, add one that exists on your distro (for example `users` or `wheel`) and re-login:

```
sudo usermod -aG users "$USER"
```

2. Open the old user file (for example `~/.config/rog/anime-default.ron`) and copy the `anime: [...]` array into `/etc/asusd/anime.ron` as `system: [...]` (same `ActionLoader` shape). Paths must stay absolute. Editing `/etc/asusd/` needs root (`sudoedit /etc/asusd/anime.ron`).

3. If you want only the custom playlist, disable firmware builtins, restart `asusd` so it reloads the cached `system` playlist, then start the loop:

```
asusctl anime --enable-powersave-anim false
sudo systemctl restart asusd
busctl --system call xyz.ljones.Asusd /xyz/ljones/aura/anime xyz.ljones.Anime RunMainLoop b true
```

Call `RunMainLoop` again after every `asusd` restart or resume from suspend. There is no `asusctl` subcommand for that method yet; use `busctl` (or any D-Bus client) as above.

Per-key / zoned Aura effect loops that lived under `asusd-user` are not stored in `aura_*.ron`. Use `asusctl` examples that call `direct_addressing_raw`, or drive that D-Bus method yourself. `/etc/asusd/asusd_user_ledmodes.ron` is unrelated — it remains the optional LED-mode override for `asusd`.

### Led keyboard control

The LED controller (e.g, aura) enables setting many of the factory modes available if a laptop supports them. It also enables per-key RGB settings but this is a WIP and will likely be similar to how AniMe sequences can be created.

#### Supported laptops

There are over 80 supported laptops as of 01-01-2023. Please see [the rog-aura crate readme for further details](/rog-aura/README.md).

### Charge control

Almost all modern ASUS laptops have charging limit control now. This can be controlled in `/etc/asusd/asusd.conf`.

```json
"bat_charge_limit": 80,
```

where the number is a percentage.

### Bios control

Some options that you find in Armory Crate are available under this controller, so far there is:

- POST sound: this is the sound you hear on bios boot post
- GPU MUX: this controls if the dGPU is the _only_ GPU, making it the main GPU and disabling the iGPU

These options are not written to the config file as they are stored in efivars. The only way to change these is to use the exposed safe dbus methods, or use the `asusctl` CLI tool.

### Profiles

asusctl supports setting power profiles via ACPI `platform_profile` drivers.

> [!IMPORTANT]
> **Compatibility with Power Profiles Daemons**:
> Running an external power profiles daemon (such as `power-profiles-daemon` or `tuned`) concurrently with `asusd`'s profile management can cause race conditions and contention over `/sys/firmware/acpi/platform_profile` and CPU EPP preferences.
> 
> You have two options for managing power profiles:
> 
> 1. **Allow `asusd` to manage profiles**: Disable the conflicting service (e.g. `power-profiles-daemon`):
> 
>    ```bash
>    sudo systemctl disable --now power-profiles-daemon.service
>    ```
> 
> 2. **Allow an external daemon (PPD/Tuned) to manage profiles**: Keep `power-profiles-daemon` active, and disable `asusd`'s profile management settings in `/etc/asusd/asusd.ron`:
> 
>    ```ron
>    change_platform_profile_on_ac: false,
>    change_platform_profile_on_battery: false,
>    platform_profile_linked_epp: false,
>    ```

A common use of asusctl is to bind the `fn+f5` (fan) key to `asusctl profile -n` to cycle through the 3 profiles:

1. Balanced
2. Performance
3. Quiet

#### Fan curves

Fan curve support requires a laptop that supports it (this is detected automatically) and the kernel patch from [here](https://lkml.org/lkml/2021/10/23/250) which is accepted for the 5.17 kernel release .

The fan curve format can be of varying formats:

- `30c:0%,40c:5%,50c:10%,60c:20%,70c:35%,80c:55%,90c:65%,100c:65%"`
- `30:0,40:5,50:10,60:20,70:35,80:55,90:65,100:65"`
- `30 0,40 5,50 10,60 20,70 35,80 55,90 65,100 65"`
- `30 0 40 5 50 10 60 20 70 35 80 55 90 65 100 65"`

the order must always be the same "temperature:percentage", lowest from left to rigth being highest.

The config file is located at `/etc/asusd/profile.conf` and is self-descriptive. On first run it is populated with the system EC defaults.

### Support controller

There is one more controller; the support controller. The sole pupose of this controller is to querie all the other controllers for information about their support level for the host laptop. Returns a json string.

## Sequence format reference (AniMe)

Entries in `system`, `boot`, `wake`, and `shutdown` share the same `ActionLoader` variants.
The examples below use JSON-like shape for readability; on disk the config is RON.

### Action types

1. AsusAnimation — ASUS-style gifs under `/usr/share/asusd/anime/asus/`
2. AsusImage — slanted PNG templates (for example `diagonal-template.png`)
3. ImageAnimation — arbitrary GIF with scale / angle / translation
4. Image — still image with the same transform fields
5. Pause — hold for a duration

File paths must be absolute. Brightness is typically `0.0`–`1.0` and combines with the daemon global brightness. Time may be a duration, a loop count, `Infinite`, or a `Fade` with `fade_in` / `show_for` / `fade_out`.

See `rog-anime` and the defaults written into `/etc/asusd/anime.ron` on first run for concrete RON examples.

## asusctl

`asusctl` is a commandline interface which intends to be the main method of interacting with `asusd`. It can be used in any place a terminal app can be used.

This program will query `asusd` for the `Support` level of the laptop and show or hide options according to this support level.

Most commands are self-explanatory.

### CLI Usage and help

Commands are given by:

```
asusctl <option> <command> <command-options>
```

Help is available through:

```
asusctl --help
asusctl <command> --help
```

Some commands may have subcommands:

```
asusctl <command> <subcommand> --help
```

### Keybinds

To switch to next/previous Aura modes you will need to bind both the aura keys (if available) to one of:
**Next**

```
asusctl aura -n
```

**Previous**

```
asusctl aura -p
```

To switch Fan/Thermal profiles you need to bind the Fn+F5 key to `asusctl profile -n`.

# License & Trademarks

Mozilla Public License 2 (MPL-2.0)

---

ASUS and ROG Trademark is either a US registered trademark or trademark of ASUSTeK Computer Inc. in the United States and/or other countries.

Reference to any ASUS products, services, processes, or other information and/or use of ASUS Trademarks does not constitute or imply endorsement, sponsorship, or recommendation thereof by ASUS.

The use of ROG and ASUS trademarks within this website and associated tools and libraries is only to provide a recognisable identifier to users to enable them to associate that these tools will work with ASUS ROG laptops.

---
