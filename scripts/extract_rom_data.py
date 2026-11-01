#!/usr/bin/env python3
"""Extract readable ROM data through the project's source-authenticated CLI."""

import argparse
from pathlib import Path
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
DESCRIPTIONS = {
    'linked-metadata4.json': 'European paired selectors/values; original live qualification scans retained',
    'linked-metadata2.json': 'European stored key/value map; distinct original forward and inverse extents',
    'default-constructor-template.json': 'European default constructor template; original live patches and alternate sources retained',
    'constructor-alternate-templates.json': 'European four alternate constructor records and parsed base pointer; live patches retained',
    'linked-metadata14.json': 'European special-unit fields and packed palette selectors; live books and overrides retained',
    'linked-metadata12.json': 'European indexed packed two-bit fields; original row and mode arithmetic retained',
    'global-pair-flags.json': 'European global packed pair flags; original live mask and early exit retained',
    'evaluation-parameters.json': 'European score coefficients and class thresholds; original live evaluation and RNG retained',
    'progression-rewards.json': 'European progression deltas and discounts; original live state and price routines retained',
    'progression-gates.json': 'European progression requirements and structural receipts; original live mask and selector retained',
    'progression-selection-gates.json': 'European hard and ANY progression selection conditions; original reset, flags and terminal stores retained',
    'aggregate-flag-keys.json': 'European ordered flag keys and terminator; original global/context counters and live predicates retained',
    'trade-parameters.json': 'European five96B parameter records; byte41 alias and original price/live flags retained',
    'item-effect-parameters.json': 'European14packed7B item-effect parameters; original getter/cap index widths and live RAM retained',
    'audio-directory.json': 'European M4A player configuration, song directory and shared headers; original sequencing retained',
    'audio-tones.json': 'European producer-bounded typed tones, keymaps and lossless signed PCM/CGB samples',
    'audio-sequences.json': 'European typed sequence instructions, canonical branch links and producer-proved driver tables',
    'audio-driver-tables.json': 'European typed M4A pitch, sample-count, CGB noise and wave-volume tables',
    'palette-effect-dispatch.json': 'European producer-bounded palette callback slots; original live object indexing retained',
    'palette-programs.json': 'European typed palette programs and producer projections; original guest scheduling retained',
    'blend-programs.json': 'European typed blending controls, coefficients, commands and producer tables; live RAM programs retained',
    'task-descriptors.json': 'European typed task callbacks, context sizes and metadata; original guest selectors and scheduling retained',
    'effect-event-directories.json': 'European typed event records, effect selectors and even same-state dispatch tables; original guest sequencing retained',
    'script-command-directory.json': 'European typed script command advances and Thumb callbacks; original guest stream and control flow retained',
    'script-archive.json': 'European typed relative script archive, key records and explicit header/fill target views; original guest execution retained',
    'script-text.json': 'European typed script-selected localized text, shared banks and live variants; original guest text expansion retained',
    'script-coordinate-paths.json': 'European typed script coordinate directory, counts and XY paths; live map height and original movement retained',
    'auxiliary-script-command-directory.json': 'European typed auxiliary script advance and Thumb callback directory; original guest dispatch retained',
    'auxiliary-script-archive.json': 'European typed u16-relative auxiliary archive with interior instruction entries; original predicates and cursors retained',
    'ui-choice-lists.json': 'European typed immutable choice entries and filtered templates; original RAM copies, input and cursor tasks retained',
    'unit-obj-script-special-normal.json': 'European conditional normal script-special poses with exact logical metadata and script witnesses; previous owners, original tasks and DMA retained',
    'unit-obj-script-special-alternate.json': 'European conditional alternate script-special poses with exact logical metadata and script witnesses; previous owners, original tasks and DMA retained',
    'unit-obj-script-override.json': 'European conditional forced42/230 script poses with exact original assignment and script-group witnesses; previous owners, original tasks and DMA retained',
    'unit-obj-script-registry-special.json': 'European conditional registry poses for exact known special-unit metadata and script witnesses; six previous owners, original tasks and DMA retained',
    'unit-obj-script-registry-generic.json': 'European conditional registry poses from exact direct generic-job fields and script witnesses; six previous owners, original tasks and DMA retained',
    'unit-obj-property-words.json': 'European raw actor property words from known normal/alternate metadata assignments; original allocation, task reuse and raw gaps retained',
    'unit-obj-byte-property-words.json': 'European UI byte-input propertyword complement; existing actor-property and palette aliases retain their owners and original guest callers remain live',
    'unit-obj-ffff-reader-fields.json': 'European numeric FFFF property and directory reader fields with conditional metadata/archive provenance; original live unmapped bus and guest control flow retained',
    'slot-job-lookup.json': 'European twenty-slot u8 job references from the original live slot selector; metadata52 properties, name lookup, input truncation and raw outside sources remain guest work',
    'observed-task-parameters.json': 'European conditional phase0/1/2 unsigned thresholds and signed vertical correction indices4..7; original old-counter compare, phase wrap, earlier owners and outside queries retained',
    'constructor-obj-animation.json': 'European conditional phases0..3 unsigned word thresholds and indirect OBJ-list pointers; original u8 counter wrap and unchecked outside-phase fallback retained',
    'task-cycle-deltas.json': 'European conditional48signed-i8 task increments; original unchecked old-u8 lookup, stored-byte wrapping and remainder48 retained',
    'world-nodes.json': 'European typed world-node coordinates, ordered path-neighbor links and opaque bytes; original placement, pathfinding and raw outside reads retained',
    'world-roads.json': 'European typed world-road sample counts, encoded pointers, signed XY samples and opaque bytes; original movement and raw outside reads retained',
    'encounter-headers.json': 'European typed encounter-header fields and opaque bytes with shared constructor-pointer dependency; original routines and prior/raw outside reads retained',
    'encounter-unit-templates.json': 'European typed encounter unit-template fields with header/reference/full48 shared-constructor coherence; original live initialization and prior/raw reads retained',
    'encounter-coordinates.json': 'European typed encounter XY lists with header pointer/count coherence; original placement, RNG and signed/unsigned readers retained',
    'linked-metadata28.json': 'European indexed 28-byte records with unsigned fields, inline source views, full flag words and opaque bytes; original live readers retained',
    'linked-records4.json': 'European ordered four-byte slots with four unsigned fields and original encoded u8-index pointers; zero slots retained, live readers unchanged',
    'conditional-records8.json': 'European ordered eight-byte conditional slots with live predicate keys, unsigned operands and original output pointers; zero gates skip, guest live logic unchanged',
    'mission-records70.json': 'European ordered70-byte mission records with lossless packed fields, raw operands and original inline address aliases; guest getter and event join unchanged',
    'selection-byte-tables.json': 'European four unsigned count thresholds and thirty ordinal weights; original live eligibility, numeric helpers and selection logic retained',
    'event-records12.json': 'European ordered12-byte event records and shared row225 interior view; original live script/event logic and uncapped outside reads retained',
    'parameter-records27.json': 'European59 observed27-byte parameter records, two sparse source runs; packed and signed fields retained, unowned indices use original raw reads',
    'world-records38.json': 'European31 observed38-byte world records, unsigned fields/packed low10/31thresholds and opaque bytes; uncapped guest views outside the catalog retain raw reads',
    'indexed-graphics.json': 'European20 indexed graphics archives/649 programs and four five-slot language groups; BE offsets, literal/fixed dictionary packets and exact decoded words, original guest decode/upload work retained',
    'ui-raw-transfers.json': 'European seven raw language tables and35 fixed source views; exact transfer words, sparse seventh-bank slice and original guest copy queues retained',
    'dynamic-raw-fragments.json': 'European bounded dynamic selector rows and five bank prefixes; original guest selectors and transfer queues retained',
    'conditional-four-byte-lookup.json': 'European source-backed conditional 74-row projection; original guest selectors retained',
    'conditional-packed-lookup.json': 'European conditional type-5 packed lookup projection; original selectors and aliases retained',
    'conditional-descriptor-banks.json': 'European conditional descriptor banks, source-backed counts and unchanged guest selectors',
    'conditional-descriptor-bank-zero.json': 'European conditional descriptor bank zero, selected records and pointer with raw row zero',
    'conditional-property-masks.json': 'European conditional property-selected 32-bit masks',
    'compact-obj-animations.json': 'European witnessed compact OBJ animation lists and sequences; original selectors and outside reads retained',
    'compact-obj-animations-secondary.json': 'European second witnessed sparse compact OBJ bank; original selectors and outside reads retained',
    'compact-obj-animations-tertiary.json': 'European third witnessed sparse compact OBJ bank, including an empty list; original selectors and outside reads retained',
    'compact-obj-animations-quaternary.json': 'European fourth witnessed compact OBJ bank; original selectors and outside reads retained',
    'compact-obj-animations-quinary.json': 'European fifth witnessed compact OBJ bank; original selectors and outside reads retained',
    'compact-obj-animations-senary.json': 'European sixth witnessed sparse compact OBJ bank; original selectors, gap and outside reads retained',
    'word-task-obj-animations.json': 'European four relative OBJ banks for u16 task selectors-ticks-phases; original old-list return, counter wrap, repeat and raw outside reads retained',
    'unit-obj-alternate-battle.json': 'European alternate actor battle poses with explicit shared index-root views, commands, OAM and decoded indexed tiles; original tasks and DMA retained',
    'unit-obj-normal-battle.json': 'European normal actor battle poses, shared source dependencies, commands, OAM and decoded indexed tiles; original tasks and DMA retained',
    'unit-obj-equipment.json': 'European item/action-rooted equipment poses, null cells, commands, OAM and decoded indexed tiles; original live tasks and DMA retained',
    'color-mode-palettes.json': 'European typed unit/UI BGR555 colors and slice selectors; original live Color-Mode, copies and fades retained',
    'unit-obj-initial-pose.json': 'European producer-rooted initial unit frames, OAM and decoded indexed tiles; original scheduler and DMA retained',
    'map-raster-palettes.json': 'European typed map raster palette indices, headers and indexed colors; original copy, wrap and HBlank DMA retained',
    'linked-metadata52.json': 'European packed metadata and redirects; original live alternate indices retained',
    'linked-metadata16.json': 'European sixteen-byte linked metadata; original live list and RNG retained',
    'item-links.json': 'European counted item-linked pairs; original live unit publication retained',
    'items.json': 'European static item fields; original live property modifiers retained',
    'manifest.toml': 'source identity, resource coverage and hashes',
    'ui-graphics.json': 'European56 customLZ graphics banks/two inline language switches/nine rawlanguage tables including clan alias; original guest decoding/scratch/upload work retained',
    'ui-palettes.json': 'European raw UI palette words for BG/OBJ shadows',
    'obj-graphics.json': 'European custom OBJ word banks and language selector tables',
    'obj-animation-banks.json': 'European indexed sprite lists, animation frames and durations',
    'obj-static-lists.json': 'European static sprite lists with coordinates, attributes, tiles and palette indices',
    'obj-dimensions.toml': 'sprite dimensions',
    'bg-descriptors.json': 'background descriptors and source links',
    'bg-base-palettes.json': 'base background palette banks',
    'bg-alternate-palettes.json': 'alternate background palette banks',
    'bg-packet-programs.json': 'background packet programs and ordered writes',
    'bg-main-graphics.json': 'European background graphics words and destinations',
    'bg-transfer-programs.json': 'European background transfer controls and shared tile banks',
    'short-texts.json': 'European short strings, game glyph codes, font slots and widths',
    'text-catalogs.json': 'European pointer strings, relative text archives and referenced font slots',
    'font-bank.json': 'European complete packed font bank and glyph widths',
    'name-keyboard.json': 'European name-entry programs, navigation widths and selectable cells',
}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('rom', type=Path, help='path to your supported FFTA ROM')
    parser.add_argument('--output', type=Path,
                        help='dataset directory (default: target/rom-data/<ROM filename stem>)')
    parser.add_argument('--recomp', type=Path,
                        help='existing recomp executable; otherwise Cargo builds/runs release')
    parser.add_argument('--wine', action='store_true',
                        help='run the executable supplied with --recomp through Wine')
    args = parser.parse_args(argv)
    rom = args.rom.expanduser().resolve()
    if not rom.is_file():
        parser.error(f'ROM file does not exist: {rom}')
    if args.wine and args.recomp is None:
        parser.error('--wine requires --recomp pointing to a Windows executable')
    output = (args.output.expanduser() if args.output is not None
              else ROOT / 'target' / 'rom-data' / rom.stem).resolve()

    if args.recomp is not None:
        executable = args.recomp.expanduser().resolve()
        if not executable.is_file():
            parser.error(f'recomp executable does not exist: {executable}')
        command = [str(executable)]
        if args.wine:
            wine = shutil.which('wine')
            if wine is None:
                parser.error('Wine was not found in PATH')
            command.insert(0, wine)
    else:
        cargo = shutil.which('cargo')
        if cargo is None:
            parser.error('Cargo was not found in PATH; install Rust or use --recomp')
        command = [cargo, 'run', '--locked', '--release', '--package', 'recomp',
                   '--bin', 'recomp', '--']
    command += ['rom-data', 'extract', str(rom), str(output)]

    print(f'Extracting ROM data into: {output}', flush=True)
    try:
        result = subprocess.run(command, cwd=ROOT)
    except OSError as error:
        print(f'Cannot run extraction: {error}', file=sys.stderr)
        return 1
    if result.returncode:
        print('Extraction failed; see the CLI error above.', file=sys.stderr)
        return result.returncode if result.returncode > 0 else 1

    try:
        if not (output / 'manifest.toml').is_file():
            raise OSError('extraction returned success without a manifest.toml')
        files = sorted(path for path in output.iterdir()
                       if path.is_file() and path.suffix in ('.json', '.toml'))
    except OSError as error:
        print(f'Cannot inspect extracted dataset: {error}', file=sys.stderr)
        return 1
    print(f'\nDataset ready: {output}\n{len(files)} readable files:')
    for path in files:
        description = DESCRIPTIONS.get(path.name, 'decoded resource')
        print(f'  {path.name} - {description}')
    print('\nOpen these JSON/TOML files in your editor to inspect the extracted data.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
