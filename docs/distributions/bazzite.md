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

### Graphics Switching

See [GPU Switching](../faq/gpu-switching.md) for how to manage the dGPU and MUX.
