# Packaging

On every published GitHub Release, [`.github/workflows/release.yml`](.github/workflows/release.yml) builds Arch Linux packages (`asusctl` and `rog-control-center`) from [`distro-packaging/PKGBUILD`](distro-packaging/PKGBUILD) and attaches them as release assets.

## Trigger

The workflow runs on `release: types: [published]` events as well as non-release triggers such as `workflow_dispatch`. For release events, packages are attached directly to the GitHub Release (requiring `permissions: contents: write`); for non-release triggers, packages are uploaded as build artifacts.

## What it does

- Runs in an `archlinux/archlinux:base-devel` container on `ubuntu-latest`.
- Installs makedepends (rust, llvm, clang, at-spi2-core, cairo, gtk3) and creates a non-root `builder` user, since `makepkg` refuses to run as root.
- Determines `pkgrel`: computes `pkgver` from `git describe --long --tags`, fetches the existing `ogc.db.tar.gz` from `BUCKET_PUBLIC_URL`, and bumps `pkgrel` to `max(existing) + 1` if the same `pkgver` is already published for either package; otherwise defaults to `1`.
- Builds via `makepkg -s --noconfirm` with `CI_BUILD=1`, `_gitref=<tag commit>`, and `pkgrel=$PKGREL`.
- Uploads `*.pkg.tar.zst` to the release using `softprops/action-gh-release@v3`. For non-release workflow triggers (such as `workflow_dispatch`), built package files are renamed with run metadata prior to uploading as build artifacts.

## Artifacts

For **release events** (`release: types: [published]`), the workflow produces and attaches two packages to the GitHub Release:

- `asusctl-<pkgver>-<pkgrel>-x86_64.pkg.tar.zst`
- `rog-control-center-<pkgver>-<pkgrel>-x86_64.pkg.tar.zst`

For **non-release events** (such as `workflow_dispatch`), the workflow uploads uniquely named packages as build artifacts:

- `asusctl-<pkgver>-<pkgrel>-x86_64-run<run_number>.<run_attempt>.pkg.tar.zst`
- `rog-control-center-<pkgver>-<pkgrel>-x86_64-run<run_number>.<run_attempt>.pkg.tar.zst`

## Required configuration

| Name | Kind | Required | Purpose |
|------|------|----------|---------|
| `BUCKET_PUBLIC_URL` | Repository variable (`vars.*`) | No | Public URL hosting `ogc.db.tar.gz`. Used to detect the existing `pkgrel` for re-releases. If unset or unreachable, `pkgrel` defaults to `1` and the workflow still succeeds. |
| `GITHUB_TOKEN` | Auto-provided secret | Yes (automatic) | Used by `softprops/action-gh-release` to attach assets. No manual setup; the workflow's `permissions: contents: write` grants the needed scope. |

No other secrets or variables are required by this workflow.

## Reproducing locally

```sh
cd distro-packaging && makepkg -s
```

CI uses the same `PKGBUILD` with `CI_BUILD=1` and an explicit `_gitref` pinned to the release tag's commit.
