#
# spec file for package asus-nb-ctrl
#
# Copyright (c) 2020-2025 Luke Jones <luke@ljones.dev>
#
# All modifications and additions to the file contributed by third parties
# remain the property of their copyright owners, unless otherwise agreed
# upon. The license for this file, and modifications and additions to the
# file, is the same license as for the pristine package itself (unless the
# license for the pristine package is not an Open Source License, in which
# case the license is the MIT License). An "Open Source License" is a
# license that conforms to the Open Source Definition (Version 1.9)
# published by the Open Source Initiative.

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

%if %{defined fedora}
%global debug_package %{nil}
%endif

%define version 6.3.9
%define specrelease %{?dist}
%define pkg_release 2%{specrelease}

# Use hardening ldflags.
%global rustflags -Clink-arg=-Wl,-z,relro,-z,now
Name:    asusctl
Version: %{version}
Release: %{pkg_release}
Summary: Control fan speeds, LEDs, graphics modes, and charge levels for ASUS notebooks
License: MPLv2

Group:   System Environment/Kernel

URL:     https://gitlab.com/asus-linux/asusctl
Source:  https://gitlab.com/asus-linux/asusctl/-/archive/%{version}/%{name}-%{version}.tar.gz

%if %{defined fedora}
BuildRequires:  rust-packaging
BuildRequires:  systemd-rpm-macros
%else
BuildRequires:  cargo-packaging
%endif
BuildRequires:  git
BuildRequires:  clang-devel
BuildRequires:  cargo
BuildRequires:  cmake
BuildRequires:  rust
BuildRequires:  rust-std-static
BuildRequires:  pkgconfig(gbm)
BuildRequires:  pkgconfig(libinput)
BuildRequires:  pkgconfig(libseat)
BuildRequires:  pkgconfig(libudev)
BuildRequires:  pkgconfig(xkbcommon)
BuildRequires:  pkgconfig(libzstd)
BuildRequires:  pkgconfig(fontconfig)
BuildRequires:  desktop-file-utils

%description
asus-nb-ctrl is a utility for Linux to control many aspects of various
ASUS laptops but can also be used with non-Asus laptops with reduced features.

It provides an interface for rootless control of some system functions such as
fan speeds, keyboard LEDs, battery charge level, and graphics modes.
asus-nb-ctrl enables third-party apps to use the above with dbus methods.

%package rog-gui
Summary: An experimental GUI for %{name}
Requires: %{name} = %{version}-%{release}

%description rog-gui
A one-stop-shop GUI tool for asusd/asusctl. It aims to provide most controls,
a notification service, and ability to run in the background.

%prep
%autosetup
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[term]
verbose = true
[net]
offline = false
EOF

%build
export RUSTFLAGS="%{rustflags}"
%if %{defined fedora}
/usr/bin/cargo build --release --locked
%else
/usr/bin/cargo auditable build --release --locked
%endif

%install
export RUSTFLAGS="%{rustflags}"

%define _target_dir target/release

# Install binaries
install -D -m 0755 %{_target_dir}/asusd %{buildroot}%{_bindir}/asusd
install -D -m 0755 %{_target_dir}/asus-shutdown %{buildroot}%{_bindir}/asus-shutdown
install -D -m 0755 %{_target_dir}/asusd-user %{buildroot}%{_bindir}/asusd-user
install -D -m 0755 %{_target_dir}/asusctl %{buildroot}%{_bindir}/asusctl
install -D -m 0755 %{_target_dir}/rog-control-center %{buildroot}%{_bindir}/rog-control-center

# Install systemd units
install -D -m 0644 data/asusd.service %{buildroot}%{_unitdir}/asusd.service
install -D -m 0644 data/asus-shutdown.service %{buildroot}%{_unitdir}/asus-shutdown.service

# Install udev rules
install -D -m 0644 data/asusd.rules %{buildroot}%{_udevrulesdir}/99-asusd.rules

# Install dbus config
install -D -m 0644 data/asusd.conf %{buildroot}%{_datadir}/dbus-1/system.d/asusd.conf

# Install asusd data
install -D -m 0644 rog-aura/data/aura_support.ron %{buildroot}%{_datadir}/asusd/aura_support.ron
cp -r rog-anime/data/anime %{buildroot}%{_datadir}/asusd/

