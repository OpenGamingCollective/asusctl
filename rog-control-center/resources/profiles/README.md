# Keyboard profiles

One YAML file per laptop model, registered as GNOME custom shortcuts by
`rog-control-center --setup-keyboard`.

## File name

The file name is the model identifier with spaces replaced by underscores,
e.g. `ROG_Strix_SCAR_18_G835LXG.yaml`. On startup the loader tries
`product_name`, `board_name` and `product_family` from `/sys/class/dmi/id`,
in that order, and uses the first one that matches a file.

To find yours:

    cat /sys/class/dmi/id/product_name

## Format

```yaml
name: "ROG Strix SCAR 18 G835LXG"
product_code: "G835LXG"
description: "Keyboard shortcuts for ROG Strix SCAR 18 G835LXG"
version: "1.0"

keyboard_shortcuts:
  - id: "control-center"          # unique, kebab-case, no `rog-` prefix
    name: "ROG Control Center"    # shown in GNOME Settings
    description: "Open ROG Control Center"
    command: "rog-control-center" # executed directly, not through a shell
    keybinding: "XF86Launch1"     # see below
    enabled: true                 # defaults to false when omitted
```

`command` is run without a shell, so pipes, `&&` and variable expansion do
not work, and `$PATH` is the session's, not your terminal's.

## Finding a keybinding

Run `xev -event keyboard` (X11) or `wev` (Wayland), press the key, and use
the keysym it reports. Modifiers use GTK syntax: `<Super>k`, `<Ctrl><Alt>t`.

## Local profiles

Files in `~/.config/rog-control-center/profiles/` take precedence over the
shipped ones, so you can test a profile without rebuilding.
