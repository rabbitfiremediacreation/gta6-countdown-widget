#!/bin/bash
# Baut GTA6.saver (Apple Silicon, macOS 11+). Aufruf: ./screensaver/build-mac.sh
set -e
cd "$(dirname "$0")"
OUT="build/GTA6.saver"
rm -rf build && mkdir -p "$OUT/Contents/MacOS" "$OUT/Contents/Resources"
swiftc -O -target arm64-apple-macos11 -module-name GTA6Saver -emit-library -Xlinker -bundle \
  -framework ScreenSaver -framework WebKit -o "$OUT/Contents/MacOS/GTA6Saver" GTA6Saver.swift
cp ../gta6-widget/assets/saver.html ../gta6-widget/assets/images.json ../gta6-widget/assets/logo.url "$OUT/Contents/Resources/"
cat > "$OUT/Contents/Info.plist" <<PL
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleDevelopmentRegion</key><string>en</string>
<key>CFBundleExecutable</key><string>GTA6Saver</string>
<key>CFBundleIdentifier</key><string>de.rabbitfire.gta6saver</string>
<key>CFBundleName</key><string>GTA6</string>
<key>CFBundlePackageType</key><string>BNDL</string>
<key>CFBundleShortVersionString</key><string>1.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>NSPrincipalClass</key><string>GTA6SaverView</string>
</dict></plist>
PL
codesign --force --deep -s - "$OUT"
echo "fertig: $OUT"
