# Maintainer: Parch GNU/Linux Team <team@parchlinux.com>

pkgname=mirrorman
pkgver=0.2
pkgrel=1
pkgdesc="GUI tool for managing Arch Linux mirrors and repositories"
arch=('any')
url="https://parchlinux.com"
license=('GPL3')
depends=('python-gobject' 'libadwaita' 'polkit')
optdepends=('python')
source=("git+https://git.xerocloud.ir/sohrab/mirrorman.git")
sha256sums=('SKIP')

package() {
  cd "$srcdir/mirrorman"
  install -d "$pkgdir/usr/bin"
  install -d "$pkgdir/usr/share/mirrorman"
  install -d "$pkgdir/usr/share/applications"
  install -d "$pkgdir/usr/share/locale/fa/LC_MESSAGES"
  install -d "$pkgdir/usr/share/locale/en/LC_MESSAGES"
  
  install src/*.py "$pkgdir/usr/share/mirrorman/"
  install -m755 src/main.py "$pkgdir/usr/share/mirrorman/"
  install -m755 mirrorman.in "$pkgdir/usr/bin/mirrorman"
  install -m644 mirrorman.desktop "$pkgdir/usr/share/applications/"

  if [ -f "locale/fa/LC_MESSAGES/mirrorman.mo" ]; then
    install -m644 locale/fa/LC_MESSAGES/mirrorman.mo "$pkgdir/usr/share/locale/fa/LC_MESSAGES/"
  fi
  if [ -f "locale/en/LC_MESSAGES/mirrorman.mo" ]; then
    install -m644 locale/en/LC_MESSAGES/mirrorman.mo "$pkgdir/usr/share/locale/en/LC_MESSAGES/"
  fi
}
