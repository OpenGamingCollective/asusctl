# Armoury Crate notebook capability and protocol map

## Why this investigation started

After moving my ROG Flow X16 (`GV601VV`) to Linux, Color Cycle, the Aura
profile I normally used in Armoury Crate, was missing. `asusd` could already
send that effect, but it did not advertise it because the model database was
treated as the complete capability list. A one-line model entry would have
fixed this laptop and left the same class of problem for every other machine
whose firmware and database disagree.

I traced the Armoury Crate implementation to find the source of the Windows
capability list. That path led beyond Aura: Armoury exposes lighting,
thermal profiles, manual tuning, fan curves, P/E-core control, GPU modes,
XG Mobile, panel functions, and hotkeys through different transports and
different kinds of capability data. This document maps those paths so that a
Linux implementation can reproduce the hardware contracts without copying
the surrounding Windows service architecture.

For this work I examined the full Armoury Crate 1.4.4.3 package. I followed
the managed assemblies in ILSpy and the native x64 plugins in IDA Pro, then
recorded the component boundaries, configuration schemas, command handlers,
firmware transactions, masks, and data layouts collected in this report.

The central result is more nuanced than "read everything from firmware."
Armoury Crate is a layered compatibility system. It combines live firmware
queries, HID descriptors and Feature reports, DMI identity, model-specific
XML/INI data, device catalog records, driver/API availability, cached state,
and Windows-side policy. Some values, such as modern notebook Aura modes and
the maximum P/E-core counts, are discoverable at runtime. Other values, such
as safe tuning limits and peripheral UI controls, are deliberately supplied
by model data. A correct `asusd` design should preserve that distinction.

## System architecture

Armoury Crate does not talk to notebook firmware from the UI process. The
relevant chain is approximately:

```text
UWP page/view model
  -> Win32 app-service bridge
  -> Armoury Crate Service plugin
  -> model configuration + runtime capability checks
  -> ATKACPI / HID / vendor GPU API / Windows display API
  -> embedded controller, BIOS, USB controller, GPU driver, or OS
```

The main components found in that chain are:

| Component | Role |
| --- | --- |
| `ArmouryCrate.dll` | .NET Native UWP views, product-mode composition, and UI gating |
| `ArmouryCrate.Win32Manager.exe` | UWP-to-Win32 app-service bridge |
| `DeviceHalDetect.dll` | SMBIOS, WMI/ACPI, SetupAPI, HID, USB, and SMBus discovery |
| `DeviceServicePlugin.dll` | Device-page orchestration, profiles, and WebSocket forwarding |
| `AuraPlugin.dll` | Notebook Aura, Matrix, Slash, lighting power, and effect application |
| `ThrottlePlugin.dll` | Performance-mode IPC, manual tuning, and profile application |
| `MonitorPlugin.dll` | CPU/GPU telemetry aggregation, fan telemetry, and resource-monitor data |
| `NativeGamingCenterHelper.dll` | Raw ASUS WMI helpers, thermal policy, fan tables, and dynamic cores |
| `ThrottleGearXMLHelper.dll` | Managed schema for model performance and tuning data |
| `ProfileHelperModel.dll` | Profile validation and import/export model |
| `FanControlPlugin.dll` | Fan-control service entry point |
| `FanControlWrapper.dll` | Fan-control library wrapper and signatures |
| `ATKWMI.dll` | Generic ASUS `DSTS`/`DEVS` transport |
| `GPUControlLibrary.dll` | NVIDIA, AMD, and Intel GPU backends |
| `GPUPowerSavingPlugin.dll` | Hybrid/UMA state and Optimized-mode policy |
| `GPUSwitchPlugin.dll` | XG Mobile/eGPU switching lifecycle |
| `ExternalGPUConfigHelper2.dll` | XG Mobile identity, capability, and tuning schema |
| `UtilityPlugin.dll` | Display, panel, boot sound, hotkey, and general hardware functions |
| `UtilityNativeGamingCenterHelper.dll` | Native WMI helpers used by `UtilityPlugin` |
| `HWPlugin.dll` | Attached-display enumeration through `AsusDisplayMonitorInfo.dll` |
| `GameVisualPlugin.dll` | Display color profiles, ICM/Splendid integration, and OLED Care |
| `DenoiseAIPlugin.dll` | Audio-device discovery and AI noise-cancellation backend selection |
| `ScenarioProfilePlugin.dll` | Application-linked scenario orchestration |
| `ConfigXML_ScenarioProfile.dll` | Managed scenario-profile XML schema |
| `CustomHotkeyPlugin.dll` | Custom-key service integration |
| `MacroKeyPlugin.dll` | Macro-key service integration |
| `MultiwheelPlugin.dll` | Multiwheel service integration |
| `SoulKeyServicePlugin.dll` | Proximity, card, and performance-event orchestration |

`ArmouryCrate.Win32Manager.exe` exposes app service
`com.asus.armourycrate.inprocess`. Requests use fields including `Target`,
`HandlerId`, `Cmd`, and `Result`. `DeviceServicePlugin.dll` also forwards
device-page requests containing `target`, `command`, `device`, and
`device_type`; commands include `SendWebSocketMessage`,
`GetDeviceinfo`, `GetDeviceLightingControl`, profile backup/restore, Aura
Creator, and Matrix operations.

These IPC layers are not hardware protocols. On Linux, D-Bus is already the
appropriate service boundary. The useful interoperability targets are the
capability rules and the final firmware/driver transactions below.

## How Armoury builds a capability set

Armoury assembles support from several independent sources:

1. `DeviceHalDetect.dll` reads SMBIOS product, board, and BIOS data and
   enumerates ACPI, USB, HID, SMBus, and Windows device nodes.
2. `DeviceWhiteList.ini` maps VID/PID and transport variants to logical device
   names, types, modes, GUIDs, HALs, and SDKs.
3. Package/catalog metadata selects a model page, HAL, firmware package, Aura
   kit, and parameter flags.
4. Notebook performance files such as `ThrottleGear_<model>.xml` describe
   supported tuning controls, bounds, and values for each product mode.
5. Peripheral page data such as `caps.json` describes exact UI controls for
   each effect, DPI stage, polling rate, calibration, and power feature.
6. Runtime firmware queries provide current values and selected capabilities.
7. Driver and OS checks decide whether an otherwise known feature can work on
   the installed system.
8. Service policy composes product concepts such as Optimized GPU mode from
   lower-level states and safety conditions.

`DeviceWhiteList.ini` also shows why a marketing model is not a sufficient
key. One peripheral may have separate USB, 2.4 GHz dongle, and Bluetooth PIDs
under one logical GUID. Parameters such as `NOSUPPORTAURA`, `NOLighting`, and
`DISABLELINK` remove features for a particular transport. A capability cache
therefore needs at least logical device, physical interface/transport, live
backend state, and firmware version.

`DeviceHalDetect.dll` uses `GetSystemFirmwareTable`, WMI/COM, SetupAPI,
`HidD_GetAttributes`, USB enumeration, and SMBus paths. Its generated records
contain fields such as `LightingDeviceType`, `GUID`, generation support,
parent identity, LED count, Matrix support, and hardware switch support. This
is broad device discovery, not a complete notebook capability descriptor.

