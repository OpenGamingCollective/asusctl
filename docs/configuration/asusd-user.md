# asusd-user Configuration

`asusd-user` is an optional per-user daemon. It plays custom Aura effects and AniMe sequences while `asusd` continues to own the hardware. It is not required for the built-in Aura modes or the system AniMe configuration.

Configuration is read from `$XDG_CONFIG_HOME/rog/` (normally `~/.config/rog/`). If the XDG config directory cannot be determined, `asusd-user` falls back to `/tmp/rog/`. The fallback is a last resort and is not persistent across reboots.

## Service and logs

Enable and start the user service:

```bash
systemctl --user enable --now asusd-user.service
systemctl --user status asusd-user.service
```

Read its logs with:

```bash
journalctl --user -b -u asusd-user
```

At startup, `asusd-user` waits for the system daemon's `xyz.ljones.Asusd` name on the system bus. If the name does not appear within 30 seconds, it exits with a timeout error and the service restarts. This is normal when `asusd` is not running or the laptop is unsupported.

## User config

The main file is `~/.config/rog/rog-user.ron`:

```ron
(
    active_anime: Some("anime-default"),
    active_aura: Some("aura-default"),
)
```

| Field | Description |
|---|---|
| `active_anime` | Name of the active AniMe sequence file, without the `.ron` extension. `None` disables the user AniMe sequence. |
| `active_aura` | Name of the active Aura effect file, without the `.ron` extension. `None` disables the user Aura sequence. |

The names refer to files in the same directory. For example, `active_aura: Some("my-aura")` loads `~/.config/rog/my-aura.ron`.

`rog-user.ron` uses `#[serde(default)]`, so a missing field is treated as `None`. An empty file containing only `()` is valid and loads with both fields set to `None`. The file created on first run is different: it sets both fields to the `*-default` names.

> [!NOTE]
> `asusd-user` only starts the AniMe part when `asusd` exposes the `xyz.ljones.Anime` interface. If your laptop has no AniMe Matrix, `active_anime` has no effect.

### First-run files

On first start, `asusd-user` creates `aura-default.ron` and `anime-default.ron` next to `rog-user.ron`:

| File | Contents |
|---|---|
| `aura-default.ron` | `Breathe` on `W`, `A`, `S`, `D` (`Low`, colours `255,0,20` to `20,255,0`); `Breathe` on `F` (`High`, red to red); `Static` on `RCtrl`, `LCtrl`, `Esc` (blue); `DoomFlicker` on `N9` (blue, `80`/`40`). |
| `anime-default.ron` | `AsusImage` of `diagonal-template.png` (fade 2 s in, no hold, 2 s out); `AsusAnimation` of `Sunset.gif` (fade 6 s in, no hold, 3 s out); `ImageAnimation` of `sonic-run.gif` (scale `0.9`, angle `0.65`, fade 2 s / hold 2 s / 2 s); `Image` of `rust.png` (scale `1.0`, fade 2 s / hold 1 s / 2 s); `Pause` for 1 s; `ImageAnimation` of `sonic-wait.gif` (scale `0.9`, translation `(3, 2)`, `Count(2)`). |

Edit these files or point `active_aura` and `active_anime` at your own files.

## Aura effects

A user Aura file is a `ConfigAura`:

```ron
(
    name: "my-aura",
    aura: (
        effects: [
            Static((
                led: W,
                colour: (r: 255, g: 0, b: 20),
            )),
            Breathe((
                led: A,
                start_colour1: (r: 0, g: 0, b: 255),
                start_colour2: (r: 255, g: 0, b: 0),
                speed: Med,
            )),
            DoomFlicker((
                led: N9,
                start_colour: (r: 0, g: 0, b: 255),
                max_percentage: 80,
                min_percentage: 40,
            )),
            DoomLightFlash((
                led: Esc,
                start_colour: (r: 255, g: 0, b: 0),
                max_percentage: 100,
                min_percentage: 20,
            )),
        ],
        zoned: false,
    ),
)
```

`name` must match the file stem. A file named `my-aura.ron` must contain `name: "my-aura"`; otherwise the daemon writes its state to a different file. `asusd-user` loads the file selected by `active_aura` at startup.

