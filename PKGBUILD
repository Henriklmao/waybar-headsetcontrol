# Maintainer: Henrik Bernhardt <57109108+Henriklmao@users.noreply.github.com >
pkgname=headset-tui-git
pkgver=r1.g
pkgrel=1
pkgdesc="Waybar integration for HeadsetControl - display battery and control sidetone"
arch=('x86_64')
url="https://github.com/Henriklmao/headset-tui"
license=('GPL3')
depends=('headsetcontrol')
makedepends=('cargo' 'rust' 'git')
install=headset-tui.install
source=('headset-tui::git+https://github.com/Henriklmao/headset-tui.git')
sha256sums=('SKIP')

pkgver() {
    cd "$srcdir/headset-tui" || exit
    local release=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
    local revision=$(git rev-list --count HEAD)
    printf "%s.r%s" "$release" "$revision"
}

build() {
    cd "$srcdir/headset-tui" || return
    cargo build --release
}

package() {
    cd "$srcdir/headset-tui" || return
    install -Dm 755 "target/release/headset-tui" "$pkgdir/usr/bin/headset-tui"
    install -Dm 755 "install-waybar-config.sh" "$pkgdir/usr/share/headset-tui/install-waybar-config.sh"
}
