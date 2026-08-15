# Asusctl On NixOS

> A simple guide for getting asusctl running on NixOS

## Contents

- [Contents](#contents)
- [Disclaimer](#disclaimer)
- [Requirement](#requirement)
- [Installation](#installation)
- [Graphics Switching](#graphics-switching)

## Disclaimer

This guide expects some previous knowledge about NixOS and its configuration system.

Please note that NixOS is not officially supported by this project, and any issues specific to it shall be reported on the nixpkgs [GitHub page](https://github.com/NixOS/nixpkgs/issues).

## Requirement

Linux 6.19 or newer is recommended. To install the latest Linux, put this in your configuration file:

```nix
boot.kernelPackages = pkgs.linuxPackages_latest;
```

## Installation

ROG Control Center is included in the asusctl module, so you only have to add to the configuration file:

```nix
services.asusd.enable = true;
```

Then rebuild your NixOS

## Graphics Switching

See [GPU Switching](../faq/gpu-switching.md) for how to manage the dGPU and MUX.

Cardwire is also packaged in nixpkgs. Enable it with:

```nix
services.cardwired.enable = true;
```

> [!NOTE]
> The `services.cardwired.enable` module is currently only available on nixpkgs unstable. It will be included in the 26.11 release.

For installation and usage instructions, refer to the [documentation](https://opengamingcollective.github.io/cardwire/).
