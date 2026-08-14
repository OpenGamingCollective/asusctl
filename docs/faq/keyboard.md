# Keyboard

## Contents

- [Can I re-map the arrow keys?](#can-i-re-map-the-arrow-keys)
- [I have a laptop where the arrow keys do not emit keycodes, can I use these?](#i-have-a-laptop-where-the-arrow-keys-do-not-emit-keycodes-can-i-use-these)
- [My keyboard isn't working properly with the driver](#my-keyboard-isnt-working-properly-with-the-driver)
- [Can I customise the Fn key?](#can-i-customise-the-fn-key)
- [My laptop has no SysRq key. Can I remap any key to SysRq?](#my-laptop-has-no-sysrq-key-can-i-remap-any-key-to-sysrq)

### Can I re-map the arrow keys?

Yes, create a file named `/etc/udev/hwdb.d/90-nkey.hwdb` with:

```conf
# Format evdev:input:b<bus_id>v<vendor_id>p<product_id>

# ** Note **
# The line evdev:input:b0003v0B05p1866* may vary on your ASUS Laptop.
# Modify the <vendor_id> and <product_id> based on the output of this command to ensure remaps work:
# $ lsusb | grep 'ASUSTek Computer, Inc. N-KEY Device' | awk -F'[: ]' '{print $7" "$8}' | tr '[:lower:]' '[:upper:]'

evdev:input:b0003v0B05p1866*
  KEYBOARD_KEY_c00b6=kbdillumdown # Fn+F2 (music prev)
  KEYBOARD_KEY_c00b5=kbdillumup   # Fn+F4 (music skip)
  KEYBOARD_KEY_ff3100c5=pagedown  # Fn+Down
  KEYBOARD_KEY_ff3100c4=pageup    # Fn+Up
  KEYBOARD_KEY_ff3100b2=home      # Fn+Left
  KEYBOARD_KEY_ff3100b3=end       # Fn+Right
```

then update hwdb with:

```bash
sudo systemd-hwdb update
sudo udevadm trigger
```

You can see a list of keycodes [here](https://github.com/torvalds/linux/blob/b76f733c3ff83089cf1e3f9ae233533649f999b3/include/uapi/linux/input-event-codes.h).

### I have a laptop where the arrow keys do not emit keycodes, can I use these?

Yes, you can.

### My keyboard isn't working properly with the driver

You may have a different keyboard. Please request support in one of the related projects on github, or in the discord server.

### Can I customise the Fn key?

No, the key is on a physically different circuit and used to physically signal the keyboard EC to switch key circuits.

There are three different circuits for the `0x8166` keyboard.

### My laptop has no SysRq key. Can I remap any key to SysRq?

Yes! Similar to remapping the Arrow-Keys above, you can remap - say the `menu (fn+RightCtrl)` key to `SysRq`.

Just add another line to `/etc/udev/hwdb.d/90-nkey.hwdb` with the following, including the leading whitespaces:

```conf
  KEYBOARD_KEY_<ScanCode>=sysrq        # force remap sysrq to Fn+RightCtrl
```

You can get the `<ScanCode>` by running

```bash
evtest /dev/input/by-id/usb-ASUSTeK_Computer_Inc._N-KEY_Device-*-kbd
```

and pressing the `RightCtrl` key.
In this case, it is `70065`

```bash
Testing ... (interrupt to exit)
Event: time 1662839073.640933, type 4 (EV_MSC), code 4 (MSC_SCAN), value 70065 <--------- Substitute this as <ScanCode>
Event: time 1662839073.640933, type 1 (EV_KEY), code 127 (KEY_COMPOSE), value 1
Event: time 1662839073.640933, -------------- SYN_REPORT ------------
```

Then update hwdb with:

```bash
sudo systemd-hwdb update
sudo udevadm trigger
```
