# AniMe Sequences

AniMe sequences are ordered lists of `ActionLoader` values. The system daemon and the user daemon use the same format.

## Where the format is used

| File | Owner | Contents |
|---|---|---|
| `/etc/asusd/anime.ron` | `asusd` | Display settings plus the `system`, `boot`, `wake`, and `shutdown` action lists. |
| `~/.config/rog/<name>.ron` | `asusd-user` | A `ConfigAnime` with a `name` and an `anime` action list, selected by `active_anime` in `rog-user.ron`. |

Both files hold the actions as a `Vec<ActionLoader>`. The system file wraps them in `AniMeConfig`; the user file wraps them in `ConfigAnime`.

## Actions

`ActionLoader` has five variants:

| Variant | Fields | Description |
|---|---|---|
| `AsusAnimation` | `file`, `time`, `brightness` | A full ASUS-format diagonal GIF. The frames are precomputed and immutable. |
| `AsusImage` | `file`, `time`, `brightness` | A PNG designed for the slanted AniMe template. With `time: Infinite` it becomes a static image; with any other `AnimTime` it is converted to an animation. |
| `ImageAnimation` | `file`, `scale`, `angle`, `translation`, `time`, `brightness` | An animated GIF. If the file extension is `.png` (case-insensitive), a static animation is built from the image using `time`. |
| `Image` | `file`, `scale`, `angle`, `translation`, `time`, `brightness` | A PNG. With `time: Infinite` it is displayed as a plain static image; with any other `AnimTime` it is converted to a static animation. |
| `Pause` | `Duration` | A pause between actions. |

Example:

```ron
(
    name: "my-anime",
    anime: [
        AsusAnimation(
            file: "/usr/share/asusd/anime/asus/rog/Sunset.gif",
            time: Fade((
                fade_in: (secs: 2, nanos: 0),
                show_for: None,
                fade_out: (secs: 2, nanos: 0),
            )),
            brightness: 0.5,
        ),
        AsusImage(
            file: "/usr/share/asusd/anime/custom/diagonal-template.png",
            time: Infinite,
            brightness: 1.0,
        ),
        ImageAnimation(
            file: "/usr/share/asusd/anime/custom/sonic-run.gif",
            scale: 0.9,
            angle: 0.65,
            translation: (0.0, 0.0),
            time: Count(2),
            brightness: 0.5,
        ),
        Image(
            file: "/usr/share/asusd/anime/custom/rust.png",
            scale: 1.0,
            angle: 0.0,
            translation: (0.0, 0.0),
            time: Time((secs: 5, nanos: 0)),
            brightness: 0.6,
        ),
        Pause((secs: 1, nanos: 0)),
    ],
)
```

`AsusAnimation` and `AsusImage` expect the ASUS diagonal layout. Use `ImageAnimation` and `Image` for ordinary GIFs and PNGs.

### GIF details

- A GIF frame lasts `frame.delay * 10` milliseconds.
- For an ordinary GIF (`ImageAnimation`), a frame marked with the `Background` disposal method starts from a blank canvas. The diagonal `AsusAnimation` path does not apply disposal.
- Pixels whose alpha is not `255` are treated as transparent and skipped.
- For an ASUS diagonal GIF (`AsusAnimation`), only the red channel is used, multiplied by `brightness`. Use a greyscale image so the red channel carries the intended brightness.
- For an ordinary GIF (`ImageAnimation`), the three colour channels are combined into one brightness value: each channel is divided by 3 and the results are added.

## Time and fade

`time` is an `AnimTime`:

| Variant | RON example | Meaning |
|---|---|---|
| `Time(Duration)` | `time: Time((secs: 5, nanos: 0)),` | Run for a fixed duration. |
| `Count(u32)` | `time: Count(2),` | Loop the animation twice. For a static PNG, one loop is about one second, so this is roughly the number of seconds. |
| `Infinite` | `time: Infinite,` | Loop animations forever. For `Image` and `AsusImage`, write a plain static image once, then advance to the next action. |
| `Fade(Fade)` | see below | Fade in, optionally hold, then fade out. |

`AnimTime::default()` is `Infinite`, but every action field still requires a `time` value; the default only applies when an `AnimTime` is defaulted in code.

A `Fade` value has three fields:

```ron
time: Fade((
    fade_in: (secs: 2, nanos: 0),
    show_for: Some((secs: 1, nanos: 0)),
    fade_out: (secs: 2, nanos: 0),
)),
```

| Field | Type | Description |
|---|---|---|
| `fade_in` | `Duration` | Time spent fading in. |
| `show_for` | `Option<Duration>` | Time to hold at full brightness. `None` means no extra hold. |
| `fade_out` | `Duration` | Time spent fading out. |

With `show_for: None`, the action runs for its natural duration: the GIF's frame time, or the combined fade-in and fade-out time for a static PNG converted to an animation. Without a `Fade`, a static PNG animation is one second long. For a static PNG converted to an animation, the total duration, including any hold, must be at least 30 ms. Shorter durations produce zero frames, and playback stalls without advancing. For `AsusImage`, use `time: Infinite` when you want a plain static image rather than a faded animation.

`Pause` takes a bare `Duration`:

```ron
Pause((secs: 1, nanos: 0)),
```

## Units and paths

| Field | Unit and range |
|---|---|
| `brightness` | `0.0` = off, `1.0` = full. Keep values in `0.0..=1.0`. |
| `scale` | Multiplier. `1.0` fits the image to the display. |
| `angle` | Radians. |
| `translation` | `(x, y)` offset in LED coordinates. |
| `file` | Path to the image or GIF. Use an absolute path. |

Relative paths are resolved against the daemon's working directory, not the directory containing the RON file, so prefer absolute paths.

## PNG images

PNG decoding accepts these raster formats:

- `Gray8`, `Graya8`
- `Rgb8`, `Rgba8`
- `Gray16`, `Graya16`
- `Rgb16`, `Rgba16`

Palette PNGs are rejected. Colour PNGs are accepted: each channel is divided by 3, the three results are added to form the single brightness value the display uses, and alpha is respected for `Image` and `ImageAnimation`. The `AsusImage` diagonal PNG path ignores alpha and uses only the colour channels. For 16-bit colour, each channel is divided by 3 and then shifted right by 8 before the results are added; 16-bit greyscale values are shifted right by 8. You do not need to convert images to greyscale first.

## See also

- [asusd configuration: AniMe Matrix](asusd.md#anime-matrix)
- [asusd-user configuration: AniMe sequences](asusd-user.md#anime-sequences)
- [asusctl anime](../usage/asusctl.md#anime)

## Source of truth

- [`rog-anime/src/sequencer.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-anime/src/sequencer.rs): `ActionLoader` variants and fields.
- [`rog-anime/src/gif.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-anime/src/gif.rs): `AnimTime`, `Fade`, and GIF/PNG animation construction.
- [`rog-anime/src/image.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-anime/src/image.rs): image transforms and PNG raster handling.
- [`rog-anime/src/data.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-anime/src/data.rs): `Animations` and playback timing.
- [`asusd/src/aura_anime/config.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/asusd/src/aura_anime/config.rs): system `anime.ron` fields.
- [`asusd-user/src/config.rs`](https://github.com/OpenGamingCollective/asusctl/blob/main/asusd-user/src/config.rs): user `ConfigAnime`.
