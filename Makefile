VERSION := $(shell /usr/bin/grep -Pm1 'version = "(\d+.\d+.\d+.*)"' Cargo.toml | cut -d'"' -f2)

INSTALL = install
INSTALL_PROGRAM = ${INSTALL} -D -m 0755
INSTALL_DATA = ${INSTALL} -D -m 0644

prefix = /usr
exec_prefix = $(prefix)
bindir = $(exec_prefix)/bin
datarootdir = $(prefix)/share
libdir = $(exec_prefix)/lib
zshcpl = $(datarootdir)/zsh/site-functions

BIN_ROG := rog-control-center
APP_ID := org.opengamingcollective.rog-control-center
BIN_C := asusctl
BIN_D := asusd
BIN_S := asus-shutdown
BIN_U := asusd-user
LEDCFG := aura_support.ron

DESTDIR_REALPATH := $(if $(DESTDIR),$(shell realpath $(DESTDIR)),)

SRC := Cargo.toml Cargo.lock Makefile $(shell find -type f -wholename '**/src/*.rs')

STRIP_BINARIES ?= 0

DEBUG ?= 0
ifeq ($(DEBUG),0)
	ARGS += --release
	TARGET = release
else
	ARGS += --profile dev
	TARGET = debug
endif

X11 ?= 0
ifeq ($(X11),1)
	ARGS += --features "rog-control-center/x11"
endif

# Always use the versions in Cargo.lock by default
ARGS += --locked

# Allow optionally freezing the build to avoid any network access and enforce Cargo.lock strictly
FROZEN ?= 0
ifeq ($(FROZEN),1)
	ARGS += --frozen
endif

VENDORED ?= 0
ifeq ($(VENDORED),1)
	ARGS += --frozen
endif

all: build

clean:
	cargo clean

distclean:
	rm -rf .cargo vendor vendor.tar.xz

target/$(TARGET)/$(BIN_D): $(SRC)
	$(MAKE) build

target/$(TARGET)/$(BIN_S): $(SRC)
	$(MAKE) build

target/$(TARGET)/$(BIN_C): $(SRC)
	$(MAKE) build

target/$(TARGET)/$(BIN_U): $(SRC)
	$(MAKE) build

target/$(TARGET)/$(BIN_ROG): $(SRC)
	$(MAKE) build

install-asusd: target/$(TARGET)/$(BIN_D)
	$(INSTALL_PROGRAM) "./target/$(TARGET)/$(BIN_D)" "$(DESTDIR)$(bindir)/$(BIN_D)"

install-asus-shutdown: target/$(TARGET)/$(BIN_S)
	$(INSTALL_PROGRAM) "./target/$(TARGET)/$(BIN_S)" "$(DESTDIR)$(bindir)/$(BIN_S)"

install-asusctl: target/$(TARGET)/$(BIN_C)
	$(INSTALL_PROGRAM) "./target/$(TARGET)/$(BIN_C)" "$(DESTDIR)$(bindir)/$(BIN_C)"

install-asusd_user: target/$(TARGET)/$(BIN_U)
	$(INSTALL_PROGRAM) "./target/$(TARGET)/$(BIN_U)" "$(DESTDIR)$(bindir)/$(BIN_U)"

install-rog_gui: target/$(TARGET)/$(BIN_ROG)
	$(INSTALL_PROGRAM) "./target/$(TARGET)/$(BIN_ROG)" "$(DESTDIR)$(bindir)/$(BIN_ROG)"

.PHONY: install-asusd install-asus-shutdown install-asusctl install-asusd_user install-rog_gui

install-program: install-asusd install-asus-shutdown install-asusctl install-asusd_user install-rog_gui

install-data-rog_gui:
	$(INSTALL_DATA) "./rog-control-center/data/$(BIN_ROG).desktop" "$(DESTDIR)$(datarootdir)/applications/$(BIN_ROG).desktop"
	$(INSTALL_DATA) "./rog-control-center/data/$(BIN_ROG).png" "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/$(BIN_ROG).png"
	$(INSTALL_DATA) "./rog-control-center/data/$(APP_ID).metainfo.xml" "$(DESTDIR)$(datarootdir)/metainfo/$(APP_ID).metainfo.xml"
	cd rog-aura/data/layouts && find . -type f -name "*.ron" -exec $(INSTALL_DATA) "{}" "$(DESTDIR_REALPATH)$(datarootdir)/rog-gui/layouts/{}" \;

	$(INSTALL_DATA) "./data/icons/asus_notif_yellow.png" "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_yellow.png"
	$(INSTALL_DATA) "./data/icons/asus_notif_green.png" "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_green.png"
	$(INSTALL_DATA) "./data/icons/asus_notif_blue.png" "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_blue.png"
	$(INSTALL_DATA) "./data/icons/asus_notif_red.png" "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_red.png"
	$(INSTALL_DATA) "./data/icons/asus_notif_orange.png" "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_orange.png"
	$(INSTALL_DATA) "./data/icons/asus_notif_white.png" "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_white.png"

	$(INSTALL_DATA) "./data/icons/scalable/gpu-compute.svg" "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/gpu-compute.svg"
	$(INSTALL_DATA) "./data/icons/scalable/gpu-hybrid.svg" "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/gpu-hybrid.svg"
	$(INSTALL_DATA) "./data/icons/scalable/gpu-integrated.svg" "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/gpu-integrated.svg"
	$(INSTALL_DATA) "./data/icons/scalable/gpu-nvidia.svg" "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/gpu-nvidia.svg"
	$(INSTALL_DATA) "./data/icons/scalable/gpu-vfio.svg" "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/gpu-vfio.svg"
	$(INSTALL_DATA) "./data/icons/scalable/notification-reboot.svg" "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/notification-reboot.svg"

