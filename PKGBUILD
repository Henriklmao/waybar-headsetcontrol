# Maintainer: Your Name <you@example.com>
pkgname=wb-headsetcontrol
pkgver=0.1.0
pkgrel=1
pkgdesc="Waybar integration for HeadsetControl - display battery and control sidetone"
arch=('x86_64')
url="https://github.com/Henriklmao/waybar-headsetcontrol"
license=('MIT')
depends=('headsetcontrol' 'gtk3' 'alacritty')
makedepends=('cargo' 'rust')
source=("$pkgname-$pkgver.tar.gz::https://github.com/Henriklmao/waybar-headsetcontrol/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$srcdir/waybar-headsetcontrol-$pkgver" || return
    cargo build --release
}

package() {
    cd "$srcdir/waybar-headsetcontrol-$pkgver" || return
    install -Dm 755 "target/release/wb-headset" "$pkgdir/usr/bin/wb-headsetcontrol"
    install -Dm 755 "install-waybar-config.sh" "$pkgdir/usr/share/wb-headsetcontrol/install-waybar-config.sh"
}

post_install() {
    echo "=== wb-headsetcontrol installed ==="
    echo ""
    echo "Configuring Waybar..."
    bash /usr/share/wb-headsetcontrol/install-waybar-config.sh
    echo ""
    echo "✅ Installation complete!"
    echo ""
    echo "Default configuration created at ~/.config/wb-headsetcontrol/config.toml"
    echo "Press 'c' in the TUI to configure keybindings and default sidetone."
    echo ""
    echo "Usage: wb-headsetcontrol"
}

post_upgrade() {
    post_install
}
