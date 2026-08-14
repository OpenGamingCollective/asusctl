# Asusctl

## Contents

- [Pressing Fn+F5 doesn't do anything](#pressing-fnf5-doesnt-do-anything)
- [I get an error "org.asuslinux.Daemon was not provided by any .service files" when I run asusctl](#i-get-an-error-orgasuslinuxdaemon-was-not-provided-by-any-service-files-when-i-run-asusctl)
- [Why am I getting errors about my keyboard?](#why-am-i-getting-errors-about-my-keyboard)
- [It's not working!](#its-not-working)
- [I don't have any power profiles or charge control](#i-dont-have-any-power-profiles-or-charge-control)
- [How do I set a custom fan curve?](#how-do-i-set-a-custom-fan-curve)

### Pressing Fn+F5 doesn't do anything

You need to map the key-combo to an action in your desktop, like this:

![Custom Shortcut Window](../assets/faq/custom_shortcut.png)

### I get an error "org.asuslinux.Daemon was not provided by any .service files" when I run asusctl

The daemon isn't running, check the logs with sudo `journalctl -b -u asusd` and look for errors.

### Why am I getting errors about my keyboard?

Please ensure you are using a recent kernel. Please use at least 6.19 so that you get all the most recent patches and fixes for ASUS laptops.

### It's not working!

Check the logs with `sudo journalctl -b -u asusd` and look for errors.

### I don't have any power profiles or charge control

We recommend to use at least 6.19 so that you get all the most recent patches and fixes for ASUS laptops.

It's also possible that your laptop doesn't support this so if the kernel update doesn't solve this feel free to make a :sadface: (sorry).

### How do I set a custom fan curve?

Custom fan curves (not speaking of the built-in power profiles) are only supported on specific models.

See the [supported laptops list](https://github.com/OpenGamingCollective/asusctl#supported-laptops) to check whether your model is included.
The necessary kernel patches are merged since 5.17.

The data format is a comma-separated list of points in the form `30c:1%,49c:2%,...`, where each point is a temperature followed by a fan speed. If the `%` is omitted the fan range is 0-255.

There are three fan profiles namely Quiet, Balanced and Performance to choose from. Each profile is linked to power profile and gets applied when the power profile is set. You can enable/disable all fan profiles at once for a profile using the following command:

```bash
asusctl fan-curve --mod-profile <profile_name> --enable-fan-curves true/false
```

To enable or disable a single fan curve for a profile use `--enable-fan-curve` together with `--fan <cpu/gpu/mid>`.

All three fan profiles can be activated at once. If no profile is activated manually then the fan curve from the BIOS is used.
To change the fan curve data for a specific profile use the following command:

```bash
asusctl fan-curve --mod-profile <profile_name> --data <fan_curve_data>
```

An optional `--fan <cpu/gpu/mid>` can be added to select the fan to apply the data to (defaults to `cpu`).
