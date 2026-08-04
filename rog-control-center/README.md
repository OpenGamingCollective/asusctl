# ROGALOG

## X11 support

X11 is not supported at all, as in I will not help you with X11 issues if there are any due to limited time and it being unmaintained itself. You can however build `rog-control-center` with it enabled `cargo build --features x11`.

### Translations

You can help with translations by following https://slint.dev/releases/1.1.0/docs/slint/src/concepts/translations#translate-the-strings

Begin by copying `rog-control-center/translations/en/rog-control-center.po` to `rog-control-center/translations/<YOUR LOCALE>/rog-control-center.po`, then edit that file.

The binary catalogs are compiled from the `.po` sources at build time, so rebuild to pick up your edits. This needs `msgfmt` from gettext installed.

To test you local translations run `RUST_TRANSLATIONS=1 rog-control-center`.
