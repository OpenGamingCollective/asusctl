# GPU Switching

> Managing the dGPU and MUX on hybrid ASUS laptops

## What GPU modes does my laptop support?

Support varies between laptops: some have Eco Mode only (`dgpu_disable`) and some have Ultimate Mode as well (`gpu_mux_mode`).

You can set your GPU configuration by modifying the setting under the `GPU Configuration` tab in the ROG Control Center. It writes both the `dgpu_disable` and `gpu_mux_mode` attributes. Alternatively, use the command line:

You can check if your device supports graphics switching by running:

```bash
asusctl armoury list
```

If your device supports disabling of the dGPU (Eco Mode), you should see an entry that looks like the following:

```bash
dgpu_disable:
  current: [(0),1]
```

When this attribute is `1`, the dGPU is disabled and only the iGPU will be used (Integrated mode). A value of `0` leaves the dGPU enabled. `dgpu_disable` is independent of the MUX, so whether the dGPU drives the display is then up to `gpu_mux_mode`.

If your device supports routing the screen to the dGPU (Ultimate Mode), you should see an entry that looks like the following:

```bash
gpu_mux_mode:
  current: [(0),1]
```

Here, a current value of `0` means that the MUX is disabled, in which case the discrete GPU is your default GPU and will be routed to your screen.

```bash
asusctl armoury set dgpu_disable 1   # disable the dGPU
asusctl armoury set dgpu_disable 0   # re-enable the dGPU
asusctl armoury set gpu_mux_mode 0   # enable the MUX (route the screen to the dGPU)
asusctl armoury set gpu_mux_mode 1   # disable the MUX
```

On MUX-capable systems the two attributes must be set in pairs; the valid combinations are:

| Mode       | `dgpu_disable` | `gpu_mux_mode` |
| ---------- | -------------- | -------------- |
| Integrated | 1              | 1              |
| Hybrid     | 0              | 1              |
| Ultimate   | 0              | 0              |

On systems without a MUX, only `dgpu_disable` is available.

> [!NOTE]
> Due to how Linux systems are configured to use the dGPU, you must reboot your system after changing your dGPU configuration. If you wish to power off your dGPU without rebooting, you should use an alternative program such as Cardwire (see below).

## Cardwire

Cardwire is the community's new replacement for the now-deprecated supergfxctl. It manages the GPU with an eBPF/LSM approach, allowing the dGPU to power down at runtime - no reboot required.

> [!CAUTION]
> Cardwire is currently still considered EXPERIMENTAL. If you choose to install this tool, expect rough edges and quirks. For support, join our Discord server.

See the [Cardwire documentation](https://opengamingcollective.github.io/cardwire/) for installation and usage instructions. On some distributions it is packaged: on Arch via the [OGC repository](../distributions/arch.md) (`pacman -S cardwire`), on Fedora-based systems via Terra (`dnf install cardwire`), and on NixOS via `services.cardwired.enable`.
