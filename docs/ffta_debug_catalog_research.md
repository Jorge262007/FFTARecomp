# EUR character catalogue and mission jump research

2026-10-20. Investigation only; no runtime changes in this step.

## Verified locally (static inspection, not live execution)

The local EUR image passes SHA256
8fab1e390cb9d5e695bfdb86e81246892e1aa8c0cb4014db8235ff76538ea632.
No original cartridge or save was modified. No ROM data is included here.

Thumb function 080D1F30 accepts a u16 record index in r0 and a u8 field selector
in r1. At 080D1F3A it loads stride70, multiplies by the index and adds the
literal table base08713B3C, before dispatching on the field selector (0..64).
This is a concrete EUR accessor, rather than a transplanted USA address.
080D2A94 also decodes a packed record index from a RAM entry and reads the
same table's byte+2. It is not yet identified as the mission-launch function.

Read-only inspection of406 records using the external reference's count/layout:

    EUR_SHA256 PASS
    MISSION_TABLE=0x08713B3C stride=70 records_inspected=406
    RECRUIT_NONZERO_RECORDS 309
    RECRUIT_IDS 01,03,0F,17,20,29,32,40,50,5E,6F,8C,92,96,98,9A,9C,9E

Those numbers are byte statistics, NOT406 verified playable missions or309
recruitable characters. The record count and recruit field meaning have not
been independently established in live EUR gameplay. Mission table indices,
displayed mission numbers and text IDs must not be conflated.

EUR0802430C..08024312 passes roster base02000080 and count24 to0809E068.
The latter walks records with stride264 (0x108), restoring HP/MP through a
stat accessor and clearing transient data. This corroborates a24-record clan
operation; it does not establish that the engine supports an enlarged clan.
The complete recruit constructor, free-slot rules and unique-character flags
are NOT yet traced/verified.

## External references (read as research; no code imported)

- [FFTA-randomizer data map](https://github.com/TojiKitten/FFTA-randomizer/blob/main/src/main/ffta/FFTAData.ts): USA mission and formation layouts.
- [Mission fields](https://github.com/TojiKitten/FFTA-randomizer/blob/main/src/main/ffta/DataWrapper/FFTAMission.ts): encounter/link flags, prerequisite triplets, recruitment and location.
- [Starting party](https://github.com/TojiKitten/FFTA-randomizer/blob/main/src/main/ffta/enginehacks/startingParty.ts): job, equipment and mastered ability distinctions.
- [Formation fields](https://github.com/TojiKitten/FFTA-randomizer/blob/main/src/main/ffta/DataWrapper/FFTAFormation.ts): map, members and deployment data are separate.
- [Community RAM discovery](https://github.com/lofimichael/ffta-agent/blob/main/DISCOVERY.md): USA-only, internally conflicting battle-array notes; not an EUR address authority.

## Initial design (superseded by the F8 menu below)

Keep F8 victory. F9 could open a character catalogue; F10 a mission catalogue.
Both require FFTA_DEBUG=1, exact EUR identity and independently checked original
function signatures. Labels/templates must be read from the user's ROM at
runtime, not copied into the repository.

Characters: offer known recruitable unique units plus race/job representatives.
Use the original constructor and free slots. A catalogue can make every
supported template available without placing unlimited units in the clan.
Story NPCs and enemies need a separate explicit test category; they cannot be
assumed equivalent to valid recruits. Do not overwrite existing units silently.
The scripted eight-unit tutorial remains a separate regression fixture.
Earliest activation must wait for valid clan initialization; title/name entry
does not imply a ready party.

Missions: choose an internal record, classify its kind and enter through the
original initialization path. A map-ID write alone cannot establish formation,
deployment, laws, scripts or objectives. Non-combat dispatch/link/unused records
must be classified and unsupported entries clearly refused, rather than claimed
to work. Start from a stable scene; active combat requires orderly cleanup first.

Use an isolated debug save/session; returning from previews should not mark the
real campaign completed. No distributed premade save, ROM fragments or assets.

## Required before shipping

Trace recruit creation and mission dispatch on EUR; prove a later mission can
start from an early initialized clan. Test different formations, indoor/outdoor
maps, scripted objectives, dispatch/link rejection, unique portraits and jobs.
Verify repeat jumps, return path and save isolation. Retain600-frame framebuffer
and OAM parity with debug disabled, plus the eight-character tutorial regression.

## 2026-10-20 partial implementation

F8 now opens the native developer menu behind FFTA_DEBUG=1 and the exact EUR
identity. It pauses guest execution. A session-only OFF/ON switch gates victory;
switching it OFF blocks new requests and does not undo previous actions.
Every debug launch isolates the resolved save to a unique session copy before
loading. The normal pause menu and normal save path remain unchanged otherwise.

The mission selector supports IDs1..405 and a dropdown, but Load is LOCKED.
MEDIDO: the full-boot probe reaches the battle callback for mission3, event32,
but the resulting framebuffer shows sky only and the scene remains state9.
Callback identity alone is not proof of a working mission jump. No guessed
mission initialization is exposed in the product runtime.

MEDIDO: original EUR080D5F3C on separate copied initialized RAM accepts recruit
codes96/98/9A/9C/9E using mission records384/385/386/400/387 respectively.
All five produce nonzero HP and ROM-backed name pointers. Other offer codes
may return0 and leave an empty unit. Raw080CCF9C types also include NPC variants
with10HP. This does not establish a valid all-character clan, so Unlock is
LOCKED. No constructor is called from the live runtime by this menu yet.

NO MEDIDO: live recruit publication, complete mission jumps, repeat jumps,
all-character validity and manual approval of the new F8 menu.
