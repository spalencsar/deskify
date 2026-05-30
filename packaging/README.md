## AUR package template (manual + CI)

This folder contains a minimal `PKGBUILD` template for a future AUR package:

- `deskify-bin` (prebuilt GitHub Releases binary)

Notes:

- Die CI übernimmt das Update von `PKGBUILD` und `.SRCINFO` automatisch bei jedem neuen Release (via `generate-pkgbuild-bin.sh`).
- Manuelle Änderungen an `packaging/PKGBUILD` sind nur noch für lokale Tests oder Notfälle nötig.

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

### Common failure: "sha256sum mismatch" right after tagging

Both the Release workflow and the AUR publish workflow trigger on the same `v*` tag push and run in parallel.
The AUR job can start downloading the release asset before GitHub has fully propagated it (HEAD succeeds but body is truncated or empty).

The `generate-pkgbuild-bin.sh` script now includes extra stabilization delay + size validation + retries to mitigate this.
If it still happens, re-run the "Publish to AUR" workflow manually for that tag via **Actions → Publish to AUR → Run workflow** (or push a dummy annotated tag like `vX.Y.Z-aur`).

Tunable environment variables in the generator script:
- `WAIT_FOR_ASSET_SECONDS` (default 300)
- `WAIT_FOR_ASSET_STABILIZE_SECONDS` (default 15) — extra sleep after HEAD succeeds
- `WAIT_FOR_ASSET_INTERVAL_SECONDS` (default 5)

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

`deskify-bin` wird automatisch veröffentlicht, sobald ein GitHub Release erfolgreich erstellt wurde:

- Der **Release**-Workflow (`.github/workflows/release.yml`) baut das Binary, lädt es hoch **und** triggert danach den `aur-publish`-Workflow per `workflow_dispatch`.
- Das verhindert die frühere Race Condition (parallele Ausführung → manchmal unvollständiges Asset → falsche `sha256sums` auf AUR).

Wichtige Workflows:
- `.github/workflows/release.yml`
- `.github/workflows/aur-publish.yml`

Benötigte GitHub Secrets (für den AUR-Push):
- `AUR_SSH_PRIVATE_KEY`
- `AUR_USERNAME` (optional)
- `AUR_EMAIL` (optional)

Manuelles Neu-Triggern für ein bestehendes Tag:
1. GitHub → Actions → "Publish to AUR"
2. "Run workflow" → Tag (z. B. `v0.1.1-alpha.2`) eingeben
