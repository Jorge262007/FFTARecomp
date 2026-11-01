# EUR visual UI review - 2026-10-21

Status: patch implemented and Windows built. Real-game visual acceptance is still
pending. Passing synthetic tests does not confirm every reported instance.
No new screenshots of the attack-name or Thunder moments were attempted.

| Area | Implemented behavior | Check in game |
| --- | --- | --- |
| PUB, Prison, Store, Monster Bank, Card Keeper | Root MENU attaches by its own left edge. Verified parent owners remain identifiable with an inactive cursor during transitions. | No initial center-to-left jump. |
| Blue fragments | Native service base scroll aligns backing, labels and OBJ. | No detached blue column or rectangle, including during transitions. |
| Missions / service LIST | Separate centered content uses owner geometry and measured row pitch. | Entire list centered in the viewport. |
| Clan Funds | Independent bottom-right layer with six native pixels of vertical margin. Detection uses the UI OBJ family and native counter geometry. | Confirm actual counter family and complete counter alignment. |
| Engage with / Victory / Notice | Classified modal bounds center horizontally and vertically. | All real prompts and transitions. |
| Action portraits | Bounds include visible foreground outside blue backing; uniform size fits available height with margin. | Long ears, portrait and last text row remain inside viewport. |
| Blue/red hover stats / forecasts | Backing orientation determines side; pieces normalize to fixed target. Partial fade keeps owner, not old pixels. | Hover allies/enemies, enter/leave targeting at each zoom. |
| Action / Abilities | UI remains classified while either BG plane is enabled. Cursor/scroll markers join owning panel. | No transient blue strip, missing cell or displaced arrows. |
| Abilities mouse | Paged EUR battle windows accept absolute/page-relative selection and measured row pitch. Legacy fallback remains if pitch cannot be measured. | Every visible row, later pages, then return. |
| Hurl / Quarter / attack announcements | Shared foreground/backing rotation unwraps 256px source map; compact capsule top-centered without Length. | Complete capsule for every real attack. |
| Magic repetition | Text/affine battle effect atlas wrapping limited outside original GBA viewport; native wrapping preserved. | Thunder and other spells at 16:9/32:9 and zoom-out. OBJ-only effects need separate evidence. |

This pass does not change character producer/culling patches or shader light
profiles. ffta_effects::layer supplies BG slot, BGCNT, map/character addresses and
affine classification for future shader integration. Optional FFTA_UI_REFS records
effect_planes and UI counter/marker roles. This is not a spell-ID decoder;
resource addresses are not unique ROM asset IDs.

## Measured validation

- Release core: 189 passed, 7 ignored.
- Release recomp: 32 passed, 1 ignored.
- Release screen: 91 passed, 1 ignored.
- Explicit GPU readback, NVIDIA GeForce RTX 5050: 1 passed.
- EUR native compatibility: 600 frames, framebuffer and OAM identical.
  framebuffer FNV a04a7922125b2159; OAM FNV 971d444336ad5341.
- Capsule fixture joins map-edge pieces into [0,16,72,24] at Length 100/150/200%.
- Effect fixtures preserve all native 240 pixels and remove extra margin copies.
- Stats fixtures cover five offsets and opaque/semitransparent backing modes.
- Mouse fixtures cover later ability pages, cursor pitch and row-zero reuse.
- Geometry fixtures cover 16:9/32:9, centered panels, margins and centered-native roots.

Logs: target/ui-review-*.log. Executable: target/release/recomp.exe.
Use FFTA_SAVE_PATH for chosen save and FFTA_DEBUG=1 for isolated session copies.
No original saves, installer, dependencies, or commits changed.
