# Packaging

On every published GitHub Release, [`.github/workflows/release.yml`](.github/workflows/release.yml) builds Arch Linux packages (`asusctl` and `rog-control-center`) from [`distro-packaging/PKGBUILD`](distro-packaging/PKGBUILD) and attaches them as release assets.

## Trigger

The workflow runs on `release: types: [published]` and declares `permissions: contents: write`, which the asset-upload step needs.

## What it does

- Runs in an `archlinux/archlinux:base-devel` container on `ubuntu-latest`.
- Installs makedepends (rust, llvm, clang, at-spi2-core, cairo, gtk3) and creates a non-root `builder` user, since `makepkg` refuses to run as root.
- Determines `pkgrel`: computes `pkgver` from `git describe --long --tags`, fetches the existing `ogc.db.tar.gz` from `BUCKET_PUBLIC_URL`, and bumps `pkgrel` to `max(existing) + 1` if the same `pkgver` is already published for either package; otherwise defaults to `1`.
- Builds via `makepkg -s --noconfirm` with `CI_BUILD=1`, `_gitref=<tag commit>`, and `pkgrel=$PKGREL`.
- Uploads `*.pkg.tar.zst` to the release using `softprops/action-gh-release@v3`.

## Artifacts

Each run produces and attaches two packages to the GitHub release that triggered it:

- `asusctl-<pkgver>-<pkgrel>-x86_64.pkg.tar.zst`
- `rog-control-center-<pkgver>-<pkgrel>-x86_64.pkg.tar.zst`

## Required configuration

| Name | Kind | Required | Purpose |
|------|------|----------|---------|
| `BUCKET_PUBLIC_URL` | Repository variable (`vars.*`) | No | Public URL hosting `ogc.db.tar.gz`. Used to detect the existing `pkgrel` for re-releases. If unset or unreachable, `pkgrel` defaults to `1` and the workflow still succeeds. |
| `GITHUB_TOKEN` | Auto-provided secret | Yes (automatic) | Used by `softprops/action-gh-release` to attach assets. No manual setup; the workflow's `permissions: contents: write` grants the needed scope. |

No other secrets or variables are required by this workflow. In particular, `PGP_SIGNING_KEY`, `BUCKET_ACCESS_KEY`, `BUCKET_SECRET_KEY`, `BUCKET_ENDPOINT`, and `S3_BUCKET` are **not** used here — they belong to the separate central packaging repo (see `PACKAGING.md.bak` for that plan).

## Reproducing locally

```sh
cd distro-packaging && makepkg -s
```

CI uses the same `PKGBUILD` with `CI_BUILD=1` and an explicit `_gitref` pinned to the release tag's commit.
