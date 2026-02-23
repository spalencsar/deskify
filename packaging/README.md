## AUR package template (manual + CI)

This folder contains a minimal `PKGBUILD` template for a future AUR package:

- `deskify-bin` (prebuilt GitHub Releases binary)

Notes:

- Update `_tag` and `pkgver` when you publish a new release tag.
- Update `sha256sums` to match the release asset + LICENSE for that tag.

### Local build test

```bash
cd packaging
makepkg -sf
```

To compute sha256 sums for a tag:

```bash
TAG="v0.1.0-alpha.7"
curl -fL -o /tmp/deskify-linux-x86_64 "https://github.com/spalencsar/deskify/releases/download/${TAG}/deskify-linux-x86_64"
curl -fL -o /tmp/LICENSE "https://raw.githubusercontent.com/spalencsar/deskify/${TAG}/LICENSE"
sha256sum /tmp/deskify-linux-x86_64 /tmp/LICENSE
```

### Manual AUR publish (first time)

1. Create an AUR account and submit the `deskify-bin` package once via the AUR web UI.
2. Configure SSH for AUR and test connectivity.
3. Clone the AUR repo, copy `PKGBUILD` and `.SRCINFO`, then push.

Example:

```bash
ssh-keygen -t ed25519 -f ~/.ssh/aur -C "deskify-aur"
# Add ~/.ssh/aur.pub to: aur.archlinux.org -> My Account -> SSH Keys

cat >> ~/.ssh/config <<'EOF'
Host aur.archlinux.org
  IdentityFile ~/.ssh/aur
  User aur
EOF

ssh -T aur@aur.archlinux.org

git clone ssh://aur@aur.archlinux.org/deskify-bin.git
cd deskify-bin

cp /path/to/deskify/packaging/PKGBUILD .
makepkg --printsrcinfo > .SRCINFO

git add PKGBUILD .SRCINFO
git commit -m "Initial release: deskify-bin"
git push
```

### CI publish on tags

This repo includes a workflow that publishes `deskify-bin` on tag pushes (`v*`):

- `.github/workflows/aur-publish.yml`

Required GitHub secrets:

- `AUR_SSH_PRIVATE_KEY` (private key content for the AUR account)
- `AUR_USERNAME` (optional, used for git commits)
- `AUR_EMAIL` (optional, used for git commits)
