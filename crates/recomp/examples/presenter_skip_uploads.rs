//! Optional GPU lifecycle regression: run with a working desktop or Xvfb using
//! cargo run --locked --release -p recomp --example presenter_skip_uploads
//! No ROM, save data, GPU timestamps or blocking GPU poll is required.
use screen::{
    present::{GridParams, PresentOutcome, Presenter},
    profile::DisplayTarget,
    ui_layer::{Frame, Layout},
};
use std::sync::Arc;
fn memory() -> String {
    match std::fs::read_to_string("/proc/self/status") {
        Ok(status) => status
            .lines()
            .filter(|line| line.starts_with("VmRSS:") || line.starts_with("VmHWM:"))
            .collect::<Vec<_>>()
            .join(", "),
        Err(_) => "process memory observation unavailable on this platform".into(),
    }
}
fn main() {
    let events = winit::event_loop::EventLoop::new().unwrap();
    #[allow(deprecated)]
    let window = Arc::new(
        events
            .create_window(
                winit::window::Window::default_attributes()
                    .with_inner_size(winit::dpi::PhysicalSize::new(852, 480)),
            )
            .unwrap(),
    );
    let mut p = Presenter::new(window.clone(), 240, 160, DisplayTarget::Srgb, false).unwrap();
    let rgba = vec![[40, 60, 80, 255]; 240 * 160];
    let ui = vec![[0, 0, 16, 0]; 480 * 160];
    let mut world = vec![0u16; 640 * 320];
    let mut lut = vec![[200, 80, 40, 255]; 32768];
    let layout = Layout {
        rect: [0., 0., 852., 480.],
        clip: [0., 0., 852., 480.],
    };
    let grid = GridParams::with_strength(0.);
    let view = [0., 0., 240., 160.];
    // Warm the normal surface path before observing repeated skipped calls.
    assert_eq!(
        p.present(&rgba, (240, 160), (852, 480), &grid, view)
            .unwrap(),
        PresentOutcome::Presented
    );
    println!("before {}", memory());
    let mut bytes = 0;
    for i in 0..1200 {
        world[0] = (i % 32768) as u16;
        if i == 600 {
            lut[0] = [40, 200, 80, 255];
        }
        let frame = Frame {
            pixels: &ui,
            pixels_revision: Some(1),
            raw_world: &world,
            world_dims: (640, 320),
            lut: &lut,
            layout,
            world_view: [0., 0., 640., 320.],
        };
        p.set_ui_layers(&[Some(frame)]);
        bytes += p.ui_uploaded_bytes();
        if i % 2 == 0 {
            p.set_ui_layer(None);
        }
        assert_eq!(
            p.present(&rgba, (240, 160), (0, 480), &grid, view).unwrap(),
            PresentOutcome::Skipped
        );
        if i == 599 || i == 1199 {
            println!("after {} skips, bytes={} {}", i + 1, bytes, memory());
        }
    }
    // Reuse pixel revisions after skips and a changed LUT/world.
    let frame = Frame {
        pixels: &ui,
        pixels_revision: Some(1),
        raw_world: &world,
        world_dims: (640, 320),
        lut: &lut,
        layout,
        world_view: [0., 0., 640., 320.],
    };
    p.set_ui_layers(&[Some(frame)]);
    assert_eq!(
        p.present(&rgba, (240, 160), (852, 480), &grid, view)
            .unwrap(),
        PresentOutcome::Presented
    );
    p.set_ui_layer(None);
    assert_eq!(
        p.present(&rgba, (240, 160), (0, 0), &grid, view).unwrap(),
        PresentOutcome::Skipped
    );
    assert_eq!(
        p.present(&rgba, (240, 160), (852, 480), &grid, view)
            .unwrap(),
        PresentOutcome::Presented
    );
    drop(p);
    drop(window);
    drop(events);
    println!(
        "PASS 1200 zero-size UI batches, hide/reactivate, LUT mutation, restore, clean teardown"
    );
}