The practical source priority for Linux should be:

```text
validated runtime capability with understood semantics
  -> live kernel/asusd attribute and its advertised choices/range
  -> exact, tested model or transport fallback
  -> feature absent
```

Lists from different layers must not be blindly unioned. A runtime response is
authoritative only for the fields its protocol actually defines. Model data
remains necessary for safe limits, layouts, defaults, and older hardware that
does not implement a query.

## ASUS WMI transport

`NativeGamingCenterHelper.dll` and related native helpers open
`\\.\ATKACPI` and call `DeviceIoControl` with IOCTL `0x22240c`. The generic
request created by `ControlWMIMethod` is:

```text
offset  size  field
0       4     ACPI method identifier
4       4     payload length in bytes
8       N     payload
```

The helper allocates an output buffer of 1024 bytes. The two principal ACPI
methods are:

| Method | ASCII | Value | Typical payload |
| --- | --- | ---: | --- |
| device status | `DSTS` | `0x53545344` | device ID, optionally followed by an index |
| device set | `DEVS` | `0x53564544` | device ID followed by value or data |

`One_WMIMethod_INT` sends one 32-bit payload value. `Two_WMIMethod_INT` sends
two 32-bit values. `General_WMIMethod_INT` passes an arbitrary payload through
the same transport. This explains both scalar controls and table operations.

For many `DSTS` IDs, `0xfffffffe` means unsupported. Bit 16 is commonly a
presence/validity flag, while the low 16 bits, low byte, or low nibble contain
the value. The exact extraction is command-specific. `DEVS` paths commonly
require a returned value of `1` before reporting success.

Transport success is not state confirmation. A Linux backend should validate
the support marker, constrain the value according to the command, write once,
and read the value back whenever the firmware provides a read path. Commands
with reboot semantics should be queued and explicitly reported as pending;
the daemon must not reboot the machine.

## WMI command index

This table is a navigation index. The subsystem sections describe the read,
write, validation, and policy paths behind each entry.

| ID | Direction | Meaning | Armoury implementation |
| ---: | --- | --- | --- |
| `0x00120075` | `DSTS`/`DEVS` | thermal policy | `NativeGamingCenterHelper.dll` |
| `0x001200d2` | `DSTS`/`DEVS` | current packed P/E-core counts | `NativeGamingCenterHelper.dll` |
| `0x001200d3` | `DSTS` | maximum packed P/E-core counts | `NativeGamingCenterHelper.dll` |
| `0x00110024` | table read/write | CPU fan curve | `NativeGamingCenterHelper.dll` |
| `0x00110025` | table read/write | GPU fan curve | `NativeGamingCenterHelper.dll` |
| `0x00110032` | table read/write | middle/system fan curve | `NativeGamingCenterHelper.dll` |
| `0x00110026` | `DSTS` | CPU fan-curve lower limit | `NativeGamingCenterHelper.dll` |
| `0x00110027` | `DSTS` | GPU fan-curve lower limit | `NativeGamingCenterHelper.dll` |
| `0x00110033` | `DSTS` | middle fan-curve lower limit | `NativeGamingCenterHelper.dll` |
| `0x00090020` | `DSTS`/`DEVS` | Hybrid (`0`) / UMA (`1`) GPU state | `GPUPowerSavingPlugin.dll` |
| `0x00090030` | `DSTS` | display connection/routing bits | `GPUPowerSavingPlugin.dll` |
| `0x00090029` | `DSTS`/`DEVS` | Dynamic GPU control | `UtilityPlugin.dll` |
| `0x00090016` | `DSTS`/`DEVS` | G-SYNC/MUX-related state | `UtilityPlugin.dll` |
| `0x00090017` | `DSTS` | XG Mobile docking status | `GPUSwitchPlugin.dll` |
| `0x00090018` | `DSTS` | XG Mobile physical lock status | `GPUSwitchPlugin.dll` |
| `0x00090019` | `DSTS`/`DEVS` | internal/external GPU selection | `GPUSwitchPlugin.dll` |
| `0x0009001c` | `DSTS` | supported external GPU vendor bits | `GPUSwitchPlugin.dll` |
| `0x0006005b` | `DSTS`/`DEVS` | notebook/dock PCIe generation coordination | `GPUSwitchPlugin.dll` |
| `0x00100057` | `DEVS` | TUF lighting/general settings path | `AuraPlugin.dll` |
| `0x00130022` | `DSTS`/`DEVS` | ROG boot sound | `UtilityPlugin.dll` |
| `0x00100023` | `DSTS`/`DEVS` | Fn-lock state | `UtilityPlugin.dll` |
| `0x00050036` | `DSTS`/`DEVS` | touchscreen state | `UtilityPlugin.dll` |
| `0x00130024` | `DSTS`/`DEVS` | Windows-key BIOS state | `UtilityPlugin.dll` |
| `0x0005001c` | buffer read/write | display profile and refresh pair | `UtilityPlugin.dll` |
| `0x0005001e` | `DSTS`/`DEVS` | first-generation Mini-LED control | `UtilityPlugin.dll` |
| `0x0005002e` | `DSTS`/`DEVS` | second-generation Mini-LED balance | `UtilityPlugin.dll` |
| `0x0005002a` | `DSTS`/`DEVS` | ABOT/optimal brightness | `UtilityPlugin.dll` |
| `0x00060091` | `DSTS`/`DEVS` | ESS control | `UtilityPlugin.dll` |

Open G-Helper code uses additional power-limit IDs used by ASUS firmware:
`0x001200a0` sPPT/PL2, `0x001200a1` EDC, `0x001200a2` TDC,
`0x001200a3` SPL/PL1, `0x001200b0` CPU PPT, `0x001200b1` slow/total PPT,
`0x001200c0` NVIDIA Dynamic Boost, `0x001200c1` fPPT, and `0x001200c2` GPU
temperature target. Armoury exposes the corresponding tuning fields, while
these exact numeric mappings are available in G-Helper's `AsusWmi.cs`.

## Notebook Aura

### Component and discovery path

The notebook implementation is native x64 code in `AuraPlugin.dll`. Its own
diagnostic strings name the relevant internal paths:

- `GetRGBKBStatus`: controller type, generation, layout, physical regions,
  packet features, and product family;
- `NBFWRunmodeReadback`: supported firmware effects, current effect, effect
  power, and lighting-region power state;
- `SetDeviceECMode`: application of an effect and its parameters;
- `GetDefaultModeFromNKEY`: one default effect and its colors;
- `ReadBassicEffectTableFromNkey`: a conditional Slash/N-Key table, not the
  normal notebook effect list;
- `GetGeneralSettings` and `SetGeneralSettings`: boot, awake, sleep, shutdown,
  region, and AC/DC lighting switches.

`Basic.ini` is a generic catalog of effect names, defaults, and UI fields. It
does not identify which firmware effects a particular notebook supports.

Modern ASUS notebooks can expose several HID interfaces under one USB device.
The useful interface is the one whose HID report descriptor declares Feature
report ID `0x5d`. Armoury obtains the Feature report length from the parsed HID
capabilities. Selecting the first hidraw node or always allocating 64 bytes is
not a portable replacement.

