# Top dialogue exit fix ? 2026-10-28

A private EUR replay from clean boot exposed the intro's upper-left speech window contracting without a reported window phase. At frames 3846..3851 its bounds changed from [0,13,205,67] through widths 173,141,109,77,45,13. Frame 3852 then published an empty full-screen rectangle. The corresponding upper-right dialogue had a verified Exiting phase, explaining why some exits already worked.

Host Panel::sync previously considered every bounding rectangle a new identity and cleared the cached complete image before starting an exit. It now interprets a contained horizontal contraction of at least one tile as exit for Dialogue panels. It freezes the full image and original geometry for the 220 ms host slide. An empty producer frame also starts/preserves exit before identity comparison, even when its fallback anchor or rectangle changes. Entry and non-dialogue policies remain the same.

The fix is shared by Windows and Android. Both motion suites pass 14 tests, including both measured side contractions, the empty full-screen fallback and existing keyboard/portrait/menu transitions. The replay writes diagnostics under AndroidPort/target only and does not access user save files.
