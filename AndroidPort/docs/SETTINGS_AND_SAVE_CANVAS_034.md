# Android preview 0.34 - settings and full-height menu backgrounds

Settings now use compact, themed dropdowns for screen model, response, grid, scaling, aspect ratio, fast-forward speed and audio mode. Popup lists accept vertical finger scrolling, retain their scroll position, initially show the selected area and dismiss on outside tap or Back. Values still use the existing persisted settings; no defaults were migrated.

Save/Load previously forced a 240x160 3:2 viewport, causing top/bottom bars on the RG Rotate. Save/Load and verified Party/Clan backgrounds now request a display-shaped menu canvas. Native foreground scale and centered 240x160 coordinates are retained: RG uses 240x240 with a 40-row native inset, rather than stretching the cards. The save background is painted across the entire canvas. Tap inversion uses the actual viewport and native inset. Gameplay aspect/zoom preferences remain in use outside these menus. Clan's illustrated backdrop uses signed reflection coordinates at its top border to avoid a phase jump.

Validation on RG Rotate / Android 12:
- 274 core tests passed, 7 ignored; 74 Android-native tests passed, 2 ignored; Gradle unit tests and APK build passed.
- Full-canvas native hit mapping checked for square, wide and original letterboxed layouts; Clan top/bottom reflection regression passed.
- Physical dropdown swipe exposed lower choices; outside tap closed the popup while retaining the Video page.
- Interactive isolated save session loaded a copy of the original SRAM, converted its normal slot, restarted, and loaded that snapshot back to the world map. System > Save wrote Save2 and returned to the map. The final geometry also allowed creation of Save3 by touch.
- Screenshots confirm full-height backgrounds on final Save, Load and Party. Clan is covered by the same menu-pattern canvas and renderer regression; the early RG save did not expose Clan for direct visual inspection.

No normal save was overwritten, renamed or removed. Interactive testing uses the debug-only ffta_save_sandbox intent, selecting benchmark.sav and benchmark-saves; release builds ignore this extra. Original diagnostic files and settings were backed up/restored, generated sandbox slots removed, and normal Saves remained empty. Original SRAM checksum remained d46cc3912a530fb3044e17f7c3f208eef6206038b5a394024fdb92eaaa956210.

Evidence is under target/android34-device-check, including before/after save screenshots, Party, dropdown captures and isolated written save containers. These private diagnostic assets are not source assets. APK: APK/FFTARecomp-android-arm64-preview.apk, versionCode34 / 0.34-settings-dropdowns; checksum in APK/SHA256.txt. No dependency or license change. Desktop files were not modified in this Android pass.