### Controller status report

After its signature Feature exchange, `GetRGBKBStatus` sends:

```text
5d 05 20 31 00 20
```

The response must echo the command header. Byte 5 acts as the populated
response end/length used by the parser. The established status fields are:

| Byte | Meaning |
| ---: | --- |
| 9 | controller type: `02` multi-zone, `03` per-key, `04` single-zone |
| 10 | keyboard generation/year field |
| 12 | firmware keyboard-layout identifier |
| 13 | physical lighting-region bit mask |
| 14 | controller and packet-format feature bits |
| 17 | product-family bit mask |

Byte 13 region bits are:

| Bit value | Region |
| ---: | --- |
| `0x01` | logo |
| `0x02` | lightbar |
| `0x10` | V-cut |
| `0x20` | Aero |
| `0x40` | Bump |
| `0x80` | rear glow |

Byte 14 contains default-color support (`0x04`), RGB wheel support (`0x08`),
the one-zone red/white marker (`0x10`), and the key-position-map packet format
(`0x40`). The last bit describes a packet format; it does not mean that the
keyboard has per-key lighting. The single-zone GV601VV reports it too.

Known byte 17 product-family bits are Strix (`0x01`), Flow (`0x02`),
Zephyrus (`0x04`), TUF (`0x08`), NR2301 (`0x10`), and desktop (`0x20`).

### Runtime effect mask

`NBFWRunmodeReadback` uses one of these requests:

```text
5d 9e 01 20 ...
5d 9e 01 15 ...
```

A valid response echoes report ID, command, subcommand, and selector, and has
success value `1` in byte 4. Bytes 20 and 21 are the firmware effect mask:

| Firmware bit | Armoury effect/index | `AuraModeNum` |
| --- | --- | --- |
| byte 20 bit 0 | Static / `0` | `Static (0)` |
| byte 20 bit 1 | Breath / `1` | `Breathe (1)` |
| byte 20 bit 2 | Color Cycle / `3` | `RainbowCycle (2)` |
| byte 20 bit 3 | Rainbow / `4` | `RainbowWave (3)` |
| byte 20 bit 4 | Star / `9` | `Star (4)` |
| byte 20 bit 5 | Rain / `10` | `Rain (5)` |
| byte 20 bit 6 | Reactive / `11` | `Highlight (6)` |
| byte 20 bit 7 | Laser / `12` | `Laser (7)` |
| byte 21 bit 0 | Ripple / `13` | `Ripple (8)` |
| byte 21 bit 1 | Strobing / `2` | `Pulse (10)` |
| byte 21 bit 2 | reserved | none |
| byte 21 bit 3 | Comet / `5` | `Comet (11)` |
| byte 21 bit 4 | Flash & Dash / `6` | `Flash (12)` |

The explicit translation is required because Armoury's effect index and
`AuraModeNum` are different namespaces.

The same response contains current state:

| Bytes | Meaning |
| --- | --- |
| 6 | current firmware effect index |
| 7..9 | primary RGB color |
| 10 | encoded speed: `e1 -> 3`, `eb -> 2`, `f5 -> 1` |
| 11 | direction, exposed by Armoury as raw value plus one |
| 13..15 | secondary RGB color |
| 19 | effect power: `00/55/aa/ff -> 0/1/2/3` |
| 20..21 | supported effect mask |
| 22..25 | classic power-region masks |
| 26..29 | optional extended AC/DC region masks |

The classic power layout is:

- Byte 22: logo boot/awake/sleep/shutdown in bits 0/2/4/6 and keyboard in
  bits 1/3/5/7.
- Byte 23: Aero awake in bit 0 and lightbar boot/awake/sleep/shutdown in bits
  1/2/3/4.
- Byte 24: V-cut states in bits 0..3 and Bump states in bits 4..7.
- Byte 25: rear-glow states in bits 0..3.

The extension is present when byte 26 bit 0 is set. On AC, byte 26 uses
keyboard bit 1, logo 2, lightbar 3, Aero 4, V-cut 6, and rear glow 7; byte 27
uses wheel bit 0 and Bump bit 1. Bytes 28 and 29 repeat that layout for DC.

### Other Aura queries

`GetDefaultModeFromNKEY` sends `5d 9b 01 01 08`. Response byte 5 is one mode,
bytes 6..8 are its color, byte 9 is an extra mode field, and bytes 10..12 are
default power RGB. It is not a supported-effect list.

The hotkey-capability status request uses `5d 05 20 31 00 10`. Matrix support
uses a separate `5d c2` probe and recognizes response type `0x80` or `0x81`.
Slash uses `5d d1 01 00 01` and response byte 5 bit 0. The function
`ReadBassicEffectTableFromNkey` sends a `0xd2` request only on the Slash path.
TUF general lighting uses WMI command `0x00100057` rather than this report.

Matrix settings are handled by `SetMatrixSettings`, not by the keyboard
effect mask. Its state contains `MasterSwitch`, `ScreenMode`, `LidMode`,
`BatterySavingMode`, `ECAnimeSwitch`, and separate EC animations for start,
shutdown, sleep, and power saving. `SetSpecifiedMatrixSwitch` updates the
master, battery-saving, or EC-animation switch independently. The plugin also
tracks `MatrixCurrentMode`, `MatrixDevicePreMode`, device battery state,
and ownership by software mode versus EC mode.

Slash has a separate `SlashLightCore`. `LoadSlashSetting` and
`ApplySlashSetting` manage theme ID, master switch, boot/shutdown, sleep,
lid-closed, low-battery, on-battery, power-saving enable/percentage, and
system-audio content. `LoadSlashContent` adds brightness, content type/path,
audio pattern, strength, decay, range, interval, and play count. Basic EC
events are written through the N-Key path; software content is forwarded to
the lighting-service interface. These settings cannot be generated from the
normal Aura effect list.

`Basic.ini` defines the wider software catalog: Static 0, Breath 1, Strobing
2, Color Cycle 3, Rainbow 4, Comet 5, Flash & Dash 6, Music 7, Temperature 8,
Star 9, Rain 10, Reactive 11, Laser 12, Ripple 13, Blade 14, Starry Night 15,
Flowtech 16, Irradiation 17, Screen Extension 18, Audio Analyzer 19, custom
slots 21/22, dark 99, sync 100, and Matrix modes 81..84. Its per-effect fields
include `Color1`, `Color2`, `Direction`, `Random`, `Ext1`, `Speed`,
`SpeedType`, and `DirectType`. Host-rendered effects and firmware effects must
not be conflated.

### Implementation rule for asusd

For controllers implementing this contract, the validated `0x9e` mask should
replace the firmware-effect fallback list, not be unioned with it. Unknown
bits are ignored. A response is accepted only after validating length, report
ID, command, selector, success byte, and the mandatory Static bit. Static model
data remains responsible for layout names, direct-addressing maps, zone maps,
Linux power-zone mappings, and compatibility fallback.

## Performance profiles and manual tuning

### Product modes versus firmware policy

`ThrottleGearXMLHelper.OverclockMode` defines Armoury's internal recipe IDs:

