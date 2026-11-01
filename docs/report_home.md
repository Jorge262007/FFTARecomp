<a id="homepage-dei-report-locali"></a>

# Local report homepage

Open `index.html` at the project root: it gathers every `index.html` and
`index.htm` found in the local copy, including historical snapshots and private
reports under `target/`. Search by name/title/path and filters separate ROM
coverage, graphics inspector, menu/text/input, audio diagnostics, code/fallback,
tasks/scripts/units, recent events, targeted campaigns, sessions/artifacts, issue
reports, comparisons and other indexes. Links work with JavaScript disabled;
search and filters require it. The maintained template and generated labels use
English, including search feedback and date formatting. Discovered report titles
and paths retain their original text.

The three quick links open coverage at its main path and the most recently
modified graphics/comparison files. The displayed date is the **file mtime**,
not a new validation date. Groups are recognized by adjacent JSON filenames;
the catalog neither verifies nor changes report results. Documentation links
open local Markdown in the browser or configured editor without converting it
into a website.

<a id="aggiornare-lelenco"></a>

## Refreshing the list

The page is static, offline and serverless. A browser cannot enumerate the
project directory itself: after generating/moving/deleting reports, regenerate
the index and reload the page.

On Windows, from PowerShell, with Docker Desktop and the image already used
for [coverage](development/ROM_COVERAGE.md):

```powershell
.\scripts\windows\report_home.ps1 -OpenReport
```

On Linux:

```bash
bash scripts/linux/report_home.sh
```

Both reuse `ffta-rom-coverage:local`, disable networking and mount the project
read-only in the container. The host process publishes only `index.html`.
They do not compile runtimes, extract data, regenerate the map or run the game.
The image must already exist: it is not downloaded or built automatically.
PowerShell accepts `-Image NAME` for another local image compatible with
Python 3.9+. Paths start at the project root even when the launcher is run
from another directory.

With Python 3.9+ already available, without Docker:

```bash
python3 scripts/report_home.py
```

The generator uses only the standard library. `--stdout` returns HTML for a
readonly mount; `--root DIRECTORY` selects another root. Publication prepares
a temporary file and replaces the index only after successful generation,
preserving the previous page on error.

<a id="ambito-e-conservazione"></a>

## Scope and retention

Scanning does not follow symlinks and skips only known service directories:
`.git`, `.hg`, `.svn`, `node_modules`, `__pycache__`, `.gradle`, `.cxx`. It also
includes outputs of older checks: presence does not mean they are current or
accepted. Limits: one million entries, five thousand indexes, first 64 KiB for
each HTML title. Exceeding a limit stops generation without publishing a partial
catalog. ROMs, saves and report JSON payloads are not opened: only HTML titles,
adjacent filenames and mtime are read.

Links are relative and encode spaces, Unicode, `#` and `%`. Moving the entire
project directory preserves navigation; moving individual reports requires
regeneration. Titles/paths are treated as text, not executed as HTML. No external
fonts, scripts or services are used.

`/index.html` is Git-ignored: the catalog contains private names and paths.
Template, generator and launchers remain versioned. In a new checkout, run one
of the commands to create the page. Each refresh replaces one file; it creates
no copies of ROMs, snapshots, images or reports.

<a id="verifica-del-28-ottobre-2026"></a>

## October 28, 2026 validation

Actual generation used the existing coverage Docker image, read-only checkout
and disabled networking. All **23 indexes** matched the independent `rg`
inventory: 8 coverage, 5 graphics, 10 comparisons. **35 local links** checked;
all 23 pre-existing HTML files retained SHA256 and mtime.

Headless Chrome at desktop and narrow layouts checked counts, three quick links,
filters, case-insensitive search, no-results/reset, focus, dates and absence of
overflow. Screenshots inspected. Temporary fixtures checked Unicode/reserved
names, HTML titles, literal tokens, symlink exclusion and scan-limit failure
without changing the previous page. The first two sandbox launcher attempts
could not reach Docker; the authorized run passed, without builds or extraction.

The PowerShell launcher was not run in this validation: that Linux host had no
PowerShell. Native Windows and other browsers were unverified by these checks.
Local evidence: `target/report-home-validation/`.

<a id="launcher-e-distribuzione-aggiornata--dbg-22"></a>

## Updated launchers and distribution — DBG-22

With host Python: PowerShell report_home.ps1 -Python [-OpenReport] or Bash
report_home.sh --python. Without the flag, the existing Docker image is reused.
Both resolve their own root and show the persistent log in target/build-logs;
an error preserves the previous page and removes only the publication temporary.
PowerShell uses a typed null for Replace, without converting it into an empty
backup filename.

The distributed catalog shows guide links only when the document exists:
it adds no links to private checkout state/evidence missing from the bundle.
The [shared guide](debug_tools.md) covers five complete procedures.
Moved-bundle validation covers host Python/Docker, failure with the old page
intact, and browser widths 1400/620 with 6 new reports; it does not reaccept
other outputs. PowerShell 7.4.6 was checked with public fixtures on Linux;
native Windows remains unverified by that validation. Details in the
[record](changes/2026-10-29-distribute-shared-debug-tools.json).
