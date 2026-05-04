#!/usr/bin/env bash
# Build dependencies for Ubuntu 22.04 LTS (Jammy) — asusctl / rog-control-center.
# Safe to re-run; only installs packages via apt. Requires: sudo, network.
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

sudo apt-get update -qq

# Core: matches .gitlab-ci.yml baseline + Slint (Wayland / femtovg) common needs.
# libgtk-3-dev: aligns with upstream CI; skip if your system has broken Pango/GTK pins
# (mixed PPAs). The GUI stack here uses Slint + winit-wayland; GTK is still pulled by CI.
PKGS=(
  build-essential
  cmake
  pkg-config
  curl
  git
  libudev-dev
  libfontconfig1-dev
  libxkbcommon-dev
  libxkbcommon-x11-dev
  libclang-dev
  llvm-dev
  libwayland-dev
  libegl1-mesa-dev
  libinput-dev
  libgtk-3-dev
  libssl-dev
  libdbus-1-dev
  gettext
  desktop-file-utils
  grep
)

set +e
sudo apt-get install -y "${PKGS[@]}"
apt_status=$?
set -e

if [[ "$apt_status" -ne 0 ]]; then
  echo "Note: full install failed (often libgtk-3-dev / Pango conflicts on mixed systems)." >&2
  echo "Retrying without libgtk-3-dev — release builds of this repo have succeeded without it." >&2
  sudo apt-get install -y build-essential cmake pkg-config curl git libudev-dev \
    libfontconfig1-dev libxkbcommon-dev libxkbcommon-x11-dev libclang-dev llvm-dev \
    libwayland-dev libegl1-mesa-dev libssl-dev libdbus-1-dev gettext
fi

echo "Done. Install Rust >= workspace MSRV (see rust-toolchain.toml), e.g.:"
echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.82"
echo "Then: source \"\$HOME/.cargo/env\" && make && sudo make install"
