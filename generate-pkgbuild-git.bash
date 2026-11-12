#!/usr/bin/env bash
# Generate the AUR PKGBUILD for the headset-tui-git VCS package.
#
# Usage: generate-pkgbuild-git.bash
# The generated PKGBUILD is written next to this script. headset-tui.install
# must exist there as well; the publish workflow uploads it as an AUR asset.
set -euo pipefail

cd "$(dirname "$0")"
source ./aur-pkgbuild-common.bash

for file in Cargo.toml headset-tui.install; do
  if [[ ! -f "$file" ]]; then
    echo "missing $file" >&2
    exit 1
  fi
done

url=$(aur_repo_url)
version=$(aur_cargo_version Cargo.toml)
pkgver=$(aur_git_pkgver "$version")

cat >PKGBUILD <<EOF
# Maintainer: Henrik Bernhardt <57109108+Henriklmao@users.noreply.github.com>
pkgname=headset-tui-git
pkgver=${pkgver}
pkgrel=1
pkgdesc='${aur_description} (git version)'
arch=('x86_64')
url='${url}'
license=('GPL3')
depends=('headsetcontrol' 'git')
provides=('headset-tui')
conflicts=('headset-tui' 'headset-tui-bin')
makedepends=('cargo' 'rust' 'git')
install=headset-tui.install
source=('headset-tui::git+${url}.git')
sha256sums=('SKIP')

pkgver() {
    cd "\$srcdir/headset-tui"
    local version
    version=\$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
    printf '%s.r%s.g%s' "\$version" "\$(git rev-list --count HEAD)" "\$(git rev-parse --short HEAD)"
}

build() {
    cd "\$srcdir/headset-tui"
    export CARGO_TARGET_DIR="\$srcdir/target"
    cargo build --release --locked
}

package() {
    cd "\$srcdir/headset-tui"
    install -Dm755 "\$srcdir/target/release/headset-tui" "\$pkgdir/usr/bin/headset-tui"
    install -Dm755 "install-waybar-config.sh" "\$pkgdir/usr/share/headset-tui/install-waybar-config.sh"
}
EOF

echo "wrote PKGBUILD for headset-tui-git ${pkgver} (${url})"
