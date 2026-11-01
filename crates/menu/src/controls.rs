//! Presentation-only hints follow the last intentional input, not the game profile.
use super::*;

impl Menu {
    /// Returns whether hints changed. This never remaps or saves input bindings.
    pub fn note_input(&self, device: Device) -> bool {
        let device = if device == Device::Gamepad {
            Device::Gamepad
        } else {
            Device::Keyboard
        };
        self.hint_device.replace(Some(device)) != Some(device)
    }

    pub(super) fn hints_gamepad(&self) -> bool {
        self.hint_device.get() == Some(Device::Gamepad)
    }

    pub(super) fn hint_button(&self, token: &str, keyboard: &str) -> String {
        if self.hints_gamepad() {
            gamepad_label(token, self.controller_layout.get())
        } else {
            keyboard.into()
        }
    }

    pub(super) fn control_hints(&self) -> [String; 2] {
        if self.hints_gamepad() {
            let pad = |token| self.hint_button(token, "");
            [
                self.tr_format(
                    "D-pad · {} OK · {} indietro · {} focus · {} dettagli",
                    &[
                        ("0", (pad("South")).to_string()),
                        ("1", (pad("East")).to_string()),
                        ("2", (pad("West")).to_string()),
                        ("3", (pad("RightThumb")).to_string()),
                    ],
                ),
                self.tr_format(
                    "VOL {}− / {}+ / {}+{} mute · FAST {}− / {}+ / {}+{} ON",
                    &[
                        ("0", (pad("LeftTrigger")).to_string()),
                        ("1", (pad("RightTrigger")).to_string()),
                        ("2", (pad("LeftTrigger")).to_string()),
                        ("3", (pad("RightTrigger")).to_string()),
                        ("4", (pad("LeftTrigger2")).to_string()),
                        ("5", (pad("RightTrigger2")).to_string()),
                        ("6", (pad("LeftTrigger2")).to_string()),
                        ("7", (pad("RightTrigger2")).to_string()),
                    ],
                ),
            ]
        } else {
            [
                self.tr("Frecce · Invio OK · Esc indietro · Tab focus · F1 dettagli")
                    .into(),
                if self.debug_enabled() {
                    self.tr("Volume / Fast: voce + Invio · Shift+Tab: vista Debug")
                } else {
                    self.tr("Volume / Fast: scegli la voce + Invio")
                }
                .into(),
            ]
        }
    }
}

#[cfg(test)]
mod tests;
