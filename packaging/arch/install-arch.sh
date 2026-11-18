#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
PKGVER="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n1)"
PKGVER="${PKGVER:-0.2.1}"

echo "==> Building OpenMAD-Rust release binary..."
cd "$ROOT_DIR"
cargo build --release --bin openmad

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/PKGBUILD" <<EOF
pkgname=openmad-rust
pkgver=${PKGVER}
pkgrel=1
pkgdesc="100% Native Rust driver and GUI for MAD68 HE / MAD60 HE Hall-effect keyboards"
arch=('x86_64')
url="https://github.com/muriloscolari/OpenMAD-Rust"
license=('GPL-3.0-or-later')
depends=('hidapi' 'gtk3' 'libayatana-appindicator' 'dbus')

package() {
  install -Dm755 "${ROOT_DIR}/target/release/openmad" "\$pkgdir/usr/bin/openmad"
  install -Dm644 "${ROOT_DIR}/packaging/linux/60-openmad.rules" "\$pkgdir/usr/lib/udev/rules.d/60-openmad.rules"
  install -Dm644 "${ROOT_DIR}/packaging/linux/openmad.desktop" "\$pkgdir/usr/share/applications/openmad.desktop"
  install -Dm644 "${ROOT_DIR}/assets/icon.png" "\$pkgdir/usr/share/icons/hicolor/256x256/apps/openmad-rust.png"
  install -Dm644 "${ROOT_DIR}/LICENSE" "\$pkgdir/usr/share/licenses/\$pkgname/LICENSE"
}
EOF

echo "==> Packaging Arch Linux .pkg.tar.zst..."
cd "$WORK_DIR"
makepkg -f --nodeps
mkdir -p "$ROOT_DIR/dist"
cp -v "$WORK_DIR"/openmad-rust-*.pkg.tar.zst "$ROOT_DIR/dist/"
echo "==> Arch package created in dist/"
