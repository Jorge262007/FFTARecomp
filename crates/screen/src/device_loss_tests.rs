//! Private, opt-in real-device destruction regression; no ROM or runtime knob.
use super::{GridParams, PresentOutcome, Presenter};
use crate::{
    profile::DisplayTarget,
    ui_layer::{Frame, Layout},
};
use std::{num::NonZeroU32, sync::Arc};
use winit::{dpi::PhysicalSize, event_loop::EventLoop, window::Window};

#[test]
#[ignore = "requires an X11/Windows desktop, GPU and software surface; run alone"]
fn destroyed_device_returns_error_then_software_can_present() {
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_x11().with_any_thread(true);
    }
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        builder.with_any_thread(true);
    }
    let events = builder.build().unwrap();
    #[allow(deprecated)]
    let window = Arc::new(
        events
            .create_window(
                Window::default_attributes()
                    .with_title("FFTA private device-loss regression")
                    .with_inner_size(PhysicalSize::new(640, 480)),
            )
            .unwrap(),
    );
    let pixels = vec![[0, 0, 16, 0]; 480 * 160];
    let world = vec![0u16; 240 * 160];
    let lut = vec![[80, 160, 220, 255]; 32768];
    let rgba = vec![[32, 96, 200, 255]; 240 * 160];
    let frame = Frame {
        pixels: &pixels,
        pixels_revision: Some(1),
        raw_world: &world,
        world_dims: (240, 160),
        lut: &lut,
        layout: Layout {
            rect: [0., 0., 640., 480.],
            clip: [0., 0., 640., 480.],
        },
        world_view: [0., 0., 240., 160.],
    };
    let grid = GridParams::with_strength(0.);
    let view = [0., 0., 240., 160.];
    let resized_rgba = vec![[32, 96, 200, 255]; 320 * 160];
    for scenario in ["plain", "cached_ui", "new_ui", "resize"] {
        let mut p = Presenter::new(window.clone(), 240, 160, DisplayTarget::Srgb, false).unwrap();
        if scenario == "plain" {
            // Recoverable scoped errors must not poison the owned-device latch.
            let scoped = super::configuration_result(&p.device, || {
                let _ = p.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("scoped-error-test"),
                    size: 4,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::UNIFORM,
                    mapped_at_creation: false,
                });
            });
            assert!(scoped.is_err());
            p.device_result().unwrap();
        }
        if scenario == "cached_ui" {
            p.set_ui_layers(&[Some(frame)]);
        }
        assert_eq!(
            p.present(&rgba, (240, 160), (640, 480), &grid, view)
                .unwrap(),
            PresentOutcome::Presented
        );
        p.device.destroy();
        if scenario == "cached_ui" || scenario == "new_ui" {
            p.set_ui_layers(&[Some(frame)]);
        }
        let (post_rgba, dims) = if scenario == "resize" {
            (&resized_rgba, (320, 160))
        } else {
            (&rgba, (240, 160))
        };
        let error = p
            .present(post_rgba, dims, (640, 480), &grid, view)
            .expect_err("destroyed device must not report a presentation or transient skip");
        eprintln!("DEVICE_LOSS scenario={scenario} returned={error}");
        if scenario == "new_ui" || scenario == "resize" {
            // Keep the original error, suppress further writes, and reject VSync
            // before it can reconfigure an already failed device.
            p.set_ui_layers(&[Some(frame)]);
            assert_eq!(p.ui_uploaded_bytes(), 0);
            assert_eq!(
                p.present(post_rgba, dims, (640, 480), &grid, view)
                    .unwrap_err(),
                error
            );
            assert_eq!(p.set_vsync(true).unwrap_err(), error);
        }
        drop(p); // Release the GPU surface before creating the CPU surface.
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
        surface
            .resize(NonZeroU32::new(640).unwrap(), NonZeroU32::new(480).unwrap())
            .unwrap();
        let mut buffer = surface.buffer_mut().unwrap();
        buffer.fill(0x002060C8);
        buffer.present().unwrap();
        drop(surface);
        drop(context);
    }
    drop(window);
    drop(events);
    eprintln!("PASS destroyed device plain/cached/new UI/resize, software present and teardown");
}
