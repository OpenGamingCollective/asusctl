# Ubuntu

> Only Ubuntu 26.04 LTS is supported; asusctl is distributed using [Homebrew](https://brew.sh/)

## Installation

1. Install Homebrew for Linux if you don't have it already:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

2. Add the ublue-os tap:

```bash
brew tap ublue-os/homebrew-tap
```

3. Install asusctl and the ROG Control Center:

```bash
brew install asusctl-linux
brew install rog-control-center-linux
```

The casks install the `asusd` systemd units, udev rules, and D-Bus configuration automatically. Verify the daemon is running:

```bash
sudo systemctl enable --now asusd.service asus-shutdown.service
systemctl --user daemon-reload
systemctl --user enable --now asusd-user.service
sudo udevadm control --reload
sudo udevadm trigger
systemctl status asusd.service
```

For everything else (platform profiles, fan curves, GPU switching), refer to the [Usage](../usage/asusctl.md) section.

## Power profiles

> [!IMPORTANT]
> **Power profiles**: `asusd` manages platform profiles and CPU EPP settings itself via the ACPI `platform_profile` interface. Running an external power profiles daemon (such as `power-profiles-daemon` or `tuned`) alongside `asusd` can cause race conditions over `/sys/firmware/acpi/platform_profile` and CPU EPP preferences. Ubuntu ships `power-profiles-daemon` by default, so you have two options:
>
> 1. **Let `asusd` manage profiles** and disable the external daemon:
>
> ```bash
> sudo systemctl disable --now power-profiles-daemon.service
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

## Graphics Switching

See [GPU Switching](../faq/gpu-switching.md) for how to manage the dGPU and MUX.
