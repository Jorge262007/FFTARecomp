# ROM data inventory and contract

Status: ROMDATA-01, 21 October 2026. This is an inventory of verified entry
points and explicit unknowns, not a claim to have located every asset. Exact
EUR and USA images are identified by `ffta_rom::{EUROPE_SHA256,SHA256}`.
Offsets below are file offsets unless prefixed with a guest memory region.
Do not apply a regional offset to another image or infer language coverage
from a shared rendering format.

## First migration unit: graphics.obj-dimensions

The first family is the static OBJ dimension metadata, not sprite pixel data
or animation. Twelve records in shape-major, size-minor order contain two
little-endian u16 values, width and height in pixels. The range is 48 bytes,
uncompressed, with no interior pointers or dependencies. Invalid shape 3 is
outside the resource; guest memory beyond this table must keep its old behavior.

The USA draw routines at 0x080014c8 and 0x080016d4 read the table at
0x149510. The USA copied RAM sprite/visibility routines use a second table at
0xa398dc, copied to IWRAM 0x03006d28. EUR draw routines at 0x08001500 and
0x0800170c use the direct ROM table at0x14ced8; their host decoder's old IWRAM
selection was corrected against the original instructions during ROMDATA-04.
EUR also contains an identical table at0xd976e8 in the copied-code source area,
with the same relative source/RAM offset as the copied code. Both ranges are
mapped. The protected EUR boot-to-battle replay now observes actual alias reads
at frame5400 (primary first read541), with every full guest/RAM state matching
legacy reads through8476frames; see the
[validation protocol/results](ROM_DATA_VALIDATION.md). This is per-frame source
coverage, not attribution to a particular DMA/PC. An independent USA clean-boot
trace now reaches the original copier at0x0808f74c, frame791. Its own literals
at0x8f764/68/6c specify source0xa38adc..0xa3991c and IWRAM0x03005f28:
3648bytes, including the48byte alias at relative0xe00. Original CpuSet SWI
at0x0814186c performs24decoded halfword reads; all13initializer steps match
legacy full states. Independently inspected EUR entry0x080924e8 and literals
at0x92500/04/08 specify source0xd968e8..0xd97728 and the same destination;
the EUR replay above does not attribute execution to that entry. These blobs
include executable code, which remains in ROM; only48metadata bytes migrate.
Existing consumers and original execution are described in
[sprite routines](../ffta_sprite_routines.md) and implemented in
`ffta_sprites::{decode,decode_battle,visibility_result,ffta_sprite_interpret}`.
Both exact local regional images have been read independently: all four
48-byte ranges have SHA-256
`6ff886b23cde2726a4ceb9cba8c5569bbb4dc6a1185769547f78dfbcb7e2d991`.
Both copied aliases are checked independently, not assumed equal to the primary.
Exact-byte occurrence searches find two copies per region. Extractor revision2
records the newly discovered EUR alias; schema/decoder remain1.
The format is language independent; this does not validate localized assets.

Decoded files retain indexed dimensions rather than a rendered picture.
Host readers must preserve any genuinely changed RAM dimensions; a static
resource cannot replace dynamic RAM without checking the consumer contract.
CPU/DMA/interpreter/native source reads and typed host consumers now use the
decoded files with --rom-data. No original instruction or decompression is
skipped. [Runtime evidence](../rom_data_runtime.md) distinguishes conditional
guest reads from ordinary in-view boot/host use; broader gates remain05.

## Other families and migration matrix

Each entry names the known decoder/runtime reader, static versus dynamic
boundary, regional evidence, remaining extraction work and residual ROM use.
BG descriptor metadata and base palettes now have managed extraction and
direct consumption as detailed below. The bounded generic LZ77/packet decoders
remain groundwork for other linked graphics families.

