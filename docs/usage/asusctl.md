# asusctl

> The command-line client for the `asusd` daemon

`asusctl` talks to the `asusd` system daemon over D-Bus. Run `asusctl info` to check the daemon is reachable and see basic system information.

All examples assume `asusd.service` is running. If a command reports an error, check the daemon logs:

```bash
sudo journalctl -b -u asusd
```

## Contents

- [info](#info)
- [profile](#profile)
- [battery](#battery)
- [fan-curve](#fan-curve)
- [leds](#leds)
- [aura](#aura)
- [anime](#anime)
- [slash](#slash)
- [scsi](#scsi)
- [armoury](#armoury)
- [backlight](#backlight)
- [xgmled](#xgmled)

## info

Show the program version and system information, and list what your laptop supports.

```bash
asusctl info
asusctl info --show-supported
```

| Option | Description |
|---|---|
| `--show-supported` | List the supported core functions, platform properties, keyboard brightness levels, aura modes, aura zones, and aura power zones |

## profile

Platform profile management. The platform profile controls the power/thermal behaviour of the laptop and is mapped to the kernel's `platform_profile` interface.

```bash
asusctl profile list            # list available profiles
asusctl profile get             # active profile, plus AC and battery profiles
asusctl profile set Performance # set the current profile
asusctl profile next            # cycle to the next profile
```

Available profiles:

> [!NOTE]
> Available profiles vary by laptop and kernel support. The table below lists those that may be present. Run `asusctl profile list` to see the profiles your system actually exposes.

| Profile | Description |
|---|---|
| `Balanced` | Default profile |
| `Performance` | Maximum performance, higher fan speed |
| `Quiet` | Lower fan speed, reduced performance |
| `LowPower` | Lowest power draw |
| `Custom` | Uses the custom fan curve of the same name |

Profile names are case-insensitive. The AC and battery profiles can be set independently with `-a` and `-b`, which makes `asusd` switch profiles automatically when the power source changes:

```bash
asusctl profile set Quiet -b        # profile to use on battery
asusctl profile set Performance -a  # profile to use on AC
```

| Subcommand | Description |
|---|---|
| `list` | List the available profiles |
| `get` | Show the active profile, the AC profile, and the battery profile |
| `set <profile>` | Set the profile |
| `next` | Switch to the next profile in the list |
| `tuning [true\|false]` | Enable or disable profile tuning; omit the argument to show the current state |

| Option | Description |
|---|---|
| `-a`, `--ac` | With `set`: also set the profile to use while on AC power |
| `-b`, `--battery` | With `set`: also set the profile to use while on battery power |

Profile tuning enables and disables the PPT (TDP) tuning group, which controls whether power limits set through the [armoury](#armoury) interface are applied. When tuning is disabled, the updated PPT configuration is stored but only applied once tuning is enabled again:

```bash
asusctl profile tuning      # show current state
asusctl profile tuning true
asusctl profile tuning false
```

## battery

Battery charge control.

```bash
asusctl battery info         # show the current charge limit
asusctl battery limit 80     # stop charging at 80%
asusctl battery oneshot      # one-shot full charge to 100%
asusctl battery oneshot 90   # one-shot charge to 90%
```

| Subcommand | Description |
|---|---|
| `info` | Show the current battery charge limit |
| `limit <20-100>` | Set the charge limit percentage |
| `oneshot [percent]` | Perform a one-shot full charge; an optional target percent overrides the default of 100 |

## fan-curve

Custom fan curves are only supported on specific models. The data format is a comma-separated list of exactly 8 points in the form `30c:1%,49c:2%,...`, where each point is a temperature in degrees Celsius followed by a fan speed. If the `%` is omitted the fan speed range is 0-255. Temperatures and fan speeds must be non-decreasing along the curve, and temperatures above 100 °C are not recommended.

```bash
asusctl fan-curve --mod-profile Balanced --data 30c:1%,49c:2%,59c:3%,69c:4%,79c:31%,89c:49%,99c:56%,100c:58%
asusctl fan-curve --mod-profile Balanced --fan cpu --data 30c:1%,49c:2%,59c:3%,69c:4%,79c:31%,89c:49%,99c:56%,100c:58%
```

There are three fan profiles (Quiet, Balanced, and Performance), each linked to the platform profile of the same name. Enable or disable them:

```bash
asusctl fan-curve --mod-profile Balanced --enable-fan-curves true
asusctl fan-curve --mod-profile Balanced --enable-fan-curve true --fan cpu
asusctl fan-curve --mod-profile Balanced --default
asusctl fan-curve --get-enabled
```

If no profile is activated manually, the fan curve from the BIOS is used.

| Option | Description |
|---|---|
| `--mod-profile <profile>` | The profile to modify. When given without any other option, prints the current curve data for that profile |
| `--data <points>` | Set the fan curve data; requires `--mod-profile` |
| `--fan <cpu\|gpu\|mid>` | Select the fan to modify; defaults to `cpu`. Required by `--enable-fan-curve` |
| `--enable-fan-curves <true\|false>` | Enable or disable all fan curves of a profile; requires `--mod-profile` |
| `--enable-fan-curve <true\|false>` | Enable or disable a single fan curve; requires `--mod-profile` and `--fan` |
| `--default` | Set the active profile's fan curves back to the defaults |
| `--get-enabled` | Print the fan curve data of the active profile |

## leds

Keyboard backlight brightness control.

```bash
asusctl leds set high    # brightness levels: off, low, med, high
asusctl leds get         # show the current brightness
asusctl leds next        # cycle to the next brightness level
asusctl leds prev        # cycle to the previous brightness level
```

| Subcommand | Description |
|---|---|
| `set <off\|low\|med\|high>` | Set the keyboard brightness level |
| `get` | Show the current keyboard brightness |
| `next` | Toggle to the next brightness level |
| `prev` | Toggle to the previous brightness level |

## aura

Aura lighting control for the keyboard, logo, lightbar, and other zones.

### aura effect

Cycle through the built-in factory modes:

```bash
asusctl aura effect --next-mode
asusctl aura effect --prev-mode
```

Or set a specific effect directly:

```bash
asusctl aura effect static -c ff00ff
asusctl aura effect breathe --colour ff0000 --colour2 0000ff --speed low
asusctl aura effect rainbow-cycle --speed med --zone one
asusctl aura effect rainbow-wave --direction left --speed med
asusctl aura effect stars --colour ff00ff --colour2 00ff00 --speed low
asusctl aura effect rain --speed med
asusctl aura effect highlight -c ff00ff --speed low
asusctl aura effect laser -c ff00ff --speed low
asusctl aura effect ripple -c ff00ff --speed low
asusctl aura effect pulse -c ff00ff
asusctl aura effect comet -c ff00ff
asusctl aura effect flash -c ff00ff
```

| Effect | Options |
|---|---|
| `static` | `-c/--colour` |
| `breathe` | `--colour`, `--colour2`, `--speed` |
| `rainbow-cycle` | `--speed` |
| `rainbow-wave` | `--direction`, `--speed` |
| `stars` | `--colour`, `--colour2`, `--speed` |
| `rain` | `--speed` |
| `highlight` | `-c/--colour`, `--speed` |
| `laser` | `-c/--colour`, `--speed` |
| `ripple` | `-c/--colour`, `--speed` |
| `pulse` | `-c/--colour` |
| `comet` | `-c/--colour` |
| `flash` | `-c/--colour` |

Every effect accepts `--zone`, which defaults to `none` (all zones).

| Option | Description |
|---|---|
| `-c`, `--colour <hex>` | RGB value, e.g. `ff00ff` |
| `--colour2 <hex>` | Second RGB value, e.g. `ff00ff` |
| `--speed <low\|med\|high>` | Effect speed |
| `--direction <up\|down\|left\|right>` | Effect direction |
| `--zone <zone>` | Zone for the effect; see the zone table below |

Zones can be given as a number or a name:

| Value | Name |
|---|---|
| `0` | `none` (all zones, default) |
| `1` | `one` |
| `2` | `two` |
| `3` | `three` |
| `4` | `four` |
| `5` | `logo` |
| `6` | `lightbar-left` |
| `7` | `lightbar-right` |

### aura power

Enable or disable individual Aura zones per power state on newer (2021+) keyboards. The `--boot`, `--awake`, `--sleep`, and `--shutdown` switches enable the zone for that state; omitting a switch sets it to false:

```bash
asusctl aura power keyboard --awake
asusctl aura power lightbar --boot --awake
asusctl aura power lid --awake --sleep
asusctl aura power ally --shutdown
```

| Zone | Description |
|---|---|
| `keyboard` | Keyboard zone |
| `logo` | Logo zone |
| `lightbar` | Lightbar zone |
| `lid` | Lid zone |
| `rear-glow` | Rear glow zone |
| `ally` | ROG Ally zone |

| Option | Description |
|---|---|
| `--boot` | Enable power while the device is booting |
| `--awake` | Enable power while the device is awake |
| `--sleep` | Enable power while the device is sleeping |
| `--shutdown` | Enable power while the device is shutting down or hibernating |

### aura power-tuf

For older ROG and TUF laptops (product ID 0x1866):

```bash
asusctl aura power-tuf --awake true --boot false
asusctl aura power-tuf --awake false --keyboard
asusctl aura power-tuf --boot true --lightbar
```

| Option | Description |
|---|---|
| `--awake <true\|false>` | Whether the LEDs are enabled while awake |
| `--boot <true\|false>` | Whether the boot animation is enabled |
| `--sleep <true\|false>` | Whether the suspend animation is enabled |
| `--keyboard` | Apply the state to the keyboard zone |
| `--lightbar` | Apply the state to the lightbar zone |

## anime

AniMe Matrix display control. The display can be configured with options on the `anime` command:

```bash
asusctl anime --enable-display true
asusctl anime --enable-display false
asusctl anime --brightness low        # brightness: off, low, med, high
asusctl anime --clear                 # clear the display
asusctl anime --enable-powersave-anim true
asusctl anime --off-when-unplugged true
asusctl anime --off-when-suspended true
asusctl anime --off-when-lid-closed true
```

| Option | Description |
|---|---|
| `--enable-display <true\|false>` | Enable or disable the display |
| `--enable-powersave-anim <true\|false>` | Enable or disable the built-in run/powersave animation |
| `--brightness <off\|low\|med\|high>` | Set the global base brightness |
| `--clear` | Clear the display |
| `--off-when-unplugged <true\|false>` | Turn the display off when external power is unplugged |
| `--off-when-suspended <true\|false>` | Turn the display off when the laptop suspends |
| `--off-when-lid-closed <true\|false>` | Turn the display off when the lid is closed |
| `--override-type <GA401\|GA402\|GU604\|G635L\|G835L>` | Override the display type if it is not detected automatically |

Display an image or animation:

```bash
asusctl anime image --path ./image.png
asusctl anime pixel-image --path ./image.png
asusctl anime gif --path ./animation.gif --loops 3
asusctl anime pixel-gif --path ./animation.gif --loops 0
```

| Subcommand | Options |
|---|---|
| `image` | `--path` (required), `--scale` (default 1.0), `--x-pos` (default 0.0), `--y-pos` (default 0.0), `--angle` (default 0.0, radians), `--bright` (default 1.0) |
| `pixel-image` | `--path` (required), `--bright` (default 1.0) |
| `gif` | `--path` (required), `--scale`, `--x-pos`, `--y-pos`, `--angle`, `--bright`, `--loops` (default 0) |
| `pixel-gif` | `--path` (required), `--bright`, `--loops` (default 0) |

`--bright` expects a value between 0.0 and 1.0. `--loops 0` plays the GIF infinitely.

Change the built-in boot/awake/sleep/shutdown animations with `set-builtins`:

```bash
asusctl anime set-builtins --awake BinaryBannerScroll --set true
asusctl anime set-builtins --shutdown SeeYa --set true
```

| Option | Description |
|---|---|
| `--boot <animation>` | Animation shown while booting |
| `--awake <animation>` | Animation shown while awake |
| `--sleep <animation>` | Animation shown while sleeping |
| `--shutdown <animation>` | Animation shown while shutting down |
| `--set <true\|false>` | Apply the animations (required) |

Available built-in animations:

| Power state | Animations |
|---|---|
| `--boot` | `GlitchConstruction`, `StaticEmergence` |
| `--awake` | `BinaryBannerScroll`, `RogLogoGlitch` |
| `--sleep` | `BannerSwipe`, `Starfield` |
| `--shutdown` | `GlitchOut`, `SeeYa` |

## slash

Slash LED bar control (found on some Zenbook models).

```bash
asusctl slash get      # show the current state
asusctl slash list     # list the available animations
```

Configure the slash light bar:

```bash
asusctl slash set --enable
asusctl slash set --disable
asusctl slash set --brightness 200
asusctl slash set --interval 3
asusctl slash set --mode Flow
asusctl slash set --show-on-boot true
asusctl slash set --show-on-shutdown true
asusctl slash set --show-on-sleep false
asusctl slash set --show-on-battery true
asusctl slash set --show-battery-warning true
```

| Subcommand | Description |
|---|---|
| `get` | Show the current state of the slash LED bar |
| `set` | Set slash LED bar options |
| `list` | List the available animations |

| Option | Description |
|---|---|
| `--enable` | Enable the slash LED bar |
| `--disable` | Disable the slash LED bar |
| `-l`, `--brightness <0-255>` | Set the brightness value |
| `--interval <0-5>` | Set the interval value |
| `--mode <mode>` | Set the animation mode; use `slash list` for the options |
| `-B`, `--show-on-boot <true\|false>` | Show the animation on boot |
| `-S`, `--show-on-shutdown <true\|false>` | Show the animation on shutdown |
| `-s`, `--show-on-sleep <true\|false>` | Show the animation on sleep |
| `-b`, `--show-on-battery <true\|false>` | Show the animation on battery power |
| `-w`, `--show-battery-warning <true\|false>` | Show the low-battery warning animation |

Available animations: `Static`, `Bounce`, `Slash`, `Loading`, `BitStream`, `Transmission`, `Flow` (default), `Flux`, `Phantom`, `Spectrum`, `Hazard`, `Interfacing`, `Ramp`, `GameOver`, `Start`, `Buzzer`.

## scsi

SCSI drive LED control (some laptops expose these LEDs):

```bash
asusctl scsi --enable true
asusctl scsi --list
asusctl scsi --mode RainbowCycle --speed med --direction forward
asusctl scsi --mode Static --colours ff0000 --colours 00ff00
```

| Option | Description |
|---|---|
| `--enable <true\|false>` | Enable or disable the SCSI drive LEDs |
| `--mode <mode>` | Set the LED mode; use `--list` for the options |
| `--speed <slowest\|slow\|med\|fast\|fastest>` | Set the animation speed |
| `--direction <forward\|reverse>` | Set the animation direction |
| `--colours <hex>` | Set the LED colours; repeat the option for up to 4 colours |
| `--list` | List the available animations |

Available modes: `Off`, `Static`, `Breathe`, `Flashing`, `RainbowCycle`, `RainbowWave`, `RainbowCycleBreathe`, `ChaseFade`, `RainbowCycleChaseFade`, `Chase`, `RainbowCycleChase`, `RainbowCycleWave`, `RainbowPulseChase`, `RandomFlicker`, `DoubleFade`.

## armoury

Read and write firmware attributes exposed by the `asus-armoury` kernel driver. Run `asusctl armoury list` to see the attributes supported by your laptop.

```bash
asusctl armoury list               # list supported firmware attributes
asusctl armoury get dgpu_disable   # read an attribute
asusctl armoury set dgpu_disable 1 # set an attribute
asusctl armoury set dgpu_disable -1 # reset an attribute to its default
```

| Subcommand | Description |
|---|---|
| `list` | List all firmware attributes supported by `asus-armoury` |
| `get <property>` | Get the value of a firmware attribute |
| `set <property> <value>` | Set a firmware attribute; a value of `-1` resets it to the default |

Attributes of type PPT (power limits) are only applied to the hardware while [profile tuning](#profile) is enabled; otherwise the new value is stored and applied the next time tuning is turned on.

See [GPU Switching](../faq/gpu-switching.md) for the full dGPU/MUX story.

## backlight

ScreenPad backlight control (Zenbook models with a ScreenPad):

```bash
asusctl backlight --screenpad-brightness 80
asusctl backlight --screenpad-gamma 1.2
asusctl backlight --sync-screenpad-brightness true
```

With no options, the current screenpad settings are printed.

| Option | Description |
|---|---|
| `--screenpad-brightness <0-100>` | Set the screen brightness |
| `--screenpad-gamma <0.5-2.2>` | Set the screenpad gamma brightness; `1.0` is linear |
| `--sync-screenpad-brightness <true\|false>` | Sync the screenpad brightness with the primary display |

## xgmled

XG Mobile LED control:

```bash
asusctl xgmled get      # show the current LED state
asusctl xgmled set 1    # 1 = on, 0 = off
```

| Subcommand | Description |
|---|---|
| `get` | Show the current XG Mobile LED state |
| `set <0\|1>` | Set the XG Mobile LED on (`1`) or off (`0`) |

## Something not working?

See the [FAQ](../faq/general.md) for common issues. In most cases the answer is in the `asusd` logs:

```bash
sudo journalctl -b -u asusd
```
