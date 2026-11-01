# Project scripts

Platform-specific entry points live in these folders:

- `linux/`: Linux Docker build/extraction entry points and the native Linux
  packager. `build_windows_docker.sh` builds the Windows package from a Linux host.
- `windows/`: PowerShell entry points, the Windows portable-packager wrapper,
  MSYS2 TinyCC builder and Windows-to-Linux cross-build helper with its C linker.
- `android/`: Android APK builders and Android-device camera helper. Run
  `python3 scripts/android/build_android_docker.py` for the maintained AndroidPort
  with Docker/Buildx and Python only; `--project-dir .` selects the root tree.
  The direct builder supports Windows/Linux SDK/NDK toolchains.
- `unix/`: TinyCC builder shared by Linux and macOS.
- `tests/`: shared Python tooling tests.

Cross-platform extraction, packaging, release, analysis and project-memory
tools stay directly in `scripts/`. For example,
`benchmark_windows_runtime.py` supports native and Wine runtimes despite its
historical filename; `package-release.sh` runs on all three CI host platforms.
The private debug-assets exporter (`export_debug_assets.py`, with Bash and
PowerShell wrappers) creates a separate, local Windows portable ZIP with the
selected debug collection. Its input selection, privacy and prerequisites are
documented in [private debug asset export](../docs/private_debug_assets.md).

The root `build-all.ps1` remains the Windows all-platform build entry point.
Use `-SkipLinux -SkipAndroid` for a Windows desktop package only; omitting only
Android retains the Windows-to-Linux installer step. Native desktop wrappers
share `build_desktop.py`, which follows Cargo's reported executable paths and
stages TinyCC in that same directory before packaging.
It launches the Linux cross-build helper with the current PowerShell executable,
so PowerShell 7 does not try to find the Windows PowerShell `powershell.exe`
inside its own installation directory.
`AndroidPort/` has its own scripts and is managed separately.
Its Docker recipe/driver remain self-contained when that directory is copied.
See [Android Docker builds](../docs/android_docker.md) for APK output, logs,
toolchain versions and the distinction between the two source trees.

The runtime's [session diagnosis](../docs/debug_diagnostics.md) is available as
`recomp diagnose` and Debug > Copy diagnosis; it needs no Python installation.
[Isolated debug sessions](../docs/debug_sessions.md) use `recomp debug-session`
and Debug > Start debug session to work on portable copies of the selected case.
[The save catalog](../docs/debug_catalog.md) uses `recomp debug-catalog list/open/import`
and Debug > Debug catalog for shared filters, integrity checks, previews and
opening a DS checkpoint in an isolated copy. These runtime commands need no Python.
[Snapshot input recordings](../docs/debug_recordings.md) use Debug > Start/Stop
input recording and `recomp debug-record start/verify`: capture relative input
from a private snapshot and compare a bounded interpreter replay's full endpoint.
[Replay profiles](../docs/debug_replays.md) use `recomp debug-replay create/run/cancel`
and Debug > Replay case. The same isolated worker reports progress and distinct
success/divergence/incompatible/timeout/interruption outcomes, without performance
measurement or game-save writes; an existing native library is optional.
[ROM read audits](../docs/debug_rom_audit.md) use `debug-rom-audit create/run/cancel`
and Debug > Audit replay, with an explicit `rom-data-read-audit` worker feature.
They add bounded events, frame/address/reader filters, actual typed/raw service
and physical-byte unions without re-extraction or automatic coverage promotion.

The [completed debug-tools checklist](../docs/development/DEBUG_TOOLS_TODO.md)
records the DBG01–22 scope. The new [integrated-debug TODO](../docs/development/DEBUG_TOOLS_V2_TODO.md)
plans in-menu content/script selection, event and command consoles, and a cheat
debug menu. Implementation starts with the readonly
[operation contract](../docs/debug_operations.md), available as
`recomp debug-operations list/show [--json]`. It inventories existing commands,
parameters and remaining menu gaps; it does not execute the listed operations.
The readonly [content catalog](../docs/debug_content.md), `debug-content scan/check`,
indexes only declared app roots and explicitly added folders, with stable IDs,
hashes, availability, declared compatibility and bounded refresh; no content is
executed. The [integrated content selector](../docs/debug_selector.md),
Debug > Content selector, adds explicit search/filter/select/refresh/import
controls over the same readonly catalog. Typed tool inputs and parameter drafts
are implemented by DBG2-04/05. Scripts and tools adds explicit declared adapters
for homepage/campaign report/artifact preview (selected Python3.11+), readonly
command recipes and a separate guest catalog/inspector link. It stages exact
public source hashes on Start, retains bounded logs and supports cancellation;
PowerShell/Bash/Docker wrappers remain separate explicit offline entry points.
DBG2-07 saves guarded relative/internal and explicit/external choices plus
selector views/history; a moved portable copy restores compatible pins and
explains missing/changed ones without launching content.
DBG2 development remains distinct from DS scene acceptance.

