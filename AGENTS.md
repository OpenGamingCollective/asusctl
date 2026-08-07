# ASUSCTL – AI contributor guide

ASUSCTL is a Linux utility for controlling ASUS laptop hardware features.

## Overview

| Area | Location | Description |
|------|----------|-------------|
| asus-shutdown | `asus-shutdown/` | Shutdown-time applier for deferred ASUS GPU firmware writes |
| asusctl (CLI) | `asusctl/` | Command-line tool for controlling laptop features via asusd |
| asusd (system D-Bus daemon) | `asusd/` | System daemon bridging kernel sysfs/udev to D-Bus interfaces |
| asusd-user (user D-Bus daemon) | `asusd-user/` | User-session daemon for per-user AniMe and RGB control |
| config-traits | `config-traits/` | Shared config load/save traits with versioned migration |
| dmi-id | `dmi-id/` | Hardware detection via SMBIOS/DMI data |
| rog-platform | `rog-platform/` | Sysfs/platform wrapper for ASUS firmware attributes and sensors |
| rog-dbus | `rog-dbus/` | D-Bus proxies and interface definitions for asusd |
| rog-anime | `rog-anime/` | Types and image conversion for the AniMe Matrix display |
| rog-aura | `rog-aura/` | Types for ASUS keyboard RGB (Aura) effects |
| rog-profiles | `rog-profiles/` | Fan-curve and platform-profile management |
| rog-control-center | `rog-control-center/` | Slint-based GUI for all laptop features |
| rog-slash | `rog-slash/` | Types for the ASUS Slash auxiliary display |
| rog-scsi | `rog-scsi/` | SCSI LED controller communication (Arion protocol) |
| simulators | `simulators/` | Hardware simulators for development without real devices |
| data (service files, udev rules) | `data/` | systemd units, udev rules, D-Bus policy, icons |
| extra (icons, web assets) | `extra/` | Application icons and web assets |
| distro-packaging | `distro-packaging/` | Packaging recipes for Arch, RPM, and install scripts |
| CI/CD (GitHub Actions) | `.github/` | CI/CD workflow definitions |

### What not to do

- Don't use `--no-verify` or `-n` with `git commit` or `git push`. All commits and pushes must run the git hooks (pre-commit/pre-push) normally.