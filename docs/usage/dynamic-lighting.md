# Dynamic Lighting and legacy fallback

asusd prefers the Linux LED Dynamic Lighting ABI when a device exposes a valid
node below `/sys/class/leds`. A valid node must provide the generic `effect`,
`effect_index`, `zone_type`, and `led_count` attributes. Optional speed,
direction, palette, power-state, enable, and direct-buffer controls are used
only when the node advertises them.

On supported ROG laptop keyboards, asusd retains the matching `hidraw` output
report as a capability-based fallback when no valid Dynamic Lighting node is
available. The two paths are mutually exclusive for a physical device:
Dynamic Lighting always wins, and hidraw is not used to retry an effect rejected
by the kernel. This preserves operation on released kernels while avoiding two
owners sending commands to the same controller.

## Topology

hid-asus registers independent chassis zones:

| Node | Role |
|------|------|
| `asus::keyboard` | Keyboard; firmware dimmer |
| `asus::lightbar` | Chassis lightbar when present (`enabled` gates the zone; dimmer only if kernel advertises brightness) |
| `asus::logo` | Chassis logo zone when the kernel registers a separate DL node |
| `asus::slash` | Slash LED (DL `effect`/`speed`/`power_states` when present; else legacy `slash_mode` / `slash_interval`) |
| `asus::anime` | AniMe Matrix (`direct_buffer` stride from `direct_sample_size`, typically 1) |
| `asus::xbox_logo` | Xbox Guide LED on Xbox Ally X (brightness 0–3; DL effects when advertised) |
| `asus::kbd_backlight` | TUF WMI keyboard when asus-wmi exposes the DL ABI |

asusd prefers `asus::keyboard`, then falls back to a Dynamic Lighting-capable
`asus::kbd_backlight`. Brightness-only `asus::kbd_backlight` (no
`effect`/`effect_index`/`zone_type`/`led_count`) is not Dynamic Lighting: it
remains the legacy TUF/`KeyboardBacklight` path only when no keyboard DL node
is present. When any keyboard DL node exists, the legacy platform brightness
handle is not kept.

There is no `asus::global` aggregate. Each node accepts its own effect,
palette, and direct writes. Whole-device (`AuraZone::None`) requests fan out
to every present zone (keyboard, lightbar, logo). Independent applies use
chassis aliases (`Key1` → keyboard, `BarLeft` → lightbar, `Logo` → logo).
asusd advertises those choices on `SupportedEffectTargets` for ROG Control
Center; `SupportedBasicZones` stays empty under Dynamic Lighting. Intensity
(`brightness`) is written primarily to the keyboard node; lightbar uses
`enabled` (and brightness only when the kernel exposes a dimmer).

`direct_sample_size` (when present) is the authoritative packed stride for
`direct_buffer`. Older kernels omit it; asusd then falls back to mono for
AniMe/`zone_type=matrix` and RGB elsewhere.

Lighting off is `enabled=false`, independent from `effect`. `brightness` stays
intensity (scaled from the D-Bus `LedBrightness` levels when
`max_brightness > 3`). Firmware effects, including static colour, are committed
with Aura `0xb3`/`0xb5`/`0xb4`; `enabled` gates the zone with `0xbd`.

Kernel direct RGB may use HID LampArray (Usage Page `0x59`) when Aura `0xBC`
cannot drive the lightbar independently; firmware animations stay on Aura
`0xb3`. That backend choice is invisible to the sysfs ABI.

## AniMe Matrix

When `asus::anime` is present, asusd drives AniMe exclusively through Dynamic
Lighting (USB `0x193b` packets are not used). Builtin stage names are the
kernel effect strings (`boot_*`, `awake_*`, `sleep_*`, `shutdown_*`). Direct
frames use `led_count` intensity bytes. USBRaw remains a fallback only on
kernels without the AniMe DL node.

## Slash

Slash prefers `asus::slash` when present. When that node is full Dynamic
Lighting with `power_states`, D-Bus boot/sleep/shutdown map to those tokens.
Battery/lid persistence remains HID/USB-only (or `NotSupported` under sysfs
without matching kernel attrs). Legacy kernels keep `slash_mode` /
`slash_interval`. Without any Slash LED node, asusd falls back to HID/USB.

## Ally / Xbox Ally

Ally effect lists come from `effect_index` (narrower than notebooks). Xbox Ally
X may advertise `reactive`. LED power uses `power_states` under DL; the legacy
hidraw Ally power fix is not used when Dynamic Lighting owns the device.

## D-Bus zones

The D-Bus API still describes historical keyboard and left/right lightbar
subzones. Under Dynamic Lighting those are not advertised
(`supported_basic_zones` is empty). `supported_effect_targets` lists the
chassis apply targets instead: `None` (synced), `Key1` (keyboard), `BarLeft`
(lightbar), and `Logo` when that node exists. Incoming legacy zone values are
still accepted as aliases: Key1–4 map to the keyboard node, BarLeft/BarRight
to the lightbar, Logo to `asus::logo` when present, and `None` fans out to
every present zone.

## Other devices

ROG NVMe enclosure lighting requires the kernel ASUS Aura SCSI Dynamic Lighting
driver. asusd matches an enclosure's block-device ancestry to its exact LED
node and briefly retries while that node is being registered. It never falls
back to the first enclosure. The public `rog_scsi` helpers (`Device`, `Task`,
`sg`, and `Device::perform`) remain available and still issue SG_IO, but for
enclosure lighting the Dynamic Lighting sysfs ABI is the recommended backend:
vendor animation commands are kernel-owned once an enclosure LED node exists.

The optional generic `frame` attribute is not currently used by asusd. Direct
streaming uses `direct_buffer` with `led_count * sample_size` bytes (RGB = 3,
AniMe mono = 1). Hardware-specific behavior still depends on the kernel driver
reporting correct topology, LED count, and optional attributes.
