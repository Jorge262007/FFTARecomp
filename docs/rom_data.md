# Decoded ROM resources

The desktop developer CLI extracts, verifies and prepares OBJ dimensions, BG
descriptors and `graphics.bg-base-palettes` from an exact supported
FFTA EUR or USA ROM. `play --rom-data` directly consumes OBJ dimensions and
the authenticated BG descriptor metadata and base-palette banks. Palette BIOS
HLE reads decoded banks; CPU/DMA/native and guest BIOS reads use an exact
file-derived source view.
`frames` and `runc` support the same opt-in. The source ROM remains required for
code and other data; ordinary startup waits for the remaining ROMDATA-05 gates.
See [runtime consumption](rom_data_runtime.md) for sources, identity, counters,
snapshots, verified coverage and limitations.
The [TODO](development/ROM_DATA_TODO.md) tracks the remaining phases and
[inventory](development/ROM_DATA_INVENTORY.md) separates known readers from
unknown asset catalogs and formats.

```sh
recomp rom-data extract /path/to/your-game.gba target/rom-data/my-dataset
recomp rom-data verify /path/to/your-game.gba target/rom-data/my-dataset
recomp rom-data prepare /path/to/your-game.gba target/rom-data/managed-resources
recomp play /path/to/your-game.gba --rom-data
```

The output directory contains `manifest.toml`, `obj-dimensions.toml`,
`bg-descriptors.json` and `bg-base-palettes.json`. OBJ dimensions store twelve indexed width/height records;
BG metadata stores 162 typed 88-byte descriptors with relative source links,
transfer triples, selectors and retained unclassified bytes. Other linked graphics
and alternate palette programs remain in the ROM. The manifest records
schema/extractor/decoder revisions, exact source ROM identity, regional profile,
decoded coverage, file hash, dependency IDs and the original ranges/hashes.
Both regional direct tables and copied aliases are verified independently.
Extractor6/decoder5 stores all162base palettes as readable JSON banks of16full
u16words, plus the exact consumed source representation for compatibility
reads. Source aliases and bit15are preserved. Earlier e5/e4/e3/e2/e1datasets
remain in separate namespaces. Manifest schema stays1; current formats are
OBJv1, `bg-descriptors-json-v2` and `bg-base-palettes-v2-json`. Resource identity is independent of regional file offsets.
There is no ROM code, image, save, absolute installation path or timestamp in
the manifest. The generated decoded values remain private game data: keep
outputs under ignored `target/` for this developer command, outside packages.

Extraction writes and syncs complete files in a temporary sibling directory,
reads them back through the validator and then renames the directory into
place. Multiple publishers may reuse a complete, source-verified winner.
Repeated extraction reuses valid existing data. An invalid existing dataset
causes an error and remains untouched; choose a fresh output directory to
regenerate it. This explicit `extract` command preserves its original behavior;
managed preparation below adds automatic recovery. No mod or save directory is
selected by the offline commands.

Manifest/OBJ use strict typed TOML, descriptors/palettes strict typed JSON;
all reject unknown fields and apply the four-file bounds listed below. The reader uses fixed
relative file names, rejects symlink files and
directories, and validates the complete manifest against the selected source.
Edited dimensions or descriptors remain invalid even if their file hashes are
updated in the manifest: base data must reconstruct the original source exactly. Mods
will need a separate loader and compatibility identity.

The commands print `ROM_DATA action=created|reused|verified|repaired`, source/profile,
schema and decoded coverage. `runtime=offline` identifies these offline commands;
OBJ, BG metadata and base-palette gameplay each report `runtime=decoded`, with separate
source/file/dataset identities and usage counters. Neither claims that sprite pixels, text, maps,
gameplay tables or audio have been exported or migrated.

## Managed preparation and startup

`rom-data prepare <rom> [resources-root]` and `play --rom-data` share one
preparer. Default storage is `Data/resources` beside a portable executable, or
the local application data directory's `FFTARecomp/resources` in installed
mode (`%LOCALAPPDATA%` on Windows, `$XDG_DATA_HOME` / `~/.local/share` on Linux).
`FFTA_RESOURCES_DIR` is an explicit root override. Portable mode never falls
back to installed storage when its own directory is unwritable.

Managed layout is `<root>/<source-sha256>/v<schema>/e<extractor>-d<decoder>/`.
This revision directory contains a persistent `prepare.lock`, immutable
`generation-*` directories and a small `current` file containing only the
selected generation's relative name. The current revision is `v1/e6-d5`;
all previous revision directories are retained. Each generation contains the
same four files as offline extraction. Source identity is rehashed from
the selected ROM contents at every preparation; file name or mtime cannot
substitute another source. Region and schema/decoder changes select separate
directories and retain previous versions.

The preparer validates `current` and all four selected files before reuse. Valid
reuse does not invoke the extractor or writer. Corrupt/missing data are
reported and regenerated under a process-owned filesystem lock, with a second
validation after lock acquisition so concurrent launches publish once.
Cancellation is checked between preparation stages and while waiting for the
lock; terminating a launcher/runtime releases the OS lock automatically.
The lock file is retained rather than deleted, so live writers cannot lock
different replacement files.

Files are synced and read back before `current` is atomically replaced.
Unpublished, partial or orphan generations are never selected. A failure before
commit preserves the old selection; a failure after commit leaves a complete
verified generation. Existing generations are retained rather than overwritten
or recursively deleted. They can be removed with the whole resource revision
directory while the game is closed; the next preparation regenerates it.
User mods, saves and native caches are outside this namespace and remain
untouched by regeneration. Generated root-workspace `Data/` is ignored by Git;
release packaging already selects explicit inputs and excludes personal Data.

