# Local cheat reference catalog

The root `cheats/` directory is a private reference collection for future work.
Open `cheats/README.md` locally to browse the Markdown index. The directory and
all derived Markdown/JSON are excluded from Git: the supplied third-party
compilations include an explicit copyright notice and their redistribution
permissions have not been established. This documentation contains no cheat
payloads and does not assign a license to the source material.

The [integrated-debug follow-up checklist](development/DEBUG_TOOLS_V2_TODO.md)
plans a local cheat menu using these stable CH identifiers, with EUR candidates,
format validation, isolated application and replay provenance. These are future
steps; the reference catalog still provides no runtime cheat execution.

## Organization

The initial collection contains 341 distinct code blocks from five supplied
documents, after merging 228 duplicate occurrences. Browsable pages separate
European Codebreaker/GameShark SP/Xploder codes, USA and Japanese GameShark
sections, and Codebreaker/Xploder and Action Replay lists with unconfirmed
regions. Separate pages retain the prose guide, modifier lookup lists, source
attributions and quality findings. The original language and values are kept.

Each block has a stable `CH-` identifier, ordered code lines, source references,
aliases and links to the master code supplied by each source. Deduplication uses
the entire ordered block, declared device format and region together. The
unconfirmed-region group records identical text without asserting that its
sources target the same ROM. Repeated instructions inside a block are retained.

