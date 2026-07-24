# asusctrl manual

**NOTE:** this manual is in need of an update in some places. If you find issues please file issue reports.

`asusd` is a utility for Linux to control many aspects of various ASUS laptops
but can also be used with non-asus laptops with reduced features.

## Programs Available

- `asusd`: The main system daemon. It is autostarted by a udev rule and systemd unit.
- `asusd-user`: The user level daemon. Currently will run an anime sequence, with RGB keyboard sequences soon.
- `asusctl`: The CLI for interacting with the system daemon

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

If you have an AniMe device a few system-level config options are enabled for you in `/etc/asusd/anime.conf`;

1. `"system": [],`: currently unused, is intended to be a default continuous sequence in future versions
2. `"boot": [],`: a sequence that plays on system boot (when asusd is loaded)
3. `"wake": [],`: a sequence that plays when waking from suspend
4. `"shutdown": [],`: a sequence that plays when shutdown begins
5. `"brightness": <FLOAT>`: global brightness control, where `<FLOAT> is 0.0-1.0

Some default examples are provided but are minimal. The full range of configuration options will be covered in another section of this manual.

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

asusctl can support setting a power profile via platform_profile drivers. This requires [power-profiles-daemon](https://gitlab.freedesktop.org/hadess/power-profiles-daemon) v0.10.0 minimum. It also requires the kernel patch for platform_profile support to be applied form [here](https://lkml.org/lkml/2021/8/18/1022) - this patch is merged to 5.15 kernel upstream.

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

## asusd-user

`asusd-user` is a usermode daemon. The intended purpose is to provide a method for users to run there own custom per-key keyboard effects and modes, AniMe sequences, and possibly their own profiles - all without overwriting the _base_ system config. As such some parts of the system daemon will migrate to the user daemon over time with the expectation that the Linux system runs both.

As of now only AniMe is active in this with configuration in `~/.config/rog/`. On first run defaults are created that are intended to work as examples.

The main config is `~/.config/rog/rog-user.cfg`

#### Config options: Aura, per-key and zoned

I'm unsure of how many laptops this works on, so please try it.

`led_type: Key` works only on actual per-key RGB keyboards.

`led_type: Zone` works on zoned laptops.

`led_type: Zone` set to `None` works on zoned ROG laptops, unzoned ROG laptops, and TUF laptops (and yes this does mean an audio EQ can be done now).

`~/.config/rog/rog-user.cfg` contains a setting `"active_aura": "<FILENAME>"` where `<FILENAME>` is the name of the Aura config to use, located in the same directory and without the file postfix, e.g, `"active_anime": "aura-default"`

An Aura config itself is a file with contents:

```toml
(
    name: "aura-default",
    aura: (
        effects: [
            Breathe((
                led: W,
                start_colour1: (255, 0, 20),
                start_colour2: (20, 255, 0),
                speed: Low,
            )),
            Breathe((
                led: A,
                start_colour1: (255, 0, 20),
                start_colour2: (20, 255, 0),
                speed: Low,
            )),
            Breathe((
                led: S,
                start_colour1: (255, 0, 20),
                start_colour2: (20, 255, 0),
                speed: Low,
            )),
            Breathe((
                led: D,
                start_colour1: (255, 0, 20),
                start_colour2: (20, 255, 0),
                speed: Low,
            )),
            Breathe((
                led: F,
                start_colour1: (255, 0, 0),
                start_colour2: (255, 0, 0),
                speed: High,
            )),
            Static((
                led: RCtrl,
                colour: (0, 0, 255),
            )),
            Static((
                led: LCtrl,
                colour: (0, 0, 255),
            )),
            Static((
                led: Esc,
                colour: (0, 0, 255),
            )),
            DoomFlicker((
                led: N9,
                start_colour: (0, 0, 255),
                max_percentage: 80,
                min_percentage: 40,
            )),
        ],
        zoned: false,
    ),
)
```

If your laptop supports multizone, `"led"` can also be `"Zone": <one of the following>`

- `SingleZone` // Keyboards with only one zone
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

Single zone example:

```toml
(
    name: "aura-default",
    aura: (
        effects: [
            DoomFlicker((
                led: SingleZone,
                start_colour: (200, 40, 5),
                max_percentage: 80,
                min_percentage: 40,
            )),
        ],
        zoned: true,
    ),
)
```

At the moment there are only three effects available as shown in the example. More will come in the future
but this may take me some time.

#### Config options: AniMe

`~/.config/rog/rog-user.cfg` contains a setting `"active_anime": "<FILENAME>"` where `<FILENAME>` is the name of the AniMe config to use, located in the same directory and without the file postfix, e.g, `"active_anime": "anime-doom"`

An AniMe config itself is a file with contents:

```json
{
  "name": "<FILENAME>",
  "anime": []
}
```

`<FILENAME>` is used as a reference internally. `"anime": []` is an array of sequences (WIP).

##### "anime" array options

Each object in the array can be one of:

1. AsusAnimation
2. ImageAnimation
3. Image
4. Pause

##### AsusAnimation

`AsusAnimation` is specifically for running the gif files that Armory Crate comes with. `asusctl` includes all of these in `/usr/share/asusd/anime/asus/`

```json
      "AsusAnimation": {
        "file": "<FILE_PATH>",
        "time": <TIME>,
        "brightness": <FLOAT>
      }
