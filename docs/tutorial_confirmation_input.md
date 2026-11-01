# Tutorial confirmation input and fast-forward

Repeat lesson reuses the dialogue interpreter pointer and yes/no cursor used by name confirmation. The pointer/cursor alone cannot identify the name keyboard. Name confirmation now requires the live name-frame object; story questions retain the dialogue panel transform and answer model instead of the keyboard hitboxes. The same classification is used by PC/Linux and Android.

Android now resolves a question before the inactive menu behind it, maps touch coordinates through the displayed host UI panel, and selects/confirms the touched answer through the original dialogue handler. Empty space does not generate A.

Fast-forward choices are 1.5x, 2x, 3x and 4x. Legacy 1x preferences migrate to 1.5x; normal playback remains 1x. The touchscreen toggle cycle also resets to 1.5x when disabled.

Validation: 22 input configuration tests, 28 desktop menu tests, 17 name/frontend input tests, three Android touch tests and 25 Android JVM tests passed. A private real-ROM Repeat lesson snapshot was replayed with both yes and no: each reached the original answer branch with global keys released. ROMs, snapshots and diagnostic images are not distributed.
