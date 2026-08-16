# Supported Distributions

> Which distributions officially support asusctl, and what to expect elsewhere

Here are the distributions with a written guide:

- [Arch Linux](../distributions/arch.md)
- [Bazzite](../distributions/bazzite.md)
- [Fedora Workstation](../distributions/fedora.md)
- [Fedora Atomic](../distributions/fedora-atomic.md)
- [NixOS](../distributions/nixos.md)
- [openSUSE Tumbleweed](../distributions/opensuse.md)
- [PikaOS](../distributions/pikaos.md)
- [Ubuntu 26.04](../distributions/ubuntu.md)
- [Ultramarine](../distributions/ultramarine.md)

Other distributions that package asusctl:

- AerynOS
- Solus

Distributions that are very popular but are not supported:

- Debian and Debian based (such as Pop!\_OS, Linux Mint)
- CentOS/RockyOS or any similar

**But why?**

Old kernel: many patches that drastically improve Linux experience on an ASUS/ROG laptop are only available in the latest kernel. The minimum kernel version required is >= 6.19 (newer is better), which is why CentOS/RockyOS/Debian are listed as unsupported. Their default kernels often fall below this, especially on newer devices.

However, if you REALLY REALLY need that very specific distro to get your job done, we strongly recommend using [DistroBox](https://github.com/89luca89/distrobox) to provide the environment that the software needs. You can find many youtube videos show you how to use it (Don't install asusctl on distrobox, asusd needs root access).

On non-supported distros, asusctl must be built from source. Instructions can be found on the [asusctl repository](https://github.com/OpenGamingCollective/asusctl).

Before starting your adventure, make sure your distro is:

- systemd based (manual configuration will be required on other init systems)
- utilizes the Linux Kernel, not BSD or so
- updated, utilizing Kernel version >= 6.19
- installed with GPU drivers
- remove any distro provided methods of graphics switching (like supergfxd, envycontrol)
- reboot