| Armoury recipe | Value |
| --- | ---: |
| Windows | 0 |
| Silent/Saves | 1 |
| Performance/Balance | 2 |
| Turbo | 3 |
| Manual | 4 |
| DC Manual | 9 |

These are not raw values for WMI ID `0x00120075`. `GetThermalPolicy` reads that
ID with `DSTS`, checks the presence result, and extracts the low policy bits.
`SetThermalPolicy` writes it with `DEVS` and accepts the change only when the
firmware returns `1`. Known firmware policy values are generally Balanced 0,
Turbo 1, Silent 2, and Manual 4. Armoury translates a high-level recipe into
thermal policy, fan data, power limits, GPU tuning, and Windows power policy.

`ThrottlePlugin.dll::ProcessRequest` exposes the service-side control map:

| Command | Recovered role |
| ---: | --- |
| `3` | initialization/status path |
| `4` | manual-profile information including `IsManualDC` and `SupportMultipleManual` |
| `6` | set current throttle recipe through `SetThrottleGearMode`; Manual on DC becomes recipe 9 |
| `9` | select user fan data using `Device`, `UserFanCurveIndex`, and `IsManualDC` |
| `10` | apply multiple-manual payload |
| `11` | report `NeedXTUDriver` |
| `16` | per-core/dynamic-core support and display data |
| `17` | report `OCControlByAP` |
| `18` | handle OC-controller change |
| `0x100` | query dynamic P/E-core state |
| `0x101` | set dynamic P/E-core state |

The model recipe normally comes from
`%ProgramData%\ASUS\ARMOURY CRATE Config\Data\ThrottleGear_<model>.xml`.
Manual state is split across `ThrottleGearManualFile.xml`, its backup,
`ThrottleGearDCManualFile.xml`, and `MultipleManualProfileAC.xml` /
`MultipleManualProfileDC.xml`. The available tuning controls and bounds are
therefore not generated entirely from BIOS data.

### Managed tuning schema

`ThrottleGearXMLHelper.dll` provides readable type names for the configuration
contract. Intel CPU fields include:

- `SupportDynamicCore`, `PCore`, and `ECore`;
- active-core ratios for one through ten active cores;
- core and cache voltage mode, value, offset, and IccMax;
- processor cache ratio;
- Turbo Boost power maximum, short maximum, and time window;
- PL1 and PL2;
- BIOS per-core values 0 through 7.

The dynamic-core UI minima recovered from the plugin are four P-cores and zero
E-cores. Runtime maxima come from firmware and must take precedence over any
model default.

AMD CPU fields include OC mode, all-core frequency, CPU voltage, PPT, EDC,
TDC, STAPM, APU sPPT, platform sPPT, and fPPT. NVIDIA fields include core and
memory offsets, power and thermal targets, Dynamic Boost, notebook thermal
target, Mode2, and Whisper level. The schema also contains AMD GPU, Intel GPU,
Qualcomm CPU/GPU, TGP, refresh-rate, and preferred-power-state models.

Every tuning item can carry mode-specific values, support flags, upper and
lower limits, DC limits, steps, and option sets. Firmware may expose a write
command, but it does not thereby declare that every value is safe. Linux
should use live range metadata when the kernel supplies it and an exact tested
model profile otherwise; it should never import a bound from a nearby model.

### Configuration-version gates

The managed helper contains feature gates tied to configuration versions:

| Feature introduced/gated | Configuration version |
| --- | --- |
| base schema | 1.4.1 |
| VCore | 1.4.8 |
| AMD CPU settings | 2.4.1 |
| AMD throttle controls | 2.5.9 |
| AMD GPU settings | 2.5.13 |
| Whisper | 3.2.11 |
| power OC | 3.3.9 |
| dynamic minimum FPS | 4.0.6 |
| combined AMD CPU/GPU path | 4.0.8 |
| fPPT | 4.1.1 |
| DC Manual | 4.2.0 |
| Intel GPU | 5.1.6 |
| Intel BIOS per-core | 5.1.9 |
| Dynamic Core | 5.4.9 |
| Mode2 | 5.7.0 |
| TGP Mode2 | 5.7.7 |
| NUC | 5.8.1 |
| Qualcomm | 5.9.0 |

These gates are parser/schema compatibility, not firmware version tests.

## Dynamic P/E-core control

`NativeGamingCenterHelper.dll` contains three clear functions:

| Function | Operation |
| --- | --- |
| `GetCurrentDynamicCore` | `DSTS 0x001200d2` |
| `GetMaxDynamicCore` | `DSTS 0x001200d3` |
| `SetCurrentDynamicCore` | `DEVS 0x001200d2` |

Both reads return a packed 32-bit value:

```text
bits  0..7   P-core count
bits  8..15  E-core count
bit      16  supported/valid
```

The write packs both counts atomically as `(ECore << 8) | PCore`. This matters:
two independent sysfs writes are not equivalent unless the kernel guarantees
the same transaction and rollback behavior.

`ThrottlePlugin` command `0x100` reports support, maximum P/E counts, minimum
P/E counts (`4` and `0`), and current counts. Command `0x101` rejects P-core
values below four or above the firmware maximum, rejects E-core values above
its maximum, skips an unchanged state, performs one packed write, waits about
300 ms, and emits notification `0x80010008`. The UI treats an accepted change
as restart-required; it does not silently restart the computer.

Support is therefore the conjunction of successful current/max firmware reads
and the model configuration flag. For Linux, a dynamic-core object should be
created only when the backend can expose current, maximum, minimum, atomic set,
and restart-required semantics. Readback of both counts is required after an
accepted write.

## Fan curves

`NativeGamingCenterHelper.dll` exposes `GetFanCurveTable`,
`SetFanCurveTable`, and `GetFanCurveLowerLimit`. Device selector values are:

| Selector | Fan | Curve ID | Lower-limit ID |
| ---: | --- | ---: | ---: |
| 0 | CPU | `0x00110024` | `0x00110026` |
| 1 | GPU | `0x00110025` | `0x00110027` |
| 2 | middle/system | `0x00110032` | `0x00110033` |

`GetFanCurveTable(device, profileIndex, out, len)` sends the selected curve ID
and profile index through the general WMI method and reads up to 16 bytes.
`SetFanCurveTable` sends the selected ID followed by up to 16 table bytes. The
same layout is used by G-Helper and the Linux ASUS platform code:

```text
bytes 0..7    eight monotonically increasing temperatures
bytes 8..15   eight corresponding duty/PWM values
```

Armoury distinguishes three factory curves, Silent, Default, and Turbo, from a
separate user/manual curve. The managed `FanCurveItem` contains
`SilentDegree`/`SilentRatio`, `DefaultDegree`/`DefaultRatio`, and
`TurboDegree`/`TurboRatio` for each point. User-curve index and cached tables
also appear in service registry/profile state; those cached values are not
live firmware capabilities.

Functions named `GetCPUFanStatus`, `GetGPUFanStatus`, and `GetIsAutoFanStatus`
read cached lower/upper/auto fields from the Armoury service registry. They
must not be mistaken for live fan telemetry. A Linux implementation should
obtain temperatures and RPM from hwmon, validate point count and monotonic
temperature order, honor live lower limits, apply the complete table, and
read it back when supported.

