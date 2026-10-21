# Decoded graphics formats and source contracts

The next resource work units extend the OBJ metadata prototype toward actual
graphics. They provide bounded offline LZ77, packet and descriptor decoders,
with verified source ranges and original-instruction consumer checks.
Descriptor metadata now belongs to the managed dataset; linked graphics and
program files remain unextracted, and direct BG metadata consumption is pending.
The [inventory](development/ROM_DATA_INVENTORY.md) and
[full TODO](development/ROM_DATA_TODO.md) retain the remaining catalog,
semantic decoding, extraction and runtime integration work.

## Observed producers

An optional test follows 1200 clean-boot frames with standard demo input, without
a save or forced PC, independently on each exact supported regional image.
It records actual executed BIOS decompression instructions through `StepEvent`,
not a guessed PC before interrupt dispatch. ROMs are read-only and all output
goes into a new private directory. The producer trace and post-SWI destination
bytes match byte-for-byte between Linux and Windows GNU/Wine for each region.

Each trace observes four LZ77 WRAM calls (SWI 0x11), no other decompression SWI:

- USA frame 801: source 0x56f4a8, 125 consumed bytes -> 160 decoded bytes at
  IWRAM 0x3860. Frame 803: 0x5704cc, 133 -> 456; 0x56fefc, 1481 -> 1996;
  0x56f52c, 2507 -> 2960, each into EWRAM 0x3cb0 scratch at its respective
  call. The source extents exclude padding.
- EUR frame 1161: 0x76c64c, 125 -> 160 at the same IWRAM destination.
  Frame 1163: 0x76d670, 133 -> 456; 0x76d0a0, 1481 -> 1996;
  0x76c6d0, 2507 -> 2960 at EWRAM 0x3cb0.
- The actual wrappers are 0x08141890 USA and 0x08145254 EUR. Caller link
  registers are 0x0801a4a9/0801f519/0801f3f1/0801f295 USA and
  0x0801b619/08020689/08020561/08020405 EUR, in the order above.

The four compressed streams and decoded outputs match across the two regions:
4246 consumed source bytes and 5572 decoded bytes per region. Decoded SHA-256,
in call order: 2a360ec828fb53a6becc94e8f1c3aa9c6044821639c44ad393b97c66ba0a6b44,
8f768d8eda371a2fa1a5fbe5b72a36f61f286e40f14088730013fdb44d824235,
3e9599c722762edd19e059a5d0bbb51e1eb907f94c44e17ebe7d8445df54429f,
3d379876034eec8b762e05ad7b8d987007b5c89d3369c0c5e05b3b8e2ccabc42.
These are exact stream-consumption bounds derived by the decoder, not a claim
that padding belongs to the payload or that every graphics source was visited.

Independent original caller disassembly also identifies a shared relative
descriptor layout: 88-byte stride, root USA 0x569104 from literal 0x1a4ac and
root EUR 0x7662a8 from literal 0x1b61c. Field 0x0c in record 0 resolves the observed 160-byte
output's source. Fields 0x04/08/10 in record 0 resolve outer `11ffffff`
wrappers with the observed LZ77 streams four bytes later. Their decoded packet
formats are described below. The contiguous 162-record metadata block and its
selector contract are now verified below. Linked payload/program formats,
other graphics catalogs and language completeness remain to be established.
Do not infer catalog or language completeness from equal initial outputs.
Scratch output preserves packet headers; the typed body decoders below preserve
ordered writes rather than declaring those outputs to be flat tile images.

## Authenticated descriptor metadata

`gba_core::rom_data::bg_descriptors::Catalog::from_rom` accepts only the exact
supported images and checks each original root literal plus the descriptor
source hash. The block is 162 records of 88 bytes, 14256 bytes total, at USA
0x569104/EUR 0x7662a8. Its SHA-256 is
2766525e0889e0ec2f3646dda7a95c143fe68931ee19923b6808b5d989519ce6.
Every mandatory and optional relative link in this block is outside the
descriptor region and within the image. Each active transfer has both sources
and an aligned VRAM destination. The lowest referenced payload starts exactly
at relative 0x37b0; treating that payload as another descriptor fails the
mandatory source bounds. This establishes the supported contiguous block,
without asserting that it contains every graphics asset or catalog.

The typed model preserves all 88 bytes:

- Offsets 0x00/04/08/0c/10 are root-relative sources: the first graphics producer,
  paired-plane packets, halfword-plane packets, palette and auxiliary patches.
  The first source's full format remains unclassified.
