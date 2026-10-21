//! DirectInput fallback for legacy Windows pads. Raw buttons can be rebound
//! without assuming an Xbox/PlayStation button order. Other platforms use gilrs.
use crate::{Button, DpadSource, PadBinding, pad_binding};

#[derive(Clone, Debug)]
pub struct PadState {
    pub name: String,
    pub buttons: [bool; 128],
    /// X right-positive, Y up-positive, matching the existing stick tokens.
    pub axes: [f32; 4],
    pub pov: Option<u32>,
}
impl PadState {
    /// A controller is considered active if any digital control is held or
    /// an axis has moved far enough to be intentional. This is used only for
    /// device selection; individual bindings still use the configured deadzone.
    pub fn has_activity(&self, threshold: f32) -> bool {
        self.buttons.iter().any(|pressed| *pressed)
            || self.axes.iter().any(|axis| axis.abs() >= threshold)
            || self.pov.is_some()
    }

    /// Stable activity signature used to detect a newly active DirectInput
    /// device without treating a held control as fresh input every frame.
    pub fn activity_tokens(&self, threshold: f32) -> Vec<String> {
        let mut out = self
            .buttons
            .iter()
            .enumerate()
            .filter(|(_, pressed)| **pressed)
            .map(|(index, _)| format!("button:{index}"))
            .collect::<Vec<_>>();
        if let Some(pov) = self.pov {
            out.push(format!("pov:{}", (pov / 4500) % 8));
        }
        for (index, axis) in self.axes.iter().enumerate() {
            if axis.abs() >= threshold {
                out.push(format!("axis:{index}:{}", if *axis > 0.0 { 1 } else { -1 }));
            }
        }
        out
    }

    pub fn pressed(&self, name: &str, dz: f32) -> bool {
        if let Some(index) = name
            .strip_prefix("DInputButton")
            .and_then(|s| s.parse::<usize>().ok())
        {
            return self.buttons.get(index).copied().unwrap_or(false);
        }
        if let PadBinding::Stick(token) = pad_binding(name) {
            let (horizontal, positive) = token.axis();
            let index = if token.left() { 0 } else { 2 } + if horizontal { 0 } else { 1 };
            return if positive {
                self.axes[index] > dz
            } else {
                self.axes[index] < -dz
            };
        }
        let index = match name {
            "South" => Some(0),
            "East" => Some(1),
            "West" => Some(2),
            "North" => Some(3),
            "LeftTrigger" => Some(4),
            "RightTrigger" => Some(5),
            "LeftTrigger2" => Some(6),
            "RightTrigger2" => Some(7),
            "Select" => Some(8),
            "Start" => Some(9),
            "LeftThumb" => Some(10),
            "RightThumb" => Some(11),
            "Mode" => Some(12),
            _ => None,
        };
        if let Some(i) = index {
            return self.buttons[i];
        }
        let Some(pov) = self.pov else {
            return false;
        };
        match name {
            "DPadUp" => pov > 29250 || pov < 6750,
            "DPadRight" => (2250..15750).contains(&pov),
            "DPadDown" => (11250..24750).contains(&pov),
            "DPadLeft" => (20250..33750).contains(&pov),
            _ => false,
        }
    }
    pub fn active(&self, name: &str, source: DpadSource, dz: f32, button: Button) -> bool {
        if self.pressed(name, dz) {
            return true;
        }
        let index = if source.uses_right_stick() { 2 } else { 0 };
        if !source.uses_left_stick() && !source.uses_right_stick() {
            return false;
        }
        match button {
            Button::Right => self.axes[index] > dz,
            Button::Left => self.axes[index] < -dz,
            Button::Up => self.axes[index + 1] > dz,
            Button::Down => self.axes[index + 1] < -dz,
            _ => false,
        }
    }
    pub fn capture_tokens(&self) -> Vec<String> {
        let mut out = self
            .buttons
            .iter()
            .enumerate()
            .filter(|(_, v)| **v)
            .map(|(i, _)| format!("DInputButton{i}"))
            .collect::<Vec<_>>();
        for token in [
            "DPadUp",
            "DPadDown",
            "DPadLeft",
            "DPadRight",
            "LeftStickUp",
            "LeftStickDown",
            "LeftStickLeft",
            "LeftStickRight",
            "RightStickUp",
            "RightStickDown",
            "RightStickLeft",
            "RightStickRight",
        ] {
            if self.pressed(token, 0.6) {
                out.push(token.into());
            }
        }
        out
    }
}
#[cfg(test)]
fn normalize(value: i32, min: i32, max: i32) -> f32 {
    if max <= min {
        return 0.0;
    }
    ((2.0 * (value as f64 - min as f64) / (max as f64 - min as f64) - 1.0) as f32).clamp(-1.0, 1.0)
}

