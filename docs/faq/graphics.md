# Graphics & Switching

## Contents

- [Nvidia card is not sleeping!](#nvidia-card-is-not-sleeping)
- [Nvidia Dynamic Boost isn't working!](#nvidia-dynamic-boost-isnt-working)

### Nvidia card is not sleeping!

After checking the usual suspects:

- Missing configuration: follow the setup steps in the [distribution guides](../distributions/index.md).
- GPU monitoring widgets: keep the GPU on to monitor it!
- ollama: if ollama is running nvidia might not be able to sleep!

One possible culprit might also be realtime audio kit being enabled.

This service can probe all available audio devices, which includes nvidia hdmi audio!

### Nvidia Dynamic Boost isn't working!

Since version 525.53, NVIDIA added official Dynamic Boost support for AMD laptop that uses Ryzen 6000 Series (or newer) CPU.

To enable it, follow the next steps:

1. Start nvidia-powerd.service

```bash
sudo systemctl start nvidia-powerd.service
```

If you don't want to start it manually each time, then:

```bash
sudo systemctl enable nvidia-powerd.service
```

2. Set your power profile to "Performance" Mode

You can do it in many ways. If you already installed asusctl, you can switch to it using:

```bash
asusctl profile set Performance
```

or ROG Control Center (GUI) to set it.

3. Test it

Using tools like "nvtop" and "mangohud", you can monitor your CPU and GPU power in realtime.

To check if the Dynamic boost works, you need to identify what is the MAX TGP your model support. Usually, it can be found on manufacturers' websites.

Here are some power data collected from Zephyrus G15 (2022), which rated 120-watt maximum TGP.

Quiet mode: 25 Watt (CPU) + 60 Watt (GPU)

Balanced Mode: 30 Watt (CPU) + 80 Watt (GPU)

Performance Mode: 25 Watt (CPU) + 115 Watt (GPU)