`aura` is an `AdvancedEffects` value with exactly two fields:

| Field | Description |
|---|---|
| `effects` | Ordered list of effects. Each effect targets one `LedCode`. |
| `zoned` | Selects the packet layout. `false` uses the per-key layout; `true` uses the four-zone/lightbar layout. |

`zoned` does not enable individual zones; it changes how the generated USB packets are addressed. Use `zoned: true` with the `ZonedKb*` and `Lightbar*` codes on a zoned keyboard, and `zoned: false` with per-key codes such as `W`, `A`, or `Esc`.

### Effects

There are four effects:

| Effect | Fields | Description |
|---|---|---|
| `Static` | `led`, `colour` | Constant colour. |
| `Breathe` | `led`, `start_colour1`, `start_colour2`, `speed` | Fades between two colours. |
| `DoomFlicker` | `led`, `start_colour`, `max_percentage`, `min_percentage` | Random flicker between brightness percentages. |
| `DoomLightFlash` | `led`, `start_colour`, `max_percentage`, `min_percentage` | Flash effect between brightness percentages. |

`Colour` is a named-field struct:

```ron
colour: (r: 255, g: 0, b: 20),
```

> [!WARNING]
> The tuple form `(255, 0, 20)` is not valid RON for `Colour`. Always use the field names `r`, `g`, and `b`.

`Speed` is `Low`, `Med`, or `High`. The enum defaults are `Speed::Med` and `Colour { r: 166, g: 0, b: 0 }`, but the effect structs still require these fields in RON.

`max_percentage` and `min_percentage` are percentages of `start_colour`, not absolute brightness. The daemon computes the maximum and minimum colours by multiplying each channel by the percentage. It does not validate that `max_percentage` is greater than or equal to `min_percentage`; keep `max >= min`.

### Zones

The `led` field accepts any `LedCode` variant. The zone-oriented variants are:

| `LedCode` | Description |
|---|---|
| `SingleZone` | Single-zone keyboard. |
| `ZonedKbLeft` | Left zone of a four-zone keyboard. |
| `ZonedKbLeftMid` | Left-centre zone of a four-zone keyboard. |
| `ZonedKbRightMid` | Right-centre zone of a four-zone keyboard. |
| `ZonedKbRight` | Right zone of a four-zone keyboard. |
| `LightbarRight` | Right end of the lightbar. |
| `LightbarRightCorner` | Right corner of the lightbar. |
| `LightbarRightBottom` | Right-bottom lightbar segment. |
| `LightbarLeftBottom` | Left-bottom lightbar segment. |
| `LightbarLeftCorner` | Left corner of the lightbar. |
| `LightbarLeft` | Left end of the lightbar. |

The remaining `LedCode` variants are grouped below. `RogApp` is the enum's default value, but `led` is required in every effect.

| Group | Variants |
|---|---|
| System and media | `VolUp`, `VolDown`, `MicMute`, `RogApp`, `RogFan`, `MediaPlay`, `MediaStop`, `MediaNext`, `MediaPrev`, `PrtSc`, `Pause` |
| Main keyboard | `Esc`, `F1`, `F2`, `F3`, `F4`, `F5`, `F6`, `F7`, `F8`, `F9`, `F10`, `F11`, `F12`, `Del`, `Tilde`, `N1`, `N2`, `N3`, `N4`, `N5`, `N6`, `N7`, `N8`, `N9`, `N0`, `Hyphen`, `Equals`, `Backspace`, `Backspace3_1`, `Backspace3_2`, `Backspace3_3`, `Home`, `Tab`, `Q`, `W`, `E`, `R`, `T`, `Y`, `U`, `I`, `O`, `P`, `LBracket`, `RBracket`, `BackSlash`, `PgUp`, `Caps`, `A`, `S`, `D`, `F`, `G`, `H`, `J`, `K`, `L`, `SemiColon`, `Quote`, `Return`, `Return3_1`, `Return3_2`, `Return3_3`, `PgDn`, `LShift`, `LShift3_1`, `LShift3_2`, `LShift3_3`, `Z`, `X`, `C`, `V`, `B`, `N`, `M`, `Comma`, `Period`, `FwdSlash`, `End`, `LCtrl`, `LFn`, `Meta`, `LAlt`, `Spacebar`, `Spacebar5_1`, `Spacebar5_2`, `Spacebar5_3`, `Spacebar5_4`, `Spacebar5_5`, `RAlt`, `RCtrl`, `Up`, `Down`, `Left`, `Right`, `RFn` |
| Numpad | `Star`, `NumPadDel`, `NumPadPlus`, `NumPadEnter`, `NumPadPause`, `NumPadPrtSc`, `NumPadHome`, `NumLock`, `Rshift`, `Rshift3_1`, `Rshift3_2`, `Rshift3_3` |
| Lid and zones | `LidLogo`, `LidLeft`, `LidRight`, `SingleZone`, `ZonedKbLeft`, `ZonedKbLeftMid`, `ZonedKbRightMid`, `ZonedKbRight` |
| No-op | `Spacing`, `Blocking` (ignored by effects) |

