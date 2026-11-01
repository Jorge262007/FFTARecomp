# Source upload

The cleaned project itself contains the sources needed for upload; keep `Installer` for release downloads. `dist` and `target` are generated, optional folders and are not needed to compile from source.

Optionally run `python scripts/prepare-source-upload.py` from the repository. It exports the current files, including uncommitted changes, into `dist/FFTARecomp-source-clean/` and a ZIP beside it. Upload the folder contents to the repository; compiled releases are separate. The script refuses to overwrite an existing export (use `--name another-name` for a fresh export).

The export excludes ROMs, saves, savestates, application binaries, SDK paths, build caches, diagnostic screenshots, and old source backups. It preserves licenses, fonts, controller mappings, build scripts, and the corresponding TinyCC source archive. It does not delete personal saves or the local Installer directory.

Experimental cinematic lighting, authored lights/shadows, their editors and profiles have been removed. Normal GPU presentation, scaling filters, display color simulation and independent UI composition remain. No DLSS/Streamline dependency is used. Older development notes describe historical experiments, not current supported features.

Validation on Windows: desktop executable check; 28 menu tests; 39 screen tests plus explicit GPU/CPU pixel comparison; 77 executable tests; menu/screen examples check; Android core check. ROM-dependent tests are excluded. These checks do not replace gameplay testing on each target device.

Release builds use `Installer` as their canonical output: Windows ZIP and Installer.exe at its root, Linux archives under `Linux/`, Steam Deck under `SteamDeck/`, and the APK under `Android/`. `build-all.ps1` invokes `scripts/package_release_outputs.py` to finish packaging and write hashes. These generated files are excluded from source uploads.

Desktop runtime archives include only the installer, player/recompiler, game database and TinyCC shared library. README files, notices and corresponding compiler source are maintained separately from those runtime archives; publish the applicable notices/source alongside the release. Android keeps its bundled notices inside the signed APK.
