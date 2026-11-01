# Hold-to-skip story sequences (desktop, EUR)

Hold **F12** for one second during dialogue, and keep holding until arrival. Release to cancel. The scene runs in the background without pixel generation, intermediate UI preparation, presentation, audio output or normal frame pacing. Original guest logic still executes actor walking, camera movement, delays and event flags in order. This preserves the state that a direct program-counter jump would bypass. Arrival can take a few seconds, depending on scene length.

The skip starts only from verified text states. Once active it continues across external waits and scripted movement, stopping at a name/frontend selector, a dialogue choice, a live battle cursor/menu, release or a thirty-second safety timeout. At an interaction barrier, release and hold again for the next sequence. It never generates a GBA A key or chooses an answer.

The current implementation freezes the last presented frame while processing; the window title displays “Skipping scene...”. Original video clocks, DMA, IRQs, CPU execution and sound generation remain active, but intermediate pixels/sound are discarded. Keyboard/gamepad gameplay input is neutral during hidden execution. Processing is divided into 35 ms host slices so the window remains responsive. F12 remains desktop-only and is not yet remappable.

Validation: 56 dialogue-related core tests passed (3 ignored). A separate regression compares CPU registers, clock, frames, RAM, IO/IRQ state and generated audio over three frames with pixel generation on/off; it passes. `cargo check -p recomp` passed. Full tutorial/name/decision arrival still requires gameplay testing. This supersedes the earlier x4 page-only implementation.
