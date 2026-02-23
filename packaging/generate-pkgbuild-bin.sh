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

curl -fL -o "${asset_path}" "${ASSET_URL}"
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
