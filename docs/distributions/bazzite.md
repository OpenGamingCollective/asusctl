# Bazzite Setup Guide

> A friendly guide for setting up Bazzite on ASUS laptops

Newcomers should start by reading the [Intro](../introduction.md) guide.

Bazzite is a gaming-oriented atomic Fedora image based on [Universal Blue](https://universal-blue.org/). Like other atomic Fedora images it uses rpm-ostree, and `asusctl` is not preinstalled. Bazzite ships Homebrew preconfigured, which is the supported way to install asusctl.

## Contents

- [Installing Asusctl](#installing-asusctl)
- [Graphics Switching](#graphics-switching)

### Installing Asusctl

The supported way on Bazzite is the `ujust asus` helper. It installs the Universal Blue Homebrew casks `asusctl-linux` and `rog-control-center-linux` from the `ublue-os/tap` tap, and enables the required services. No reboot is needed and the installation survives rebasing:

```bash
# <https://github.com/ublue-os/bazzite/blob/b798170d195f8466f687b61be1831c1ec785d942/system_files/desktop/shared/usr/share/ublue-os/just/82-bazzite-apps.just#L265-L405>
ujust asus
```

The services are enabled automatically.

### Power profiles

> [!IMPORTANT]
> **Power profiles**: `asusd` manages platform profiles and CPU EPP settings itself via the ACPI `platform_profile` interface. Running an external power profiles daemon (such as `power-profiles-daemon` or `tuned`) alongside `asusd` can cause race conditions over `/sys/firmware/acpi/platform_profile` and CPU EPP preferences. You have two options:
>
> 1. **Let `asusd` manage profiles** and disable the external daemon. Note that KDE Plasma's PowerDevil can respawn `power-profiles-daemon` through DBus activation even after it is disabled, so be sure to mask it instead:
>
> ```bash
> sudo systemctl mask --now power-profiles-daemon.service
> # or, if you use tuned:
> sudo systemctl mask --now tuned.service tuned-ppd.service
> ```
>
> 2. **Keep the external daemon** and disable `asusd`'s profile management by setting the following to `false` in `/etc/asusd/asusd.ron`:
>
> ```ron
> change_platform_profile_on_ac: false,
> change_platform_profile_on_battery: false,
> platform_profile_linked_epp: false,
> ```
>
> A common way to switch profiles is binding the `Fn+F5` key to `asusctl profile next`
> Available profiles vary by system, see `asusctl profile list`.

### Graphics Switching

See [GPU Switching](../faq/gpu-switching.md) for how to manage the dGPU and MUX.