- Offsets 0x14/18/1c form an optional control-source/data-source/absolute-VRAM
  transfer triple. Four further triples use the arrays at 0x20, 0x30 and 0x40.
  A zero triple is absent. Payload lengths and transfer-program semantics are
  separate work; checking link addresses does not prove complete transfer bounds.
- Offset 0x50 is an optional root-relative variant-program source. Original
  readers both test its presence and add it to the root; it is not a boolean.
- Bytes 0x54/55 are conditional selectors, not dimensions. The original getter
  chooses the second when the environment byte at EWRAM 2fc2 has low bits 1 or
  2 and that selector differs from ff; otherwise it chooses the first. Their
  game-level names remain unverified. Byte 0x56 indexes alternative palette
  tables in original loaders. Byte 0x57 is retained without assigning a meaning.

`Descriptors::decode/validate` bounds record count at 512, rejects partial/empty
records, inward/out-of-image source links, partial transfer triples and invalid
VRAM destinations. `source_bytes()` reconstructs the metadata exactly; it does
not reconstruct linked assets. Serde rejects unknown keys, while the explicitly
unclassified metadata bytes retain their source values.

Three synthetic tests pass on Linux and Windows GNU/Wine for typed roundtrip,
selectors/fallback/low-bit masking and malformed counts/links/transfers. Four
exact-ROM cases also pass, one per region/target. They reconstruct the source,
refuse an extended prefix and an edited base ROM, and produce byte-identical
106309-byte TOML files (SHA-256
e408f8122bc7689e02c72d6e5844b6176d217f25d921afc35a8f2980b7c60f81).
Each case executes 2612 original selector calls/134629 instructions: all 162
IDs, eight environment values covering upper-bit aliases, direct/current-ID
calls, ff sentinels and a post-authentication in-memory ff-fallback probe.
All EWRAM remains read-only. The fallback probe restores its changed ROM byte;
edited base data remain rejected. There is no boot replay or guest-code copy.

Evidence: ignored `target/rom-data-bg-descriptors/` structural-boundary.json,
core/runtime build metadata/logs, native/windows results and original-selector
logs, four private TOML/report directories and verification-summary.json.
The initial standalone files above were private test outputs. The later managed
integration below publishes the same metadata under its explicit file budget
and dataset revision. All derived data stay outside distribution.

## Managed descriptor extraction and preparation

`rom-data extract|verify|prepare` now handles the descriptor file together with
OBJ dimensions. Schema stays 1, extractor/decoder become 3/2, selecting
`v1/e3-d2` while retaining earlier generations. The three fixed names are
`manifest.toml`, `obj-dimensions.toml` and `bg-descriptors.toml`. Manifest and OBJ
keep 16 KiB caps; BG gets 256 KiB and the whole set gets 272 KiB. No manifest
path controls file access. All files are serialized within budget, synced and
read back before atomic publication or selection. Reuse verifies the complete
set and performs no writes; missing/corrupt BG data trigger a fresh generation.

