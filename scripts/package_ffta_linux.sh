#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $(uname -s) != Linux ]]; then
    printf 'This script requires Linux. On Windows, run build-all.ps1.\n' >&2
    exit 1
fi
cargo build --release -p installer -p recomp -p ffta-play
python3 TinyCC/build.py --platform linux
package=Installer/Linux
mkdir -p "$package/payload"
install -m 755 target/release/installer "$package/FFTAInstaller"
install -m 755 target/release/installer "$package/FFTAInstaller-SteamDeck"
install -m 755 target/release/recomp "$package/payload/recomp"
install -m 755 target/release/ffta-play "$package/payload/FFTARecomp"
install -m 644 gamedb.sqlite "$package/payload/gamedb.sqlite"
mkdir -p "$package/payload/tcc"
install -m 755 target/release/tcc/libtcc.so "$package/payload/tcc/libtcc.so"
tar -C "$package" --exclude=payload/toolchain -czf "$package/FFTAInstaller-linux-x86_64.tar.gz" \
    FFTAInstaller payload/recomp payload/FFTARecomp payload/gamedb.sqlite payload/tcc/libtcc.so
printf 'Created %s/FFTAInstaller-linux-x86_64.tar.gz\n' "$package"

# Keep the ordinary Linux payload unchanged; only this build adds the Y shortcut.
cargo build --release -p recomp --features steamdeck --target-dir target/steamdeck-build
deck_stage=target/steamdeck-release-package
mkdir -p "$deck_stage"
cp -R "$package/payload" "$deck_stage/"
install -m 755 target/steamdeck-build/release/recomp "$deck_stage/payload/recomp"
install -m 755 "$package/FFTAInstaller-SteamDeck" "$deck_stage/FFTAInstaller-SteamDeck"
mkdir -p Installer/SteamDeck
tar -C "$deck_stage" --exclude=payload/toolchain -czf "$package/FFTAInstaller-SteamDeck-linux-x86_64.tar.gz" \
    FFTAInstaller-SteamDeck payload/recomp payload/FFTARecomp payload/gamedb.sqlite payload/tcc/libtcc.so
cp "$package/FFTAInstaller-SteamDeck-linux-x86_64.tar.gz" Installer/SteamDeck/
printf 'Created %s/FFTAInstaller-SteamDeck-linux-x86_64.tar.gz\n' "$package"