#[cfg(any(windows, test))]
fn normalize_centered(value: i32, center: i32, min: i32, max: i32) -> f32 {
    if max <= min || center < min || center > max {
        return 0.0;
    }
    let value = value as f64;
    let center = center as f64;
    let min = min as f64;
    let max = max as f64;
    let normalized = if value >= center {
        let span = max - center;
        if span <= 0.0 {
            0.0
        } else {
            (value - center) / span
        }
    } else {
        let span = center - min;
        if span <= 0.0 {
            0.0
        } else {
            (value - center) / span
        }
    };
    (normalized as f32).clamp(-1.0, 1.0)
}

#[cfg(any(windows, test))]
fn changed_pov(values: [u32; 4], centers: [u32; 4]) -> Option<u32> {
    values
        .iter()
        .zip(centers)
        .find_map(|(&value, center)| (value < 36000 && value != center).then_some(value))
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::{
        ffi::c_void,
        mem::{offset_of, size_of},
        time::{Duration, Instant},
    };
    use windows::{
        Win32::{
            Devices::HumanInterfaceDevice::*,
            Foundation::{HINSTANCE, HWND},
            System::LibraryLoader::GetModuleHandleW,
            UI::WindowsAndMessaging::{
                CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WINDOW_STYLE,
            },
        },
        core::{BOOL, GUID, Interface, w},
    };
    struct Device {
        guid: GUID,
        name: String,
        device: Option<IDirectInputDevice8W>,
        ranges: [(i32, i32); 8],
        axis_centers: Option<[i32; 6]>,
        pov_centers: Option<[u32; 4]>,
        objects: Vec<DIOBJECTDATAFORMAT>,
        format: Option<Box<DIDATAFORMAT>>,
    }
    pub struct DirectInput {
        api: IDirectInput8W,
        devices: Vec<Device>,
        last_scan: Option<Instant>,
        window: HWND,
    }
    unsafe extern "system" fn enumerate(
        info: *mut DIDEVICEINSTANCEW,
        context: *mut c_void,
    ) -> BOOL {
        let list = unsafe { &mut *(context as *mut Vec<(GUID, String)>) };
        let info = unsafe { &*info };
        let end = info
            .tszInstanceName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(260);
        list.push((
            info.guidInstance,
            format!(
                "DirectInput: {} [{:?}]",
                String::from_utf16_lossy(&info.tszInstanceName[..end]),
                info.guidInstance
            ),
        ));
        BOOL(1)
    }
    impl DirectInput {
        pub fn new() -> Option<Self> {
            unsafe {
                let module = GetModuleHandleW(None).ok()?;
                let mut raw = std::ptr::null_mut();
                DirectInput8Create(
                    HINSTANCE(module.0),
                    0x0800,
                    &IDirectInput8W::IID,
                    &mut raw,
                    None,
                )
                .ok()?;
                if raw.is_null() {
                    return None;
                }
                let api = IDirectInput8W::from_raw(raw);
                // Own an invisible top-level window for background/nonexclusive
                // polling; no dependency on the foreground window or its lifetime.
                let window = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!("FFTA DirectInput"),
                    WINDOW_STYLE(0),
                    0,
                    0,
                    0,
                    0,
                    None,
                    None,
                    Some(HINSTANCE(module.0)),
                    None,
                )
                .ok()?;
                Some(Self {
                    api,
                    devices: Vec::new(),
                    last_scan: None,
                    window,
                })
            }
        }
        fn refresh(&mut self) {
            if self
                .last_scan
                .is_some_and(|t| t.elapsed() < Duration::from_secs(1))
            {
                return;
            }
            self.last_scan = Some(Instant::now());
            let mut found = Vec::<(GUID, String)>::new();
            if unsafe {
                self.api.EnumDevices(
                    DI8DEVCLASS_GAMECTRL,
                    Some(enumerate),
                    &mut found as *mut _ as *mut c_void,
                    DIEDFL_ATTACHEDONLY,
                )
            }
            .is_err()
            {
                return;
            }
            self.devices
                .retain(|d| found.iter().any(|(id, _)| *id == d.guid));
            for (guid, name) in found {
                if !self.devices.iter().any(|d| d.guid == guid) {
                    self.devices.push(Device {
                        guid,
                        name,
                        device: None,
                        // DirectInput's default absolute range is 0..=65535.
                        // Keep a useful fallback if a driver refuses the
                        // DIPROP_RANGE query below.
                        ranges: [(0, 65535); 8],
                        axis_centers: None,
                        pov_centers: None,
                        objects: Vec::new(),
                        format: None,
                    });
                }
            }
        }
        pub fn names(&mut self) -> Vec<String> {
            self.refresh();
            self.devices.iter().map(|d| d.name.clone()).collect()
        }
        fn prepare(api: &IDirectInput8W, d: &mut Device, hwnd: HWND) -> Option<()> {
            unsafe {
                let mut device = None;
                api.CreateDevice(&d.guid, &mut device, None).ok()?;
                let device = device?;
                static AXES: [GUID; 8] = [
                    GUID_XAxis,
                    GUID_YAxis,
                    GUID_ZAxis,
                    GUID_RxAxis,
                    GUID_RyAxis,
                    GUID_RzAxis,
                    GUID_Slider,
                    GUID_Slider,
                ];
                let mut objects = Vec::new();
                for (i, guid) in AXES.iter().enumerate() {
                    objects.push(DIOBJECTDATAFORMAT {
                        pguid: guid,
                        dwOfs: (i * 4) as u32,
                        dwType: DIDFT_ABSAXIS | DIDFT_ANYINSTANCE | 0x80000000,
                        dwFlags: 0,
                    });
                }
                for i in 0..4 {
                    objects.push(DIOBJECTDATAFORMAT {
                        pguid: &GUID_POV,
                        dwOfs: (offset_of!(DIJOYSTATE2, rgdwPOV) + i * 4) as u32,
                        dwType: DIDFT_POV | (i as u32) << 8 | 0x80000000,
                        dwFlags: 0,
                    });
                }
                for i in 0..128 {
                    objects.push(DIOBJECTDATAFORMAT {
                        pguid: &GUID_Button,
                        dwOfs: (offset_of!(DIJOYSTATE2, rgbButtons) + i) as u32,
                        dwType: DIDFT_BUTTON | (i as u32) << 8 | 0x80000000,
                        dwFlags: 0,
                    });
                }
                let mut format = Box::new(DIDATAFORMAT {
                    dwSize: size_of::<DIDATAFORMAT>() as u32,
                    dwObjSize: size_of::<DIOBJECTDATAFORMAT>() as u32,
                    dwFlags: DIDF_ABSAXIS,
                    dwDataSize: size_of::<DIJOYSTATE2>() as u32,
                    dwNumObjs: objects.len() as u32,
                    rgodf: objects.as_mut_ptr(),
                });
                device.SetDataFormat(format.as_mut()).ok()?;
                device
                    .SetCooperativeLevel(hwnd, DISCL_BACKGROUND | DISCL_NONEXCLUSIVE)
                    .ok()?;
                for i in 0..8 {
                    let mut range = DIPROPRANGE {
                        diph: DIPROPHEADER {
                            dwSize: size_of::<DIPROPRANGE>() as u32,
                            dwHeaderSize: size_of::<DIPROPHEADER>() as u32,
                            dwObj: (i * 4) as u32,
                            dwHow: DIPH_BYOFFSET,
                        },
                        lMin: 0,
                        lMax: 0,
                    };
                    // DIPROP_RANGE is a MAKEDIPROP integer pointer, not an actual GUID object.
                    if device
                        .GetProperty(4usize as *const GUID, &mut range.diph)
                        .is_ok()
                    {
                        d.ranges[i] = (range.lMin, range.lMax);
                    }
                }
                let _ = device.Acquire();
                d.device = Some(device);
                d.objects = objects;
                d.format = Some(format);
                Some(())
            }
        }
        pub fn poll(&mut self, name: &str) -> Option<PadState> {
            self.refresh();
            let index = self
                .devices
                .iter()
                .position(|d| d.name == name)
                .or_else(|| {
                    if name.starts_with("DirectInput:") {
                        None
                    } else {
                        (!self.devices.is_empty()).then_some(0)
                    }
                })?;
            let d = &mut self.devices[index];
            if d.device.is_none() {
                Self::prepare(&self.api, d, self.window)?;
            }
            let device = d.device.as_ref()?;
            let mut state = DIJOYSTATE2::default();
            state.rgdwPOV.fill(u32::MAX);
            unsafe {
                let _ = device.Poll();
                if device
                    .GetDeviceState(
                        size_of::<DIJOYSTATE2>() as u32,
                        &mut state as *mut _ as *mut c_void,
                    )
                    .is_err()
                {
                    device.Acquire().ok()?;
                    let _ = device.Poll();
                    device
                        .GetDeviceState(
                            size_of::<DIJOYSTATE2>() as u32,
                            &mut state as *mut _ as *mut c_void,
                        )
                        .ok()?;
                }
            }
            let raw_axes = [
                state.lX, state.lY, state.lZ, state.lRx, state.lRy, state.lRz,
            ];
            // Some DirectInput drivers report a non-centered raw value while
            // idle (notably when first acquired). Calibrate each attached pad
            // from its first successful sample so connecting it cannot hold
            // a game direction before the player touches a control.
            let centers = *d.axis_centers.get_or_insert(raw_axes);
            let axis = |i| {
                let (min, max) = d.ranges[i];
                normalize_centered(raw_axes[i], centers[i], min, max)
            };
            let (rx, ry) = if d.ranges[3].1 > d.ranges[3].0 && d.ranges[4].1 > d.ranges[4].0 {
                (axis(3), -axis(4))
            } else {
                (axis(2), -axis(5))
            };
            let pov_centers = *d.pov_centers.get_or_insert(state.rgdwPOV);
            let pov = changed_pov(state.rgdwPOV, pov_centers);
            Some(PadState {
                name: d.name.clone(),
                buttons: state.rgbButtons.map(|v| v & 128 != 0),
                axes: [axis(0), -axis(1), rx, ry],
                pov,
            })
        }
        /// Poll every attached device so switching pads is detected from
        /// actual input instead of a stale saved device name.
        pub fn poll_all(&mut self) -> Vec<PadState> {
            let names = self.names();
            names.iter().filter_map(|name| self.poll(name)).collect()
        }
    }
    impl Drop for Device {
        fn drop(&mut self) {
            if let Some(d) = &self.device {
                let _ = unsafe { d.Unacquire() };
            }
        }
    }
    impl Drop for DirectInput {
        fn drop(&mut self) {
            self.devices.clear();
            let _ = unsafe { DestroyWindow(self.window) };
        }
    }
}
#[cfg(not(windows))]
mod platform {
    use super::PadState;
    pub struct DirectInput;
    impl DirectInput {
        pub fn new() -> Option<Self> {
            None
        }
        pub fn names(&mut self) -> Vec<String> {
            Vec::new()
        }
        pub fn poll(&mut self, _: &str) -> Option<PadState> {
            None
        }
        pub fn poll_all(&mut self) -> Vec<PadState> {
            Vec::new()
        }
    }
}
pub use platform::DirectInput;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_buttons_and_pov_diagonals() {
        let mut pad = PadState {
            name: String::new(),
            buttons: [false; 128],
            axes: [0.0; 4],
            pov: Some(4500),
        };
        pad.buttons[27] = true;
        assert!(pad.has_activity(0.2));
        assert_eq!(pad.activity_tokens(0.2), ["button:27", "pov:1"]);
        assert!(pad.pressed("DInputButton27", 0.2));
        assert!(!pad.pressed("DInputButton128", 0.2));
        assert!(pad.pressed("DPadUp", 0.2) && pad.pressed("DPadRight", 0.2));
        pad.pov = None;
        pad.buttons.fill(false);
        assert!(!pad.pressed("DPadUp", 0.2));
        assert!(!pad.has_activity(0.2));
    }
    #[test]
    fn signed_ranges_and_stick_bindings() {
        assert_eq!(normalize(-32768, -32768, 32767), -1.0);
        assert_eq!(normalize(65535, 0, 65535), 1.0);
        assert!(normalize(32767, 0, 65535).abs() < 0.001);
        assert_eq!(normalize(0, 0, 0), 0.0);
        let pad = PadState {
            name: String::new(),
            buttons: [false; 128],
            axes: [0.0, 1.0, 0.0, 0.0],
            pov: None,
        };
        assert!(pad.pressed("LeftStickUp", 0.2));
    }

    #[test]
    fn directinput_first_sample_is_neutral_even_for_noncentered_idle_values() {
        assert_eq!(normalize_centered(0, 0, 0, 65535), 0.0);
        assert_eq!(normalize_centered(32767, 32767, -32768, 32767), 0.0);
        assert_eq!(normalize_centered(65535, 32767, 0, 65535), 1.0);
        assert_eq!(normalize_centered(0, 32767, 0, 65535), -1.0);
    }

    #[test]
    fn directinput_does_not_treat_an_initial_hat_value_as_a_held_direction() {
        assert_eq!(
            changed_pov(
                [0, u32::MAX, u32::MAX, u32::MAX],
                [0, u32::MAX, u32::MAX, u32::MAX]
            ),
            None
        );
        assert_eq!(
            changed_pov(
                [4500, u32::MAX, u32::MAX, u32::MAX],
                [0, u32::MAX, u32::MAX, u32::MAX]
            ),
            Some(4500)
        );
        assert_eq!(changed_pov([u32::MAX; 4], [u32::MAX; 4]), None);
    }
}
