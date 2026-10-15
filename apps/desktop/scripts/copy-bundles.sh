#!/bin/sh
# Copy built Linux installers into the website so install.html offers
# one-click browser downloads. Tolerates missing files (build first).
# Called automatically by the desktop `build` script.
set -e
cd "$(dirname "$0")/.."
SRC="src-tauri/target/release/bundle"
DST="../website/downloads"
mkdir -p "$DST"
copied=0
for f in "$SRC/deb/"*.deb "$SRC/rpm/"*.rpm "$SRC/appimage/"*.AppImage; do
  [ -e "$f" ] || continue
  cp -f "$f" "$DST/"
  echo "copied $(basename "$f") -> website/downloads/"
  copied=1
done
if [ "$copied" = "0" ]; then
  echo "no bundles found yet (run the build first)"
fi
