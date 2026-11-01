<a id="fftarecomp---instalador"></a>

# FFTARecomp installer

Desktop installer (Rust + egui) that takes **your own ROM** of Final Fantasy
Tactics Advance and installs it with the player and your settings.
This repository **contains no ROM, game asset or game-derived file**.

<a id="compilar-windows"></a>

## Building on Windows

Requirements: Rust (rustup) and the MSVC toolchain.
Supply your own `assets/logo.png` and `assets/icon.ico`, then run:

    cargo run --release

The installer looks for a `payload/` directory beside the `.exe`, containing
`FFTARecomp.exe` and settings (widescreen, etc.). It copies that directory to
`%LOCALAPPDATA%\FFTARecomp`, together with the ROM as `ffta.gba`.

<a id="aviso"></a>

## Notice

Hobby project, unaffiliated with Square Enix or Nintendo. Final Fantasy Tactics
Advance is a trademark and property of its respective owners. Use only a ROM
from a cartridge you own.

## Linux

Build on Linux with Rust, clang, pkg-config, ALSA, udev, X11, XKB,
Wayland and GTK development libraries. On Ubuntu:

```sh
sudo apt-get install clang pkg-config libasound2-dev libudev-dev libx11-dev libxkbcommon-dev libwayland-dev libgtk-3-dev
bash scripts/linux/package_ffta_linux.sh
```

Run this from the repository root. The archive is
`Installer/Linux/FFTAInstaller-linux-x86_64.tar.gz`. Extract it and run `./FFTAInstaller`.
The payload contains native Linux `recomp`, `FFTARecomp`, and the small
`tcc/libtcc.so` compiler. Players do not need clang or gcc installed. The
`TinyCC/` folder supplies its license, exact source and rebuild instructions.
No ROM, save, compiled ROM library or
runtime assets directory is included. Source assets remain necessary for builds.

The installer uses the same ROM hash/byte checks on Windows and Linux. Linux
shortcuts are desktop entries, and installed binaries receive execute permission.
File selection uses Windows native dialogs, KDE `kdialog`, or GTK `zenity`.
On Linux the desktop's selector is preferred when installed; if neither tool
can run, the installer opens its built-in browser. Cancelling closes the dialog.
The manual workflow `installer-linux.yml` builds and packages on Ubuntu; it does
not publish a release. Linux execution has not been tested on the Windows host.

## v3.0 presentation and Steam Deck variant

The desktop installer uses the game's parchment, brass and dark-green visual
style, with embedded Inter and Cinzel fonts and scalable native egui controls.
USA and European ROM checks are unchanged. The bundled TinyCC compiler needs
no developer-toolchain installation; preparation time and cache size depend on
the device. No fixed speed or disk-size promise is made.

The Linux build also creates `Installer/Linux/FFTAInstaller-SteamDeck` and
`FFTAInstaller-SteamDeck-linux-x86_64.tar.gz`. Extract the whole archive, keeping
`payload/` beside the installer. Its filename selects fullscreen setup with a
1280x720 layout. The same mode is available with `FFTAInstaller --steamdeck`;
`--windowed` previews it in a window. Fullscreen uses the compositor's current
display mode; the installer does not change the monitor resolution.

The standard installer stays windowed. The Deck setup keeps a scrollable form
and a visible footer on small/scaled desktops. Native KDE/GTK file dialogs and
the built-in fallback remain available. After installation, add the installed
FFTARecomp game executable to Steam. This variant is a fullscreen installer,
not a persistent single-file launcher.

`build-all.ps1` and `scripts/linux/package_ffta_linux.sh` produce both Linux packages.
On Windows, `scripts/windows/build-linux-windows.ps1 -InstallerOnly` refreshes just the
installer binaries and archives, retaining an already prepared Linux payload.
Without that flag it rebuilds the game and TinyCC as before.

For visual testing only: build with `--features installer-preview`, set
`EFRAME_SCREENSHOT_TO` to a PNG path, and `FFTA_INSTALLER_PREVIEW` to intro,
setup or done. egui saves a real rendered screenshot and exits automatically.
This preview feature is not enabled in distributed binaries.

Windows always starts windowed, including when passed --steamdeck or renamed.
Only Linux Steam Deck mode enables fullscreen. Setup targets 1280x720 and keeps
its Back/Install footer outside the scroll area for high display scaling.
The Important notice explains local ROM compilation, a user-supplied USA/EUR
ROM, and Square Enix ownership. Version remains in the header, not under credits.

Windows and standard Linux use a fixed 900x740 logical-point window, with resize
and maximize disabled and identical minimum/maximum dimensions. Only the Linux
Steam Deck variant starts fullscreen (1280x720 layout).