## Monitoring, sensors, and battery state

`MonitorPlugin.dll` does not read a single Armoury-specific telemetry
descriptor. It builds one CPU/GPU snapshot from several interchangeable
providers selected by platform, vendor, installed SDK, and support flags.
`ProcessRequest` returns the resulting fields to the UWP and to
`ResourceMonitorOSD.exe`.

The CPU object uses the following field names:

| Field | Value |
| --- | --- |
| `MaxCPUProcessorFreq` | advertised maximum processor frequency |
| `MaxCPUVoltage` | configured/display maximum voltage |
| `MaxCPUFanSpeed` | maximum fan scale |
| `CPUFanSpeed` | current CPU fan speed |
| `CPUMemoryFrequency` | configured memory frequency |
| `ProcessorFrequency` | current processor frequency |
| `Usage` | CPU utilization |
| `Temprature` | CPU temperature; the misspelling is part of the IPC field |
| `Voltage` | CPU voltage |
| `PowerConsumption` | CPU package power where the selected provider supplies it |
| `CPUFanAcoustic` | acoustic value from the fan-acoustic configuration/backend |

On Intel systems, temperature, voltage, power, and frequency can come from
`IntelOverclockingSDK.dll`/XTU. If that monitor is unavailable, the plugin
uses its CPU information reader, BIOS WMI, or AsIO path according to the
`Support BIOS Temperature/Voltage` and `Support ASIO
Temperature/Voltage` flags. Processor frequency also has a Windows PDH/WMI
path using `Processor Information(_Total)`, `MaxClockSpeed`, and
`CurrentClockSpeed`. Memory frequency comes from
`Win32_PhysicalMemory.ConfiguredClockSpeed`. AMD systems use the selected
`AMDCPULibrary*.dll` SDK for temperature, voltage, frequency, and status.

The GPU object contains:

| Field group | Values |
| --- | --- |
| current load | `ProcessorFrequency`, `MemoryFrequency`, `Voltage`, `Temprature`, `Usage`, `MemoryUsage`, `PowerConsumption` |
| clock reference | `ProcessorBaseFrequency`, `NVBoostFrequency`, `ROGBoostFrequency`, `OCFrequency`, `VRAMBaseFrequency`, `VRAMOCFrequency` |
| cooling | `MaxGPUFanSpeed`, `GPUFanSpeed`, `GPUFanAcoustic`, `EGPUFanSpeed`, `EGPUFanAcoustic` |

`GPUControlLibrary.dll` supplies vendor-specific maximum and current GPU
records through NVIDIA, AMD/ADLX, or Intel paths. Notebook fan speed can come
from the display-adapter API or ASUS ATK, while XG Mobile fan data comes from
`CDockingController::GetFanSpeed`. System-fan speed and acoustic level are
queried separately. A TGP value can also be added from WMI.

`MonitorSettings.ini` controls monitor behavior. Fan-acoustic configuration
is loaded from `AC_Config.FanAcoustic.*`. The OSD path publishes supported
levels and sampled values through `Global\ResourceMonitorShareMemory`;
recording has its own start/stop events. These are service and presentation
contracts, not additional firmware capabilities.

Battery state is another policy input rather than one universal firmware
control. `ArmouryCrate.dll` implements `BatteryLifeTileViewModel` with
`Windows.Devices.Power.Battery`, `RequestAggregateBatteryReport`, and the
`AggregateBattery_ReportUpdated` callback. `GPUPowerSavingPlugin.dll`
subscribes to AC/DC changes and `GUID_BATTERY_PERCENTAGE_REMAINING`; it uses
that state for Optimized GPU mode and low-battery notices. `AuraPlugin.dll`
uses `LowBattery`, `OnBatteryPower`, and `BatterySavingMode` for Matrix
and Slash behavior. These paths consume battery state; they do not define a
`DEVS` command for a charge threshold. Charge-limit control exposed by Linux
is therefore a separate kernel/asusd capability.

## GPU modes and power saving

### Hardware states

`GPUPowerSavingPlugin.dll` distinguishes two lower-level mechanisms:

- WMI `GPUStatus` ID `0x00090020`: Hybrid `0`, UMA/Eco `1`;
- MUX state: unknown `0`, iGPU `1`, dGPU `2`, with automatic/manual switch
  types supplied by NVIDIA DDS or AMD SAG paths.

Armoury's product modes are compositions:

| Product mode | Effective behavior |
| --- | --- |
| Standard | Hybrid, automatic policy disabled |
| Eco | UMA/dGPU disabled |
| Ultimate | hardware MUX routed to dGPU; restart/shutdown may be required |
| Optimized | automatic Standard-on-AC and Eco-on-battery policy |

Optimized is not a fourth firmware value.

### Display-routing blocker

`GPUPowerSavingPlugin` reads WMI `0x00090030`. The low byte describes both
connection and routing:

| Bits | Meaning |
| --- | --- |
| 0..3 | HDMI1, HDMI2, DP1, DP2 connected |
| 4..7 | corresponding port routed through the dGPU |

The plugin converts this to blocker flags for connected HDMI or DP outputs
that depend on the dGPU. This prevents Eco mode from disconnecting an active
external display without explanation.

### Optimized policy

`ImplementAutoMode` first checks that `UMA.ini` exists and has automatic mode
enabled. `DoSupportUMAControl` itself uses existence of that file as a model
support gate. On AC power it requests Hybrid. On battery it attempts UMA only
after checking dGPU-routed displays, processes occupying the dGPU, XG Mobile,
VBIOS update state, and incompatible DDS/SAG MUX states. A blocker results in
a notification rather than a forced transition.

`CheckSwitchEco` applies related checks when a recent Flow or other supported
model requests Standard-to-Eco directly. The plugin also coordinates vendor
services and VBIOS-update state. A Linux equivalent should implement
Optimized in a user/session policy agent over daemon-provided hardware states,
not as a fabricated firmware enum.

`UtilityPlugin` also reads/writes Dynamic GPU ID `0x00090029`. Its path first
normalizes Hybrid/MUX state, so it is a separate feature and not an alias for
Eco or Optimized.

## XG Mobile and external GPU switching

`GPUSwitchPlugin.dll` uses:

| ID | Meaning |
| ---: | --- |
| `0x00090017` | dock connected |
| `0x00090018` | physical switch/lock state |
| `0x00090019` | internal GPU versus external GPU selection |
| `0x0009001c` | BIOS-supported external GPU vendor mask |
| `0x0006005b` | notebook/dock PCIe generation |

For `0x00090019`, value zero selects the internal path. The external value has
low bit one and a vendor in the high byte: `0x0001` NVIDIA, `0x0101` AMD, and
`0x0201` Intel. The corresponding vendor enumeration values are 0, `0x100`,
and `0x200`.

`0x0009001c` uses bit 0 for NVIDIA, bit 1 for AMD, and bit 2 for Intel; the
plugin falls back to NVIDIA when that capability read is unavailable.
`0x0006005b` yields PCIe generations 1 through 5 and uses generation 3 as its
fallback. When both sides are generation 4, the service contains a coordination path
that can set value 3 and require a reboot for compatibility.