The full enum, including the lightbar variants, is in [`rog-aura/src/keyboard/advanced.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-aura/src/keyboard/advanced.rs).

> [!NOTE]
> `asusd-user` sends a new direct-addressing packet every 33 ms. While a custom Aura sequence is active it continuously updates the keyboard, so built-in mode changes made with `asusctl aura` may not be visible until you disable the user sequence by setting `active_aura: None`.
> After editing `rog-user.ron`, restart `asusd-user.service`; the daemon reads the file only at startup. Then select the built-in mode again.

## AniMe sequences

A user AniMe file is a `ConfigAnime`:

```ron
(
    name: "my-anime",
    anime: [
        Pause((secs: 1, nanos: 0)),
    ],
)
```

| Field | Description |
|---|---|
| `name` | Must match the file stem. |
| `anime` | Ordered list of `ActionLoader` actions. |

The action format is shared with `/etc/asusd/anime.ron`; see [AniMe sequences](anime-sequences.md) for all five action variants, timing, units, and PNG handling.

## Recovery

Invalid RON is handled by `config-traits`:

- `rog-user.ron` is renamed to `rog-user.ron-old` and a fresh default is written.
- A broken `ConfigAura` or `ConfigAnime` file is renamed to `<name>.ron-old`; the daemon continues with built-in defaults.
- A zero-length `rog-user.ron` is not renamed. The loader logs a zero-length read and writes the first-run defaults (both active fields set). A file containing `()` parses as an empty struct and loads with both active fields set to `None`.
- A file that parses but points at a missing or undecodable image/GIF is not renamed. `asusd-user` exits with an error while building the sequence and the user unit restarts it. Check the log for the failing path.
- The service logs the parse failure and the rename. Check `journalctl --user -b -u asusd-user`.

A broken custom sequence can cause the daemon to rewrite `aura-default.ron` or `anime-default.ron`; back them up before restarting.

To recover, edit the `-old` file, rename it back over the generated file, and restart `asusd-user.service`. Keep the `name` field equal to the file stem.

## Source of truth

- [`asusd-user/src/config.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/asusd-user/src/config.rs): `rog-user.ron`, `ConfigAura`, and `ConfigAnime`.
- [`config-traits/src/lib.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/config-traits/src/lib.rs): loading, `-old` recovery, and `.cfg`/`.conf` migration.
- [`asusd-user/src/daemon.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/asusd-user/src/daemon.rs): service startup and active-sequence selection.
- [`rog-aura/src/effects/mod.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-aura/src/effects/mod.rs): `AdvancedEffects` and the `Effect` enum.
- [`rog-aura/src/effects/static_.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-aura/src/effects/static_.rs), [`rog-aura/src/effects/breathe.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-aura/src/effects/breathe.rs), and [`rog-aura/src/effects/doom.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-aura/src/effects/doom.rs): effect fields.
- [`rog-aura/src/keyboard/advanced.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-aura/src/keyboard/advanced.rs): `LedCode` and packet layout.
- [`rog-aura/src/builtin_modes.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-aura/src/builtin_modes.rs): `Colour` and `Speed`.
- [`data/asusd-user.service`](https://github.com/OpenGamingCollective/asusctl/blob/main/data/asusd-user.service): user service unit.
