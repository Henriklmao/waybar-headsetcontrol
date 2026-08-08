# Maintainer: Henrik Bernhardt <57109108+Henriklmao@users.noreply.github.com >
pkgname=wb-headsetcontrol-git
pkgver=r1.g
pkgrel=1
pkgdesc="Waybar and Quickshell integration for HeadsetControl with an interactive TUI"
arch=('x86_64')
url="https://github.com/Henriklmao/waybar-headsetcontrol"
license=('GPL3')
depends=('headsetcontrol')
makedepends=('cargo' 'rust' 'git')
install=wb-headsetcontrol.install
source=('waybar-headsetcontrol::git+https://github.com/Henriklmao/waybar-headsetcontrol.git')
sha256sums=('SKIP')

pkgver() {
    cd "$srcdir/waybar-headsetcontrol" || exit
    local release=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
    local revision=$(git rev-list --count HEAD)
    printf "%s.r%s" "$release" "$revision"
}

build() {
    cd "$srcdir/waybar-headsetcontrol" || return
    cargo build --release
}

package() {
    cd "$srcdir/waybar-headsetcontrol" || return
    install -Dm 755 "target/release/headset-tui" "$pkgdir/usr/bin/headset-tui"
}
