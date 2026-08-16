# General Recommendations

> General recommendations for the best experience on ASUS ROG laptops

These recommendations apply to any distribution as long as you use the power profile daemon (ppd) or manually configure the settings (Asusctl was developed with PPD in mind).

## Recommended desktop environment

This rule applies to almost the entire project, but rogcc runs on Wayland;

> [!CAUTION]
> X11 is not supported. That is why KDE Plasma and GNOME are recommended for the best and most complete experience.

> [!NOTE]
> Some integrated GPUs, such as AMD (Radeon 680M in my case), can cause the desktop to freeze. In this case, we recommend KDE Plasma, as it usually restarts the compositor successfully, so you won't need to force-restart your laptop.

## Improve battery on AMD Laptops

Read docs for more details: [PPD Documentation](https://gitlab.freedesktop.org/upower/power-profiles-daemon)

If you followed any guide, you should have daemon power profiles, which have two functions that are disabled by default: Panel power savings and AMDGPU Dynamic power management. These actions apply only to laptops with integrated Radeon graphics. Check which of them are available on your device:

```bash
powerprofilesctl list-actions
```

Only enable the actions that are shown as available in the output, following the steps below.

### Panel power savings

```bash
powerprofilesctl configure-action amdgpu_panel_power --enable
```

> [!NOTE]
> `amdgpu_panel_power` only takes effect while running on battery, and only with the balanced or power-saver profile active.

Check if it is working. First, find the path to the `panel_power_savings` file for your internal display, as the card and connector names vary per device:

```bash
ls /sys/class/drm/card*-eDP-*/amdgpu/panel_power_savings
```

Then read it:

```bash
cat /sys/class/drm/card*-eDP-*/amdgpu/panel_power_savings
```

This option should be above 0, It just dims the screen a little to save battery life, but it depends on your screen model.

### AMDGPU Dynamic power management

```bash
powerprofilesctl configure-action amdgpu_dpm --enable
```

> [!NOTE]
> `amdgpu_dpm` lowers the clocks only under the power-saver profile. Select the power-saver profile (e.g. `powerprofilesctl set power-saver`) before expecting `power_dpm_force_performance_level` to report low.

Check if it is working:

```bash
cat /sys/class/drm/card2/device/power_dpm_force_performance_level
```

This option is the most important, because in battery it needs to say low. With this, you should get a battery that is very close to Windows.

> [!NOTE]
> 2 is the number of your iGPU, this can be different on your device.

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
