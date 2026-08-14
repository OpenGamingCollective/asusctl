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

### Cardwire

Cardwire is the community's new replacement for the now-deprecated supergfxctl.

> [!CAUTION]
> Cardwire is currently still considered EXPERIMENTAL. If you choose to install this tool, expect rough edges and quirks. For support, join our Discord server.

Cardwire is also packaged in nixpkgs. Enable it with:

```nix
services.cardwired.enable = true;
```

> [!NOTE]
> The `services.cardwired.enable` module is currently only available on nixpkgs unstable. It will be included in the 26.11 release.

For installation and usage instructions, refer to the [documentation](https://opengamingcollective.github.io/cardwire/).
