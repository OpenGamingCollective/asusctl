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

Read docs for more details: [PPD Documentation](https://gitlab.gnome.org/Infrastructure/Mirrors/lorry-mirrors/gitlab_freedesktop_org/hadess/power-profiles-daemon)

If you followed any guide, you should have daemon power profiles, which have two functions that are disabled by default: Panel power savings and AMDGPU Dynamic power management. These are dedicated to AMD iGPU, and you must follow these steps:

### Panel power savings

```bash
powerprofilesctl configure-action amdgpu_panel_power --enable
```

Check if it is working:

```bash
cat /sys/class/drm/card2/card2-eDP-2/amdgpu/panel_power_savings
```

This option should be above 0, It just dims the screen a little to save battery life, but it depends on your screen model.

### AMDGPU Dynamic power management

```bash
powerprofilesctl configure-action amdgpu_dpm --enable
```

Check if it is working:

```bash
cat /sys/class/drm/card2/device/power_dpm_force_performance_level
```

This option is the most important, because in battery it needs to say low. With this, you should get a battery that is very close to Windows.

> [!NOTE]
> 2 is the number of your iGPU, this can be different on your device.

### Audio powersaving

You can enable audio powersaving features:

```bash
/etc/modprobe.d/audio.conf

# enable audio power savings
options snd_hda_intel power_save=1
```
