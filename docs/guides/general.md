# General Distribution Install

> General steps to install asusctl on most distributions

Distros that we have full guide and official package supported:

- Fedora Workstation
- Arch Linux
- Ultramarine
- OpenSUSE

You can find all the guides in our `Guides` page.

Distros that very popular but we don't have official supported:

- Debian and Debian based (such as Ubuntu/PopOS)
- Manjaro
- CentOS/RockyOS or any similar

> [!NOTE]
> Ubuntu-based Distribution support is coming soon.

But why?

1. Old kernel: many patches that drastically improve Linux experience on an ASUS/ROG laptop are only available in the latest kernel. The minimum kernel version we recommend now is >= 6.19 (newer is better), which is why you should never run CentOS/RockyOS on newer devices, especially a laptop.
2. Too many custom changes: such as PopOS and Manjaro, all the custom kernel/package stuff will very likely conflict with asusctl and will not be functional.

However, if you REALLY REALLY need that very specific distro to get your job done, we strongly recommend using [DistroBox](https://github.com/89luca89/distrobox) to provide the environment that the software needs. You can find many youtube videos show you how to use it (Don't install asusctl on distrobox, you need root access and access to some services on the host like ppd).

On non-supported distros, asusctl must be built from source. Instructions can be found on the [asusctl repository](https://gitlab.com/asus-linux/asusctl).

Before starting your adventure, make sure your distro is:

- systemd based (manual configuration will be required on other init systems)
- utilizes the Linux Kernel, not BSD or so
- updated, utilizing Kernel version >= 6.19
- installed with GPU drivers
- remove any distro provided methods of graphics switching (like supergfxd, envycontrol)
- reboot after removing the conflicting graphics-switching tools

For dGPU control, look into [Cardwire](https://opengamingcollective.github.io/cardwire/).
