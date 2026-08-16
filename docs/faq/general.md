# General

## Contents

- [How do I get desktop notifications for asusctl?](#how-do-i-get-desktop-notifications-for-asusctl)
- [How can I enable S3 (legacy) suspend?](#how-can-i-enable-s3-legacy-suspend)
- [Note for ROG Zephyrus G15 (2022)](#note-for-rog-zephyrus-g15-2022)
- [What steps are needed if I want to dual boot?](#what-steps-are-needed-if-i-want-to-dual-boot)
- [Note for ROG Flow X13 (2021)](#note-for-rog-flow-x13-2021)
- [Is `<distro>` supported by asusctl?](#is-distro-supported-by-asusctl)

### How do I get desktop notifications for asusctl?

This function is now integrated into the ROG Control Center, so long as you run it in the background you will get the notifications.

You can find all notify settings in the "App Settings" in the ROG Control Center.

### How can I enable S3 (legacy) suspend?

Depending on your kernel version, you may occasionally experience issues with the 2021/2022 versions of the Zephyrus G14/G15 which affect the proper use of newer suspend methods, like s0ix.

A potential fix is to patch your DSDT tables so your machine uses the older suspend method, called S3. In our tests this works great on the 2021 / 2022 G14 and G15. Those patches are not part of the main repo and can't be. It will always be a manual matter and cannot be integrated into the kernel.

> [!IMPORTANT]
> If you update the BIOS be sure you disable your DSDT table and create a new one. DSDT tables could change with newer BIOS versions!

You can find the script here: https://gitlab.com/marcaux/g14-2021-s3-dsdt

### Note for ROG Zephyrus G15 (2022)

After BIOS version 313, ASUS fixed ACPI support for Linux, which is crucial if you want Performance mode to work properly.

And ASUS optimized power distribution between CPU and GPU, which before caused stuttering/frame drops in performance mode that confuse many users for a long time.

### What steps are needed if I want to dual boot?

Be sure to consider the following:

- disable fast boot within the BIOS
- disable fast boot in Windows
- always fully shutdown after using and switchting to another OS so the hardware gets correctly initialized

If you still experience an issue, hold down the power button while on battery for a few seconds to force a shutdown. This often helps to reset some things.

These steps are not needed if you are running Linux exclusively.

### Note for ROG Flow X13 (2021)

BIOS versions 408 & 409 cannot boot a Linux kernel newer than 5.15.x so you will need to upgrade to the 410 BIOS from the [official ROG Flow X13 (2021) BIOS support page](https://rog.asus.com/bt/laptops/rog-flow/2021-rog-flow-x13-series/helpdesk_bios/).

### Is `<distro>` supported by asusctl?

See the [Supported Distributions](../getting-started/supported-distributions.md) page for the officially supported distributions and what to expect on others. In short: The kernel must be >=6.19.
