# `asusctl` for ASUS ROG

[![Become a Patron!](https://github.com/codebard/patron-button-and-widgets-by-codebard/blob/master/images/become_a_patron_button.png?raw=true)](https://www.patreon.com/bePatron?u=7602281) [![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/V7V5CLU67) - [Asus Linux Website](https://asus-linux.org/)

**WARNING:** Many features are developed in tandem with kernel patches. If you see a feature is missing you either need a patched kernel or latest release.

`asusd` is a utility for Linux to control many aspects of various ASUS laptops
but can also be used with non-asus laptops with reduced features.

Now includes a GUI, `rog-control-center`.

# OGC Migration

This project has been migrated to the [OGC](https://github.com/opengamingcollective) on [GitHub](https://github.com/opengamingcollective/asusctl) and future development will happen there.

## Kernel support

Due to ongoing driver work, the minimum suggested kernel version is always **the latest*, as improvements and fixes are continuous.

Support for TDP is tied to the new asus-armoury driver: available mainline since Linux 6.19: everything older is not supported.

## X11 support

X11 is not supported at all, as in I will not help you with X11 issues if there are any due to limited time and it being unmaintained itself. You can however build `rog-control-center` with it enabled `cargo build --features "rog-control-center/x11"`.

## Goals

The main goal of this work is to provide a safe and easy to use abstraction over various laptop features via D-Bus, and to provide some helpful defaults and other behaviour such as toggling throttle/profile on AC/battery change.

1. Provide safe D-Bus interface
2. Respect the users resources: be small, light, and fast

Point 4: asusd currently uses a tiny fraction of cpu time, and less than 1MB of RAM, the way
a system-level daemon should. Languages such as JS and python should never be used for system level daemons (please stop).

## Keyboard LEDs

The level of support for laptops is dependent on folks submitting data to include in [`./rog-aura/data/layouts/aura_support.ron`](./rog-aura/data/layouts/aura_support.ron), typically installed in `/usr/share/asusd/aura_support.ron`. This is because the controller used for keyboards and LEDs is used across many years and many laptop models, all with different firmware configurations - the only way to track this is with the file mentioned above. Why not just enable all by default? Because it confuses people.

See the [rog-aura readme](./rog-aura/README.md) for more details.

## Discord

[![Discord](https://img.shields.io/badge/Discord-7289DA?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/B8GftRW2Hd)

## SUPPORTED LAPTOPS

Most ASUS gaming laptops that have a USB keyboard. If `lsusb` shows something similar
to this:

```plain
Bus 001 Device 002: ID 0b05:1866 ASUSTek Computer, Inc. N-KEY Device
```

or

```plain
Bus 003 Device 002: ID 0b05:19b6 ASUSTek Computer, Inc. [unknown]
```

then it may work without tweaks. Technically all other functions except the LED
and AniMe parts should work regardless of your laptop make.

## Implemented

The list is a bit outdated as many features have been enabled in the Linux kernel with upstream patches and then supported in asusctl suite.

- [x] System daemon
- [x] GUI app (includes tray and notifications)
- [x] Setting/modifying built-in LED modes
- [x] Per-key LED setting
- [x] Fancy LED modes (See examples) (currently being reworked)
- [x] AniMatrix display on G14 and M16 models that include it
- [x] Set battery charge limit (with kernel supporting this)
- [x] Fan curve control on supported laptops (G14/G15, some TUF like FA507)
- [x] Toggle bios setting for boot/POST sound
- [x] Toggle GPU MUX (g-sync, or called MUX on 2022+ laptops)

## GUI

A gui is now in the repo - ROG Control Center. At this time it is still a WIP, but it has almost all features in place already.

**NOTE**: X11 is not supported.

## BUILDING

Rust and cargo are required, they can be installed from [rustup.rs](https://rustup.rs/).

Distro packaging should work with the stable toolchain. If your distro does not provide a recent Rust toolchain, install rustup and use the stable toolchain.

**fedora:**

```sh
dnf install cmake clang-devel libxkbcommon-devel systemd-devel expat-devel pcre2-devel libzstd-devel gtk3-devel
make
sudo make install
```

**openSUSE:**

Works with KDE Plasma (without GTK packages)

```sh
zypper in -t pattern devel_basis
zypper in rustup make cmake clang-devel libxkbcommon-devel systemd-devel expat-devel pcre2-devel libzstd-devel gtk3-devel
make
sudo make install
```

**Debian (unsupported):**

officially unsupported,but you can still try and test it by yourself (some features may not be available).

```sh
sudo apt install libclang-dev libudev-dev libfontconfig-dev build-essential cmake libxkbcommon-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
make
sudo make install
```

**Ubuntu, Pop!_OS (unsupported):**

```sh
sudo apt install make cargo gcc pkg-config openssl libasound2-dev cmake build-essential python3 libfreetype6-dev libexpat1-dev libxcb-composite0-dev libssl-dev libx11-dev libfontconfig1-dev curl libclang-dev libudev-dev checkinstall libseat-dev libinput-dev libxkbcommon-dev libgbm-dev

make
sudo make install
```

## Installing

- Ultramarine/Nobara: `dnf install asusctl`. Enable the services: `systemctl enable --now asusd.service; systemctl enable --now  asus-shutdown.service`.
- Fedora = Install [Terra](https://terrapkg.com/), then `dnf install asusctl`. Enable the services: `systemctl enable --now asusd.service; systemctl enable --now  asus-shutdown.service`.
- openSUSE = https://download.opensuse.org/repositories/home:/luke_nukem:/asus/
- Arch = Via the AUR, install `asusctl`
- Nix/NixOS = `asusctl`
- Solus = `eopkg install asusctl`

Some other distros may have asusctl packaged, we recommend checking before building from source.

=======

The default init method is to use the udev rule, this ensures that the service is
started when the device is initialised and ready.

You may also need to activate the service for debian install. If running Pop!\_OS, I suggest disabling `system76-power` gnome-shell extension and systemd service.

## Upgrading

If you are upgrading from a previous installed version, you will need to restart the service or reboot.

```sh
systemctl daemon-reload && systemctl restart asusd
```

## Uninstalling

Run `sudo make uninstall` in the source repo, and remove `/etc/asusd/`.
If you have installed with a package manager, use your package managers uninstall function.

## Contributing

See `CONTRIBUTING.md`. Additionally, also do `cargo clean` and `cargo test` on first checkout to ensure the commit hooks are used (via `cargo-husky`).

Generation of the bindings with `make bindings` requires `typeshare` to be installed.

D-Bus introspection XML requires with `make introspection` requires `anime_sim` to be running before starting `asusd`.

## OTHER

### AniMe Matrix simulator

A simulator using SDL2 can be built using `cargo build --package rog_simulators` and run with `./target/debug/anime_sim`. Once started `asusd` will need restarting to pick it up. If running this sim on a laptop _with_ the display, the simulated display will be used instead of the physical display.

### Supporting more laptops

Please file a support request.

## License & Trademarks

Mozilla Public License 2 (MPL-2.0)

---

ASUS and ROG Trademark is either a US registered trademark or trademark of ASUSTeK Computer Inc. in the United States and/or other countries.

Reference to any ASUS products, services, processes, or other information and/or use of ASUS Trademarks does not constitute or imply endorsement, sponsorship, or recommendation thereof by ASUS.

The use of ROG and ASUS trademarks within this website and associated tools and libraries is only to provide a recognisable identifier to users to enable them to associate that these tools will work with ASUS ROG laptops.

---

## AI Disclaimer

Portions of this code have been written by various AI tools and reviewed by the maintainer exactly as with every other contribution.
