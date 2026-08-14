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

To avoid [problems with tuned](https://gitlab.com/asus-linux/asusctl/-/issues/724), use `power-profiles-daemon`:

```bash
sudo dnf install power-profiles-daemon --allowerasing
sudo systemctl enable --now power-profiles-daemon.service
```

### ROG Control Center

ROG Control Center is a GUI tool for configuring some aspects of `asusctl`. It is available as a separate package:

```bash
sudo dnf install asusctl-rog-gui
```

![ROG Control Center](../assets/guides/shared/rog-control-center.png)

![ROG Control Center fan curve](../assets/guides/shared/rog-control-center-fan-curve.png)

Reboot after installing `asusctl`:

```bash
sudo systemctl reboot
```

> [!NOTE]
> ASUS releases new products every year, so not every device is guaranteed to work with the current Fedora kernel. Depending on your device, you may need a kernel with newer ASUS patches, such as the ASUS Armoury driver available in Linux 6.19 and later, or the CachyOS kernel. This is optional and depends on your device and needs.

### Graphics Switching

It is now possible to manage your graphics card using `asusctl` or the ROG Control Center. You can check if your device supports graphics switching by running the following command:

```bash
asusctl armoury list
```

If your device supports disabling of the dGPU, you should see an entry that looks like the following:

```bash
dgpu_disable:
  current: [(0),1]
```

Here, a current value of 0 means that your dgpu is not disabled (i.e., enabled).

You can set whether you want to utilize your dGPU by modifying the setting under the `GPU Configuration` tab in the ROG Control Center. Alternatively, use the command `asusctl armoury set dgpu_disable 1` to disable the dgpu, and 0 to re-enable it.

> [!NOTE]
> Due to how Linux systems are configured to use the dGPU, you must reboot your system after changing your dGPU configuration. If you wish to power off your dgpu without rebooting, you should use an alternative program such as Cardwire (see below).

#### Cardwire

Cardwire is the community's new replacement for the now-deprecated supergfxctl.

> [!CAUTION]
> Cardwire is currently still considered EXPERIMENTAL. If you choose to install this tool, expect rough edges and quirks. For support, join our Discord server.

Cardwire is available for install on the Terra repository. You can install it with:

```bash
sudo dnf install cardwire
```

For installation and usage instructions, refer to the [documentation](https://opengamingcollective.github.io/cardwire/).

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
