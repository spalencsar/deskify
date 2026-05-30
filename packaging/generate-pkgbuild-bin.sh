#!/usr/bin/env bash
set -euo pipefail

TAG="${1:-}"
if [[ -z "${TAG}" ]]; then
  TAG="$(git describe --tags --abbrev=0)"
fi

if [[ "${TAG}" != v* ]]; then
  echo "Error: expected tag like v0.1.0-alpha.7, got: ${TAG}" >&2
  exit 1
fi

VERSION="${TAG#v}"

# Arch PKGBUILD pkgver must not contain '-'. Use dots instead.
PKGVER="${VERSION//-/.}"

ASSET_URL="https://github.com/spalencsar/deskify/releases/download/${TAG}/deskify-linux-x86_64"
LICENSE_URL="https://raw.githubusercontent.com/spalencsar/deskify/${TAG}/LICENSE"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

asset_path="${tmp_dir}/deskify-linux-x86_64"
license_path="${tmp_dir}/LICENSE"

wait_seconds="${WAIT_FOR_ASSET_SECONDS:-300}"
interval_seconds="${WAIT_FOR_ASSET_INTERVAL_SECONDS:-5}"
deadline=$((SECONDS + wait_seconds))

echo "Waiting for release asset to appear (timeout: ${wait_seconds}s)..."
while ! curl -fsI "${ASSET_URL}" >/dev/null 2>&1; do
  if (( SECONDS >= deadline )); then
    echo "Error: release asset not available yet: ${ASSET_URL}" >&2
    echo "Tip: Warte bis der Release-Workflow fertig ist, oder starte 'Publish to AUR' manuell mit diesem Tag (Actions → Publish to AUR)." >&2
    exit 1
  fi
  sleep "${interval_seconds}"
done

# GitHub release assets can return 200/302 on HEAD slightly before the body is fully available
# (CDN propagation, S3 eventual consistency, etc.). Give it a bit more time.
stabilize_seconds="${WAIT_FOR_ASSET_STABILIZE_SECONDS:-15}"
echo "Asset HEAD succeeded. Waiting ${stabilize_seconds}s for propagation..."
sleep "${stabilize_seconds}"

# Download the binary with retries + size validation.
# The Linux x86_64 binary is currently ~13-14 MiB; require at least 5 MiB to catch truncations.
min_size_bytes=$((5 * 1024 * 1024))
download_attempts=3
asset_ok=false

for attempt in $(seq 1 "${download_attempts}"); do
  echo "Downloading binary (attempt ${attempt}/${download_attempts})..."
  if curl -fL --retry 2 --retry-delay 3 -o "${asset_path}" "${ASSET_URL}"; then
    size=$(stat -c%s "${asset_path}" 2>/dev/null || wc -c < "${asset_path}")
    if (( size >= min_size_bytes )); then
      asset_ok=true
      echo "Downloaded binary: ${size} bytes (OK)"
      break
    else
      echo "WARNING: downloaded file too small (${size} bytes < ${min_size_bytes}). Retrying..." >&2
    fi
  else
    echo "WARNING: curl failed on attempt ${attempt}" >&2
  fi
  sleep 8
done

if [[ "${asset_ok}" != true ]]; then
  echo "Error: failed to obtain a valid release asset after ${download_attempts} attempts." >&2
  echo "Last size: $(stat -c%s "${asset_path}" 2>/dev/null || echo 'unknown')" >&2
  exit 1
fi

echo "Downloading LICENSE..."
curl -fL -o "${license_path}" "${LICENSE_URL}"

ASSET_SHA256="$(sha256sum "${asset_path}" | awk '{print $1}')"
LICENSE_SHA256="$(sha256sum "${license_path}" | awk '{print $1}')"

cat > packaging/PKGBUILD <<EOF
pkgname=deskify-bin
pkgver=${PKGVER}
pkgrel=1
pkgdesc="Turn websites into Linux desktop apps (prebuilt binary package)"
arch=('x86_64')
url="https://github.com/spalencsar/deskify"
license=('MIT')
depends=('glibc')
optdepends=('chromium: for --backend chromium')
provides=('deskify')
conflicts=('deskify')

_tag="${TAG}"
source=("deskify::https://github.com/spalencsar/deskify/releases/download/\${_tag}/deskify-linux-x86_64"
        "LICENSE::https://raw.githubusercontent.com/spalencsar/deskify/\${_tag}/LICENSE")

sha256sums=('${ASSET_SHA256}'
            '${LICENSE_SHA256}')

package() {
  install -Dm755 "\${srcdir}/deskify" "\${pkgdir}/usr/bin/deskify"
  install -Dm644 "\${srcdir}/LICENSE" "\${pkgdir}/usr/share/licenses/\${pkgname}/LICENSE"
}
EOF

echo "Wrote packaging/PKGBUILD for ${TAG}"
echo "  deskify sha256: ${ASSET_SHA256}"
echo "  LICENSE sha256: ${LICENSE_SHA256}"
