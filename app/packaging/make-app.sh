#!/usr/bin/env bash
# Builds Craftalina.app from a compiled binary.
#
#   bash app/packaging/make-app.sh <binary> <version> <output-dir>
#
# The app gets: the binary, install.sh and patches/ of this repository (as an offline fallback; the
# running app downloads the newest ones), an icon made from packaging/icon-1024.png, an Info.plist
# for macOS 10.15 and an ad-hoc signature. Needs macOS (sips, iconutil, codesign). Set
# ALLOW_NO_ICON=1 to skip the icon and signature on other systems (layout checks only).
set -euo pipefail

BIN="${1:?usage: make-app.sh <binary> <version> <output-dir>}"
VERSION="${2:?version missing}"
OUT="${3:?output dir missing}"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
APP="$OUT/Craftalina.app"

[ -f "$BIN" ] || { echo "binary not found: $BIN" >&2; exit 1; }
[ -f "$REPO/install.sh" ] && [ -d "$REPO/patches" ] || { echo "install.sh or patches/ not found in $REPO" >&2; exit 1; }
case "$VERSION" in *[!0-9.]* | '') echo "bad version: $VERSION" >&2; exit 1 ;; esac

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/craftalina"
chmod 755 "$APP/Contents/MacOS/craftalina"
cp "$REPO/install.sh" "$APP/Contents/Resources/install.sh"
cp -R "$REPO/patches" "$APP/Contents/Resources/patches"

ICON_KEY=""
if command -v sips >/dev/null 2>&1 && command -v iconutil >/dev/null 2>&1; then
  SET="$(mktemp -d)/Craftalina.iconset"
  mkdir -p "$SET"
  SRC="$HERE/icon-1024.png"
  for s in 16 32 128 256 512; do
    sips -z "$s" "$s" "$SRC" --out "$SET/icon_${s}x${s}.png" >/dev/null
    d=$((s * 2))
    sips -z "$d" "$d" "$SRC" --out "$SET/icon_${s}x${s}@2x.png" >/dev/null
  done
  iconutil -c icns "$SET" -o "$APP/Contents/Resources/Craftalina.icns"
  ICON_KEY='<key>CFBundleIconFile</key><string>Craftalina</string>'
elif [ "${ALLOW_NO_ICON:-}" != "1" ]; then
  echo "sips/iconutil not found (needs macOS). Set ALLOW_NO_ICON=1 to build without an icon." >&2
  exit 1
fi

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Craftalina</string>
  <key>CFBundleDisplayName</key><string>Craftalina</string>
  <key>CFBundleIdentifier</key><string>io.github.sudo-mk-sandwich.craftalina</string>
  <key>CFBundleExecutable</key><string>craftalina</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>10.15</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  $ICON_KEY
</dict>
</plist>
PLIST

if command -v plutil >/dev/null 2>&1; then plutil -lint "$APP/Contents/Info.plist"; fi
if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "$APP"
  codesign --verify --deep --strict "$APP"
fi
echo "built $APP"
