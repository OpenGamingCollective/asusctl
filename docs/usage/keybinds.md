# Keybinds

> `asusctl` and `asusd` do not install global desktop shortcuts. The firmware, the kernel, and your desktop environment handle hardware function keys; if a key does nothing, bind it to an `asusctl` command yourself.

## No default shortcuts

There is no asusctl keybind daemon and no default shortcut list. The only optional shortcut in the project is ROG Control Center's window show/hide shortcut, which is disabled by default and is not a hardware action. ROG Control Center registers it through the XDG `GlobalShortcuts` portal as `toggle_rog` ("Open/Close ROG Control Center") and controls it with the `enable_global_shortcut` setting in its own config.

## Commands to bind

| Action | Command |
|---|---|
| Next platform profile | `asusctl profile next` |
| Next built-in Aura mode | `asusctl aura effect --next-mode` |
| Previous built-in Aura mode | `asusctl aura effect --prev-mode` |
| Next keyboard brightness | `asusctl leds next` |
| Previous keyboard brightness | `asusctl leds prev` |

Run these as your normal user. `asusctl` talks to the system daemon over D-Bus; do not prefix them with `sudo`.

The D-Bus policy allows the `adm`, `sudo`, `users`, and `wheel` groups. If your user is in none of them, `asusctl` fails with `AccessDenied`. On systems where `wheel` grants sudo, joining that group also grants administrative privileges; check the group policy before adding the user.

`asusctl profile next` uses a fixed order: `Balanced` → `Performance` → `LowPower` if the kernel exposes it, otherwise `Quiet` → `Balanced`. It never selects `Custom`. If `Performance` is unavailable, the command can fail.

`asusctl leds next` goes `Off` → `Low` → `Med` → `High` → `Off`; `asusctl leds prev` reverses that order. `asusctl aura effect --next-mode` and `--prev-mode` wrap around the supported mode list.

## Fn+F5

On many ASUS laptops Fn+F5 is the key used to switch platform profiles. On supported machines, `asus-wmi` handles the key and cycles profiles itself, and the event does not reach the desktop. Check whether Fn+F5 already works before binding a desktop shortcut. If it does not, bind it to:

```bash
asusctl profile next
```

The distribution guides recommend this binding. If your model does not emit a key event for Fn+F5, or the firmware reserves it, choose a different key combination.

If the kernel exposes `low-power` instead of `quiet`, `profile next` uses `LowPower`; `asusd` treats the two names as aliases when it applies a profile.

To control keyboard backlight brightness instead, bind a key to `asusctl leds next` or `asusctl leds prev`.

## Aura modes

`asusctl aura effect --next-mode` and `--prev-mode` cycle the built-in factory modes supported by the keyboard. The list comes from the keyboard's support data; use `asusctl info --show-supported` to see it. If the current mode is not in that list, `--next-mode` selects the first mode and `--prev-mode` selects the last.

Custom Aura effects are different. They are `ConfigAura` files played by `asusd-user` (see [Aura effects](../configuration/asusd-user.md#aura-effects)). While a custom sequence is active, `asusd-user` continuously writes keyboard packets, which can override built-in mode changes. Set `active_aura: None` in `rog-user.ron` and restart `asusd-user.service` to return control to the built-in modes. The daemon does not select a built-in mode at startup, so run a built-in mode command such as `asusctl aura effect --next-mode` afterward.

## Desktop setup

The exact dialog depends on the desktop, but every desktop that supports custom shortcuts can run a command.

KDE Plasma: open System Settings → Keyboard → Shortcuts, choose Add New → Command or Script, enter `asusctl profile next`, and assign the key.

GNOME: open Settings → Keyboard → Custom Shortcuts (the exact label varies by GNOME version), add a shortcut with the command `asusctl profile next`, and assign the key.

For other desktops, use the equivalent custom-shortcut or hotkey dialog. No root access is required.

## See also

- [asusctl CLI reference](asusctl.md): all commands and options.
- [asusd-user Aura effects](../configuration/asusd-user.md#aura-effects): custom per-key and zoned effects.
- [Power Profiles Daemon](../configuration/asusd.md#power-profiles-daemon): choosing whether `asusd` or an external daemon owns platform profiles.

## Source of truth

- [`asusctl/src/cli_opts.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/asusctl/src/cli_opts.rs): `profile next`, `aura effect`, and `leds next`/`prev`.
- [`asusctl/src/main.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/asusctl/src/main.rs): command handling and built-in mode cycling.
- [`rog-control-center/src/config.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-control-center/src/config.rs) and [`rog-control-center/src/shortcuts.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-control-center/src/shortcuts.rs): optional ROG Control Center window shortcut.