The switch workflow is substantially more than one `DEVS` call. It checks
dock presence, physical lock, external power, GPU-using processes, attached
displays, BitLocker state, notebook and dock PCIe generations, four-part PCI
identity, vendor driver state, and VBIOS compatibility. Some outcomes require
a reboot. A Linux API should expose these blockers and pending state rather
than reporting success as soon as the firmware write returns.

`ExternalGPUConfigHelper2.dll` supplies a managed configuration schema.
`EgcBaseInfo` contains device/subsystem IDs, feature version, base-feature bits
for manual/NVIDIA/Whisper functions, GPU-feature bits for processor and memory
clock, NVIDIA bits for power/texture controls, and Whisper-mode bits for two
manual modes and Silent. `GpuParameter` carries Saves, Balance, Turbo, Manual,
Upper, and Lower values. This is another example of runtime identity combined
with static safe parameters.

## Display and general notebook controls

`UtilityPlugin.dll` builds a capability bit mask from WMI support, provider
availability, registry/component presence, and OS APIs:

| Capability bit | Feature | Backing source |
| ---: | --- | --- |
| `0x000001` | Windows-key status | service state/registry |
| `0x000002` | ROG-key status | service/runtime state |
| `0x000004` | ESS | WMI `0x00060091` |
| `0x000008` | G-SYNC state | WMI `0x00090016` |
| `0x000010` | touchpad/runtime control | runtime provider |
| `0x000020` | XMP | service state/registry |
| `0x000040` | ROG boot sound | WMI `0x00130022` |
| `0x000080` | panel overdrive | signed external provider |
| `0x000100` | HDR | Windows display/registry APIs |
| `0x000200` | touchscreen | WMI `0x00050036` |
| `0x000400` | Windows-key BIOS control | WMI `0x00130024` |
| `0x000800` | panel power saving | ASUSFRQ component/registry |
| `0x008000` | display profile | WMI `0x0005001c` |
| `0x010000` | MUX switching | WMI `0x00090016` plus GPU backend |
| `0x020000` | Mini-LED generation 1 | WMI `0x0005001e` |
| `0x100000` | Dynamic GPU | WMI `0x00090029` |
| `0x200000` | ABOT/optimal brightness | WMI `0x0005002a` |
| `0x400000` | Mini-LED generation 2/balance | WMI `0x0005002e` |
| `0x800000` | Fn lock | WMI `0x00100023` plus HControl/OSD path |

Most scalar WMI reads reject the unsupported marker and use a low value nibble
or byte. Most sets require `DEVS` to return `1`.

The display-profile buffer is not a scalar:

```text
byte 0       display profile type in low bit
bytes 4..5   UHD refresh rate, little-endian u16
bytes 6..7   FHD refresh rate, little-endian u16
```

The set path writes the low profile bit. Additional high input bits drive
Windows-side resolution synchronization. Linux should enumerate actual DRM or
desktop display modes rather than treating those two cached refresh numbers as
a universal mode list.

Mini-LED generations 1 and 2 are separate controls named Backlight Control and
Backlight Control with Balance in the plugin. ABOT is optimal-brightness
policy and can be disabled under dGPU/AC conditions. HDR is implemented with
Windows display and registry APIs, not ASUS BIOS. Panel power saving is tied
to an ASUSFRQ component.

Panel overdrive is particularly instructive. `UtilityPlugin` reads
`ODControlPath` from the ASUS utility configuration, verifies and loads a
signed provider, calls its `IsInstalled` export, then uses generic `Get("Mode")`
and `Set("Mode")` exports. G-Helper uses raw WMI ID `0x00050019`, but
this Armoury package deliberately delegates the feature to a driver/provider.
Linux should prefer the kernel/asusd `panel_overdrive` attribute when present.

Fn lock also crosses layers: WMI support is combined with an HControl window
message/OSD path. A successful raw write alone does not reproduce the complete
Windows behavior.

## GameVisual, color management, and OLED Care

`HWPlugin.dll` delegates attached-display enumeration to
`AsusDisplayMonitorInfo.dll` and returns `MONITORINFOJSON` through
`getAllMonitorsJson`. `GameVisualPlugin.dll` then combines the selected
display, DMI model/family, Windows DisplayConfig state, HDR/advanced-color
state, an ASUS Splendid provider, and model-specific ICM files.

The service command surface contains:

| Command/function | Role |
| --- | --- |
| `GetGVSupported` | build the available GameVisual feature/mode flags |
| `GetGVMode` / `SetGVMode` | read or apply the active visual profile |
| `GetCTValue` / `SetCTValue` | color-temperature value |
| `GetAllCTValues` | available color-temperature choices |
| `GetECLevel` / `SetECLevel` | Eye Care level |
| `GetCSMode` / `SetCSMode` | color-space mode |
| `DeployICMFiles` | deploy the model/display ICM profiles |
| `ReloadSplendidConfigEntries` | reload the ASUS display-provider configuration |
| `GetDisplayTopology` | query active Windows display paths |

Named GameVisual modes include Normal, Vivid, Eyecare, Racing, Scenery, and
Cinema. The plugin also handles ASUS sRGB, Display P3/DCI-P3, factory
calibration tags, and per-display ICM names. Applying a mode goes through
`SetGVModeCore`/the Splendid provider; HDR changes can reset or override the
selected mode. This is display-driver and color-management integration, not an
ASUS WMI mode table.

OLED Care is another policy layer in the same plugin. Its handlers include
`GetDimmingValue`/`SetDimmingValue`, Pixel Refresh, Pixel Shift, Target
Mode, taskbar hiding/transparency, and screensaver configuration. Pixel Shift
and Target Mode are delegated to `AsusOLEDShifter.exe` and
`AsusTargetMode.exe`; other items modify Windows desktop state. The plugin
first checks `OLEDPanelSupported`, display topology, HDR, and the installed
provider.

For Linux, ICC/color-space handling belongs to the compositor and color
management stack. OLED desktop policy belongs to the user session. Only a
real kernel/asusd panel attribute should become a privileged daemon control.

## Hotkeys

`UtilityPlugin` opens the notebook HID interface using its descriptor-sized
report and sends:

```text
5d 05 20 31 00 10
```

The response is checked under the same Aura mutex. The recovered capability
rules are:

- byte 10 satisfying `(value & 0x22) == 0x22` selects the 2022 layout;
- byte 14 bit `0x20` indicates Fn+ROG support;
- on the 2022 layout, byte 15 bit 0 is custom-key support and bit 1 is ROG-key
  support;
- on older layouts, byte 6 bit 1 is the custom-key marker and bit 7 is the
  ROG-key marker.

G-Helper's open dispatcher maps common ASUS event IDs: M3/microphone
124, M4/ROG 56, older M4/Fn+F12 134, performance cycling 93/153/174, and Aura
cycling 178. Some M1/M2 keys arrive as ordinary volume events. These event IDs
are family-dependent and should be treated as mappings, not universal firmware
capability bits.

The capability report only establishes which controls exist and which report
layout applies. Armoury divides assignment and execution among four additional
components:

