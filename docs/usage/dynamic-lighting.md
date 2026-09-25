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
| `asus::keyboard` | Keyboard (and logo power tracking) |
| `asus::lightbar` | Chassis lightbar when present |
| `asus::slash` | Slash LED when present |
| `asus::anime` | AniMe Matrix when present |

There is no `asus::global` aggregate and no userspace `aura_mode` selector.
Each node accepts its own effect, palette, and direct RGB writes. Whole-device
requests fan out to every present zone.

Lighting off is `enabled=false`, independent from `effect`. `brightness` stays
intensity (scaled from the D-Bus `LedBrightness` levels when
`max_brightness > 3`). Firmware effects, including static colour, are committed
with Aura `0xb3`/`0xb5`/`0xb4`; `enabled` gates the zone with `0xbd`.

Kernel direct RGB may use HID LampArray (Usage Page `0x59`) when Aura `0xBC`
cannot drive the lightbar independently; firmware animations stay on Aura
`0xb3`. That backend choice is invisible to the sysfs ABI.

## D-Bus zones

The D-Bus API still describes historical keyboard and left/right lightbar
subzones. Under Dynamic Lighting those are not advertised
(`supported_basic_zones` is empty). Incoming legacy zone values are accepted
only as aliases: Key1–4 map to the keyboard node, BarLeft/BarRight to the
lightbar, and `None` fans out to every present zone.

## Other devices

ROG NVMe enclosure lighting requires the kernel ASUS Aura SCSI Dynamic Lighting
driver. asusd matches an enclosure's block-device ancestry to its exact LED
node and briefly retries while that node is being registered. It never falls
back to the first enclosure. The old public `rog_scsi` SG_IO API was removed
intentionally: vendor commands are kernel-owned, and applications must use the
Dynamic Lighting sysfs ABI.

The optional generic `frame` attribute is not currently used by asusd. Direct
streaming uses `direct_buffer` and requires exactly `led_count * 3` RGB bytes.
Hardware-specific behavior still depends on the kernel driver reporting correct
topology, LED count, and optional attributes.
