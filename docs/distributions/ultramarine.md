# Ultramarine Setup Guide

> A friendly guide for setting up Ultramarine on ASUS laptops

Newcomers should start by reading the [Intro](../introduction.md) guide.

For general Ultramarine setup and usage information, see the [official Ultramarine guide](https://wiki.ultramarine-linux.org/en/setup/requirements/).

> [!WARNING]
> This guide is maintained by the community. If you find an issue with the Ultramarine documentation, please contribute a fix or report it to the community.

## Contents

- [About Ultramarine Versions](#about-ultramarine-versions)
- [Post-Installation](#post-installation)
- [Setup](#setup)
  - [Asusctl](#asusctl)
  - [ROG Control Center](#rog-control-center)
  - [Graphics Switching](#graphics-switching)
- [Optional Steps](#optional-steps)
  - [Enabling Secure Boot](#enabling-secure-boot)
    - [CachyOS Kernel](#cachyos-kernel)
      - [Install CachyOS Kernel](#install-cachyos-kernel)

## About Ultramarine Versions

This guide is written for the current stable release of Ultramarine. Ultramarine is based on Fedora and follows the same release pattern.

You need to keep Ultramarine up to date. If you are two versions behind, your operating system is no longer supported with updates or security fixes.

For example, if Ultramarine 44 is the current stable release, Ultramarine 42 is unsupported.

## Post-Installation

Follow the [Ultramarine post-installation guide](https://wiki.ultramarine-linux.org/en/setup/postinstall/) for useful steps such as installing NVIDIA drivers.

## Setup

### Asusctl

This section covers installing `asusctl` and its supporting software. It enables controls for ASUS ROG hardware on the laptop.

```bash
sudo dnf install asusctl
```

> [!IMPORTANT]
> **Power profiles**: `asusd` manages platform profiles and CPU EPP settings itself via the ACPI `platform_profile` interface. Running an external power profiles daemon (such as `power-profiles-daemon` or `tuned`) alongside `asusd` can cause race conditions over `/sys/firmware/acpi/platform_profile` and CPU EPP preferences. You have two options:
>
> 1. **Let `asusd` manage profiles** and disable the external daemon. Since Ultramarine is Fedora-based, `tuned` is the default power profile daemon. Note that KDE Plasma's PowerDevil can respawn `power-profiles-daemon` through DBus activation even after it is disabled, so be sure to mask it instead:
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

### ROG Control Center

ROG Control Center is a GUI tool for configuring some aspects of `asusctl`. It is available as a separate package:

```bash
sudo dnf install asusctl-rog-gui
```

![ROG Control Center](../assets/shared/rog-control-center.png)

![ROG Control Center fan curve](../assets/shared/rog-control-center-fan-curve.png)

Reboot after installing `asusctl`:

```bash
sudo systemctl reboot
```

> [!NOTE]
> ASUS releases new products every year, so not every device is guaranteed to work with the current Fedora kernel. Depending on your device, you may need a kernel with newer ASUS patches, such as the ASUS Armoury driver available in Linux 6.19 and later, or the CachyOS kernel. This is optional and depends on your device and needs.

### Graphics Switching

See [GPU Switching](../faq/gpu-switching.md) for how to manage the dGPU and MUX.

## Optional Steps

### Enabling Secure Boot

The recommended and easiest way to sign the kernel, whether you switched to systemd-boot, installed NVIDIA drivers, or changed the kernel, is to use [`sbctl`](https://wiki.ultramarine-linux.org/en/setup/postinstall/#secure-boot-with-systemd-boot).

#### CachyOS Kernel

> [!NOTE]
> Newer devices may require a custom kernel with additional patches. The CachyOS kernel includes newer patches and can be tried if the stock kernel does not support your device properly.

#### Install CachyOS Kernel

Ultramarine provides [`umcli`](https://wiki.ultramarine-linux.org/en/usage/umcli/) to simplify switching to the CachyOS kernel:

```bash
um tweaks enable cachyos-kernel
```

> [!NOTE]
> If Secure Boot is enabled, sign the new kernel with [`sbctl`](#enabling-secure-boot).
