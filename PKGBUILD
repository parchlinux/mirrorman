# Maintainer: Parch GNU/Linux Team

pkgname=mirrorman
pkgver=0.6.0
pkgrel=1
pkgdesc="Pacman mirror and repository manager for Parch Linux"
arch=('x86_64')
url="https://github.com/parchlinux/mirrorman"
license=('GPL-3.0-or-later')
depends=(
  'gtk4'
  'libadwaita'
  'glib2'
  'polkit'
  'pacman'
  'gettext'
)
optdepends=(
  'pacman-contrib: cache cleaning via paccache'
  'diffutils: mirrorlist diff preview'
)
makedepends=('cargo' 'git' 'gettext')
install=mirrorman.install
source=(
  "git+https://github.com/parchlinux/mirrorman.git#tag=v0.6.0"
)
sha256sums=('SKIP')

build() {
  cd "$srcdir/mirrorman"
  cargo build --workspace --release
  msgfmt assets/po/fa.po -o assets/locale/fa/LC_MESSAGES/mirrorman.mo
  gzip -9 -n -c assets/man/mirrorman-cli.1 > assets/man/mirrorman-cli.1.gz
}

package() {
  cd "$srcdir/mirrorman"

  install -Dm755 "target/release/mirrorman" \
    "$pkgdir/usr/bin/mirrorman"

  install -Dm755 "target/release/mirrorman-helper" \
    "$pkgdir/usr/bin/mirrorman-helper"

  install -Dm755 "target/release/mirrorman-cli" \
    "$pkgdir/usr/bin/mirrorman-cli"

  install -Dm755 "target/release/mirrorman-dev" \
    "$pkgdir/usr/bin/mirrorman-dev"

  install -Dm644 "assets/man/mirrorman-cli.1.gz" \
    "$pkgdir/usr/share/man/man1/mirrorman-cli.1.gz"

  install -Dm644 "assets/data/com.parchlinux.mirrorman.desktop" \
    "$pkgdir/usr/share/applications/com.parchlinux.mirrorman.desktop"

  install -Dm644 "assets/data/com.parchlinux.mirrorman.dev.desktop" \
    "$pkgdir/usr/share/applications/com.parchlinux.mirrorman.dev.desktop"

  install -Dm644 "assets/data/com.parchlinux.mirrorman.svg" \
    "$pkgdir/usr/share/icons/hicolor/scalable/apps/com.parchlinux.mirrorman.svg"

  install -Dm644 "assets/data/com.parchlinux.mirrorman.dev.svg" \
    "$pkgdir/usr/share/icons/hicolor/scalable/apps/com.parchlinux.mirrorman.dev.svg"

  install -Dm644 "assets/data/com.parchlinux.mirrorman.policy" \
    "$pkgdir/usr/share/polkit-1/actions/com.parchlinux.mirrorman.policy"

  install -Dm644 "assets/data/com.parchlinux.mirrorman.Helper.service" \
    "$pkgdir/usr/share/dbus-1/system-services/com.parchlinux.mirrorman.Helper.service"

  install -Dm644 "assets/data/com.parchlinux.mirrorman-helper.conf" \
    "$pkgdir/usr/share/dbus-1/system.d/com.parchlinux.mirrorman-helper.conf"

  install -Dm644 "assets/data/mirrorman-helper.service" \
    "$pkgdir/usr/lib/systemd/system/mirrorman-helper.service"

  install -Dm644 "assets/data/mirrorman-refresh.service" \
    "$pkgdir/usr/lib/systemd/user/mirrorman-refresh.service"

  install -Dm644 "assets/data/mirrorman-refresh.timer" \
    "$pkgdir/usr/lib/systemd/user/mirrorman-refresh.timer"

  install -Dm644 "assets/locale/fa/LC_MESSAGES/mirrorman.mo" \
    "$pkgdir/usr/share/locale/fa/LC_MESSAGES/mirrorman.mo"
}
