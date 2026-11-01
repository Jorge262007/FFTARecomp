# New-game stale OBJ lifetime fix

The host-expanded sprite list was invalidated only by DMA publication. The EUR intro clears OAM through CpuSet/direct writes instead. A clean boot/new-game interpreter trace retained four title sprites from frame 625 through frame 822 after guest OAM was parked at (248,168). The same stale ownership also occurred at the initial language transition. OBJ tiles can then be reused while the old host objects still reference them.

At scanline zero, validate the published segments against the live first three OAM attributes of each replaced object. Drop only mismatching segments and rebuild the sorted host list. Affine matrix words are intentionally ignored; unchanged OAM buffers and their offscreen characters remain valid indefinitely. No timer, guest-memory write, input, ROM, or save mutation is added. Work is once per frame, rather than per scanline. Applied to desktop and Android core copies.

Measured: clean EUR boot/new-game replay (1050 frames) no longer retains mismatching objects; host count at frame 660 changes from four stale title pieces to zero. Three synthetic regressions cover direct clear, independent partial segments, and eight retained actors over 300 affine updates. Existing zero-extra rendering remains gated out. A 30-frame visual sampling did not capture the brief user-reported flash; visual confirmation on the user's device is still required.

Local reproduction: cargo run --release -p gba-core --example startup_object_probe -- PATH_TO_OWN_EUR_ROM. All captures stay under target, excluded from upload.
