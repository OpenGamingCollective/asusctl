# General Recommendations

> General recommendations for the best experience on ASUS ROG laptops

These recommendations apply to any distribution as long as you use the power profile daemon (ppd) or manually configure the settings (Asusctl was developed with PPD in mind).

## Recommended desktop environment

This rule applies to almost the entire project, but rogcc runs on Wayland;

> [!CAUTION]
> X11 is not supported. That is why KDE Plasma and GNOME are recommended for the best and most complete experience.

> [!NOTE]
> Some integrated GPUs, such as AMD (Radeon 680M in my case), can cause the desktop to freeze. In this case, we recommend KDE Plasma, as it usually restarts the compositor successfully, so you won't need to force-restart your laptop.

### Audio powersaving

You can enable audio powersaving features. Create or edit `/etc/modprobe.d/audio.conf` as root (e.g. `sudo nano /etc/modprobe.d/audio.conf`) and add:

```conf
# enable audio power savings
options snd_hda_intel power_save=1
```

For the setting to take effect, reboot the system.

### Wi-Fi powersaving

If your wireless card is managed by `iwlwifi`, you can enable Wi-Fi power saving. Create or edit `/etc/modprobe.d/iwlwifi.conf` and add:

```conf
options iwlwifi power_save=1
```

For the setting to take effect, reboot the system.
