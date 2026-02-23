## AUR package template (manual)

This folder contains a minimal `PKGBUILD` template for a future AUR package:

- `deskify-bin` (prebuilt GitHub Releases binary)

Notes:

- Update `_tag` and `pkgver` when you publish a new release tag.
- Update `sha256sums` to match the release asset + LICENSE for that tag.

Local build test:

```bash
cd packaging
makepkg -sf
```

To compute sha256 sums for a tag:

```bash
TAG="v0.1.0-alpha.6"
curl -fL -o /tmp/deskify-linux-x86_64 "https://github.com/spalencsar/deskify/releases/download/${TAG}/deskify-linux-x86_64"
curl -fL -o /tmp/LICENSE "https://raw.githubusercontent.com/spalencsar/deskify/${TAG}/LICENSE"
sha256sum /tmp/deskify-linux-x86_64 /tmp/LICENSE
```
