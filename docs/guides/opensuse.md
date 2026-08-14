# openSUSE Tumbleweed Setup Guide

> A friendly setup guide for openSUSE on ASUS laptops

Newcomers should start by reading the [Intro](../introduction.md) guide.

> [!WARNING]
> This page will not be updated directly by the core team; this guide has been updated by the community on Discord. If there are any issues with the openSUSE documentation, feel free to contribute.

## Contents

- [About openSUSE Tumbleweed](#about-opensuse-tumbleweed)
  - [Considerations](#considerations)
- [Preparations](#preparations)
- [Installation](#installation)
  - [Installation Media](#installation-media)
  - [Installing openSUSE Tumbleweed](#installing-opensuse-tumbleweed)
- [Setup](#setup)
  - [Notes on Btrfs Snapshots](#notes-on-btrfs-snapshots)
  - [Remove the Installer ISO Repository](#remove-the-installer-iso-repository)
  - [Update the System](#update-the-system)
  - [Adding the wheel group](#adding-the-wheel-group)
  - [Third Party Repositories](#third-party-repositories)
  - [Install Nvidia Graphics Drivers](#install-nvidia-graphics-drivers)
  - [Update System Boot Configuration for Nvidia](#update-system-boot-configuration-for-nvidia)
- [Asus-Linux Software](#asus-linux-software)
  - [Adding the Repository Copr Repo](#adding-the-repository-copr-repo)
  - [Asusctl](#asusctl)
  - [ROG Control Center](#rog-control-center)
  - [Graphics Switching](#graphics-switching)
- [Optional Steps](#optional-steps)
  - [Enabling ZRAM](#enabling-zram)
  - [Open Build Service](#open-build-service)
  - [Packman](#packman)
  - [Flatpaks](#flatpaks)

### About openSUSE Tumbleweed

[openSUSE Tumbleweed](https://get.opensuse.org/tumbleweed/) is a rolling release distribution and should be chosen over openSUSE Leap. Tumbleweed gives users access to the latest software packages and updates, providing better compatibility with ASUS laptops than Leap. However, users with newer laptop models may run into issues. Please see the [Considerations](#considerations) section for more details.

All updates to the official repos are incorporated into snapshots and tested with [openQA](https://openqa.opensuse.org/) prior to release. This means that Tumbleweed is slightly behind a bleeding-edge distro, such as Arch, but that all updates to the official repos are thoroughly tested.

KDE Plasma is the flagship openSUSE Tumbleweed Desktop Environment. The installer also supports GNOME, Xfce, and many others (including Window Managers).

#### Considerations

Users coming from other distributions may run into issues when using Tumbleweed due to its default settings. This guide attempts to point out some of these within the relevant sections. A few general items that a user should be aware of before trying openSUSE are:

1. Custom kernels are difficult with openSUSE. This means the **rog/asus-kernel is not supported**. Newer laptops will need to wait until any required patches are merged from the rog/asus-kernel into the upstream kernel.
2. The package manager is `zypper`, which is RPM-based and relatively similar to Fedora's `dnf`. The Arch wiki provides a [helpful table](https://wiki.archlinux.org/title/Pacman/Rosetta) comparing common package manager commands.
3. `zypper` should be used to install packages instead of YaST, KDE Discover, or GNOME Software. These GUI tools may still be useful for flatpaks and searching, but do not use them to install other packages.
4. **Occasional mirror issues, while rare, do exist. Workarounds are required to continue updating the system and installing packages.**
5. Package updates in Tumbleweed are performed through `zypper dup` (dist-upgrade), not `zypper up` (update). For more information, see the [openSUSE wiki](https://en.opensuse.org/openSUSE:Desktop_FAQ#Package_Management).
6. OpenSUSE is not as widely used as Fedora or Arch, so some software may not be available. Flatpaks, the [Open Build Service](https://build.opensuse.org/), and third-party repositories help alleviate this issue. **Note that only official packages are tested as part of the Tumbleweed openQA testing.**
7. OpenSUSE uses the concept of **patterns**, which include base and recommended software packages. These patterns may result in packages being re-installed, even if you specifically removed them. More information is available on the [openSUSE user documentation project](https://opensuse.github.io/openSUSE-docs-revamped-temp/safety_usability/#the-lazy-way).
8. OpenSUSE community websites may provide "one-click installers", which can cause more trouble than they're worth. Avoid using these.
9. OpenSUSE provides excellent support for the btrfs file system, and supports automatic bootable btrfs snapshots out of the box.
10. OpenSUSE provides the YaST GUI tool for system management. YaST is polarizing, but may be helpful for new users to get a better idea of the system configuration. Many operations should still be performed via the terminal instead.
11. The Asus-Linux core team currently has no members with experience in openSUSE or Fedora, so if the COPR repository stops generating new packages (or its signing keys expire, as they currently have), the best thing you can do is report the issue; or, if you can fix it, it would be best if you became a package maintainer.

### Preparations

Prior to installation of openSUSE Tumbleweed, follow the [preparations in the Introduction guide](../introduction.md). The list below gives a summary of the important subsections to review.

1. [Backup Proprietary eSupport Drivers Folder](../introduction.md#backup-proprietary-esupport-drivers-folder) - Backing up the proprietary ASUS drivers.
2. [Disable Secure Boot](../introduction.md#disable-secure-boot) - Disabling secure boot for compatibility with the proprietary Nvidia drivers.
3. [Use the Laptop Screen](../introduction.md#use-the-laptop-screen) - Background on installation problems that may arise due to external screens.

### Installation

#### Installation Media

The [openSUSE Tumbleweed website](https://get.opensuse.org/tumbleweed/) contains all of the installation images. The offline image is much larger than the network image, but allows installing all packages included without an internet connection. All of the packages will be dated to when the offline image was created. The net installer will pull the latest packages from the openSUSE repositories. It is possible to do this in the offline installer by connecting to the internet and enabling the internet repositories.

Live images also exist on the [openSUSE Tumbleweed website](https://get.opensuse.org/tumbleweed/). These are located under the **Alternative Downloads** link on the page. These Live images are similar to what users are used to with other distributions. Live images should be safe for most users. However, the [openSUSE Tumbleweed website](https://get.opensuse.org/tumbleweed/) and [openSUSE User Documentation Project](https://opensuse.github.io/openSUSE-docs-revamped-temp/image_choice/) warn that issues may occur when installing with a Live image. Please review these warnings on the associated links prior to continuing.

#### Installing openSUSE Tumbleweed

1. Download the chosen ISO file from the [openSUSE Tumbleweed website](https://get.opensuse.org/tumbleweed/) and write it to a USB stick. The network and Live ISOs requires a fast internet connection due to the number of downloads.

> [!NOTE]
> Ventoy has known issues and is not considered supported by openSUSE. It can be used, but manual intervention is required after installation.

2. The online repositories can be enabled with the offline ISO to pull the latest packages. The network ISO does this automatically.

3. Follow the steps of the installer, choosing a partitioning scheme and Desktop Environment.

4. The final Summary page allows disabling **Secure Boot**, if it will not be used, and allows customizing the Software that will be installed.

5. The system will automatically reboot after installation.

6. At the grub menu after reboot, press the **E** key to modify the boot parameters. Nvidia users should add `modprobe.blacklist=nouveau` to the Kernel command line arguments to ensure the Nouveau drivers do not cause issues.

> [!NOTE]
> If Ventoy was used, the Kernel command line arguments may contain `rdinit=/vtoy/vtoy`. This **MUST** be removed as it will prevent the system from booting. Include this change in the step below.

7. Once the system boots, log in and make any necessary changes to the `GRUB_CMDLINE_LINUX_DEFAULT` parameter in `/etc/default/grub`.

8. Regenerate grub to prevent repeating the previous steps on every boot.

```bash
sudo grub2-mkconfig -o /boot/grub2/grub.cfg
```

### Setup

#### Notes on Btrfs Snapshots

It is helpful to take snapshots throughout the setup process if using the btrfs filesystem. Pre and post snapshots are automatically taken when installing and updating packages. The "openSUSE-style" snapshots allow booting into the last-known working state through the grub menu. The system will boot into a read-only filesystem based on the chosen snapshot, which the user can explore to ensure the system is working. The system can be restored to the snapshot using the command `snapper rollback`. More information is available on the [openSUSE user documentation project](https://opensuse.github.io/openSUSE-docs-revamped-temp/snapper/#rolling-back).

#### Remove the Installer ISO Repository

The openSUSE installer automatically adds a repo entry for the installation media. This feature is more geared toward offline or enterprise systems, not personal devices. Removing the repository will ensure that updates will not fail if the installation media is not found. Command line is the preferred method of removal, but the YaST GUI can be used.

1. List the currently active repositories:

```bash
sudo zypper lr -d
```

2. From the outputs, identify the repo number that corresponds to the installation media. The offline image will likely have **DVD** in the title. The network image should include a date in the title.

3. Disable the repo, replacing `#` with the number identified from the previous steps:

```bash
sudo zypper mr -d -F #
```

4. Remove the repo, replacing `#` with the number identified from the previous steps:

```bash
sudo zypper rr #
```

5. Force a refresh of the repositories:

```bash
sudo zypper ref -f
```

#### Update the System

1. Refresh the repositories:

```bash
sudo zypper ref
```

2. Update the system via dist-upgrade (dup):

```bash
sudo zypper dup
```

#### Adding the wheel group

By default, openSUSE does not automatically add the user to the **wheel** group. This prevents **asusctl** and **ROG Control Center** accessing DBus when run with user privileges. The resulting error messages are similar to `Error: org.freedesktop.DBus.Error.AccessDenied`. Additionally, the default setup requires the **root** password when a user runs `sudo`. This may cause confusion if the user is coming from other distributions. The [openSUSE wiki](https://en.opensuse.org/SDB:Administer_with_sudo) has a guide for setting up sudo behavior in a way that users may be more familiar with.

1. Open the terminal and switch to the root user:

```bash
su
```

2. The wheel group should be present by default, but can be verified with:

```bash
grep ^wheel /etc/group
```

3. If the wheel group is somehow not present, add it:

```bash
groupadd wheel
```

4. Add a user to the wheel group, replacing USERNAME with the desired user:

```bash
usermod -aG wheel USERNAME
```

5. *Optionally*, verify the user was added to the wheel group, replacing USERNAME with the desired user:

```bash
id USERNAME
```

6. Log out or restart for the changes to apply.

#### Third Party Repositories

The official openSUSE repositories have many restrictions due to strict copyright and IP laws. Third party repositories contain additional packages that are not or cannot be included in the openSUSE OSS and non-OSS repos. Using dependencies from multiple repositories may result in instability and issues. To avoid this, it is recommended to change the vendor for packages to the third party repositories being used. The [openSUSE wiki](https://en.opensuse.org/Additional_package_repositories) has a guide for setting up commonly used third party repos. Note that only the official repos are included in the openQA testing, so it will not catch any issues related to the additional packages.

All repos use priorities (higher number = lower priority). The official repos are added with the priority of 99 (default). Adding third party repos with a lower number results in packages being installed from them, if available.

#### Install Nvidia Graphics Drivers

A guide to installing the proprietary Nvidia drivers can be found on the [openSUSE wiki](https://en.opensuse.org/SDB:NVIDIA_drivers). Verify the numbering scheme for the Nvidia card using the instructions on the wiki. The instructions below show how to install the current G07 series, for newer devices. The older G06 series is now considered legacy.

1. Open the file `/etc/zypp/zypp.conf`.

2. Ensure the following line is not commented out:

```conf
multiversion = provides:multiversion(kernel)
```

3. The Nvidia repository is usually added automatically during installation (package `openSUSE-repos-Tumbleweed-NVIDIA`). If it is missing, add it with auto-refresh and a non-default priority (e.g., 90):

```bash
sudo zypper ar --priority 90 --refresh https://download.nvidia.com/opensuse/tumbleweed NVIDIA
```

4. For GPUs supported by the open source module, install the `nvidia-open-driver-G07-signed-kmp-meta` package. Otherwise, install the proprietary driver with `nvidia-driver-G07-kmp-meta`, which should result in all needed packages being installed:

```bash
sudo zypper in nvidia-open-driver-G07-signed-kmp-meta
```

5. Search for **G07** to verify the additional packages (e.g., `nvidia-glG07`) were installed:

```bash
zypper se G07
```

6. If matching **G07** Nvidia packages do not have an `i` or `i+` next to them, install them.

7. Note that the driver installation process may take 10+ minutes.

8. After installation, one can verify the driver version:

```bash
sudo modinfo -F version nvidia
```

9. Reboot the system.

#### Update System Boot Configuration for Nvidia

1. The Nvidia driver installation should handle adding the required Kernel command line arguments.

2. If issues are encountered, try adding the following to **GRUB_CMDLINE_LINUX** in `/etc/default/grub`:

```conf
modprobe.blacklist=nouveau rd.driver.blacklist=nouveau nvidia-drm.modeset=1
```

3. After the modifications, regenerate grub with:

```bash
sudo grub2-mkconfig -o /boot/grub2/grub.cfg
```

### Asus-Linux Software

#### Adding the Repository Copr Repo

As mentioned in consideration 11, the only community repository at this time is Copr.

> [!WARNING]
> The COPR repository is currently broken: its GPG signing keys have expired, so package installation fails with signature verification errors. The instructions below will not work until the keys are renewed. If installation fails, build asusctl from source following the instructions in the [asusctl README](https://github.com/OpenGamingCollective/asusctl).

```bash
# add the repository and install asusctl
sudo curl -Lo /etc/zypp/repos.d/lukenukem-asus-linux.repo \
  https://copr.fedorainfracloud.org/coprs/lukenukem/asus-linux/repo/opensuse-tumbleweed/lukenukem-asus-linux-opensuse-tumbleweed.repo

# refresh the repositories and install asusctl
sudo zypper refresh
sudo zypper install --allow-vendor-change asusctl
```

#### Asusctl

1. The **tlp** software is known to conflict with **power-profiles-daemon** and should be removed. It is installed with the default openSUSE installer settings.

```bash
sudo zypper rm tlp
```

2. Next, install **asusctl**:

```bash
sudo zypper in asusctl
```

3. Ensure that **power-profiles-daemon** is enabled and running:

```bash
sudo systemctl enable --now power-profiles-daemon.service
```

4. Either restart the system, or run the following command to immediately begin using asusctl:

```bash
sudo systemctl start asusd
```

5. Note that due to the difficulty of custom kernel support on openSUSE Tumbleweed, features for the newest laptop models may not always work.

#### ROG Control Center

The optional ROG Control Center GUI tool can be installed to assist with control of **asusctl** settings.

```bash
sudo zypper in asusctl-rog-gui
```

![ROG Control Center](../assets/guides/shared/rog-control-center.png)

![ROG Control Center fan curve](../assets/guides/shared/rog-control-center-fan-curve.png)

#### Graphics Switching

It is now possible to manage your graphics card using the ASUS GPU with `asusctl` or the ROG Control Center. You can check if your device supports graphics switching by running the following command:

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

##### Cardwire

Cardwire is the community's new replacement for the now-deprecated supergfxctl.

> [!CAUTION]
> Cardwire is currently still considered EXPERIMENTAL. If you choose to install this tool, expect rough edges and quirks. For support, join our Discord server.

Cardwire is not yet packaged for openSUSE, so it must be built from source; see its [documentation](https://opengamingcollective.github.io/cardwire/).

If you still require VFIO for virtual machines, `supergfxctl` remains available from the official Tumbleweed repositories:

```bash
sudo zypper in supergfxctl
sudo systemctl enable --now supergfxd
```

### Optional Steps

#### Enabling ZRAM

Unlike Fedora, zram is not enabled by default. The `zram-generator` package can be installed to set up and use zram instead of traditional swap. The Arch wiki's [guide on zram](https://wiki.archlinux.org/title/Zram) can be used as a reference. The steps below set up zram using the Fedora 37 default configuration and one additional option:

1. Ensure zswap is disabled to avoid conflicts with zram. The process can be found on the [Arch wiki](https://wiki.archlinux.org/title/zswap#Toggling_zswap).

2. Install **zram-generator** to handle zram creation:

```bash
sudo zypper in zram-generator
```

3. Create a zram configuration file at one of **zram-generator**'s expected locations, such as `/usr/lib/systemd/zram-generator.conf`.

4. Edit the newly created configuration file.

5. **Fedora 37 Default:** add the following to the configuration file:

```conf
[zram0]
zram-size = min(ram, 8192)
```

6. **Optionally**, the `compression-algorithm` parameter can be added to specify the compression algorithm used by zram. If it is not set, the kernel's default algorithm is used.

```conf
compression-algorithm = zstd
```

7. Reboot the system.

8. Verify zram was added by using the `zramctl` or `swapon` commands. A `zram0` item should be present.

#### Open Build Service

The [Open Build Service](https://build.opensuse.org/) (OBS) is a service that supports community-maintained packages, such as asusctl. The [openSUSE wiki](https://en.opensuse.org/Additional_package_repositories) has instructions on how to add some popular repos from the build service, such as the latest Wine (based on Wine-HQ), games, and more.

With the exception of **home:luke_nukem**, a common [recommendation](https://opensuse.github.io/openSUSE-docs-revamped-temp/safety_usability/) is to avoid home repos in the OBS. These repos can cause issues unless you know or trust the maintainer.

The [OBS Package Installer](https://github.com/openSUSE/opi) (opi) is a helpful tool that allows users to search the OBS and other vendors (e.g., Packman) for specific packages. It can be used to identify what packages are available on the OBS or elsewhere, along with getting links to review them. Opi prompts the user based on matching package names and build locations it finds (there are usually multiple). The chosen repo will be automatically added by opi when installing packages.

#### Packman

[Packman](http://packman.links2linux.org/) is one of the most commonly used third party repositories for openSUSE. It contains a large set of additional packages like codecs, Discord, OBS Studio, and Steam. Follow the guide on the [openSUSE wiki](https://en.opensuse.org/Additional_package_repositories) to add the Packman repo. Be sure to follow the `zypper dup` step to ensure that any existing packages (from the official repos) are updated to pull from Packman. This will help prevent dependency issues. Each future `zypper dup` will then pull updates for the packages from Packman instead.

The opi tool can be used to [install multimedia codecs](https://opensuse.github.io/openSUSE-docs-revamped-temp/codecs/#installing-the-open-build-service-package-installer-opi) directly from Packman. This is an alternative to running the zypper commands manually. Adding the Packman repo prior to `opi codecs` may result in the Packman repo being added twice. This occurs if the alias used is not what opi expects (e.g., "packman" versus "Packman"). If this happens, one repo can be deleted through the method to remove the installation ISO repository (earlier in this guide).

#### Flatpaks

Flatpaks also solve some of the issues with the lack of software available in the openSUSE repositories. They also provide the benefits of containerized applications. Flatpak may be automatically installed with the default installation settings, but the Flathub repo may need to be added. More information is available on the [openSUSE user documentation project](https://opensuse.github.io/openSUSE-docs-revamped-temp/alternative_procurement/#flatpaks).