- **BG tiles, palettes and image graphics.** `ppu.rs` reads hardware VRAM
  (4/8-bpp indexed tiles and bitmap modes) and palette RAM (BGR555). Those are
  dynamic destinations, not ROM asset boundaries. CPU copies, DMA and BIOS
  HLE feed these regions. `hle.rs` implements LZ77, Huffman and run-length
  decoding, but support for a codec does not locate a game's compressed blocks.
  New1200frame clean boots independently observe four LZ77 sources per region:
  USA56f4a8/5704cc/56fefc/56f52c and EUR76c64c/76d670/76d0a0/76c6d0.
  Consumed lengths125/133/1481/2507decode to160/456/1996/2960bytes, matching
  actual IWRAM3860/EWRAM3cb0post-SWI outputs on Linux and Wine. Original callers
  independently reference88byte descriptors atUSA569104/EUR7662a8; record0
  field0c resolves the first source. Fields04/08/10in record0point to outer
  wrappers containing the observed LZ77 streams. Typed halfword and indexed-pair
  packets now match actual regional writer instructions on Linux/Wine, including
  staged/chunked application. Literal pair syntax retains a destination guard
  after a contradictory original-code probe. The contiguous162-record descriptor
  block is now source-authenticated/typed, with full metadata roundtrip and
  original conditional-selector checks for all records on both targets. Relative
  sources, optional transfer triples/variant source and tail bytes are retained.
  Linked formats/programs, other catalog bounds, sharing and language completeness
  remain unverified; this is not a complete graphics catalog.
  [Decoder contract, exact evidence and remaining work](../rom_data_graphics.md).
  Managed descriptor extraction/preparation: implemented, v1/e6-d5; bounded
  offline LZ77/packet decoders also tested. Authenticated BG descriptor reads
  now consume a file-derived session projection through CPU/DMA/native APIs,
  with independent counters, real-file regional provenance and native boot
  parity. Graphics payloads still reach VRAM/palette through original producers.
  The complete base-palette links now identify127unique sources per region
  (125LZ77,2raw), matching source/decoded hashes across regions and original
  producers for all162records on Linux/Wine. Preserve allu16bits, including
  bit15. Alternate environment streams now have a separate verified catalog:
  157valid keys in each of two regional tables,314streams/256Boutputs per region,
  custom LZ/RLE with BEsize header. All source/output hashes agree and original
  producers match on Linux/Wine. Five FFdescriptor keys are unavailable;
  caller preconditions remain unknown. Typed production decoding, managed
  extraction and runtime consumption of alternates remain open. Base words now have typed readable extraction/preparation and direct04
  consumption: decoded banks for palette HLE, exact source views for CPU/DMA/
  native/guest BIOS reads, with original writes/cycles and state parity. Needed:
  indexed pixels, palette identity/indices, transparency, tile order and all
  writes/timing, not only screenshot similarity.
- **Terrain maps and movement planes.** `ffta_stage.rs` observes DMA and
  verifies two calculated 64x64 tile planes in EWRAM, then checks their VRAM
  crop. `ffta_movement.rs` observes a separate dynamic movement plane.
  `ffta_rom::map_producers_verified` independently pins producer code
  0x1af08..0x1b5d0 for each region; this is code, not a map archive. Static map
  descriptors, terrain geometry, pointer roots, compression and cross-map
  sharing remain unknown. Extractor: absent; decoder/runtime: dynamic plane
  observers and PPU. The calculated planes cannot be exported as static maps.
- **OBJ graphics, descriptor lists and animation.** `ffta_sprites.rs` decodes
  a signed/unsigned count followed by packed six-byte OBJ records, reads tile,
  priority/palette/flip fields and observes partial OAM DMA. Lists can live in
  ROM or RAM; animation, owner state, matrices and OAM are dynamic. Profiles
  and copied routine source ranges are independently hash gated per region in
  `ffta_rom.rs`; these are not a catalog of all lists or frames. Extractor:
  absent except the selected dimension unit; runtime: original producers and
  host descriptor readers. Static list roots, frame timing, graphics extent,
  compression and cross-resource links still need discovery.
- **Text and fonts.** `ffta_dialogue.rs` classifies already rendered layers
  and portraits, not strings, font encodings or localized text archives.
  `ffta_mouse.rs` identifies a name-entry row-count table via independently
  checked regional literals: EUR 0x3ad2fe (three pages, six bytes each), USA
  0x3a8320 (two pages). That table describes navigation widths, not character
  codes or text resource pointers. Extractor: absent; runtime: guest text/font
  rendering and host geometry. Encodings, terminators, control codes, language
  root tables, glyph metrics/bitmaps and compressed bounds remain unknown.
  EUR language selection observations do not verify all five text catalogs.
- **Gameplay tables.** Unit structures at EWRAM 0x020223a4, stride 0x90,
  are live state, not a static unit database; see
  [character metrics](../ffta_character_metric.md). Battle/shop mouse readers
  validate transient tasks/inventory. Static item/job/ability/law/AI tables,
  record layouts, references, code dependencies and regional offsets remain
  unknown. Extractor: absent; runtime: guest logic and semantic host readers.
- **Audio samples, instruments, songs and sequencing.** `engine.rs` detects
  driver families; `mp2k.rs`, `gax.rs` and `rdrv.rs` shadow live voices while
  leaving original mixing/sequencing running. `MemView` resolves ROM/RAM
  ranges, and each driver has its own sample/loop/instrument contract. Driver
  detection is not an exhaustive instrument or song index. FFTA-specific bank
  roots, compressed/sample extents, sequencer commands, language variants and
  references remain to be verified per image. Extractor: absent; runtime:
  guest mixer/DMA and shadow samplers. A PCM export alone loses sequencing,
  pitch, envelopes, loops and original guest timing.
