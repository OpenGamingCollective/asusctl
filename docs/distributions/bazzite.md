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
ujust asus install
```

The services are enabled automatically.

### Graphics Switching

See [GPU Switching](../faq/gpu-switching.md) for how to manage the dGPU and MUX.
