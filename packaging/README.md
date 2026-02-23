## AUR package template (manual)

This folder contains a minimal `PKGBUILD` template for a future AUR package:

- `deskify-bin` (prebuilt GitHub Releases binary)

Notes:

- `sha256sums` are currently set to `SKIP` and should be replaced with real checksums before publishing to AUR.
- Update `_tag` and `pkgver` when you publish a new release tag.

Local build test:

```bash
cd packaging
makepkg -sf
```

