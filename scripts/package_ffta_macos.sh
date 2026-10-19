#!/usr/bin/env bash
# Build the Apple Silicon installer: FFTAInstaller.app carries the game
# payload in Contents/Resources/payload; the installer copies it to
# ~/Applications/FFTARecomp (FFTARecomp.app + ROM, gamedb and saves beside it).
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $(uname -s) != Darwin ]]; then
    printf 'This script requires macOS. On Windows, run build-all.ps1.\n' >&2
    exit 1
fi
VERSION=3.0
TARGET=aarch64-apple-darwin
export MACOSX_DEPLOYMENT_TARGET=11.0
cargo build --release --target "$TARGET" -p installer -p recomp -p ffta-play
bin="target/$TARGET/release"
python3 TinyCC/build.py --platform macos --output "$bin/tcc/libtcc.dylib"

package=Installer/macOS
stage="$package/stage"
rm -rf "$stage"
mkdir -p "$stage"

write_info_plist() { # path name executable identifier
    cat > "$1" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>               <string>$2</string>
  <key>CFBundleDisplayName</key>        <string>$2</string>
  <key>CFBundleExecutable</key>         <string>$3</string>
  <key>CFBundleIconFile</key>           <string>FFTARecomp</string>
  <key>CFBundleIdentifier</key>         <string>$4</string>
  <key>CFBundlePackageType</key>        <string>APPL</string>
  <key>CFBundleShortVersionString</key> <string>${VERSION}</string>
  <key>CFBundleVersion</key>            <string>${VERSION}</string>
  <key>LSApplicationCategoryType</key>  <string>public.app-category.role-playing-games</string>
  <key>LSMinimumSystemVersion</key>     <string>11.0</string>
  <key>NSHighResolutionCapable</key>    <true/>
</dict>
</plist>
EOF
}

icns="$stage/FFTARecomp.icns"
iconset="$stage/FFTARecomp.iconset"
mkdir -p "$iconset"
for spec in \
    "16 icon_16x16.png" "32 icon_16x16@2x.png" \
    "32 icon_32x32.png" "64 icon_32x32@2x.png" \
    "128 icon_128x128.png" "256 icon_128x128@2x.png" \
    "256 icon_256x256.png" "512 icon_256x256@2x.png" \
    "512 icon_512x512.png" "1024 icon_512x512@2x.png"
do
    set -- $spec
    sips -z "$1" "$1" Image/logo.png --out "$iconset/$2" >/dev/null
done
iconutil -c icns "$iconset" -o "$icns"
rm -rf "$iconset"

sign() { codesign --force --sign - --timestamp=none "$@"; }

# ---- Game bundle: player + recomp + TinyCC --------------------------------
release="$stage/FFTARecomp-macOS"
installer="$release/FFTAInstaller.app"
payload="$installer/Contents/Resources/payload"
game="$payload/FFTARecomp.app"
mkdir -p "$game/Contents/MacOS" "$game/Contents/Resources/tcc"
install -m 755 "$bin/ffta-play" "$game/Contents/MacOS/FFTARecomp"
install -m 755 "$bin/recomp" "$game/Contents/MacOS/recomp"
install -m 755 "$bin/tcc/libtcc.dylib" "$game/Contents/Resources/tcc/libtcc.dylib"
cp "$icns" "$game/Contents/Resources/FFTARecomp.icns"
write_info_plist "$game/Contents/Info.plist" FFTARecomp FFTARecomp io.github.jorge262007.fftarecomp
sign "$game/Contents/Resources/tcc/libtcc.dylib"
sign "$game/Contents/MacOS/recomp"
sign "$game/Contents/MacOS/FFTARecomp"
sign "$game"

install -m 644 gamedb.sqlite "$payload/gamedb.sqlite"
cp -R TinyCC "$payload/"
mkdir -p "$payload/Licenses"
cp -R Licenses/. "$payload/Licenses/"

# ---- Installer bundle ------------------------------------------------------
mkdir -p "$installer/Contents/MacOS"
install -m 755 "$bin/installer" "$installer/Contents/MacOS/FFTAInstaller"
cp "$icns" "$installer/Contents/Resources/FFTARecomp.icns"
write_info_plist "$installer/Contents/Info.plist" "FFTARecomp Installer" FFTAInstaller \
    io.github.jorge262007.fftarecomp.installer
sign "$installer/Contents/MacOS/FFTAInstaller"
sign "$installer"
codesign --verify --deep --strict "$game"
codesign --verify --deep --strict "$installer"

cat > "$release/README.txt" <<'EOF'
FFTARecomp v3.0 - macOS (Apple Silicon)
Open FFTAInstaller.app and select your own FFTA USA or Europe ROM.
The game installs to ~/Applications/FFTARecomp. TinyCC is bundled; no Xcode is needed.
The apps are not notarized: on first launch, if macOS blocks them, open
System Settings > Privacy & Security and click "Open Anyway".
No ROM, save, compiled ROM library or assets folder is included.
EOF

zip="$package/FFTAInstaller-macos-arm64.zip"
rm -f "$zip"
ditto -c -k --keepParent "$release" "$zip"
printf 'Created %s\n' "$zip"