- **Executable code and opaque data.** Native labels and copied IWRAM
  instruction ranges are code, explicitly excluded from decoded assets.
  Unknown regions stay in the original ROM. Do not label a sweep of plausible
  compression headers as a verified asset catalog. Code/data interleaving,
  embedded pointers/constants and opaque regions prevent ROM-free startup.

## Version 1 base dataset contract

The managed set contains `graphics.obj-dimensions`, `graphics.bg-descriptors`
and `graphics.bg-base-palettes`. IDs remain semantic: OBJ index `shape * 4 + size`,
BG record index within the authenticated162-record block; source offsets are
provenance, not resource IDs. All three resources have direct consumption;
other linked payloads/programs remain ROM.

- `manifest.toml`: schema revision, extractor revision, decoder revision,
  exact source ROM SHA-256/size and regional profile; a coverage list naming
  only the decoded families; resource ID, relative file name, typed format,
  decoded-file SHA-256, dependency IDs and all original source ranges/hashes.
- `obj-dimensions.toml`: twelve typed width/height records in index order.
  The file hash covers its exact bytes. The decoder verifies dimensions,
  length, source ranges and that canonical little-endian encoding reproduces
  the source table. No absolute paths, timestamps, ROM bytes or host pointers
  appear in the manifest. Generated assets remain private local files.
- `bg-descriptors.json`: all162typed metadata records with exact source
  roundtrip, validated relative links/transfer triples and retained tail bytes.
  It does not contain linked graphics/program payloads. Its source range is
  independently authenticated for each profile; file hashes cannot authorize
  edited base semantics. All three resource files contribute to complete dataset identity.
- `bg-base-palettes.json`: all162record-indexed base palettes as fullu16banks,
  with source aliases preserved in162manifest ranges and a dependency on BG
  descriptor IDs. Known source codecs raw0/LZ7710are bounded and exact extents
  exclude padding. Preserve bit15and bank/index order; alternate environment
  programs are distinct, unclassified assets. Exact consumed source bytes are
  retained beside banks for compatibility reads; both forms must agree.
  Decoded banks feed verified LZ77 HLE while raw CpuSet uses file-derived reads.
- Schema and decoder identity are separate from source identity and compiled
  code identity. For a base dataset, matching its own declared hashes is
  insufficient: verify the expected source ranges and reconstructed source
  digest against the selected exact image. User edits belong to a future mod
  layer; do not silently accept them as base data.
- Keep fixed names and bounded input:64KiB manifest (including162palette
  provenance entries),16KiB OBJ,256KiB BG descriptors,1MiB base palettes and
  1280KiB total. These guard malformed input, not extracted-data space targets.
  Reject unsupported schema/decoder revisions, duplicate/missing resources,
  bad hashes, unknown fields, truncated/out-of-range data and path escapes.
  Publication must validate all files before making a directory visible.
- Automatic path: portable `Data/resources/<rom-sha256>/v1/e6-d5/`;
  installed uses local application data `FFTARecomp/resources`. Immutable
  `generation-*` directories contain four files; an atomic relative `current`
  file selects one verified generation. Revisions have separate paths and an OS
  lock coordinates writers. `rom-data prepare` and opt-in `play --rom-data`
  implement reuse/recovery, retaining e1/e2/e3/e4/e5 generations; direct OBJ, BG
  metadata and base-palette consumption is opt-in. Other linked payloads/programs
  remain ROM.
  Normal enablement awaits05.
  Native caches and saves remain separate.
- Decoded base data are immutable host resources, shared for a session and
  rebuilt/rebound on load/Undo/clean-boot restore. Do not serialize their
  pointers or duplicate files in `.ffstate`. Existing ROM/BIOS identity remains
  valid only after strict base equivalence is proven. Modified data require
  a separate content identity and compatibility policy before enabling mods.

## Gates before normal runtime use

ROMDATA-02 must first validate synthetic malformed inputs and both local
regional images. ROMDATA-03 needs preparation/reuse/recovery/move tests.
ROMDATA-04 must trace every consumer, demonstrate decoded-file provenance
and make fallback visible. A host-only migration does not close guest reads.
ROMDATA-05 needs deterministic state/image/audio equality, lifecycle checks
and measured startup/memory/frame cost against the existing in-memory ROM.
No speedup, removal of the ROM dependency, complete graphics migration or mod
support follows from this first family.
