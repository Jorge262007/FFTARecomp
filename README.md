<p align="center">
  <img src="Image/logo.png" alt="FFTARecomp logo" width="50%">
</p>

<h1 align="center">FFTARecomp</h1>

<p align="center">
  <b>Final Fantasy Tactics Advance, running natively on your Windows PC, Linux PC or Steam Deck.</b><br>
  Widescreen, video and audio options, and a portable Windows edition.
</p>

<p align="center">
  <a href="https://github.com/Jorge262007/FFTARecomp/releases"><img src="https://img.shields.io/github/v/release/Jorge262007/FFTARecomp?include_prereleases&label=release" alt="Release"></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11%20%7C%20Linux%20%7C%20Steam%20Deck-blue" alt="Platforms">
  <img src="https://img.shields.io/badge/built%20with-Rust-orange" alt="Rust">
</p>

---

## ✨ What is this?

**FFTARecomp** turns *Final Fantasy Tactics Advance* (USA version) into a native program for Windows, Linux and Steam Deck. Instead of running in an emulator, the game is recompiled from your own ROM, so it runs smoothly, needs no BIOS, and gets modern options. It is based on [gba-recomp](https://github.com/JRickey/gba-recomp) by JRickey.

## 🆕 What's new in v3.0

- 🤖 **Native Android version.** FFTARecomp now runs natively on Android. It also includes touch support that works like the PC mouse support: tap to interact with the game's menus and interface. Touch mode (without a touchpad) is experimental for now; to enable it, go to **Input > Touchscreen > Touch Controls**. More solid touch support is coming soon.
- ⚡ **Big PC optimization.** The compiler was switched, so the install size drops from roughly 800 MB to only about 11 MB, and compile time goes from roughly 120 seconds to about 4 seconds.
- 💬 **Fully reworked game UI and text bubbles.** They are now editable: a new **Text Bubbles** option in the Settings menu lets you adjust their length and size in-game.
- 🖱️ **Mouse support (experimental).** It doesn't cover the whole game yet, but it is already usable and enjoyable, and support will keep expanding. To enable it, go to **Input > Mouse and Keyboard** and turn it on. On Windows, mouse mode comes with its own remappable layout that doesn't affect the keyboard controls.
- 🇪🇺 **European ROM support.** Saves are compatible between regions.
- 🛠️ **Reworked installer.**
- 🎮 **Reworked XInput gamepad support.**
- 🕹️ **Steam Deck now has a dedicated installer** (the Linux installer also works).

## 🎮 Features

- 🖥️ **Widescreen:** Auto, 16:10, 16:9 (default), 20:9, 21:9 and 32:9.
- 🔍 **Zoom in and out.**
- 🎨 **Video options:** screen model, grid, scaling and vsync.
- 📊 **FPS counter:** internal, external or both, configurable in the Video menu.
- 🔊 **Audio options** and volume control.
- 🔊 **Normal audio mode:** the game fast-forwards while the audio keeps playing at normal speed.
- ⏩ **Fast forward.**
- 🎛️ **Completely redesigned menu.**
- 🎮 **Gamepad support** (generic XInput).
- 🎮 **Configurable menu shortcuts:** keyboard and gamepad button combinations.
- 💾 **Save states:** 9 controller-friendly slots with screenshot previews.
- ↩️ **Undo save/load:** undo the last save-state operation.
- 🛟 **Automatic save backups:** keep two startup copies and restore from the menu.
- 🐧 **Native Linux version.**
- 🕹️ **Steam Deck support.**
- 📦 **Portable on Windows:** extract, pick your ROM and keep all data in one folder.
- 📦 **Portable on Linux:** keep settings, saves and cache in the game folder.
- 🐳 **Docker builds:** compile Linux and Windows editions with containerized build dependencies.
- 🚫 **No console BIOS needed.**

## 🕹️ Controls

> 💡 To open the **Options Menu**, press **Esc** while in-game.

| PC | Game Boy Advance |
|----|------------------|
| Z | A |
| X | B |
| A | L |
| S | R |
| Enter | Start |
| Right Shift | Select |
| Up Arrow | Up |
| Down Arrow | Down |
| Left Arrow | Left |
| Right Arrow | Right |
| Tab | Fast Forward |
| + | Zoom in |
| - | Zoom out |

Controls can be remapped from the Options Menu.

## 🎮 Gamepad default

| Gamepad | Action |
|---------|--------|
| A | GBA A |
| B | GBA B |
| LT | GBA L |
| RT | GBA R |
| LB | Zoom out |
| RB | Zoom in |
| Left stick click | Hold Fast Forward |
| Right stick click | Toggle Fast Forward |
| Start / Menu | Start |
| Back / View | Select |
| D-pad or left stick | Move |
| Guide | Options Menu |
| Start + Back | Options Menu (configurable combo) |

Use **Input > Keyboard/Gamepad > Open menu** to bind a single control or a
combination. Release the activation button, hold the desired combination,
then release all its controls to save it. Escape and Guide remain available.

## ⏩ Fast forward modes

| Mode | Behavior |
|------|----------|
| Silent | No audio (default) |
| Fast audio | Audio speed follows the fast forward speed |
| Normal | Audio keeps playing at normal speed |

## 💬 Text Bubbles

| Option | Behavior |
|--------|----------|
| Manual length | Changes the length of the text bubbles |
| Size | Changes the size of the text bubbles |

## 🚀 Installation

### Windows

1. Extract `FFTARecomp-windows-portable.zip` to a writable folder.
2. Run `FFTARecomp.exe` and choose your own FFTA USA/Europe ROM on first launch.
3. Move the whole folder to keep ROM, settings, saves, backups and state slots.

There is no installer. To import existing progress, close the game and copy your
raw `.sav` to `Data/saves/ffta.sav`, keeping a copy of any existing destination.
See [portable storage, migration and builds](docs/portable.md).

### Linux / Steam Deck

Extract the native `FFTARecomp-linux-portable.tar.gz` and run:

```bash
tar -xzf FFTARecomp-linux-portable.tar.gz
cd FFTARecomp-portable
./FFTARecomp
```

Choose your own FFTA USA/Europe ROM on first launch. Settings, cartridge saves,
backups and state slots stay inside `Data`; move the whole folder while closed.
No Wine or installation step is needed. The current native archive is tested
on Ubuntu 26.04 x86_64; see [portable requirements and migration](docs/portable.md).
Steam Deck compatibility for this host-built archive has not been verified.

## 🔨 Building from source

For Linux, build with dependencies contained in Docker:

```bash
bash scripts/build_linux_docker.sh
```

The portable archive and build information appear in `dist/docker-linux/`.
The host needs Docker with Buildx; compiler and development packages stay in the
container. See [Docker builds and runtime requirements](docs/docker.md).

For Windows with dependencies inside Docker Desktop (Linux containers), run
from PowerShell:

```powershell
.\scripts\build_docker.ps1
```

The Windows ZIP appears in `dist/docker-windows/`. Add `-Platform linux` to
build Linux from the same Windows host. No Rust, MSYS2 or Python host install
is required for this Docker workflow. See [Docker builds](docs/docker.md).

For native Windows builds outside Docker, you need [Rust](https://rustup.rs) and Windows PowerShell. Then run a single script from the repo root:

```powershell
git clone https://github.com/Jorge262007/FFTARecomp.git
cd FFTARecomp
.\build-all.ps1
```

The script builds the Windows workspace, bundles TinyCC and creates
`dist/FFTARecomp-windows-portable.zip`. It also retains the separate Linux/Steam
Deck installer and Android build steps. Use `-SkipAndroid` to omit the APK.
For a Windows-only build, see [portable build commands](docs/portable.md).

If PowerShell blocks the script, run it with:

```powershell
powershell -ExecutionPolicy Bypass -File .\build-all.ps1
```

> ⏱️ The first build can take several minutes.

## 🐞 Known issues

- 👻 In the first level, some sprite silhouettes may appear on screen for 1 to 3 seconds. They don't show up again for the rest of the game.

## ⚠️ Good to know

- 🚧 This is a work in progress.

> ### 🛡️ Windows or your antivirus shows a warning?
> **The executables are not code-signed**, so Windows SmartScreen or your antivirus may show a warning. Click **More info → Run anyway**, or right-click the `.zip` → Properties → tick **Unblock** before extracting.
>
> If you have any doubts, you can check the **VirusTotal report** included in each release.

## Cartridge save backups

Launching a game automatically backs up its selected `.sav`, keeping the **two
most recent startup copies**. Use **Save backups** in the pause menu to restore
one with the controller; confirm Restore and restart, then Resume to boot the
restored game. See [backup storage and recovery](docs/save_backups.md).

## Save states

Open the pause menu > **Save states**. Choose one of **9 slots** with Left/Right,
then Save state or Load state. Each slot shows a screenshot preview; controller
navigation, overwrite confirmation and feedback stay inside the menu.
**Undo last operation** restores the previous slot file after saving, or the
pre-load game state after loading (one level, current session only).
Files are named `slot-1.ffstate` through `slot-9.ffstate` under
`SaveStates/<ROM SHA-256>/` beside the selected cartridge `.sav`.
See [save state behavior and compatibility](docs/save_states.md); use ordinary
in-game `.sav` saves for transfer to a GBA or another emulator.

## Development history and project skills

See [CHANGELOG.md](CHANGELOG.md) for implemented changes and validation.
See the [desktop performance TODO](docs/development/PERFORMANCE_TODO.md) for
prioritized optimization work, CPU/GPU candidates and correctness gates.
[AGENTS.md](AGENTS.md) routes the local development, Windows validation and
project memory skills. Follow the [memory workflow](docs/development/MEMORY_WORKFLOW.md)
to record changes and run `python3 scripts/project_memory.py check` before handoff.

## 📜 License and credits

- License: MIT, see [LICENSE](LICENSE). It keeps the original author's copyright line.
- Based on [gba-recomp](https://github.com/JRickey/gba-recomp) by JRickey (MIT OR Apache-2.0).
- `gamedb.sqlite` is by JRickey, released under CC0 1.0.
- Includes [w64devkit](https://github.com/skeeto/w64devkit) (GCC, binutils and other tools under GPL and compatible licenses), unmodified. Their licenses are in `payload/toolchain`.
- *Final Fantasy Tactics Advance* is owned by Square Enix / Nintendo. This project is not affiliated with them and does not distribute ROMs or copyrighted game data.

---

<p align="center">Made by a fan, for fans.</p>
