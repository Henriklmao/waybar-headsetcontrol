#!/usr/bin/env bash
# Generate the AUR PKGBUILD for the pre-built headset-tui release binary.
#
# Usage: generate-pkgbuild-bin.bash <tag>
# The generated PKGBUILD is written next to this script.
set -euo pipefail

cd "$(dirname "$0")"
source ./aur-pkgbuild-common.bash

tag="${1:-}"
pkgver=$(aur_tag_pkgver "$tag")
url=$(aur_repo_url)
# This is the asset name produced by .github/workflows/rust.yml.
asset="headset-tui-${tag}-x86_64-linux-gnu"

cat >PKGBUILD <<EOF
# Maintainer: Henrik Bernhardt <57109108+Henriklmao@users.noreply.github.com>
pkgname=headset-tui-bin
pkgver=${pkgver}
pkgrel=1
pkgdesc='${aur_description} (pre-built binary)'
arch=('x86_64')
url='${url}'
license=('GPL3')
depends=('headsetcontrol')
provides=('headset-tui')
conflicts=('headset-tui' 'headset-tui-git')
source=("\$pkgname-\$pkgver::${url}/releases/download/${tag}/${asset}")
sha256sums=('SKIP')

package() {
    install -Dm755 "\$srcdir/\$pkgname-\$pkgver" "\$pkgdir/usr/bin/headset-tui"
}
EOF

echo "wrote PKGBUILD for headset-tui-bin ${pkgver} (asset ${asset})"
