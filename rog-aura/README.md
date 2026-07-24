# rog-aura

## What is it?

rog-aura is a helper crate for interacting with the RGB keyboards found on many ASUS ROG gaming laptops such as the Zephyrus, Strix, TUF, and a few others.

The crate is primarily used in the asusctl suite of tools.

The majority of the crate deals with converting from the API to USB packets suitable for sending raw to the USB device.

## Features

- Detect USB keyboard type, or if the kayboard is an I2C connected one (typical on TUF)
- Set various basic modes
- Set various basic zones
- Set advanced/direct addressing of:
  - Single zone
  - Multizone
  - Per-key
- Physical layout mapping

## Config files

The crate includes config files for helping to determine what laptop models support what feature. This is heavily dependant on folks testing and contributing data.

It also includes layouts for some laptops. Also heavily dependant on contributions.

# Support list

`aura_support.toml` is the support listing file. It functions as a database of which models support which features.

```toml
[[device]]
device_name = "FA506I"
product_id = ""
layout_name = "fa506i"
basic_modes = ["Static", "Breathe", "RainbowCycle", "RainbowWave", "Pulse"]
basic_zones = []
advanced_type = "None"
power_zones = ["Keyboard"]
```

in the above example the board name is found from `cat /sys/devices/virtual/dmi/id/board_name`. In some model ranges the last letter (which is likely the dGPU/feature variant) can be omitted. `layout_name` is the first part of a related filename for the layout as described in the next section - the filename should be postfixed with a locale such as `g513i_US.toml`.

`basic_modes` are the default inbuilt modes the keyboard supports. Not all keyboards have the same set of modes. `basic_zones` is a secondary part of `basic_modes` where this lists which zones can be set as part of the basic mode. Each zone requires a full basic mode setting. The zones supported here are

- `Key1`
- `Key2`
- `Key3`
- `Key4`
- `Logo`
- `BarLeft`
- `BarRight`

note that the zone support seems to have changed with new generations of keyboards and is shifted to `advanced_type`. The `advanced_type` field is taken in to account when setting advanced effects. It can be combined with the keyboard layout also to be used in a GUI.

`advanced_type` can be one of:

- `None`, no advanced aura at all
- `PerKey`, can use any of `LedCode` except for the `Zoned` items below which work in a different way
- `Zoned`, takes an array such as:
  - `Zoned([SingleZone])`, only one zone
  - `Zoned([ ... ]),`, array with any combination of:
    - `ZonedKbLeft` // keyboard left
    - `ZonedKbLeftMid` // keyboard left-middle
    - `ZonedKbRightMid` // etc
    - `ZonedKbRight`
    - `LightbarRight`
    - `LightbarRightCorner`
    - `LightbarRightBottom`
    - `LightbarLeftBottom`
    - `LightbarLeftCorner`
    - `LightbarLeft`

# Layouts

The layout structure is kept in a `.toml`. The way this works is best demonstrated:

