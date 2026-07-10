# Maintainer: Parch GNU/Linux Team

pkgname=mirrorman
pkgver=0.4.0
pkgrel=1
pkgdesc="Pacman mirror and repository manager for Parch Linux"
arch=('x86_64')
url="https://github.com/parchlinux/mirrorman"
license=('GPL3')
depends=(
  'gtk4'
  'libadwaita'
  'glib2'
  'libsoup3'
  'polkit'
  'pacman'
  'gettext'
)
makedepends=('cargo')
source=(
  "git+https://github.com/parchlinux/mirrorman.git#tag=v$pkgver"
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

  install -Dm644 "data/com.parchlinux.mirrorman.desktop" \
    "$pkgdir/usr/share/applications/com.parchlinux.mirrorman.desktop"

  install -Dm644 "data/com.parchlinux.mirrorman.svg" \
    "$pkgdir/usr/share/icons/hicolor/scalable/apps/com.parchlinux.mirrorman.svg"

  install -Dm644 "data/com.parchlinux.mirrorman.policy" \
    "$pkgdir/usr/share/polkit-1/actions/com.parchlinux.mirrorman.policy"

  install -Dm644 "locale/fa/LC_MESSAGES/mirrorman.mo" \
    "$pkgdir/usr/share/locale/fa/LC_MESSAGES/mirrorman.mo"
}