```

##### AsusImage

Virtually the same as `AsusAnimation` but for png files, typically created in the same "slanted" style using a template (`diagonal-template.png`) as the ASUS gifs for pixel perfection.

```json
      "AsusImage": {
        "file": "<FILE_PATH>",
        "time": <TIME>,
        "brightness": <FLOAT>
      }
```

##### ImageAnimation

`ImageAnimation` can play _any_ gif of any size.

```json
      "ImageAnimation": {
        "file": "<FILE_PATH>",
        "scale": <FLOAT>,
        "angle": <FLOAT>,
        "translation": [
          <FLOAT>,
          <FLOAT>
        ],
        "time": <TIME>,
        "brightness": <FLOAT>
      }
    },
```

##### Image

`Image` currently requires 8bit greyscale png. It will be able to use most in future.

```json
    {
      "Image": {
        "file": "<FILE_PATH>",
        "scale": <FLOAT>,
        "angle": <FLOAT>,
        "translation": [
          <FLOAT>,
          <FLOAT>
        ],
        "time": <TIME>,
        "brightness": <FLOAT>
      }
    },
```

##### Pause

A `Pause` is handy for after an `Image` to hold the `Image` on the AniMe for a period.

```json
    {
      "Pause": {
        "secs": <INT>,
        "nanos": <INT>
      }
    },
```

##### Options for objects

**<FILE_PATH>**

Must be full path: `"/usr/share/asusd/anime/asus/gaming/Controller.gif"` or `/home/luke/Downloads/random.gif`.

**<FLOAT>**

A number from 0.0-1.0.

- `brightness`: If it is brightness it is combined with the system daemon global brightness
- `scale`: 1.0 is the original size with lower number shrinking, larger growing
- `angle`: Rotation angle in radians
- `translation`: Shift the image X -/+, and y -/+

**<TIME>**

Time is the length of time to run the gif for:

```json
        "time": {
          "Time": {
            "secs": 5,
            "nanos": 0
          }
        },
```

A cycle is how many gif loops to run:

```json
        "time": {
          "Cycles": 2
        },
```

`Infinite` means that this gif will never end:

```json
        "time": "Infinite",
```

`Fade` allows an image or gif to fade in and out, and remain at max brightness to n time:

```json
        "time": {
          "Fade": {
            "fade_in": {
              "secs": 2,
              "nanos": 0
            },
            "show_for": {
              "secs": 1,
              "nanos": 0
            },
            "fade_out": {
              "secs": 2,
              "nanos": 0
            }
          }
        },
```

`show_for` can be `null`, if it is `null` then the `show_for` becomes `gif_time_length - fade_in - fade_out`.
This is period for which the gif or image will be max brightness (as set).

**<INT>**

A plain non-float integer.

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