`cheats/catalogo.json` is a local schema-1 export for readonly indexing. It contains
source hashes, `entries`, format/region, canonical code lines, aliases, notes,
status and provenance. Each provenance item retains the source title/category,
original line numbers and raw code lines (or the site's code ID), plus its
`master_id`. Existing IDs should remain stable when extending the catalog.

## Readonly index and CLI — DBG2-17 in progress

`recomp debug-cheats list` reads `cheats/catalogo.json` beside the executable;
if it is absent, the result is an explicit empty state. Select another local
file for one invocation with `--catalog FILE`. Listing is paginated with
`--offset N --limit 1..64`; `show CH-0001 --catalog FILE` returns the complete
original entry, including its ordered lines, aliases, notes, provenance and
source-specific master links. `info` returns the original catalog metadata.
The JSON response includes the SHA-256 of the source and the total count.
Import is readonly and bounded to 4 MiB and 2,048 entries; malformed entries,
duplicate IDs and unsupported schemas are rejected before an index is returned.

`show` also reports each original provenance record's master reference, in
source order. It distinguishes a same-source match, source mismatch, self
reference, unknown identifier, missing or null reference, invalid identifier,
and invalid source. It counts matching provenance records in the linked entry
and reports format/region metadata equality where a target exists. These are
metadata comparisons only: they do not establish which master should be used,
authenticate encryption, or activate a dependency. `inspect` returns the same
report next to its explicitly assumed plaintext plan. The original entry is
preserved unchanged. A focused Windows GNU/Wine test and a synthetic CLI
check exercised these states, the agreement between `show` and `inspect`, and
source preservation; no real catalog or guest execution was needed.

`--rom-sha256 SHA` records a supplied target identity in the response. For the
two exact ROM hashes already recognized by the project, the index compares the
recognized region with the region declared by the source and labels the result
`declared-match` or `declared-mismatch`; an unrecognized hash or source region
remains `unknown`. The CLI does not authenticate the caller's ROM bytes, and a
regional declaration never verifies a code. Every entry is disabled and
non-executable: the device version is unknown, a verified source decoder is
unsupported, and runtime behavior is unverified. An explicitly assumed
plaintext inspection is available as described below; no code is applied.
Catalog contents are not included in source or
public packages; CLI output itself may contain private third-party text.
Focused Wine tests and a synthetic CLI check cover recognized EUR and USA hashes,
a declared mismatch and an unknown hash without loading ROM bytes. A separate
synthetic Wine/Xvfb view confirms that the loaded ROM hash and the unknown state
are shown without advancing the guest.
The isolated Windows GNU build and focused Wine tests passed. A readonly Wine
check against the local catalog compared all 341 entries and the catalog
metadata with the source, checked paginated order and disabled compatibility
states, and confirmed that the source hash was unchanged. It also checked an
absent catalog and an unknown ID. This does not validate cheat semantics.

The Debug / Cheat page computes the exact hash from the currently loaded ROM
and shows it alongside the catalog hash and the same declared-region comparison.
It opens a separate readonly view with search, filters and
scrollable details. A catalog next to the executable is loaded when present;
otherwise the page explains the empty state. **Importa catalogo** opens the
internal content selector: refresh its declared roots, select `catalogo.json`,
return and choose **Ricarica**. External file/folder dialogs remain explicit
buttons in that selector. The selected file is checked against its pinned hash
before loading. Selecting a CH only opens its details; it cannot enable or run
it. Details occupy a separate scrollable panel with the compatibility reasons
before the original record. Long individual records are marked as abbreviated in the view and remain
available in full through `debug-cheats show`. Decoder and controlled
application remain unfinished.

### Assumed plaintext CodeBreaker inspection — DBG2-18 in progress

`recomp debug-cheats inspect CH-0001 --catalog FILE
--assume-plaintext-codebreaker` interprets a single entry declared CodeBreaker
as a **hypothetical plaintext** block. It never infers encryption from the
characters or selects a master. The returned
ordered typed plan includes widths, addresses and values; it is explicitly
non-executable and does not verify the source device, ROM compatibility or
runtime behavior. The original catalog is unchanged. Callers must not use the
plan as authorization to write guest memory.

The initial subset recognizes byte/halfword assignment (3/8), halfword OR,
AND and add (2/6/E), halfword equality/inequality conditions (7/A) guarding
one following simple operation, and a halfword slide (4) with its continuation.
Halfwords use little endian in the guest; slide count and increments are
reported explicitly. Addresses must be width-aligned; slide count is limited to
1..1024 and all its addresses must fit the encoded address range. An invalid
line, placeholder, unsupported opcode, encryption control, master, incomplete
continuation, nested condition or conditional slide rejects the entire block
without a partial plan. This intentionally does not cover every device or
CodeBreaker opcode; even a syntactically successful block may be encrypted.
GameShark/Action Replay remain unsupported. This assumed-plaintext inspection
never applies guest writes; the separate source-bound manual one-shot below
has its own interpreter-only preflight and limitations.

These interpretations are based on the public mGBA
[CodeBreaker opcode definitions](https://github.com/mgba-emu/mgba/blob/master/include/mgba/internal/gba/cheats.h)
and [decoder](https://github.com/mgba-emu/mgba/blob/master/src/gba/cheats/codebreaker.c),
cross-checked against the [VBA GX CodeBreaker type descriptions](https://github.com/dborth/vbagx/blob/master/source/vba/gba/Cheats.cpp).
They are independent descriptions of emulator behavior, not a device-version
or game-specific compatibility proof. Synthetic tests cover the accepted
subset and atomic rejection; the existing local catalog remains private.

### Explicit seeded CodeBreaker inspection — DBG2-18 in progress

For a block the caller has independently identified as encrypted CodeBreaker,
`recomp debug-cheats inspect CH-0001 --catalog FILE --codebreaker-seed
9XXXXXXX:XXXX` decrypts each entry line under that explicit seed before applying
the same bounded typed inspection, including continuation lines. The seed option
and plaintext assumption are mutually exclusive. Neither mode chooses a source
master or detects encryption; in particular, a ciphertext line can itself begin
with `9`. Unsupported decrypted opcodes, incomplete blocks and invalid seeds
fail the whole inspection. A wrong seed may still produce a plausible plan, so
the result remains non-executable and unverified even when decoding succeeds.
The response includes the caller-supplied seed for reproducibility.

The cipher follows the independently described [GBATEK algorithm](https://problemkaputt.de/gbatek.htm#gbacheatcodescodebreakerxploder),
cross-checked against [mGBA's implementation](https://github.com/mgba-emu/mgba/blob/master/src/gba/cheats/codebreaker.c)
and a [public converter's synthetic collision test](https://github.com/SkillerCMP/CMP-GBACheatConverter/blob/main/tests/test_codebreaker.cpp).
It does not establish the device version, required source master, exact ROM
compatibility or runtime effect, and does not implement automatic reseeding,
master execution or guest memory writes.

Validation compared an independently expressed GBATEK formula with the mGBA
cipher on 257 synthetic cases. The Windows GNU/Wine CLI then produced the same
256 synthetic halfword-write operations across four supplied seeds, including
ciphertext beginning with `9`; tests also covered a decrypted slide and its
continuation, malformed/unsupported input and mutually exclusive modes. The
catalog was unchanged. These checks do not establish correctness for a real
catalog entry or any guest effect.

### Explicit source and master inspection — DBG2-17/18 in progress

`recomp debug-cheats inspect CH-0001 --catalog FILE --provenance-index 0
--master-id CH-0002` selects one original provenance record and the exact master
ID it names. The master must have exactly one provenance record from the same
source and matching declared format and region. Its three source lines must
decode as seed, game ID and hook; the entry is interpreted from that source's
original lines, rather than silently substituting a different source. The result
identifies the selected source and hashes its entry/master line arrays; it also
reports whether the entry's canonical lines match those source lines. Missing,
ambiguous, mismatched or unsupported records reject the complete request.

The typed master description reports its checksum and flags, ROM hook offset,
Thumb mode and handler slot/address. The existing CLI can optionally compute
the whole-file SHA-256 and the CodeBreaker checksum of the first 64 KiB from
`--rom-file FILE` (short input is padded with `0xff`). It reports a match or
mismatch when the master requests a checksum. `--rom-file` and caller-supplied
`--rom-sha256` are mutually exclusive. This reads the selected file only; it
does not install the hook. A checksum or declared region match is not a proof
that the device, source, native backend or cheat effect is compatible. Every
plan remains unverified and non-executable.

With the local reference catalog and the project's exact recognized EUR ROM,
Wine CLI inspection was compared with mGBA decryption for the seven priority
entries. CH-0321 and CH-0336 each produced a slide; CH-0323/0324/0325/0331
each produced one conditional operation; CH-0334 produced two halfword writes.
Each selected source master had a checksum matching those ROM bytes, and the
source lines matched the canonical entry. The full operations remain available
locally through the explicit CLI query; private source payload is not included
in this documentation or the recorded test result. ROM and catalog hashes were
unchanged. This is static decoding and checksum evidence, not proof of actual
game effects, correct hook integration, safe activation or reversibility.

### Manual one-shot in an isolated session — DBG2-19 in progress

After selecting and refreshing a catalog in an isolated session, use the command
console or `debug-command` with `cheats prepare CH-0001 0 CH-0002 GENERATION`.
The response contains `status.cheat_preview`: source/master identity, typed
operations, all potential read/write ranges, frame/clock, state hash and a
preview token. Review it, then explicitly issue `cheats apply-once CH-0001 0
CH-0002 GENERATION TOKEN`. The same typed actions are available as
`debug-control cheat-prepare` and `cheat-apply`. A successful one-shot consumes
the preview; retry uses the existing request ledger and does not duplicate it.

Preparation and application require an idle paused isolated copy without an
active recording or replay, an interpreter backend, the exact recognized ROM,
a matching declared region and a source master checksum matching the loaded ROM
bytes. The selected catalog is rechecked; application recomputes the plan and
requires the same source, generation and complete paused-state hash as the
preview. Preflight validates the entire block before any guest bus access, permits
at most 256 writes to aligned canonical EWRAM/IWRAM and allows condition reads
there or from KEYINPUT. Writes go through the guest bus with their proper width.
Other reads, I/O writes, mirrors, ROM writes, native code and oversized plans
are refused. Operations execute once at the paused frame/clock, in their source
order, without installing the original device hook or maintaining a toggle.
This is an explicit manual experiment, not verification of the code's intended
game effect; restoring a checkpoint remains a separate action. Maintained
cheats, overlap management, broader memory semantics and provenance-aware
recording are unfinished.
Until cheat provenance is supported, starting an input recording or saving a
new state after a one-shot is refused for the lifetime of that isolated process;
existing checkpoints may still be loaded.

In an isolated EUR interpreter session under Wine/Xvfb, a synthetic block
performed three bus writes with the expected bytes while frame, clock and CPU
registers stayed unchanged during application. The proof rejected a wrong
token, changed source, changed selection with identical content and changed
paused state; retry did not repeat the writes. One controlled frame was stepped
to check stale-state refusal. Session pins and guarded original ROM, catalog
and save inputs remained unchanged. The session and display were closed.
This did not apply a real catalog entry or verify gameplay, native Windows,
Linux runtime, physical controllers or the save/recording guard interactively.

A separate readonly-source Wine/Xvfb experiment explicitly applied EUR
CH-0334 once in a fresh isolated interpreter copy. Both decoded halfword writes
changed the selected RAM values to the plan's values; frame, clock and CPU
registers stayed unchanged during the application. After one controlled frame,
closing and reopening from the same seed restored the original complete state
and the two original memory values. The source catalog, ROM, seed and guarded
original saves stayed unchanged. This establishes a bounded RAM effect, not a
visible gameplay effect, maintained activation, device-hook equivalence or a
reversible toggle. The retained result excludes the private code payload.

In an isolated session, the local control channel and command console can
select a discovered cheat catalog by ID, refresh an immutable snapshot, list
bounded pages and retrieve complete entry JSON in UTF-8 chunks. The snapshot
reports the source hash and ROM identity checked at refresh; it does not claim
that the underlying file remains unchanged afterward. See
[debug control](debug_control.md) for syntax, cancellation and limits.

Wine/Xvfb validation exercised the empty state, explicit internal import,
search from the first through the last of 341 public synthetic entries,
scrollable details and simulated XInput navigation. A separate readonly test
compared the menu representation of all 341 local entries with their original
records without printing them. The guest state/frame/clock and catalog sources
remained unchanged. Native Windows, Linux and physical controllers have not
been checked.

## Review and limitations

Validation checks preservation and organization; none of these codes has been
applied to the runtime, a ROM or a save. USA/Japan labels follow the supplied
sources. English text alone does not establish a ROM region. Device versions,
encrypted-code semantics and compatibility with FFTARecomp remain unverified.

The local quality page flags two different Codebreaker master codes, apparently
swapped P3/P4 level labels, a possible Player 3 modifier typo and unresolved
placeholders. Values remain unchanged. The saved European web page contains
23 actual code blocks but 25 empty groups loaded on demand, and one missing
popup note; its collection is explicitly incomplete.

The static check verified all 796 source code-line occurrences, the 341 rendered
blocks and their source-specific master links, 1,544 local Markdown links, guide
and modifier-list preservation, and Git exclusions. It also verified all 42
original files against their SHA-256 archive manifest. The active directory now
contains ten Markdown files and one JSON catalog; 37 saved-browser support files
and the five superseded source documents were removed from that directory.

The reversible original archive, manifest, conversion/verification scripts and
cleanup receipt are retained locally under
`target/cheats-catalog-2026-10-28/`. They are ignored and unavailable in a fresh
clone. Missing local copies do not authorize downloading third-party collections
or recreating game fixtures automatically. The scripts describe this one-time
conversion and are not a product importer.

## Adding references

Record the source and attribution, ROM identity/region, exact device version,
complete ordered block and actual verification status. Keep master-code
relationships and regional variants explicit. Review suspected typos against
an independent source or an isolated validation before changing values.
Reassess redistribution permission before publishing any collection content;
the local ignore rule alone is not a license or a packaging exclusion.
