# Configuration Overview

`asusd` and `asusd-user` keep their state in [RON](https://ron-rs.github.io/ron/) files. RON (Rusty Object Notation) maps directly to the Rust structs the daemons use, so field names in the files match the names in the source. RON supports `//` comments and trailing commas.

There are two configuration roots:

| Scope | Directory | Used by |
|---|---|---|
| System | `/etc/asusd/` | `asusd`, running as root |
| User | `$XDG_CONFIG_HOME/rog/` (normally `~/.config/rog/`) | `asusd-user`, running as your user |

System configuration controls hardware-wide behaviour: platform profiles, battery charging, fan curves, the built-in Aura modes, and the AniMe Matrix display. User configuration holds the optional per-user Aura and AniMe sequences that `asusd-user` plays.

## Programs

| Binary | Role | Started by |
|---|---|---|
| `asusd` | System daemon. Owns the hardware and exposes it on the system D-Bus as `xyz.ljones.Asusd`. | The `asusd.rules` udev rule starts `asusd.service` when an `asus-nb-wmi` device appears. The unit is `Type=dbus`; do not enable it manually. |
| `asus-shutdown` | Deferred shutdown handler. Applies pending ASUS GPU firmware settings (such as `dgpu_disable`) after the GPU is idle, then releases the shutdown inhibitor. | The same udev rule starts `asus-shutdown.service`; the unit requires `asusd.service`. |
| `asusd-user` | Per-user daemon for custom Aura and AniMe sequences. | A user service: `systemctl --user enable --now asusd-user.service`. |
| `asusctl` | Command-line client for `asusd`. | Run by the user. |
| `rog-control-center` | Graphical client for `asusd`. | Run by the user. |

> [!NOTE]
> `asusd` is triggered by udev and is not meant to be enabled with `systemctl enable`. Check it with `systemctl status asusd.service` and read its logs with `sudo journalctl -b -u asusd`.

## Configuration files

| File | Owner | Reload behaviour |
|---|---|---|
| `/etc/asusd/asusd.ron` | `asusd` (root) | Watched with inotify and reloaded when the file changes. |
| `/etc/asusd/fan_curves.ron` | `asusd` (root) | Read at startup; restart `asusd.service` after editing. |
| `/etc/asusd/anime.ron` | `asusd` (root) | Read at startup; restart `asusd.service` after editing. |
| `/etc/asusd/scsi.ron` | `asusd` (root) | Read at startup; restart `asusd.service` after editing. Only used on laptops with SCSI drive LEDs. |
| `/etc/asusd/slash.ron` | `asusd` (root) | Read at startup; restart `asusd.service` after editing. Only used on laptops with a Slash LED bar. |
| `/etc/asusd/aura_<product_id>.ron` | `asusd` (root) | Generated and rewritten by `asusd`; do not edit. |
| `$XDG_CONFIG_HOME/rog/rog-user.ron` | `asusd-user` | Read at startup; restart `asusd-user.service` after editing. |
| `$XDG_CONFIG_HOME/rog/<name>.ron` | `asusd-user` | Read at startup; restart `asusd-user.service` after editing. |

`$XDG_CONFIG_HOME` defaults to `~/.config`. Replacing `asusd.ron` while another hard link points to the old inode leaves the watcher on the old inode; later edits can be missed. Restart `asusd.service` after such a replacement.

`aura_<product_id>.ron` is generated from the support database and the detected keyboard. Do not hand-edit it; use `asusctl aura` instead. Edit the other files while the relevant service is stopped. `asusd.ron` is watched with inotify, but editing it while `asusd` is running can reset `base_charge_control_end_threshold`; stop `asusd.service` before editing. Stopping `asusd.service` also stops `asus-shutdown.service` (`PartOf=`), so deferred GPU changes are discarded; queue them again after restarting the service.

`asusd` creates `scsi.ron` and `slash.ron` on the first start on a machine that exposes the matching device. They are ordinary RON files you can edit by hand; see [SCSI LEDs](asusd.md#scsi-leds) and [Slash LED bar](asusd.md#slash-led-bar).

> [!NOTE]
> The udev rule only starts `asusd.service` and `asus-shutdown.service` on ASUS systems whose DMI family contains `TUF`, `ROG`, `Zephyrus`, `Strix`, `Vivobook`, `ASUSLaptop`, `Zenbook`, `ProArt`, `TX Air`, `TX Gaming`, or `EXPERTBOOK`. On other ASUS models the units are not started automatically. The system unit is also sandboxed: it creates `/etc/asusd` itself, and its persistent write paths are `/etc/asusd` and `/sys`. `asus-shutdown.service` is `PartOf=asusd.service`, requires it, and has a 45-second stop timeout.

## Back up before editing

Keep a copy of the directory before making changes:

```bash
sudo cp -a /etc/asusd "/etc/asusd.bak.$(date +%F-%H%M%S)"
cp -a "${XDG_CONFIG_HOME:-$HOME/.config}/rog" "${XDG_CONFIG_HOME:-$HOME/.config}/rog.bak.$(date +%F-%H%M%S)"
```

For `asusd.ron`, `asusd` rewrites the file when it saves state, so a backup is the quickest way to recover from a bad edit.

## Invalid RON and migration

The `config-traits` crate loads the configuration files. When a file cannot be deserialized as the current format, the loader tries the known older formats. If none match, it renames the file to `<file>-old`, writes a fresh default file, and continues with the defaults. For example, a broken `/etc/asusd/asusd.ron` becomes `/etc/asusd/asusd.ron-old` and a new `asusd.ron` is created.

To recover the old values, stop the relevant service. Repair a copy of the `-old` file first; copying it back unchanged will fail deserialization again. Copy the repaired file over the generated file. For `asusd.ron`, start `asusd.service` and `asus-shutdown.service` again so deferred GPU changes apply at shutdown. An existing `-old` file is overwritten. If `-old` is a directory or the file is not valid UTF-8, the rename does not happen and the file is replaced with defaults. Keep a separate backup.

An invalid named Aura or AniMe file is reset to the `aura-default` or `anime-default` name and overwrites that default sequence file. Back up both the custom file and the corresponding default file before restarting. To recover the custom sequence, restore the repaired file under its original custom name, not over `aura-default.ron` or `anime-default.ron`.

The rename and the fresh default happen when a service loads the file at startup. For `asusd.ron`, an invalid edit made while the daemon is running is logged and the previous values are kept; the inotify watcher uses `read_new`, which returns `None` on a parse failure instead of replacing the in-memory config.

> [!WARNING]
> A malformed file is not rejected line by line: the whole file must deserialize. A missing required field makes the current format fail; an older format may still match, and migration can overwrite current settings without creating an `-old` copy. Keep a separate backup.

The loader creates the configuration directory if it does not exist (for example `/etc/asusd` or `~/.config/rog`). If the directory cannot be created, the service panics with the path in the error message.

When a `.ron` file does not exist, `config-traits` also looks for a legacy file with the same stem and a `.cfg` or `.conf` extension, and renames it to `.ron`. For example, `asusd.cfg` is renamed to `asusd.ron`. The contents are not converted; they must already be valid RON.

Two cases are handled differently from a syntax error:

- An empty file is not renamed to `-old`. The loader logs a zero-length read and writes a fresh default over the empty file.
- A file that parses is still rewritten in pretty RON after loading. Comments (`//`), blank lines, and custom formatting are not preserved, so keep notes outside the file or in a backup.

Writes use a temporary file and rename it over the live file. A crash during a write leaves the previous file intact, and you can delete a leftover `.ron.tmp`. This does not protect against power loss, and two writers using the same temporary path can race; do not run two instances against the same file.

## Pages

- [asusd configuration](asusd.md): `asusd.ron`, power profiles, fan curves, AniMe, Aura, and Armoury.
- [asusd-user configuration](asusd-user.md): user service, `rog-user.ron`, custom Aura effects, and custom AniMe sequences.
- [AniMe sequences](anime-sequences.md): the `ActionLoader` format shared by `anime.ron` and user sequence files.

## Source of truth

- [`config-traits/src/lib.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/config-traits/src/lib.rs): RON loading, `.cfg`/`.conf` migration, and `-old` recovery.
- [`asusd/src/config.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/asusd/src/config.rs): `asusd.ron` fields and defaults.
- [`asusd-user/src/config.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/asusd-user/src/config.rs): `rog-user.ron`, `ConfigAura`, and `ConfigAnime`.
- [`data/asusd.rules`](https://github.com/OpenGamingCollective/asusctl/blob/main/data/asusd.rules), [`data/asusd.service`](https://github.com/OpenGamingCollective/asusctl/blob/main/data/asusd.service), [`data/asusd-user.service`](https://github.com/OpenGamingCollective/asusctl/blob/main/data/asusd-user.service), and [`data/asus-shutdown.service`](https://github.com/OpenGamingCollective/asusctl/blob/main/data/asus-shutdown.service): service startup.
