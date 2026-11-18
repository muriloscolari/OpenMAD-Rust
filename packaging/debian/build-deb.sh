#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${1:-0.1.0}"
ARCH="amd64"
BIN_PATH="$ROOT_DIR/target/release/openmad"
OUT_DEB="$ROOT_DIR/dist/openmad_${VERSION}_${ARCH}.deb"

if [[ ! -f "$BIN_PATH" ]]; then
  echo "==> Building release binary first..."
  (cd "$ROOT_DIR" && cargo build --release --bin openmad)
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

PKG_ROOT="$WORK_DIR/pkg"
mkdir -p "$PKG_ROOT/DEBIAN"
mkdir -p "$PKG_ROOT/usr/bin"
mkdir -p "$PKG_ROOT/lib/udev/rules.d"
mkdir -p "$PKG_ROOT/usr/share/applications"
mkdir -p "$PKG_ROOT/usr/share/icons/hicolor/256x256/apps"

install -m 755 "$BIN_PATH" "$PKG_ROOT/usr/bin/openmad"
install -m 644 "$ROOT_DIR/packaging/linux/60-openmad.rules" "$PKG_ROOT/lib/udev/rules.d/60-openmad.rules"
install -m 644 "$ROOT_DIR/packaging/linux/openmad.desktop" "$PKG_ROOT/usr/share/applications/openmad.desktop"
install -m 644 "$ROOT_DIR/assets/icon.png" "$PKG_ROOT/usr/share/icons/hicolor/256x256/apps/openmad-rust.png"

INSTALLED_SIZE="$(du -sk "$PKG_ROOT/usr" "$PKG_ROOT/lib" | awk '{s+=$1} END {print s}')"

cat > "$PKG_ROOT/DEBIAN/control" <<EOF
Package: openmad
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Installed-Size: ${INSTALLED_SIZE}
Depends: libhidapi-hidraw0, libgtk-3-0, libayatana-appindicator3-1, libdbus-1-3
Maintainer: Murilo Scolari <https://github.com/muriloscolari/OpenMAD-Rust>
Homepage: https://github.com/muriloscolari/OpenMAD-Rust
Description: Native Rust driver and GUI for MAD68 HE / MAD60 HE keyboards
 100% native Rust replacement for Madlions web configurator with live travel
 telemetry, rapid trigger, per-key RGB, macros, profiles, and auto-switching.
EOF

cat > "$PKG_ROOT/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v udevadm >/dev/null 2>&1; then
  udevadm control --reload-rules || true
  udevadm trigger --subsystem-match=hidraw --subsystem-match=usb || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t /usr/share/icons/hicolor || true
fi
exit 0
EOF
chmod 755 "$PKG_ROOT/DEBIAN/postinst"

mkdir -p "$ROOT_DIR/dist"

if command -v dpkg-deb >/dev/null 2>&1; then
  dpkg-deb --build --root-owner-group "$PKG_ROOT" "$OUT_DEB"
else
  # Build valid .deb archive cleanly using tar + ar when dpkg-deb is absent (e.g. Arch)
  echo "2.0" > "$WORK_DIR/debian-binary"
  (cd "$PKG_ROOT/DEBIAN" && tar --owner=0 --group=0 -czf "$WORK_DIR/control.tar.gz" .)
  (cd "$PKG_ROOT" && tar --owner=0 --group=0 --exclude='./DEBIAN' -czf "$WORK_DIR/data.tar.gz" .)
  rm -f "$OUT_DEB"
  (cd "$WORK_DIR" && ar r "$OUT_DEB" debian-binary control.tar.gz data.tar.gz)
fi

echo "==> Created $OUT_DEB"
