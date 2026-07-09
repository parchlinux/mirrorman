# Maintainer: Parch GNU/Linux Team

pkgname=mirrorman
pkgver=0.3.0
pkgrel=1
pkgdesc="Pacman mirror and repository manager"
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
  "mirrorman-$pkgver.tar.gz::https://github.com/parchlinux/mirrorman/archive/v$pkgver.tar.gz"
)
sha256sums=('SKIP')

prepare() {
  cd "$srcdir/mirrorman-$pkgver"
}

build() {
  cd "$srcdir/mirrorman-$pkgver"
  cargo build --release --frozen
  msgfmt po/fa.po -o locale/fa/LC_MESSAGES/mirrorman.mo
}

package() {
  cd "$srcdir/mirrorman-$pkgver"

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