# Install rog-gui data
install -D -m 0644 rog-control-center/data/org.opengamingcollective.rog-control-center.desktop %{buildroot}%{_datadir}/applications/org.opengamingcollective.rog-control-center.desktop
install -D -m 0644 rog-control-center/data/rog-control-center.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/rog-control-center.png
mkdir -p %{buildroot}%{_datadir}/rog-gui/layouts
cp -r rog-aura/data/layouts/*.ron %{buildroot}%{_datadir}/rog-gui/layouts/

# Install icons
install -D -m 0644 data/icons/asus_notif_yellow.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/asus_notif_yellow.png
install -D -m 0644 data/icons/asus_notif_green.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/asus_notif_green.png
install -D -m 0644 data/icons/asus_notif_blue.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/asus_notif_blue.png
install -D -m 0644 data/icons/asus_notif_red.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/asus_notif_red.png
install -D -m 0644 data/icons/asus_notif_orange.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/asus_notif_orange.png
install -D -m 0644 data/icons/asus_notif_white.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/asus_notif_white.png
install -D -m 0644 data/icons/scalable/gpu-compute.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/status/gpu-compute.svg
install -D -m 0644 data/icons/scalable/gpu-hybrid.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/status/gpu-hybrid.svg
install -D -m 0644 data/icons/scalable/gpu-integrated.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/status/gpu-integrated.svg
install -D -m 0644 data/icons/scalable/gpu-nvidia.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/status/gpu-nvidia.svg
install -D -m 0644 data/icons/scalable/gpu-vfio.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/status/gpu-vfio.svg
install -D -m 0644 data/icons/scalable/notification-reboot.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/status/notification-reboot.svg

# Install docs
install -D -m 0644 README.md %{buildroot}%{_docdir}/%{name}/README.md
install -D -m 0644 rog-anime/README.md %{buildroot}%{_docdir}/%{name}/README-anime.md
install -D -m 0644 rog-anime/data/diagonal-template.png %{buildroot}%{_docdir}/%{name}/diagonal-template.png

# Install LICENSE to asusctl datadir
install -D -m 0644 LICENSE %{buildroot}%{_datadir}/asusctl/LICENSE

desktop-file-validate %{buildroot}%{_datadir}/applications/org.opengamingcollective.rog-control-center.desktop

%post
%systemd_post asusd.service
%systemd_post asus-shutdown.service

%preun
%systemd_preun asusd.service
%systemd_preun asus-shutdown.service

%postun
%systemd_postun_with_restart asusd.service
%systemd_postun_with_restart asus-shutdown.service

%files
%license LICENSE
%{_bindir}/asusd
%{_bindir}/asus-shutdown
%{_bindir}/asusd-user
%{_bindir}/asusctl
%{_unitdir}/asusd.service
%{_unitdir}/asus-shutdown.service
%{_udevrulesdir}/99-asusd.rules
%{_datadir}/dbus-1/system.d/asusd.conf
%{_datadir}/icons/hicolor/512x512/apps/asus_notif_yellow.png
%{_datadir}/icons/hicolor/512x512/apps/asus_notif_green.png
%{_datadir}/icons/hicolor/512x512/apps/asus_notif_red.png
%{_datadir}/icons/hicolor/512x512/apps/asus_notif_blue.png
%{_datadir}/icons/hicolor/512x512/apps/asus_notif_orange.png
%{_datadir}/icons/hicolor/512x512/apps/asus_notif_white.png
%{_datadir}/icons/hicolor/scalable/status/gpu-compute.svg
%{_datadir}/icons/hicolor/scalable/status/gpu-hybrid.svg
%{_datadir}/icons/hicolor/scalable/status/gpu-integrated.svg
%{_datadir}/icons/hicolor/scalable/status/gpu-nvidia.svg
%{_datadir}/icons/hicolor/scalable/status/gpu-vfio.svg
%{_datadir}/icons/hicolor/scalable/status/notification-reboot.svg
%{_docdir}/%{name}/
%{_datadir}/asusctl/
%{_datadir}/asusd/

%files rog-gui
%{_bindir}/rog-control-center
%{_datadir}/applications/org.opengamingcollective.rog-control-center.desktop
%{_datadir}/icons/hicolor/512x512/apps/rog-control-center.png
%{_datadir}/rog-gui

%changelog
