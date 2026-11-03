# Local package built from this checkout; no upstream release URL is required.
pkgname=plex-tray
pkgver=0.1.0
pkgrel=3
pkgdesc='Desktop tray controls for the system Plex Media Server service'
arch=('x86_64')
url='https://github.com/bpierre/plex-tray'
license=('MIT')
depends=('glibc' 'gcc-libs' 'dbus' 'systemd' 'polkit' 'plex-media-server' 'xdg-utils' 'hicolor-icon-theme')
makedepends=('cargo')
checkdepends=('desktop-file-utils')
optdepends=('libnotify: desktop notifications for service errors')
options=('!debug')

# makepkg normally uses ./src, which is the Rust source directory here.
# Keep its build/clean operations away from the source checkout.
if [[ ${BUILDDIR:-$startdir} -ef "$startdir" ]]; then
    BUILDDIR="$startdir/.makepkg"
fi

prepare() {
    local project="$srcdir/plex-tray"
    install -Dm644 "$startdir/src/main.rs" "$project/src/main.rs"
    install -Dm644 "$startdir/src/settings.rs" "$project/src/settings.rs"
    install -Dm644 "$startdir/tests/cli.rs" "$project/tests/cli.rs"
    install -Dm644 "$startdir/icons/plex-tray.svg" "$project/icons/plex-tray.svg"
    install -Dm644 "$startdir/icons/plex-tray-symbolic.svg" "$project/icons/plex-tray-symbolic.svg"
    local file
    for file in Cargo.toml Cargo.lock LICENSE README.md CONTRIBUTING.md RELEASING.md plex-tray.desktop 50-plex-tray.rules; do
        install -Dm644 "$startdir/$file" "$project/$file"
    done
    cd "$project"
    cargo fetch --locked
}

build() {
    cd "$srcdir/plex-tray"
    cargo build --release --frozen
}

check() {
    cd "$srcdir/plex-tray"
    desktop-file-validate plex-tray.desktop
    cargo test --release --frozen
    ./target/release/plex-tray --help >/dev/null
}

package() {
    cd "$srcdir/plex-tray"
    install -Dm755 target/release/plex-tray "$pkgdir/usr/bin/plex-tray"
    install -Dm644 plex-tray.desktop "$pkgdir/usr/share/applications/plex-tray.desktop"
    install -Dm644 icons/plex-tray.svg "$pkgdir/usr/share/icons/hicolor/scalable/apps/plex-tray.svg"
    install -Dm644 icons/plex-tray-symbolic.svg "$pkgdir/usr/share/icons/hicolor/scalable/apps/plex-tray-symbolic.svg"
    install -Dm644 50-plex-tray.rules "$pkgdir/usr/share/polkit-1/rules.d/50-plex-tray.rules"
    install -Dm644 README.md "$pkgdir/usr/share/doc/plex-tray/README.md"
    install -Dm644 CONTRIBUTING.md "$pkgdir/usr/share/doc/plex-tray/CONTRIBUTING.md"
    install -Dm644 RELEASING.md "$pkgdir/usr/share/doc/plex-tray/RELEASING.md"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/plex-tray/LICENSE"
    ln -s ../../licenses/plex-tray/LICENSE "$pkgdir/usr/share/doc/plex-tray/LICENSE"
}
