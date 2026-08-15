# Ubuntu

> Only Ubuntu 26.04 LTS is supported; asusctl is distributed using [Homebrew](https://brew.sh/)

## Installation

1. Install Homebrew for Linux if you don't have it already:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

2. Add the ublue-os tap:

```bash
brew tap ublue-os/homebrew-tap
```

3. Install asusctl and the ROG Control Center:

```bash
brew install asusctl-linux
brew install rog-control-center-linux
```

The casks install the `asusd` systemd units, udev rules, and D-Bus configuration automatically. Verify the daemon is running:

```bash
systemctl status asusd.service
```

For everything else (platform profiles, fan curves, GPU switching), refer to the [Usage](../usage/asusctl.md) section.
