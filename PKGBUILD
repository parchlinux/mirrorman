# Maintainer: Parch GNU/Linux Team

pkgname=mirrorman
pkgver=0.5.0.beta1
pkgrel=1
pkgdesc="Pacman mirror and repository manager for Parch Linux"
arch=('x86_64')
url="https://github.com/parchlinux/mirrorman"
license=('GPL3')
depends=(
  'gtk4'
  'libadwaita'
  'glib2'
  'polkit'
  'pacman'
  'gettext'
)
makedepends=('cargo' 'git')
source=(
  "git+https://github.com/parchlinux/mirrorman.git#branch=dev"
)
sha256sums=('SKIP')

prepare() {
  cd "$srcdir/mirrorman"
}

build() {
  cd "$srcdir/mirrorman"
  cargo build --release
  msgfmt po/fa.po -o locale/fa/LC_MESSAGES/mirrorman.mo
}

package() {
  cd "$srcdir/mirrorman"

  install -Dm755 "target/release/mirrorman" \
    "$pkgdir/usr/bin/mirrorman"

  install -Dm755 "target/release/mirrorman-helper" \
    "$pkgdir/usr/bin/mirrorman-helper"

  install -Dm755 "target/release/mirrorman-cli" \
    "$pkgdir/usr/bin/mirrorman-cli"

  install -Dm644 "data/com.parchlinux.mirrorman.desktop" \
    "$pkgdir/usr/share/applications/com.parchlinux.mirrorman.desktop"

  install -Dm644 "data/com.parchlinux.mirrorman.svg" \
    "$pkgdir/usr/share/icons/hicolor/scalable/apps/com.parchlinux.mirrorman.svg"

  install -Dm644 "data/com.parchlinux.mirrorman.policy" \
    "$pkgdir/usr/share/polkit-1/actions/com.parchlinux.mirrorman.policy"

  install -Dm644 "data/com.parchlinux.mirrorman.Helper.service" \
    "$pkgdir/usr/share/dbus-1/system-services/com.parchlinux.mirrorman.Helper.service"

  install -Dm644 "data/com.parchlinux.mirrorman-helper.conf" \
    "$pkgdir/usr/share/dbus-1/system.d/com.parchlinux.mirrorman-helper.conf"

  install -Dm644 "data/mirrorman-helper.service" \
    "$pkgdir/usr/lib/systemd/system/mirrorman-helper.service"

  install -Dm644 "data/mirrorman-refresh.service" \
    "$pkgdir/usr/lib/systemd/user/mirrorman-refresh.service"

  install -Dm644 "data/mirrorman-refresh.timer" \
    "$pkgdir/usr/lib/systemd/user/mirrorman-refresh.timer"

  install -Dm644 "locale/fa/LC_MESSAGES/mirrorman.mo" \
    "$pkgdir/usr/share/locale/fa/LC_MESSAGES/mirrorman.mo"
}