| Component | Recovered behavior |
| --- | --- |
| `CustomHotkeyPlugin.dll` | `GetHKSupportFlags` and `GetHKLayout` describe the available keys. `GetInitHKSettings`, `GetHKConfigurations`, `SetHKOpt`, and `SetHKAllOpts` read or write their assignments. Per-key configuration is stored as `hk_%02u_opc` and `hk_%02u_arg`; supported firmware values are read from the EC and written with `SetHotkeyFunctSingle` under `Global\AacNBHalMutex`. Linked applications and macro profiles are handled separately through `SetLinkApp`, `UpdateLinkedAppList`, and the app-profile commands. |
| `MultiwheelPlugin.dll` | `GetMWSupportFlags`, `GetInitSWModes`, `SWReadVals`, and `SWWriteVals` manage the wheel feature reported by the EC. Its modes include volume, mouse scrolling, microphone level, panel brightness, keyboard brightness, task switching, and performance-mode switching. `wheel_mode`, `SmartWheel`, tutorial state, and per-application profiles form the software policy around those hardware events. |
| `MacroKeyPlugin.dll` | `StartRecordKey`, `GetKeys`, and `StopRecordKey` capture keyboard and mouse input; `ReloadMacroFileList` and `GetMacroFileList` manage `.GMAC` files. Device/profile XML records carry the target, command, device type, macro ID, name, path, and device identifiers. Recording, application matching, backup, restore, and replay are user-session behavior rather than firmware capabilities. |
| `SoulKeyServicePlugin.dll` | `SoulKeyEvent` joins proximity/card events and software actions with the selected `ThrottleGearMode`; the plugin keeps separate `HyperFanModeAC` and `HyperFanModeDC` values and forwards performance changes to the throttle service. |

This split matters when reproducing the feature. The EC can retain a compact
opcode/argument assignment for keys that support it, while executable paths,
AUMIDs, command arguments, macro files, per-application matching, and OSD state
belong to the user session. A missing EC feature report causes Armoury to reset
or hide the corresponding SmartWheel/custom-key configuration instead of
inventing support from the product name.

On Linux, the daemon should expose only hardware events that the kernel or a
validated HID path actually reports. User-selected actions belong in an
unprivileged session agent; hardware changes remain daemon calls. Arbitrary
shell text must never be executed as root.

## AI noise cancellation and audio routing

`DenoiseAIPlugin.dll` is not a notebook firmware effect. It discovers the
Windows capture/render endpoints, checks the installed audio services and APO
providers, chooses a vendor backend, and changes that backend's endpoint
properties.

`CheckAINoiseSupport` and `CheckAINoiseServicesSupportOrNot` combine product
identity, BIOS support flags, driver/service versions, known USB VID/PID
devices, and available audio endpoints. The backend flags named in the plugin
are IG/Intelligo, Realtek, Forte, Realtek Gen5, and IGAPO. The corresponding
paths include:

| Path | Main handlers |
| --- | --- |
| generic endpoint | `SetDefaultAudioDevice`, capture/render MMDevice enumeration |
| Intelligo/APO | `GetIGAPODeviceList`, `GetIGAPODeviceInfomation`, `SetIGAPODeviceLevel` |
| Realtek | `SetRtkGenFBypass`, `SetRtkGenFMode`, `SetRtkGenFLevel`, microphone UTS settings |
| C-Media/USB | VID/PID matching and `Enable_NOISEREDUCTION_LFX` |
| software engine | Intelligo libraries, `ArmouryCrate.DenoiseAI.exe`, and PortAudio session monitoring |

The plugin watches endpoint add/remove/default changes and maintains separate
uplink/capture and downlink/render devices such as AI Noise-cancelling Input,
AI Noise-cancelling Output, Intelligo AI Microphone, and Intelligo AI Speaker.
Its level and bypass values live in Windows audio endpoint/APO properties and
vendor service state.

A Linux implementation belongs in PipeWire/WirePlumber or a dedicated user
audio service. Presence of an ASUS WMI transport in this plugin is not a
reason to place audio routing or DSP state in `asusd`.

## Scenario profiles

`ConfigXML_ScenarioProfile.dll` defines the managed XML contract. The root is
`ASUS_ROG_ARMOURY_CRATE`, with an `ARMOURY_CRATE_PROFILECONFIG` section.
Hardware attributes include `ESSStatus`, `TouchPadStatus`, `ROGKeyStatus`,
`OverDriveStatus`, `WinKeyStatus`, `TouchScreenStatus`, and
`ReleaseResourceStatus`. Separate nodes describe third-party applications,
service applications, peripheral devices, and linked applications.

Linked applications can be matched by application ID, local/relative target
and start path, or AUMID. `ScenarioProfilePlugin.dll` stores the original
state, caches the active scenario, applies linked settings through the other
plugins, and restores state when no matching application remains. It also
coordinates system volume/mute, HyperFan/performance recipes, and peripheral
profiles.

Scenario profiles are orchestration and policy, not a firmware capability
source. A Linux implementation should keep them in the user session and
compose only capabilities currently advertised by the daemon.

## Peripherals and non-notebook devices

The same architectural lesson appears in peripheral pages. A device-specific
`caps.json` can enumerate effect controls, DPI stages, polling options,
calibration, power behavior, and transport restrictions. `DeviceWhiteList.ini`
can bind several physical PIDs to one logical GUID while disabling Aura for a
Bluetooth variant. `DeviceHalDetect.dll` contributes live physical identity,
parentage, LED count, and hardware switches.

Therefore a future general peripheral descriptor should contain logical and
physical IDs, transport, backend, firmware version, per-feature read/write
state, choices/ranges, restart requirement, and data source. Notebook HID
report `0x5d` cannot be generalized into a descriptor for all ASUS devices.

## What this means for asusd

The Armoury implementation is not a clean single capability protocol. It is a
large compatibility stack built from plugins, model files, cached state,
driver probes, runtime firmware queries, and policy. Reproducing its feature
coverage by adding model names to scattered hard-coded tables will continue to
lose capabilities and will also risk exposing unsafe ones.

A maintainable Linux model is:

```text
transport layer
  HID Feature reports / kernel attributes / ASUS WMI / vendor GPU APIs

validated protocol parsers
  typed status, choices, ranges, support markers, readback

capability resolver
  runtime facts first, exact tested fallback second, no speculative union

daemon objects
  stable D-Bus properties with pending/restart/error semantics

session policy
  Optimized GPU mode, scenarios, hotkey actions, display policy
```

Each exposed capability should record its source (`firmware`, `kernel`,
`driver`, `model fallback`, or `policy`). It should include
current value, writable state, choices or range, units, reboot requirement,
and a readback result. This makes unsupported controls disappear naturally and
makes partial support diagnosable.

Runtime discovery is appropriate for:

- modern notebook Aura effect masks and controller regions;
- dynamic P/E current and maximum counts;
- WMI support markers and live scalar states;
- actual fan table and lower limits where exposed;
- dock presence, lock, vendor mask, and PCIe generation;
- live kernel/asusd possible values.

Static tested data is still appropriate for:

