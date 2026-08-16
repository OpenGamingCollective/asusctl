# PikaOS

> Debian Sid-based gaming distribution with asusctl included

[PikaOS](https://pika-os.com/) is a Debian Sid-based gaming-oriented distribution that ships asusctl in its own repositories.

## Installation

asusctl is packaged by PikaOS and can be installed directly with `apt`:

```bash
sudo apt install asusctl rog-control-center
```

## After installation

Enable and start the services:

```bash
sudo systemctl enable --now asusd.service asus-shutdown.service
systemctl --user daemon-reload
systemctl --user enable --now asusd-user.service
```

For everything else (platform profiles, fan curves, GPU switching), refer to the [Usage](../usage/asusctl.md) section.

## Graphics Switching

See [GPU Switching](../hardware/gpu-switching.md) for how to manage the dGPU and MUX.
