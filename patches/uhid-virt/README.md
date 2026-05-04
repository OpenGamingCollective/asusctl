# uhid-virt

uhid-virt provides a safe wrapper around uhid-sys

Forked from uhid-fs, so what changed? Removal of `ArrayVec` dependency, and attempts to write
in to borrows slices rather than creating and cloning new vectors everywhere. Where possible
it tries to be an almost drop-in replacement for [tokio_linux_uhid](https://crates.io/crates/tokio-linux-uhid).

## What is UHID?

UHID lets you write userspace drivers for HID devices in Linux. No need for a kernel module, just run your program and you can register an HID device.

There are a lot of things you can do with this, to name a few:

* Forwarding or emulating keypresses through [control daemon](https://gitlab.com/flukejones/rog-core)
* Emulate a mouse/keyboard for shortcuts/macros/task automation (independent of X11/Wayland/system console)
* Add support for HID devices that are only supported on Windows / other platforms
* Write drivers for a new HID device (i.e. [DIY Arduino water touchpad](https://www.open-electronics.org/guest_projects/diy-0-water-touchpad/))

### Kernel Docs Description

> UHID allows user-space to implement HID transport drivers. Please see [hid-transport.html](https://www.kernel.org/doc/html/latest/hid/hid-transport.html) for an introduction into HID transport drivers. This document relies heavily on the definitions declared there.

> With UHID, a user-space transport driver can create kernel hid-devices for each device connected to the user-space controlled bus. The UHID API defines the I/O events provided from the kernel to user-space and vice versa.

See the [Kernel UHID doc page](https://www.kernel.org/doc/html/latest/hid/uhid.html) for a full explanation of the mechanics.


## Examples

See the example folder. Sending a newline will make the mouse move to the right.

See [flukejones/rog-core](https://gitlab.com/flukejones/rog-core) and 
[sameer/gearvr-controller-uhid](https://github.com/sameer/gearvr-controller-uhid/) for a real-world use cases.
