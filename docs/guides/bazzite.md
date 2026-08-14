# Bazzite Setup Guide

> A friendly guide for setting up Bazzite on ASUS laptops

Newcomers should start by reading the [Intro](../introduction.md) guide.

Bazzite is a gaming-oriented atomic Fedora image based on [Universal Blue](https://universal-blue.org/). Like other atomic Fedora images it uses rpm-ostree, and `asusctl` is not preinstalled. Bazzite ships Homebrew preconfigured and the Terra repository preconfigured but disabled, which gives you two ways to install asusctl.

## Contents

- [Recommended: ujust asus](#recommended-ujust-asus)
- [Alternative: Terra repository](#alternative-terra-repository)
- [Graphics Switching](#graphics-switching)
- [ROG Ally and Ally X](#rog-ally-and-ally-x)

### Recommended: ujust asus

The supported way on Bazzite is the `ujust asus` helper. It installs the Universal Blue Homebrew casks `asusctl-linux` and `rog-control-center-linux` from the `ublue-os/tap` tap, and enables the required services. No reboot is needed and the installation survives rebasing:

```bash
ujust asus install
```

The services are enabled automatically.

> [!NOTE]
> There is no `asusctl` formula in homebrew-core, so `brew install asusctl` does not work. The Universal Blue tap is the only Homebrew source.

### Alternative: Terra repository

Bazzite ships the Terra repository preconfigured but disabled. If you prefer RPMs (which are usually newer than the Homebrew casks), enable it and layer the packages:

```bash
sudo sed -i 's/enabled=0/enabled=1/' /etc/yum.repos.d/terra.repo /etc/yum.repos.d/terra-extras.repo
sudo rpm-ostree install asusctl asusctl-rog-gui
```

Reboot after layering. Keep in mind that rpm-ostree layered packages can pause updates and may not survive rebasing to a new image, so the Homebrew method is recommended.

### Graphics Switching

It is now possible to manage your graphics card with `asusctl` or the ROG Control Center. You can check if your device supports graphics switching by running the following command:

```bash
asusctl armoury list
```

If your device supports disabling of the dGPU, you should see an entry that looks like the following:

```bash
dgpu_disable:
  current: [(0),1]
```

Here, a current value of 0 means that your dgpu is not disabled (i.e., enabled).

You can set whether you want to utilize your dGPU by modifying the setting under the `GPU Configuration` tab in the ROG Control Center. Alternatively, use the command `asusctl armoury set dgpu_disable 1` to disable the dgpu, and 0 to re-enable it.

> [!NOTE]
> Due to how Linux systems are configured to use the dGPU, you must reboot your system after changing your dGPU configuration. If you wish to power off your dgpu without rebooting, you should use an alternative program such as Cardwire (see below).

#### Cardwire

Cardwire is the community's new replacement for the now-deprecated supergfxctl.

> [!CAUTION]
> Cardwire is currently still considered EXPERIMENTAL. If you choose to install this tool, expect rough edges and quirks. For support, join our Discord server.

Bazzite ships Cardwire out of the box, so no installation is needed.

For installation and usage instructions, refer to the [documentation](https://opengamingcollective.github.io/cardwire/).