```toml
locale = "US"

[key_shapes.regular.Led]
width = 1.0
height = 1.0
pad_left = 0.1
pad_right = 0.1
pad_top = 0.1
pad_bottom = 0.1
```
            height: 1.0,
            pad_left: 0.1,
            pad_right: 0.1,
            pad_top: 0.1,
            pad_bottom: 0.1,
        ),
        // There is nothing in this space but it takes up room
        "func_space": Blank(
            width: 0.2,
            height: 0.0,
        ),
        // This backspace button is composed of 3 individual LED
        "backspace1": Led(
            width: 0.65,
            height: 1.0,
            pad_left: 0.1,
            pad_right: 0.0,
            pad_top: 0.1,
            pad_bottom: 0.1,
        ),
        "backspace2": Led(
            width: 0.7,
            height: 1.0,
            pad_left: 0.0,
            pad_right: 0.0,
            pad_top: 0.1,
            pad_bottom: 0.1,
        ),
        "backspace3": Led(
            width: 0.65,
            height: 1.0,
            pad_left: 0.0,
            pad_right: 0.1,
            pad_top: 0.1,
            pad_bottom: 0.1,
        ),
    },
    key_rows: [
        (
            // Padding generally isn't required but is available just in case
            pad_left: 0.1,
            pad_top: 0.1,
            // Each row is a horizontal row of keys of the keyboard
            row: [
                // Declare a tuple of `Key`, and the String name to use from the hashmap above
                (Spacing, "rog_spacer"),
                (VolDown, "rog_row"),
                (VolUp, "rog_row"),
                (MicMute, "rog_row"),
                (Rog, "rog_row"),
            ],
        ),
        (
            pad_left: 0.1,
            pad_top: 0.1,
            row: [
                (Esc, "func_key"),
                // There are two non-led types, `Blocking` which is intended to block something like a row-laser
                (Blocking, "esc_func_spacing"),
                (F1, "func_key"),
                (F2, "func_key"),
                (F3, "func_key"),
                (F4, "func_key"),
                // and `Spacing` which is intended to act like a non-visible LED
                (Spacing, "func_space"),
                (F5, "func_key"),
                (F6, "func_key"),
                (F7, "func_key"),
                (F8, "func_key"),
                (Spacing, "func_space"),
                (F9, "func_key"),
                (F10, "func_key"),
                (F11, "func_key"),
                (F12, "func_key"),
                (Spacing, "func_space"),
                (Del, "func_key"),
            ],
        ),
        (
            pad_left: 0.1,
            pad_top: 0.1,
            row: [
                (Tilde, "regular"),
                (N1, "regular"),
                (N2, "regular"),
                (N3, "regular"),
                (N4, "regular"),
                (N5, "regular"),
                (N6, "regular"),
                (N7, "regular"),
                (N8, "regular"),
                (N9, "regular"),
                (N0, "regular"),
                (Hyphen, "regular"),
                (Equals, "regular"),
                (Backspace3_1, "backspace1"),
                (Backspace3_2, "backspace2"),
                (Backspace3_3, "backspace3"),
                (Spacing, "func_space"),
                (Home, "regular"),
            ],
        ),
    ]
)
```

**There are two types of layouts to be considered when building one; per-key, and zoned.**

A zoned keyboard layout includes single zoned + no zones (but not per-key). The layout for this is fairly freeform, and can be built using regular keys from `LedCode` but can not include these per-key specific codes:

- `LidLogo`
- `LidLeft`
- `LidRight`

it can include regular keys and:

- `SingleZone`, if this is used then the `ZonedKb*` should not be used
- `ZonedKbLeft`
- `ZonedKbLeftMid`
- `ZonedKbRightMid`
- `ZonedKbRight`
- `LightbarRight`
- `LightbarRightCorner`
- `LightbarRightBottom`
- `LightbarLeftBottom`
- `LightbarLeftCorner`
- `LightbarLeft`

#### `Key`

Every `Key` in the enum maps to a USB packet + RGB index in that packet. The raw mapping is seen in `per_key_raw_bytes.ods` in the data dir, for example there is a single LED backspace, and a 3-LED backspace.

#### `key_shapes`

This is a hashmap of `String`:`ShapeType`, as shown by the previous example such as:

```
        // This is a regular LED spot, it has a size (width x height), and padding around each edge.
        // The final size should be (width + pad_left + pad_right, height + pad_top + pad_bottom)
        "regular": Led(
            width: 1.0,
            height: 1.0,
            pad_left: 0.1,
            pad_right: 0.1,
            pad_top: 0.1,
            pad_bottom: 0.1,
        ),
```

"regular" being the key used by the keys in each key row.

# Testing

When working with Rog Control Center you can test layouts by starting the app on CLI with options:

```
  -h, --help            print help message
  -v, --version         show program version number
  -b, --board-name      set board name for testing, this will make ROGCC show only the keyboard page
  -l, --layout-viewing  put ROGCC in layout viewing mode - this is helpful for finding existing layouts that might match your laptop
```

