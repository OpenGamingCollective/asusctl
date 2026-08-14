# Fedora Silverblue Setup Guide

> A Quickstart Guide to Fedora Silverblue and Asus-Linux

## Contents

- [Installation](#installation)
  - [Enabling the Terra Repository](#enabling-the-terra-repository)
  - [Asusctl](#asusctl)
  - [ROG Control Center](#rog-control-center)
  - [After rebooting](#after-rebooting)
- [Optional Steps](#optional-steps)
  - [Installing RPM Fusion](#installing-rpm-fusion)
  - [Hardware Accelerated codecs](#hardware-accelerated-codecs)
  - [Flatpak Cleaning](#flatpak-cleaning)
  - [Replace Firefox RPM with Flathub Flatpak and Force Wayland](#replace-firefox-rpm-with-flathub-flatpak-and-force-wayland)
  - [Nvidia](#nvidia)
  - [Recommended approach](#recommended-approach)
  - [Manual approach](#manual-approach)

### Installation

> [!NOTE]
> Official Fedora packages are maintained by Fyra Labs, the creators of Ultramarine Linux, in Terra repository: they are part of OGC as asus-linux is.

Read the [Intro guide](../introduction.md) first to avoid bad surprises.

#### Enabling the Terra Repository

ASUS Linux packages and tools are currently packaged on the Terra Repository for Fedora. Add the Terra repo with the following commands:

```bash
curl -fsSL https://github.com/terrapkg/subatomic-repos/raw/main/terra.repo | pkexec tee /etc/yum.repos.d/terra.repo
rpm-ostree install --apply-live terra-release
```

#### Asusctl

This section covers installing asusctl and its supporting software. This enables controls for the Asus ROG hardware on the laptop.

```bash
rpm-ostree install asusctl
```

To avoid [problems with tuned](https://gitlab.com/asus-linux/asusctl/-/issues/724), you should use ppd.

```bash
rpm-ostree override remove tuned-ppd --install power-profiles-daemon
```

> [!WARNING]
> This ppd section is outdated

#### ROG Control Center

ROG Control Center is a GUI tool for configuring few aspects of asusctl. After adding the Terra repository as described above, you can now install the tool:

```bash
rpm-ostree install asusctl-rog-gui
```

now reboot your system to apply the changes

#### After rebooting

Once the asusd service has been restarted, it should be enabled by default. If this is not the case, you can enable it as follows:

```bash
systemctl enable --now asusd.services
```

> [!NOTE]
> ASUS releases new products every year, so it is not possible to guarantee that everything will work on the current Fedora vanilla kernel. For this reason, depending on your device, you may require a kernel that has the latest patches such as ASUS Armoury (this driver is available in Linux 6.19 and later versions) or similar, for example, CachyOS Kernel (at least until the OGC kernel is ready). However, depending on your case and your needs, this may be optional. Read Custom Kernel.

### Optional Steps

#### Installing RPM Fusion

Usually, when you enable third-party repositories, RPM-Fusion is enabled automatically. However, if it is not, you can follow this [guide](https://rpmfusion.org/Configuration).

```bash
sudo dnf install https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm
```

#### Hardware Accelerated codecs

The Atomic versions of Fedora do not include codecs in the system image, because these distros are focused on modifying as little as possible to ensure stability. It is recommended to use Flatpak, Distrobox, Toolbox, or any other type of container, as these do not share codecs with the system and prevent you from having to install them. However, if you want to install them, you can follow this guide but remember install RPM-Fusion repos first.

> [!TIP]
> Universal Blue and its images already include the codecs in the system, so it is not necessary to perform these procedures.

#### Flatpak Cleaning

In order to streamline our dependency on flatpak it is worthwhile to have everything working with the same fundamentals.

```bash
flatpak remote-delete fedora
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
```

The steps above will uninstall the packages from the Fedora remote, so the following command will install the Flathub versions instead (Apply only on Silverblue version).

```bash
flatpak install org.gnome.Calculator org.gnome.Calendar org.gnome.Characters org.gnome.Connections org.gnome.Contacts org.gnome.Papers org.gnome.Logs org.gnome.Loupe org.gnome.Maps org.gnome.NautilusPreviewer org.gnome.Snapshot org.gnome.Weather org.gnome.baobab org.gnome.clocks org.gnome.font-viewer org.gnome.Showtime org.gnome.TextEditor org.gnome.Decibels
```

##### Replace Firefox RPM with Flathub Flatpak and Force Wayland

```bash
rpm-ostree override remove firefox firefox-langpacks
flatpak install flathub org.mozilla.firefox org.freedesktop.Platform.ffmpeg-full
```

#### Nvidia

Nvidia in Atomic versions requires key enrollment. However, you may need to repeat this process after an update, although you can use [this](https://github.com/CheariX/silverblue-akmods-keys), so below you will find two options.

##### Recommended approach

The recommended approach is to use or rebase [Bazzite](https://bazzite.gg/), [Bluefin](https://projectbluefin.io/), [Aurora](https://getaurora.dev/en), or a vanilla image of [Universal Blue](https://github.com/orgs/ublue-os/packages?tab=packages&q=silverblue-nvidia) with the Nvidia driver already configured.

If you want rebase follow this [guide](https://docs.getaurora.dev/guides/alternate-install-guide).

##### Manual approach

If you are unable or unwilling to use the method described above, you can follow these steps:

Add RPM-Fusion repos:

```bash
rpm-ostree install --apply-live https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm
```

> [!NOTE]
> If your laptop has an NVIDIA card older than Turing, the repo to install is [Negativo17](https://negativo17.org/nvidia-driver-580-lts-repository/).

Install Nvidia drivers:

```bash
rpm-ostree install akmod-nvidia xorg-x11-drv-nvidia xorg-x11-drv-nvidia-cuda
rpm-ostree kargs --append=rd.driver.blacklist=nouveau,nova_core --append=modprobe.blacklist=nouveau,nova_core
```