The manifest pins both family IDs, formats, hashes and source ranges. Typed BG
values must reproduce the actual authenticated metadata, even if an edited
file's hash was recomputed. Activation checks both files before changing the
machine, and every prepared resource ID/file hash contributes to dataset
identity. Equal semantic BG data with a comment and a new file hash changes
dataset identity while preserving OBJ file identity and guest snapshots.
The managed unit authenticated BG but did not yet consume it. The subsequent
[direct runtime unit](#direct-descriptor-consumption) now retains both typed
families and serves BG source reads from file-derived data. Preparation alone
was not treated as guest-use evidence.

Affected release checks pass: Linux2core extraction+4core runtime+7file+10
preparation cases, Wine2+4+6+9; two Unix symlink checks explain the difference.
Two further private tests per target cover both exact regions, actual command
handlers, source-authenticated activation and existing Load/Undo/backup restore.
Rehashed edits/missing BG leave the live object and guest snapshot intact.
All resource hashes participate in identity. The version-isolation test imports
a real retained e2-d1dataset and proves it stays unchanged; no legacy fixture
is regenerated. Simulated partial BG writes, three-file process-death cases, size/total-budget,
concurrency, corruption/missing repair and relocation cases pass.

Production CLI builds are warning-free. Each actual Linux/Wine executable passes
18checks from unrelated working directories: help, regional first extraction/
reuse/verification, managed create/no-write reuse, corrupt/missing BG recovery,
resource-root move/reuse and rejection/retention of the EUR legacy dataset.
Three output files are byte-identical across targets for each region;
logical set size is107861bytes. Two synthetic tar/zip archive scenarios also
exclude new BG markers beside selected package inputs. Canonical executables
are restored by SHA-256 and source mtime refreshed for later Cargo builds.
No archive rebuild, export or AndroidPort change is inferred.

Private evidence: `target/rom-data-bg-managed/verification-summary.json`, core/
file/preparation/regional/lifecycle logs, production build logs/candidates,
native/windows results, imported old-generation copies, three-file datasets,
CLI operation logs/results and package marker log. Native Windows/physical
power loss, combined-dataset GUI and new startup/frame costs remain unverified.
The previous OBJ scene/performance/GUI/archive outcomes are retained at their
old revisions; no unchanged scene is replayed just for context.

## Direct descriptor consumption

ROMDATA-04-BG-CONSUMPTION retains the authenticated typed metadata and a
14256byte exact source projection in the immutable session. Fixed ROM-page
checks coverUSA569..56c/EUR766..769alongside the four OBJ pages. CPU reads,
DMA and RtApi loads use the projection; neighbors, mirrors, GPIO/EEPROM and
open-bus ordering retain their existing behavior. No hot I/O/allocation,
original instruction/decompression skip or new runtime lookup by manifest is
introduced. Linked payloads and programs still use ROM.

Schema/extractor/decoder remain1/3/2:the file contract is unchanged. Source,
save/native-cache identity, RtApi and state schema1 stay compatible for these
source-equal base data. Distinct BG activation/file/dataset diagnostics and
bus-operation/projected-byte counters expose actual use. Owned limits were
set before testing:shared/OBJ<=1KiB including Arc, additionalBG<=64KiB.
Both targets measure440+28672=29112bytes. Allocator/RSS/startup/frame-time costs
are separate05work, not inferred from type-size accounting.

Linux and Windows/Wine release suites pass204core tests each (7optional
ignored) and98/97frontend defaults (18optional ignored). Three new synthetic
core cases cover every byte/width/mirror/boundary/native callback, CPU loads,
16/32bit DMA, guest timing and transactional snapshots with poisoned raw BG
source bytes. The shared bus change is the reason for the full core suites.

Four real-file regional cases reuse existing e3datasets and compile only the
original selector/query functions through the production decoder/emitter/
TinyCC loader. Each executes2608calls/134424original instructions, compares
registers/CPSR/clock/EWRAM/IWRAM at native boundaries and every interpreter
step, and ends with equal complete snapshots after restoring only the test's
raw-ROM poison. Each reports3888BG reads,3867from translated byte-load
callbacks,5836native dispatches and660legitimate event-boundary interpreter
shells. These are actual native loads, not an API-only synthetic claim.
Sentinels and all162records/eight environment values/direct-current arguments
are covered; the earlier offline fallback counterfactual is not rerun.
Load/Undo/backup preserve the immutable owner/counters; corrupt/missing new
activation fails transactionally, while current reads survive removal of the
BG file from the directory used to activate that owner.

Two new EUR native1200-frame clean boots independently match every full state,
central/wide image, OAM, dispatch and complete mixed/PSG/FIFO/HLE stream, with
53BG reads and all five audio streams nonempty. HLE compares float bits.
Linux6479719/Wine4067051native blocks execute, with each platform's existing
TinyCC ELF object. No cross-target dispatch/timing equality is inferred.
Actual warning-free production executables pass updated help, interpreter
1200-frame demo pairs and native CLI1200-frame idle/explicit-demo pairs. Each
pair matches its legacy framebuffer hash; both native pairs also match final
full-state hashes. Demo interpreter/native runs each report53BG reads. With no
input,runc correctly reports0; its explicit demo script is required to leave
the initial screen. The first CLI helper incorrectly demanded positive reads
without input; its completed frames/idle runs are retained, and only the new
input pair is executed. Existing e3files are reused without writes during that
pair; canonical/ROM/library hashes match. No extraction fixture is recreated.

The frontend helper initially omitted dispatcher frame-ready clearing and
stalled at a legitimate event yield. Only that harness was corrected; the
negative output is retained. Wine's background D-Bus diagnostics coexist with
successful test exits; no production workaround is introduced.

Private evidence:target/rom-data-bg-runtime/verification-summary.json,
core/default/region/native-boot logs, result.txt reports, bounded component
objects, frozen test/build metadata, source hashes and production candidates.
No game code/data or these artifacts enter Git/packages. Broad BG scene/GUI/
cost/archive coverage, linked asset/program formats, other catalogs/languages,
normal enablement, native Windows and mods remain open. Old OBJ outcomes
retain their frozen revision scope; they were not rerun for context.

## Readable JSON datasets

The current managed contract is schema1/extractor4/decoder3 at `v1/e4-d3`.
`bg-descriptors.json` uses `bg-descriptors-json-v2` and standard two-space
pretty JSON; manifest/OBJ remain TOML. Direct typed deserialization avoids a
whole intermediate document tree. The model, exact source projection, family
ID, bus/native/state behavior and immutable session owner are unchanged.
Earlier e3 TOML generations and e1/e2 remain in separate directories, unmodified.
No format fallback silently interprets a previous dataset as the new revision.

The user explicitly removed space-minimization as a priority. The one-space
prototype was therefore not adopted. Current BG output149261bytes and complete
generation150818bytes are kept for readability; former128KiB disk performance
budgets are historical observations, not reasons to pack/compress the data.
Parser input bounds remain16KiB manifest/OBJ,256KiB BG and272KiB total as
malformed-input guards. Previous e3 RSS results remain recorded; no extra
benchmark/RSS campaign is run solely to satisfy the old space target. The
isolated parser heap result is not a claim about current whole-process RSS.

Release affected checks pass32Linux/30Windows-Wine including private cases:
2core extraction+7runtime+8/7file+10/9preparation defaults, regional command/
identity/old e2 preservation, original Load/Undo/backup, actual old e3 TOML
retention and two regional JSON-native provenance/lifecycle tests per target.
The new parser rejects unknown/duplicate/missing fields, invalid UTF-8,
truncation/trailing data, numeric overflow, bad arrays and empty records.
Rehashed semantic edits still fail source authentication. Output is deterministic
and typed source bytes match the previous catalog. Native component cases retain
2608calls/134424instructions,3867native BG callbacks/3888total reads each.

Both production builds are warning-free and pass18actual CLI checks per target:
regional extract/reuse/verify, managed create/no-write reuse, corruption/missing
repair, move from unrelated cwd and prior e2 retention/refusal. New e3 retention
uses the actual existing three-file fixture; no old dataset is regenerated.
Current output files are byte-identical across targets. Actual interpreter and
native CLI1200-frame demo pairs match legacy framebuffer hashes, and native
final full-state hashes also match; both report53BG reads. Ready-generation
file bytes/mtimes, ROM, inputs, native caches and canonical runtime hashes stay
unchanged. Two tar/zip marker scenarios exclude both old TOML and new JSON data.

Evidence:target/rom-data-bg-json/verification-summary.json, final build/test/
regional/revision/provenance logs, actual three-file datasets, CLI/runtime
results, protected hashes and package marker log. No guest ABI/state/native-key
change, compression, archive export, AndroidPort update, agent commit or push.
Broader BG GUI/archives, actual payload/catalog migration and native Windows
remain separate work; completed scene/timing results are not rerun for context.

## Base background palette catalog

ROMDATA-01-BG-PALETTES inventories every palette link in the authenticated
162-record descriptor block for each exact region. There are127distinct
sources each:125LZ77tag10and2tag0raw blocks. Record5uses one raw block;
records54/56share the other. Unique source extents total15612B excluding
alignment padding; decoded payloads total23008B/11504little-endian u16words.
All source/decoded hashes and alias membership match across regions.
Sizes64/128/160/192/224/256B are multiples of32, retaining groups of16words.
476unique-source words (480in the record-indexed catalog) have bit15set;
retain all16bits and do not flatten them to RGBA/alpha or mask flags.

Original producer entries are independently located atUSA0801a42c and
EUR0801b59c. With environment byteEWRAM2fc2low bits0/3, the producer reads
field0c's relative source and checks its first byte. Tag10calls the existing
LZ77wrapper; the raw branch skips the4Bsize header and calls CpuSet for
size/2halfwords. All known raw sources have tag0; other tags are not declared
supported by this catalog. The current-ID argumentFFFFreadsEWRAM7f10.
Environment low bits1/2instead select distinct alternate-palette program
tables via descriptor byte56. Those programs remain unclassified, not
silently treated as base palette colors.

Four original execution oracles pass:each region on Linux and Windows/Wine,
162records×4base environments×2direct/current arguments =1296calls and
72792original instructions per case. All output atIWRAM3860matches the strict
LZ77decoder or validated raw slice, with complete surrounding guards, unchanged
EWRAM and restored SP. High bits are preserved exactly. Record0also matches
the four existing original160Bboot captures without a new boot replay.
No complete clock/native/scene equivalence or runtime palette replacement is
inferred from this producer-output oracle.

Evidence:target/rom-data-bg-palettes/verification-summary.json, header-inventory,
per-region source/output hashes and private decoded buffers, exact original
producer disassembly, warning-free test builds and four oracle logs/results.
That inventory unit added only cfg(test) code, with no producer/HLE change.
The typed model and subsequent runtime integration are recorded below.
Alternate programs, other catalogs/languages and the remaining matrix stay open.

## Typed palette extraction and preparation

ROMDATA-02-BG-BASE-PALETTE-DATA adds `bg_palettes` with completeu16banks of16
words and record-indexed palettes. Both raw tag0and LZ77tag10have exact
consumed extents, excluding padding. Decoded sizes must be nonzero multiples
of32B, at most512B/16banks; truncated/unsupported sources and malformed banks
are rejected. Preserve high bits and all bank/index order, without RGBA/alpha
conversion. All162records are decoded, including127source aliases; no
compression/deduplication is needed for file size.

The completed preparation unit used schema1/extractor5/decoder4 at `v1/e5-d4`
with four fixed files:
manifest/OBJ TOML, BG descriptors JSON and `bg-base-palettes.json` using
`bg-base-palettes-v1-json`. Palette resource ID is `graphics.bg-base-palettes`,
dependent on descriptor IDs. Its162manifest provenance entries preserve every
record's source offset/extent/hash/role, including aliases. Every typed bank
matches independently decoded source and retained original output. Rehashed
palette edits remain invalid base data. All IDs/files participate in one
source-authenticated dataset identity through a borrowed complete-file view.
A semantic-equal whitespace change to the palette file changes dataset identity
while OBJ/BG identities and complete guest snapshots remain unchanged.

Input bounds64KiB manifest/16KiB OBJ/256KiB descriptors/512KiB palettes/768KiB
total protect malformed data; the manifest bound grows for palette provenance.
Readable palette JSON is270075B and complete USA/EURsets446114/446120B.
No space tuning or extra budget campaign follows the user's preference.
Four-file write/sync/validation precedes atomic publication/selection; partial
palette writes, cancellation/death and missing/corrupt repair preserve committed
or prior generations. Actual old e4three-file generations remain intact and
are refused by the new full-set verifier; new data use a separate revision.

Affected release defaults32Linux/30Wine and seven private cases per target
pass (39/37total). Three palette-core tests cover exact raw/LZ77consumption,
endianness/high bits and malformed headers/sizes/tags/banks. JSON/source tests
reject overflow, unknown fields, bad arrays/counts, missing files, forged paths/
dependencies/ranges and rehashed edits. Regional data equal retained buffers
for all162records. Four original producer oracles now consume the new typed
model as expected data and retain1296calls/72792instructions each, matching
all output/guards and unchanged EWRAM/SP. The original producer code/control
has not changed. Existing native BG metadata provenance and Load/Undo/backup
checks also pass with the expanded complete-file API.

Both actual production builds are warning-free.22CLI checks per target pass
regional extraction/reuse/verification, creation/no-write reuse, BG and palette
corruption/missing repair, unrelated-cwd relocation and old e2refusal/retention.
All files match across targets; earlier OBJ/BG file bytes are unchanged. Actual
interpreter/native1200-frame pairs match legacy framebuffer hashes and native
final full states, with53BG reads. These prove unchanged existing consumers;
palettes explicitly report `prepared ... runtime=rom`, not file use by gameplay.
Source/input/cache/canonical hashes and ready-generation mtimes are protected.
Tar/zip marker scenarios exclude the new palette data, without a public export.

Evidence:target/rom-data-bg-palette-data/verification-summary.json, source
preimages/hashes, core/default/private/regional/oracle/provenance logs, four-file
CLI datasets/results, runtime pairs, old revision imports and package marker
log. This was the preparation-only outcome; the subsequent consumption unit
below establishes runtime use and its distinct verification scope. Alternate programs, other catalogs/languages and
the remaining resource matrix remain open.

## Direct base-palette consumption

ROMDATA-04-BG-BASE-PALETTE-CONSUMPTION retains the authenticated model in the
immutable session. Palette v2JSON adds `encoded_source`, the exact consumed
original header/stream bytes beside banks. No recompression is performed:
unused flag bits and byte reads must remain source-equal, including for guest
BIOS code. Model validation decodes this metadata, checks exact consumption
and requires bank equality; source authentication separately rejects coherent
re-encodings with the same decoded output. Schema1/extractor6/decoder5 selects
`v1/e6-d5`, retaining e5/e4 and all earlier data. Four filenames and semantic
resource IDs stay unchanged. Current input guards are64KiBmanifest,16KiBOBJ,
256KiBdescriptors,1MiBpalettes and1280KiBtotal. Standard readable palette JSON
is513639B; complete sets689678USA/689684EURB. These bounds protect malformed
input; no storage tuning follows from them.

All CPU/DMA/native reads in verified palette spans come from files. Fixed asset
pages keep catalog searches away from ordinary code reads. Unique sorted source
bindings preserve aliases and crossing neighbors. Raw payload bytes derive from
decoded banks; headers/compressed bytes use the exact source representation.
The BIOS LZ77HLE returns an owned borrow of decoded output only for verified
LZ77source starts/mirrors. It calls the unchanged ordered `write_out`; raw
CpuSet, other ROM and dynamic RAM retain their loops. Source ROM reads here
have no device/clock effects and the same SWI/instruction cost/return hook runs.
No guest instruction is bypassed, no native ABI/cache or snapshot revision
changes. Exact real-BIOS source views are verified, not a BIOS-ROM execution.

Measured owner bytes are448shared/OBJ+28672BG+94713palette=123833total on both
64-bit targets. No per-read allocation/I/O or new palette memory budget is
introduced. Resident accounting is distinct from allocator overhead and RSS.

Release checks:210core each and101Linux/100Winefrontend defaults pass; three
new core cases verify widths/mirrors/edges, native APIs, DMA16/32, raw CpuSet,
SWI11/12toRAM/paletteRAM/VRAM/OAM, fallback and snapshot/ownership behavior.
Four regional original/native cases each pass1296calls/72792instructions /
4208native dispatches with entire original palette source spans poisoned after
activation. Full states match; each owner records1272LZ77calls/231936decoded
bytes and3544source bus reads/5720source bytes. Missing/corrupt activation
preserves the owner; deleted files do not prevent subsequent use or Load/Undo/
backup clean boot. Current regional extraction also matches all retained
original outputs and passes four-file repair/identity/retention checks.

New EURnative1200-frame pairs pass every full-state/image/OAM/dispatch/audio
comparison on Linux/Wine with one160Bdecoded palette and53BGreads. Actual
production CLI checks add e5refusal/retention to extract/reuse/verify/repair/
move; exact source views and banks match across targets. Canonical executables,
ROMs, native libraries and old datasets remain hash-identical. New actual
interpreter/native1200-frame pairs show positive palette use and match legacy
frame/final-state hashes with no dataset rewrite. No benchmark speedup or
broader scene/GUI/archive/native-Windows claim follows from these checks.

Evidence:target/rom-data-bg-palette-runtime/verification-summary.json, core and
frontend logs, four producer reports, regional four-file datasets, CLI/runtime
pairs and protected hashes. Alternate palette programs, other catalogs and
languages, remaining05coverage and the rest of the resource matrix stay open.

## Alternate background palette catalogs

ROMDATA-01-BG-ALTERNATE-PALETTE-PROGRAMS classifies the valid alternate streams
linked by descriptor byte56. Independent original literals select USA427a04/
42cc80and EUR489bb0/48ee2c. Environment low bits2select the first catalog;
low bits1select the second. Each has157little-endian relativeu16links keyed
0..156(314Bplus2Balignment before the first stream). The shared original
consumer is USA0800543c/EUR080054a0, with destinationr0and sourcer1.
There is no evidence for game-level names such as day/night.

All157keys occur once in the162descriptors; records29/31/144/145/146instead
containFF. The original producer does not locally guard FF and indexing it
reads beyond the verified table, yielding nonsensical size headers. Preserve
that unavailable marker; do not dereference or invent fallback content. The
callers' exclusion/precondition for those five records is still unknown.

The valid314streams per region each decode256bytes (128fullu16words/eight
16word banks), with39933consumed source bytes and80384decoded bytes total.
Their exact source/output hashes and both relative-link tables match across
regions. No source ranges overlap within a catalog; consumed lengths exclude
padding. This is a distinct custom LZ/RLE format, not BIOS LZ77. A BEu32output
length precedes tokens. Token classes are literal data, short/medium/long
back-references and short/long zero runs; an FFfill branch exists in code but
is absent from these valid streams. Retain allu16bits and source semantics.
A bounded private decoder establishes the inventory hypothesis; a production
parser and malformed-input contract remain the next02unit.

Four original interpreter cases (EUR/USAonLinux/Wine) each pass1256calls /
2655156guest instructions:157valid records×four environment bytes1/2/FD/FE×
direct/current-ID arguments. Each call enters the independently verified
consumer with the expected source/destination, produces the complete expected
256Bwith surrounding guards, leaves EWRAM unchanged and restores SP. The five
FFrecords are deliberately not invoked. These are functional output/instruction
checks, with no performance measurement or native/runtime replacement claim.

Evidence:target/rom-data-bg-alternate-palettes/verification-summary.json,
regional producer/consumer disassembly, exact literals/calls, source and decoded
inventories/private buffers, bounded decoder hypothesis, warning-free test
builds and four original oracle reports. Only an ignored cfg(test) module and
registration are added. No active dataset revision, production decoder, native
ABI, save schema, runtime/archive or canonical executable changes in this unit.
Typed decoding/managed extraction/consumption, caller preconditions for FF,
other catalogs/languages and remaining GUI checks stay open.

## Decoder contract and checks

`gba_core::rom_data::lz77::decode` accepts a bounded input slice and explicit
output budget. It requires type 0x10 and a nonzero 24-bit decoded size, checks every
input read, bounds allocation before decoding and rejects references before
the output history. Overlapping matches and a final match clipped to the
declared size follow the existing BIOS HLE contract. The result contains exact
decoded bytes and consumed header/stream length, leaving trailing padding to
the caller. The BIOS HLE itself retains its existing malformed-reference
behavior; strict offline decoding refuses such data rather than treating it
as a supported asset.

Four synthetic tests pass on Linux and Wine: literal/overlapping matches with
exact consumption and guard bytes against both BIOS 0x11/0x12 variants, partial
final tokens/flag groups, all truncated prefixes/invalid history/type/size and
the maximum 4096-byte history distance. Both target test builds are warning-free.
Separate optional tests use the already captured traces and destination dumps
to check all eight regional source ranges against actual guest outputs, on
both targets, with a 2 MiB output budget. No boot replay is required for these
offline decoder checks; original ROM, traces and outputs remain unchanged.

Private evidence lives in ignored `target/rom-data-graphics/`: per-target build
logs/executable metadata, core decoder test logs, regional calls.toml and output
captures, lz77-verified.toml reports, cross-target checks and helper results.
An initial observer compile typo was corrected before tracing. A stale native
Cargo library tracked the previous isolated measurement source rather than
the edited checkout; its original fingerprint/logs are retained, and only the
affected library fingerprint was invalidated. Successful traces and earlier
measurements were reused, with no reference-source mutation.

## Typed background packets

`gba_core::rom_data::bg_packets` decodes the next stage after LZ77. The caller
supplies separate destination, cumulative written-byte and packet budgets.
Parsing checks every read, alignment, counts, dictionary references and budgets
before allocating each packet. Repeated writes retain their original order;
this is a write program, not a replacement for mutable RAM. Terminal offsets
and consumed stream lengths are distinct from padding and dictionary bytes.

The halfword format begins with a destination byte offset and a byte count,
both little-endian u16, followed by literal u16 values. A dictionary variant
uses a u8 count and u8 indices into a separately supplied halfword dictionary.
Offset zero is valid for the first packet. A zero *next* offset terminates the
stream. Odd byte counts and misaligned/out-of-budget writes are refused.
The captured 456-byte body has 14 packets/196 values, consuming 450 bytes before
six padding bytes. Original USA 0801f098/EUR 08020208 apply it at EWRAM 7cb0.
The 1996-byte body has 112 packets/771 values, consuming 1992 bytes before four
padding bytes. Original USA 0801efb0/EUR 08020120 apply it at EWRAM d1a0,
through the original staged-pointer mechanism. These destinations are proven;
the small plane's complete gameplay meaning remains unclassified.

The paired-plane body has a u32 header. Bit 31 selects a dictionary at the
low-28-bit relative offset; bit 30 selects u16 indices instead of u8. Each
indexed packet has a u16 destination offset, u8 count and its indices. A clear
bit 15 in a dictionary halfword implies the pair `(value, value+1)`; a set bit
selects an explicit second halfword with both marker bits cleared. This yields
ordered u32 pairs. The captured 2960-byte body has 107 packets/770 pairs,
consumes 1867 stream bytes and references the dictionary at 1872. Its flags
select u16 indices; it writes into the combined calculated planes at EWRAM
91a0..d1a0. Original USA 0801ed24/EUR 0801fe94 give the same writes with unlimited
processing and with a 2048-byte budget requiring two original invocations.

Literal u32 packet syntax is retained, but its destination semantics have a
separate guard: `calculated_plane_writes()` accepts only indexed packets.
The first synthetic literal application failed the predicted-plane assertion.
A subsequent recorded-write probe independently confirms that the original
isolated literal branch attempts `(address0, value)` and leaves the calculated
planes unchanged. This is retained as a negative result on both regions and
targets. It does not justify changing guest code or applying those words at an
assumed plane address. A real producer context is needed before enabling that
destination interpretation.

Three synthetic core tests pass on Linux and Windows GNU/Wine, covering literal
and indexed payload syntax, both index widths, implicit/explicit pairs, overlap
order, padding/terminators, all truncated prefixes and invalid offsets, lengths,
dictionary references and budgets. The optional original-code test also passes
for both exact regions on both targets: seven original invocations and 87160
instructions per case, using the saved post-LZ77 bodies plus original synthetic
fixtures. It compares all EWRAM at component completion, checks original chunk
continuation and narrow indices, and records the literal branch's region-0
write. The negative probe observes CPU/bus instructions without advancing the
hardware clock; it is not a timing oracle. Other cases use normal Machine steps.
No complete CPU/IRQ/audio/scene equivalence or runtime migration is inferred.

Evidence: ignored `target/rom-data-bg-packets/` build metadata/logs, core test
logs, per-target regional original-packet logs, protected fixture/canonical
hashes and verification summary. The failed pre-guard probe is retained under
`before-literal-destination-evidence/`; its giant assertion output was replaced
with a bounded first-differences report. Exact original callers, packet/setup/
writer disassembly and regional root literal references remain private under
`target/rom-data-graphics/`. Neither original instructions nor resource bytes
are embedded in tracked sources; public fixtures use original synthetic data.

## Reproduction and remaining work

Build the `recomp` test executable using the existing Windows/native build
setup. With `FFTA_STATE_TEST_ROM` selecting an exact supported image,
`FFTA_WIDESCREEN=0`, `FFTA_STATE_TEST_FRAMES=1200` and an absent
`FFTA_STATE_TEST_TRACE_ROOT`, run the ignored test
`rom_data::compression_tests::observe_boot_graphics_decompression` with
`--exact --ignored --test-threads=1 --nocapture`. It requires an explicit
1..12000 frame count and refuses an existing output directory.
Then use that existing trace root and ROM with
`rom_data::compression_tests::decode_observed_lz77_matches_guest_outputs`.
The latter refuses to overwrite its completed verification report.
The four self-contained core tests use filter `rom_data::lz77::tests`.
The three packet tests use `rom_data::bg_packets::tests`. To compare the original
packet writers, select the same private ROM and existing trace root, then run
`rom_data::bg_packet_tests::decoded_packets_match_original_regional_writers`
with `--exact --ignored --test-threads=1 --nocapture`. No boot replay is needed.
Descriptor core tests use `rom_data::bg_descriptors::tests`. The optional
`rom_data::bg_descriptor_tests::descriptor_catalog_matches_source_and_original_selectors`
uses the exact private `FFTA_STATE_TEST_ROM` and an absent
`FFTA_STATE_TEST_CATALOG_ROOT` for its local TOML/report output, with the same
ignored-test flags. It also needs no boot replay.

Authenticated descriptor source reads now consume the managed file-derived
session projection. Current scene/cost validation remains separate from
establishing linked asset/program
formats, other catalog bounds, sharing and language coverage for the remaining
graphics migration.
No BIOS call, guest instruction, cycle, DMA/IRQ, audio path, save/native
identity, current dataset schema/revision or default startup behavior changes.
This work gives no decompression speedup or native Windows result. Windows
hardware is unavailable to the user; Linux/Wine remain distinct evidence.
