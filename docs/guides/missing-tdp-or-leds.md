# Missing TDP or LED Control

> Dealing with TDP or LED control in ASUS laptops

So you installed a supported distro and either LEDs or TDP are not controllable. That is not good, but these problems can usually be solved with a bit of help.

## Why?

There are two big tables: one in the kernel and one in `asusd` for TDP and LEDs, respectively. Those tables are probably missing your model, and adding it will solve the issue.

## Prerequisites

To solve the issue, you will need a Win-to-Go installation as described in the [Introduction](../introduction.md) guide.

## Missing TDP control

Collect your data and send it in the [PPT data collection issue](https://github.com/OpenGamingCollective/asusctl/issues/124), then drop a note on Discord. It will be added when there is time.

For a technical analysis, see [Adding PPT values from Armoury Crate](https://youtu.be/s0GWSvmiB00).

## Missing LED control

The process is similar for missing LED control, except the table is in the `asusd` software: [`aura_support.ron`](https://github.com/OpenGamingCollective/asusctl/blob/main/rog-aura/data/aura_support.ron).

Add your model to this file locally and test the changes, rebooting your laptop afterward. If it works, fork the repository, add the model, and submit a pull request to the original repository. The change will then be available to other users.

The capabilities of your model can be found in the official ASUS Armoury Crate software.
