<a id="esportazione-privata-degli-asset-di-debug"></a>

# Private debug asset export

The script prepares a **local, private ZIP** containing a separate portable
Windows folder. It contains copies of saves, snapshots and recordings: it is
not the distribution package and must not be published or added to Git.
ROM and BIOS files are excluded. Supply your own compatible EUR ROM at first launch.

From the checkout, with Python 3 available:

```bash
bash scripts/linux/export_debug_assets.sh
```

```powershell
.\scripts\windows\export_debug_assets.ps1
```

Alternatively, run `python3 scripts/export_debug_assets.py` (`py -3` also works
on Windows). The script prints the timestamped ZIP path under `dist/debug-assets/`.
Extracting it into the project root creates `FFTARecomp-debug-EUR/`; launch
`FFTARecomp.exe` from that folder and select your own EUR ROM. If that folder
already exists, extract elsewhere to preserve the previous copy. The extracted
folder is ignored by Git at the root and contains its own protective `.gitignore`.
The game writes to its portable folder; Debug and Extended saves are already
enabled in the initial configuration.

The ZIP includes:

- The selected Windows debug-tools package, with separately selected Windows
  launcher and runtime. `PACKAGE-MANIFEST.json` describes only the base package;
  `DEBUG-ASSETS-MANIFEST.json` lists every file in the export.
- The `debug_saves/ready/` collection under `Data/config/debug-catalog/`, including
  the catalog and EUR/USA saves and snapshots. USA entries are retained as an
  existing collection; this export does not verify their playability.
- Working copies of the EUR slots and `ffta.sav` under `Data/saves/`, so
  **Game > Saves** finds them directly. The catalog keeps separate copies
  selectable through **Debug > Sessions and cases > Debug catalog**.
- Remaining `debug_saves/` files, the `saves/debug-cases/` collection, sources
  named by the local snapshot manifest when available, complete recordings
  from the dedicated check and the F-09 replay pair when available, in
  `DebugAssets/` or `Data/config/recordings/` as described by
  `LEGGIMI-DEBUG.txt` in the ZIP.

These sources are selected explicitly: the script does not copy all of
`target/`, ROM datasets, caches, Wine prefixes, incomplete sessions or personal
files outside the named collections. Select another collection with
`--assets`, `--legacy`, `--source-manifest`, `--recordings` and `--replay`;
`--help` shows the defaults. Complete recordings and catalog entries are
checked against their SHA-256 values. Snapshots and recordings retain their
original ROM, BIOS, runtime and provenance constraints; inclusion in the
package does not replace compatibility checks.

Required default inputs are `dist/debug-tools/FFTARecomp-windows-debug-tools.zip`,
`target/x86_64-pc-windows-gnu/release/recomp.exe` and `ffta-play.exe` in the same
folder. First build a debug-tools package and the Windows binaries, or select
existing inputs with `--base`, `--runtime` and `--launcher`. Before publishing
the ZIP locally, the script checks PE architecture, the base package manifest
and hashes, symbolic links, paths, ROM/BIOS exclusions and ZIP CRC/hash values.
Use `--output` for an explicit destination; an existing file is not overwritten.
No build, emulation or source modification is performed.

Local validation on 2026-10-31 checked 104 catalog files, 10 original sources,
two recordings and the F-09 replay. Through Wine, the extracted Windows runtime
listed the EUR catalog and created an isolated DS-20 copy; the copy's GUI opened
the catalog while paused, preserving state and clock. The final copy adds the
verified sources and updates instructions/Git protection compared with the ZIP
used in the GUI; runtime, configuration, catalog and saves are identical.
Native Linux, native Windows, physical controllers and USA playability remain
unverified. Local evidence: `target/private-debug-assets-validation/`.