install-data-asusd:
	$(INSTALL_DATA) "./data/$(BIN_D).rules" "$(DESTDIR)$(libdir)/udev/rules.d/99-$(BIN_D).rules"
	$(INSTALL_DATA) "./rog-aura/data/$(LEDCFG)" "$(DESTDIR)$(datarootdir)/asusd/$(LEDCFG)"
	$(INSTALL_DATA) "./data/$(BIN_D).conf" "$(DESTDIR)$(datarootdir)/dbus-1/system.d/$(BIN_D).conf"

	$(INSTALL_DATA) "./data/$(BIN_D).service" "$(DESTDIR)$(libdir)/systemd/system/$(BIN_D).service"
	$(INSTALL_DATA) "./data/$(BIN_S).service" "$(DESTDIR)$(libdir)/systemd/system/$(BIN_S).service"

	cd rog-anime/data && find "./anime" -type f -exec $(INSTALL_DATA) "{}" "$(DESTDIR_REALPATH)$(datarootdir)/asusd/{}" \;

.PHONY: install-data-asusd install-data-asusd_user

install-data: install-data-asusd install-data-asusd_user install-data-rog_gui

install: install-program install-data
	$(INSTALL_DATA) "./LICENSE" "$(DESTDIR)$(datarootdir)/asusctl/LICENSE"

uninstall:
	rm -f "$(DESTDIR)$(bindir)/$(BIN_ROG)"
	rm -f "$(DESTDIR)$(datarootdir)/applications/$(BIN_ROG).desktop"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/$(BIN_ROG).png"
	rm -f "$(DESTDIR)$(datarootdir)/metainfo/$(APP_ID).metainfo.xml"

	rm -f "$(DESTDIR)$(bindir)/$(BIN_C)"
	rm -f "$(DESTDIR)$(bindir)/$(BIN_D)"
	rm -f "$(DESTDIR)$(bindir)/$(BIN_S)"
	rm -f "$(DESTDIR)$(bindir)/$(BIN_U)"
	rm -f "$(DESTDIR)$(libdir)/udev/rules.d/99-$(BIN_D).rules"
	rm -f "$(DESTDIR)$(datarootdir)/asusd/$(LEDCFG)"
	rm -f "$(DESTDIR)$(datarootdir)/dbus-1/system.d/$(BIN_D).conf"
	rm -f "$(DESTDIR)$(libdir)/systemd/system/$(BIN_D).service"
	rm -f "$(DESTDIR)$(libdir)/systemd/system/$(BIN_S).service"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_yellow.png"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_green.png"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_blue.png"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_red.png"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_orange.png"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/512x512/apps/asus_notif_white.png"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/gpu-compute.svg"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/gpu-hybrid.svg"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/gpu-integrated.svg"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/gpu-nvidia.svg"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/gpu-vfio.svg"
	rm -f "$(DESTDIR)$(datarootdir)/icons/hicolor/scalable/status/notification-reboot.svg"
	rm -rf "$(DESTDIR)$(datarootdir)/asusd"
	rm -rf "$(DESTDIR)$(datarootdir)/asusctl"
	rm -rf "$(DESTDIR)$(datarootdir)/rog-gui"

update:
	cargo update

vendor:
	mkdir -p .cargo
	cargo vendor | head -n -1 > .cargo/config
	echo 'directory = "vendor"' >> .cargo/config
	mv .cargo/config ./cargo-config
	rm -rf .cargo
	rm -rf vendor
	# Ensure cargo-vendor-filterer is installed (CI installs it already)
	command -v cargo-vendor-filterer >/dev/null 2>&1 || cargo install --locked cargo-vendor-filterer
	cargo vendor-filterer --all-features --platform x86_64-unknown-linux-gnu vendor
	tar pcfJ vendor_asusctl_$(VERSION).tar.xz vendor
	rm -rf vendor

translate:
	find -name \*.slint | xargs slint-tr-extractor -o rog-control-center/translations/en/rog-control-center.po

build:
ifeq ($(VENDORED),1)
	cargo vendor
	@echo "version = $(VERSION)"
	tar pxf vendor_asusctl_$(VERSION).tar.xz
endif
	cargo build $(ARGS)
ifeq ($(STRIP_BINARIES),1)
	strip -s ./target/$(TARGET)/$(BIN_C)
	strip -s ./target/$(TARGET)/$(BIN_D)
	strip -s ./target/$(TARGET)/$(BIN_S)
	strip -s ./target/$(TARGET)/$(BIN_U)
	strip -s ./target/$(TARGET)/$(BIN_ROG)
endif


.PHONY: all clean distclean install uninstall update build bindings