Startup preparation runs after ROM/package verification and before save
discovery, backup capture or native translation. Errors abort startup instead
of silently selecting the old ROM resource path. Progress uses readable
`resources:` messages and the existing `STATUS building` protocol with game-data
phase labels; lock-wait messages are deduplicated. The prepared typed data are
loaded from validated files and connected to the OBJ family's guest/host
consumers. BG descriptors remain immutable typed session data; a reconstructed
source projection serves their CPU/DMA/native bus reads. Base-palette banks
feed the LZ77HLE with exact file-derived source views for other bus consumers.
All prepared files
contribute to dataset identity and are authenticated before activation. This remains an opt-in partial migration, not complete ROM extraction.

## Validation and limits

The historical OBJ-only extraction/preparation results below cover their
original revisions. The descriptor metadata contract and managed dataset work
are recorded separately; preparation alone does not close BG consumption or
its startup/runtime performance gates.

The earlier three-file integration passed the affected extraction/runtime/file/
preparation tests (25Linux/23Wine including private regional/lifecycle cases),
warning-free production builds and18actual CLI checks per target. These cover
source authentication, exact output/reuse, BG corruption/missing repair,
transactional activation, whole-set identity, old e2-d1retention and resource-root
relocation. Three files total107861logical bytes for the supported data. Synthetic
tar/zip marker scenarios exclude both old and new private resource names.
Canonical executables/old fixture bytes remain unchanged. The subsequent
[direct BG runtime unit](rom_data_graphics.md#direct-descriptor-consumption)
passes all-width/mirror/DMA, real-file regional interpreter/native provenance,
Load/Undo/backup and1200-frame native boot parity on Linux/Wine. Combined-data
GUI and native Windows remain open; the frozen e3scene/cost outcomes are
recorded in the [validation protocol](development/ROM_DATA_VALIDATION.md).

Release-mode targeted tests pass on Linux (3 core, 6 CLI) and Windows GNU/Wine
(3 core, 5 CLI; the additional symlink test is Unix-only). These include the
opt-in private-ROM checks for both exact regions, independent source/alias
roundtrips, identical decoded payloads with distinct regional manifests,
source/region mismatch rejection, deterministic output/reuse/relocation,
malformed/bounded input, forged base data, missing files and four concurrent
publishers. No game bytes or personal fixtures are checked in.

Production CLI builds also pass without warnings on both targets. Actual Linux
and Windows/Wine executables pass command help and first extraction/reuse/verify
for both regions; their generated manifests and decoded files are byte-identical.
The pre-existing canonical executables were restored and checked by SHA-256;
portable archives were not rebuilt.

The first Wine launch could not create its server lock in the sandbox's
read-only runtime directory. The same compiled tests pass when Wine can create
its normal server lock. The offline unit did not change machine/save formats,
native cache identity, guest cycles or audio processing.

The subsequent preparation unit passes 14 targeted runtime tests on Linux and
12 on Windows GNU/Wine, plus the affected private regional CLI regression on
each. Two extra Unix-only checks cover symlinks. A separate ignored test helper
is invoked by the process-interruption regression, which kills child preparers
after the first file, before commit and after commit, then proves old data
preservation, unpublished-data rejection, committed-data reuse and lock release.
Tests also cover four simultaneous preparers, cancellation while waiting,
simulated storage exhaustion, failed publication, invalid generation paths,
source/version separation, relocation and mod/save/cache preservation.

Both production CLI builds are warning-free. Actual Linux and Windows/Wine
commands verify managed creation/reuse/repair for corrupt and missing files,
regional switching, portable moves from an unrelated working directory,
explicit overrides, installed roots and startup failure before save writes.
Actual opt-in interpreter GUI runs on Linux and Windows/Wine prepare data on
first boot, reuse the identical generation after a portable move, display the
EUR language screen and exit cleanly. Captures were inspected; no image or save
fixture is committed. The first Linux display setup was blocked by sandbox
socket access; the same prepared fixture passes with an isolated Xvfb socket.
A Windows smoke helper initially failed to decode an extended-length path;
only the helper was corrected, and the existing dataset was reused.

Native Windows and physical power-loss testing remain unverified. Directory
metadata is explicitly synced on Unix; Windows uses synced files and atomic
replacement without a portable directory-flush API. Old/orphan generations are
retained for recovery rather than automatically pruned. Direct reads and
load/Undo/backup rebinding are now verified for this family in the
[runtime evidence](rom_data_runtime.md); broader scene/lifecycle and measured
performance gates remain. Canonical executables are restored by hash;
archives and AndroidPort are unchanged.

BG descriptors and base palettes use readable JSON with standard indentation.
The user explicitly prioritizes functionality and readability over reducing the
space used by extracted data. File/dataset size checks bound malformed input;
they are not compression targets. The historical e3cost observations remain
recorded, without further formatting or packing work to meet their former disk
budget. [Current JSON revision and checks](rom_data_graphics.md#readable-json-datasets).

Current four-file input bounds are64KiB manifest,16KiB OBJ,256KiB descriptors,
1MiB palettes and1280KiB total. The manifest's larger bound accommodates all
162original palette source ranges; OBJ stays small. These are malformed-input
guards, not storage targets. The standard palette file is513639bytes; complete
sets are689678USA/689684EURbytes. [Typed palette contract and checks](rom_data_graphics.md#typed-palette-extraction-and-preparation).

The user's current priority is functional migration without performance
measurement or space-reduction work; optimization comes later. Prior benchmark/
RSS limits and results are historical, and no performance campaign should be
resumed from an older next-action line. Guest clock/state/audio correctness,
source provenance and save/native compatibility checks remain required.
