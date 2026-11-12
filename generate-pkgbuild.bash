#!/usr/bin/env bash
# Generate the AUR PKGBUILD for a source build of headset-tui.
#
# Usage: generate-pkgbuild.bash <tag>
# The generated PKGBUILD is written next to this script.
set -euo pipefail

cd "$(dirname "$0")"
source ./aur-pkgbuild-common.bash

tag="${1:-}"
pkgver=$(aur_tag_pkgver "$tag")
url=$(aur_repo_url)

cat >PKGBUILD <<EOF
# Maintainer: Henrik Bernhardt <57109108+Henriklmao@users.noreply.github.com>
pkgname=headset-tui
pkgver=${pkgver}
pkgrel=1
pkgdesc='${aur_description}'
arch=('x86_64')
url='${url}'
license=('GPL3')
depends=('headsetcontrol')
makedepends=('git' 'rust')
conflicts=('headset-tui-bin' 'headset-tui-git')
source=("\$pkgname::git+${url}.git#tag=${tag}")
sha256sums=('SKIP')

build() {
    cd "\$srcdir/\$pkgname"
    export CARGO_TARGET_DIR="\$srcdir/target"
    cargo build --release --locked
}

package() {
    cd "\$srcdir/\$pkgname"
    install -Dm755 "\$srcdir/target/release/headset-tui" "\$pkgdir/usr/bin/headset-tui"
    install -Dm755 "install-waybar-config.sh" "\$pkgdir/usr/share/headset-tui/install-waybar-config.sh"
}
EOF

echo "wrote PKGBUILD for headset-tui ${pkgver} (tag ${tag})"
