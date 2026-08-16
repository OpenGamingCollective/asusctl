# Introduction

> A friendly guide for using Linux on ASUS laptops

Welcome to the Linux on ASUS documentation! This book is written by the ASUS-Linux team.

## What is asusctl?

`asusctl` is a system control utility designed primarily for ASUS laptops. It consists of three core components:

- **`asusd`** - the system daemon that talks to your hardware, controlled through D-Bus.
- **`asusctl`** - the command-line client for `asusd`.
- **ROG Control Center** - a graphical user interface for `asusd`.

It provides safe access to platform profiles, custom fan curves, battery charge thresholds, keyboard lighting (Aura), AniMe Matrix and Slash displays, GPU MUX switching, and more, on hardware where the Linux kernel supports it.

## Kernel requirement

> [!WARNING]
> Many features are developed alongside Linux kernel updates. The minimum supported kernel version is **Linux 6.19 or newer** and we always recommend using the latest kernel available. Support for Thermal Design Power (TDP) and other features is tied to the `asus-armoury` driver, which is only available mainline since 6.19.

## Where to start

- **New to Linux on an ASUS laptop?** Start with [Prerequisites and BIOS Preparation](getting-started/prerequisites.md), things to do in Windows and the UEFI before installing.
- **Picking a distribution?** See [Supported Distributions](getting-started/supported-distributions.md) and the [Distribution Guides](distributions/index.md).
- **Just installed?** Read the [General Recommendations](getting-started/recommendations.md), then explore the [Usage](usage/asusctl.md) section.
- **Something not working?** Check the [FAQ](faq/general.md).