From the repository root, extract readable ROM data with Docker:

```bash
bash scripts/linux/extract_rom_data_docker.sh
```

```powershell
.\scripts\windows\extract_rom_data_docker.ps1
```

For portable builds:

```bash
bash scripts/linux/build_linux_docker.sh
bash scripts/linux/build_windows_docker.sh
```

```powershell
.\scripts\windows\build_docker.ps1
```

Both extraction entry points use
`rom/1194 - Final Fantasy Tactics Advance (E)(Surplus).gba` relative to the
repository root by default. Override it with a first ROM argument on Bash or
`-RomPath` on PowerShell; absolute ROM paths are accepted and relative ROM paths
always start at the repository root, even when launched from another directory.

Scripts resolve their repository from their own location. Explicit output
paths remain relative to the calling directory. See
[Docker workflows](../docs/docker.md) and [extraction options](../docs/rom_data.md).
The EUR extractor also publishes `word-task-obj-animations.json` in e77-d76:
four source-authenticated banks for u16 task counters, with readable OBJ lists,
relative indices, frame selectors and durations. See the [managed runtime
contract](../docs/rom_data_obj.md#managed-runtime-for-the-u16-task-obj-banks)
for validation, actual-CLI evidence and the source-backed ordinary Bank check. USA remains e8-d7.
EUR e78-d77 additionally publishes `world-nodes.json`: typed coordinates,
ordered path/neighbor links and losslessly preserved opaque fields. The
[world-node runtime contract](../docs/rom_data_maps.md#managed-world-nodes-and-typed-guest-reads)
records the reduced Wine integration checks and actual CLI create/reuse/normal
coordinate use. The subsequent ordinary travel360check passes coordinates,
paths and neighbors with complete state/clock/image/audio equivalence. Other
nodes/maps, opaque ordinary use and new Linux/USA checks remain pending.


The Linux Docker builder saves each run's output and errors under
`target/build-logs/linux-*.log`. On failure it waits for Enter in an interactive
terminal; noninteractive automation receives the original exit code immediately.

After recording successful managed/native validation, retire disposable test
copies with `python3 scripts/local_artifacts.py complete target/<unit> --apply`.
Preview without `--apply`; retain the accepted offline dataset and proof logs.
See [local artifact retention](../docs/development/LOCAL_ARTIFACTS.md) for safety
checks, optional superseded-binary retirement and coverage cache limits.


EUR e79-d78 extraction additionally publishes `world-roads.json`
(`gameplay.world-roads`), covering45conditional8-byte headers and2,188signed
XY samples through46source spans. Schema1 and USAe8-d7 stay unchanged. Runtime
preserves guest movement and bytewise old-owner/raw fallback; the reduced Wine
integration proof is recorded in [managed world roads](../docs/rom_data_maps.md#managed-world-roads-and-typed-guest-reads).
Production CLI and ordinary-scene proof remain next; Linux/USA tests pending.


Actual e79-d78CLI creation/reuse/startup and one Giza360ordinary road pair now
pass on Wine, with72files/71resources/74managedpaths unchanged and full states/
clock/images/OAM/dispatch/fiveaudio. All seven road counts match CLI, including
positive count/pointer/sample bytes. See [the scoped result](../docs/rom_data_maps.md#ordinary-sampled-road-travel-equivalence).
Linux/USA/nativeWindows and broader coverage remain pending; no measurements.

I launcher ROM coverage accettano audit DBG-06 ripetibili (`--audit FILE` su Bash,
`-AuditPath @('a.json','b.json')` su PowerShell) e una fotografia precedente
(`--compare FILE` / `-ComparePath FILE`). Importano soltanto metadati locali,
con mount in sola lettura; nessun replay. La mappa distingue unioni osservate,
servizio Typed/Raw e intere famiglie positive, con filtri e limiti espliciti.
Vedi [istruzioni ed esempi](../docs/development/ROM_COVERAGE.md#scene-letture-e-confronto-dbg-07).


EUR e80-d79 extraction publishes `encounter-headers.json`
(`gameplay.encounter-headers`) with a constructor-alternate dependency. Its two
fragments migrate17,676bytes while retaining existing ownership of the shared
four-byte pointer. The reduced Wine integration result is documented under
[managed encounter headers](../docs/rom_data_maps.md#managed-encounter-headers-and-typed-guest-reads).
Actual CLI/ordinary coverage and Linux/USA/nativeWindows remain pending; no
performance measurement is included.

Il comando runtime `debug-capture create/run/cancel` e Debug > Capture on trigger
producono punti derivati da replay isolati: prima lettura di una risorsa,
ingresso interprete in un PC ROM verificato, oppure N frame completati.
Richiedono `rom-data-read-audit`, input registrato e limite esplicito; il risultato
è un bundle/catalogo privato o `not-reached` senza snapshot. Vedi
[uso, provenienza e limiti](../docs/debug_captures.md).


EUR e81-d80 extraction adds `encounter-unit-templates.json`
(`gameplay.encounter-unit-templates`) with five explicit header/constructor/item/
metadata14/job dependencies. Two source fragments skip the192 bytes already
owned by constructor alternates; strict coherence and typed reads preserve
original live initialization. [Contract and checks](../docs/rom_data_gameplay.md#managed-encounter-unit-templates-and-typed-guest-reads).

`recomp debug-compare create/run/cancel` e Debug > Compare first difference
confrontano ogni frame dal medesimo snapshot/input: interprete/raw come
riferimento e interprete/native con dataset opzionale come candidato. Il primo
confine diverso produce un report HTML/JSON privato con campi CPU/Bus esatti,
immagini affiancate e stream; un prefisso uguale conserva solo metadati.
Non richiede la feature audit. Vedi [uso, ambito e verifiche](../docs/debug_comparisons.md).

`recomp debug-control status/pause/step/resume SESSION` controlla la finestra
di una copia isolata. `step SESSION N [--keys HEX]` avanza esattamente 1–600 frame
e rimane in pausa; menu e F6/F7/F8 condividono le stesse azioni.
`debug-session open SESSION --paused` evita frame prima della prima richiesta.
Vedi [DBG-10: procedura, audio e limiti](../docs/debug_control.md).


EUR e82-d81 extraction adds `encounter-coordinates.json`
(`gameplay.encounter-coordinates`) with header pointer/count/role coherence.
Its691 spans retain raw XY bytes and original signed/unsigned placement/RNG
logic. [Managed resource and checks](../docs/rom_data_maps.md#managed-encounter-coordinates-and-typed-guest-reads).

`recomp debug-control inspect SESSION ADDRESS LENGTH` legge buffer e registri
senza effettuare accessi I/O. `break SESSION PC arm|thumb` e `watch SESSION
read|write START END` cercano un punto nell'interprete con limiti di passi/frame;
watch richiede `rom-data-read-audit`. Vedi [DBG-11 e granularità](../docs/debug_probes.md).

`recomp debug-control graphics SESSION [--compare-last]` cattura dalla copia
in pausa un [report grafico DBG-12](../docs/debug_graphics.md), con PNG/JSON e
buffer PPU privati, proiezioni readonly e confronto con l'ultimo report.
Non richiede audit e non genera un nuovo snapshot.

`report_home.py` genera `index.html` nella root, con tutti gli indici HTML locali,
ricerca e filtri. `windows/report_home.ps1 -OpenReport` e
`bash scripts/linux/report_home.sh` usano l'immagine Docker coverage già esistente,
con rete disabilitata e checkout readonly; non eseguono estrazione o runtime.
Il solo catalogo generato resta ignorato. [Uso e verifiche](../docs/report_home.md).

`recomp debug-control ui-start SESSION` / `ui-report SESSION` e le voci Debug
Start UI / input trace / Save UI / input report raccolgono osservazioni limitate
di focus, font, input/rimappatura e dispatch host/guest nella copia isolata.
`status.host_menu` espone il focus reale prima di automatizzare la navigazione.
I report privati sono riconosciuti dalla homepage. [DBG-13](../docs/debug_ui.md).

`recomp debug-control audio-start SESSION [--frames 1..180] [--streams LIST]` /
`audio-report SESSION` e le voci Start audio capture / Save audio report raccolgono
buffer guest e callback con contesto mute/Fast/reset nella copia isolata.
`debug-audio states --rom ROM --left STATE --right STATE --dir NEW` confronta
buffer già conservati senza eseguire il gioco; exit0equal/2diverged/3incompatible.
WAV/raw/report privati sono indicizzati dalla homepage. [DBG-14](../docs/debug_audio.md).

`recomp debug-control code-start SESSION` / `code-report SESSION` osservano
un prefisso PC/modo, istruzione al punto, label statiche e scelta dispatch
native/missing/system/hook/interpreter nella copia isolata. Filtri e limiti
espliciti, nessun nuovo snapshot o percentuale di codice dedotta dai conti.
[DBG-15](../docs/debug_code.md).

`recomp debug-report create PROFILO/replay.json --symptom TESTO` e il menu
Create issue report riuniscono contesto, input/identità ed evidenza conservata
in un HTML/JSON locale con launcher relativi, senza avviare il caso o inviare dati.
`debug-control issue-report SESSION` è l’equivalente sulla finestra aperta.
[DBG-16 e privacy](../docs/debug_issues.md).

`recomp debug-control gameplay-start SESSION [--frames N --events N --family FAMIGLIA --context HEX]` /
`gameplay-report SESSION` e Start task / script trace / Save gameplay report
osservano campi verificati di task, script e unità con provenienza RAM/ROM/registri,
filtri e limiti espliciti. Link alle risorse e campi sconosciuti rimangono distinti
dalle identità persistenti degli attori. [DBG-17](../docs/debug_gameplay.md).

`debug_artifacts.py preview --session DIR --keep DIR --unit target/rom-data-NOME --out NEW`
produce un piano HTML/JSON; `apply PLAN --receipt NEW.json` lo verifica sotto
coordinamento e registra ogni rimozione. Input/report/errori e sessioni tenute
restano conservati. Launcher `windows/debug_artifacts.ps1` e
`linux/debug_artifacts.sh [--docker]` salvano i log. Docker usa la derivata
`Dockerfile.debug-tools` con Git, senza build runtime. [DBG-18](../docs/debug_artifacts.md).

**recomp debug-control recent-start SESSION** / **recent-freeze SESSION** /
**recent-report SESSION** e Start recent events / Freeze / save recent events
mantengono una finestra mobile1.024eventi/256KiB. Il trigger errore osservato salva
contesto utile; disponibilità delle letture, espulsioni e limiti restano espliciti.
Nessun nuovo snapshot. [DBG-19](../docs/debug_recent.md).

**recomp debug-control status SESSION** funziona anche mentre un comando attende.
**cancel SESSION --request ID** annulla quel budget al prossimo rientro frontend;
**--timeout-ms N** (20–120000) aggiunge attesa limitata e scadenza server.
JSON distingue timeout non confermato, stato server e identità manifest/run/ID.
Nessun servizio di rete. [DBG-20](../docs/debug_control.md#richieste-cancellazione-e-scadenza--dbg-20).

**debug_campaign.py report MANIFEST --case ID --out NEW** seleziona DS/famiglie/
transizioni e riusa prove JSON con pin, senza replay automatici. Esiti positivo/
negativo/incompatibile/non eseguito e zero letture nella sola finestra restano
espliciti. **run-case MANIFEST ID --reason TEXT --out NEW** delega un solo profilo
al replay esistente; una prova precedente richiede --reopen-reason. Launcher
**windows/debug_campaign.ps1** e **linux/debug_campaign.sh [--docker]** conservano
log; Docker offre solo report/help. [DBG-21](../docs/debug_campaigns.md).


The [shared debug guide](../docs/debug_tools.md) covers recording, bug reports,
runtime comparison, uncovered ROM reads and retention from a moved portable
bundle. **windows/debug_runtime.ps1** and **linux/debug_runtime.sh** preserve
command logs in Data/config/debug-logs and select the adjacent runtime.
Offline launcher logs remain in target/build-logs; home launchers accept
-Python / --python, retain the old page on failure and hide absent bundle docs.
PowerShell artifact/campaign -Docker comes first; --out and other tool flags
are forwarded literally. Public checks: **tests/test_package_ffta_portable.py**
and **tests/test_debug_windows_launchers.ps1** (Linux PowerShell container,
public mocked applications only; see its prerequisites).

The portable packager selects public docs/tools/notices explicitly and writes
PACKAGE-MANIFEST.json; no existing Data, ROM/save/input/dump/cache/test files are
copied. Shell execute permissions survive the Linux tar. Both Docker source
stages include the same module/template/doc closure.

Host distribution builds should remap compiler source paths before packaging:
Rust --remap-path-prefix for checkout/home/cache roots, and TinyCC/build.py
maps its source and adapter roots with -ffile-prefix-map. The shared packager
accepts repeatable --private-prefix PREFIX to reject embedded UTF-8/UTF-16LE
private paths before replacing the previous archive. It never rewrites binaries;
the manifest records only the number of prefix checks. DBG-22 uses this guard
with its isolated remapped Windows binaries.