- safe manual tuning limits and mode recipes;
- direct-addressing and keyboard layout maps;
- old firmware without a capability query;
- peripheral presentation/control schemas;
- model-specific ordering or safety quirks that runtime data cannot express.

## Accompanying Aura patch

The code accompanying this report implements only the validated modern
notebook Aura discovery path. It does not claim to convert every subsystem in
this document to runtime discovery.

The implementation:

```text
enumerates hidraw interfaces
  -> selects the descriptor declaring Feature report 0x5d
  -> derives the report length from the HID descriptor
  -> reads and validates controller status 0x05
  -> reads and validates firmware state/effect mask 0x9e
  -> maps known mask bits to AuraModeNum
  -> publishes the list through existing asusd configuration and D-Bus paths
```

`rog-platform` now parses HID report descriptors, including global Push/Pop,
and performs variable-length Linux Feature-report ioctls. `rog-aura` owns pure
parsers for status, effect mask/current state, and lighting power. `asusd`
selects the correct HID interface before device deduplication, probes modern
`0x19b6` and `0x1a30` notebook controllers, and falls back to
`aura_support.ron` when a response is absent or invalid.

A valid firmware list is authoritative. Saved settings remain for modes still
supported; an obsolete current mode falls back to Static. The exact GV601VV
fallback also contains Color Cycle, so the laptop remains correct when runtime
probing is temporarily unavailable.

The other sections are an implementation map for subsequent focused changes.
They should be developed and reviewed per subsystem because fan, power, core,
GPU, and dock writes have a materially larger safety surface than Aura mode
enumeration.

## GV601VV validation

The Aura implementation was tested on a ROG Flow X16 with board `GV601VV`,
BIOS `GV601VV.314`, and Aura USB device `0b05:19b6`. The selected interface
declared a 64-byte Feature report for ID `0x5d`.

The live data was:

```text
0x05 bytes 0..17:
5d 05 20 31 00 1a 01 40 00 04 23 04 01 00 46 03 11 02

0x9e header:       5d 9e 01 {20|15} 01
0x9e bytes 20..21: 07 02
```

The status identifies a single-zone Flow controller. Both selectors returned
the same mask. `07 02` decodes to Static, Breathe, Color Cycle, and
Strobing/Pulse. Before runtime discovery D-Bus exposed `[0, 1, 10]`; afterward
it exposed `[0, 1, 2, 10]`.

All four effects were selected through the normal daemon API and read back in
this order:

```text
2 -> 10 -> 0 -> 1 -> 2
```

Color Cycle remained active. The test did not require a reboot.

A read-only capability snapshot also confirmed that this machine currently
exposes daemon objects for boot sound, charge mode, dGPU disable, eGPU
connected/enable, GPU MUX, NVIDIA Dynamic Boost and temperature target, panel
overdrive, SPL, and sPPT. It does not currently expose dynamic-core or Mini-LED
objects. That absence is important: this report documents those protocols but
does not pretend the local firmware/backend supports them.

The patch passes:

```text
cargo fmt --check
cargo check --workspace
cargo test -p rog_aura -p rog_platform -p asusd --lib
cargo clippy -p rog_aura -p rog_platform -p asusd --all-targets -- -D warnings
git diff --check
```

The combined library run passes 33 tests; five existing environment-dependent
tests remain ignored.

## Source map for further work

| Topic | Armoury source | Useful functions/types/data |
| --- | --- | --- |
| physical discovery | `DeviceHalDetect.dll` | SMBIOS, SetupAPI/HID, ACPI/WMI, USB/SMBus paths |
| device/transport catalog | `DeviceWhiteList.ini` | VID/PID sections, GUID, mode, SDK, feature-removal flags |
| device-page routing | `DeviceServicePlugin.dll` | device info, lighting control, profiles, Aura Creator/Matrix messages |
| notebook Aura | `AuraPlugin.dll` | `GetRGBKBStatus`, `NBFWRunmodeReadback`, `SetDeviceECMode`, general settings |
| generic Aura UI | `Basic.ini` | effect indices and editable field schema |
| performance IPC | `ThrottlePlugin.dll` | `ProcessRequest`, `SetThrottleGearMode`, dynamic-core handlers |
| telemetry | `MonitorPlugin.dll` | CPU/GPU provider selection, monitor fields, fan/acoustic values, OSD shared memory |
| WMI and fans | `NativeGamingCenterHelper.dll` | `ControlWMIMethod`, thermal policy, fan tables, dynamic cores |
| tuning model | `ThrottleGearXMLHelper.dll` | `OverclockMode`, CPU/GPU setting classes, `FanCurveItem`, limits |
| GPU Eco/Optimized | `GPUPowerSavingPlugin.dll` | `ImplementAutoMode`, `DoSupportUMAControl`, `CheckSwitchEco` |
| vendor GPU state | `GPUControlLibrary.dll` | NVAPI/NVML, ADL/ADLX, Intel backends |
| XG Mobile | `GPUSwitchPlugin.dll` | docking, lock, vendor, PCIe generation, switch blockers |
| XG parameter model | `ExternalGPUConfigHelper2.dll` | `EgcBaseInfo`, `GpuParameter`, NVIDIA/Whisper settings |
| display/general | `UtilityPlugin.dll` | capability mask, display profile, Mini-LED, ABOT, OD provider, hotkeys |
| display discovery | `HWPlugin.dll`, `AsusDisplayMonitorInfo.dll` | `getAllMonitorsJson`, `MONITORINFOJSON` |
| color/OLED | `GameVisualPlugin.dll` | GameVisual mode/temperature/color-space handlers, Splendid/ICM, OLED Care |
| audio DSP | `DenoiseAIPlugin.dll` | endpoint discovery, backend selection, APO/vendor level and bypass handlers |
| custom controls | `CustomHotkeyPlugin.dll`, `MultiwheelPlugin.dll`, `MacroKeyPlugin.dll`, `SoulKeyServicePlugin.dll` | EC key assignments, wheel modes, per-app actions, `.GMAC` macros, and performance-event forwarding |
| scenarios | `ScenarioProfilePlugin.dll`, `ConfigXML_ScenarioProfile.dll` | linked-app matching, state save/restore, XML schema |
| open implementation | G-Helper `AsusWmi.cs`, `AsusHid.cs`, `Aura.cs` | WMI IDs, HID framing, fan table layout, event mappings |

## Known gaps

The semantics of several utility IDs seen in adjacent
paths (`0x00130023`, `0x00060095`, and `0x00090041`), complete peripheral
schemas beyond the inspected device families, and the safe tuning files for
models not present in the package. These should remain unknown rather than be
assigned a guessed control.

## Conclusion

The missing Color Cycle exposed a broader architectural problem: a static
model table had been used as if it were a complete hardware description.
Armoury's own implementation shows that no single source is complete. It
queries firmware where a protocol exists, supplements that data with exact
model and transport configuration, checks the installed driver stack, and
then applies service policy.

The Aura patch reproduces that model for one well-understood runtime protocol
and fixes the GV601VV without blindly enabling effects. This report separates
the remaining firmware contracts from Armoury's Windows-specific plumbing so
that future asusd work can replace hard-coded assumptions with validated,
typed capabilities one subsystem at a time.
