# Android 0.35 - menu dismissal immersion

Touchscreen settings now label the existing toggle Touch selection and omit the experimental preview paragraph. Its stored preference and behavior are unchanged.

GameScreen reapplies transient-by-swipe immersive bars once, 250 ms after the pause/editor overlay closes, and only while fullscreen is enabled. A keyed Compose effect cancels the pending action if the menu reopens or fullscreen is disabled. This avoids polling and leaves windowed mode unchanged.

Physical LineageOS validation remains pending: the remotely connected Note10 disconnected from ZeroTier before this change. The prior 0.34 isolated test session and diagnostic/settings restoration remain pending; backups are in target/note10-zerotier34/manifest.json. Do not clear app data or remove normal saves when resuming those checks.

Gradle assembleDebug and testDebugUnitTest succeeded. APK versionCode35 / 0.35-immersive-menu, checksum in APK/SHA256.txt. No native engine or save format changes.
