# Asusctl On NixOS

> A simple guide for getting asusctl running on NixOS

## Contents

- [Contents](#contents)
- [Disclaimer](#disclaimer)
- [Requirement](#requirement)
- [Installation](#installation)

## Disclaimer

This guide expects some previous knowledge about NixOS and it's configuration system.

Please note that NixOS in not officially supported by this project, and any issues specific to them shall be reported on the nixpkgs's [GitHub page](https://github.com/NixOS/nixpkgs/issues).

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
