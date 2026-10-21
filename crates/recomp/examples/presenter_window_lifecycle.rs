//! Optional no-ROM X11/Windows window-manager lifecycle check.
//! Run `cargo run --locked --release -p recomp --example presenter_window_lifecycle`
//! with a real window manager. Bare Xvfb and Wayland cannot prove this contract.
//! Add `--monitors` to check two distinct monitors, moving/resizing/rendering
//! left/right twice instead of repeating the minimize lifecycle.
use screen::{
    present::{GridParams, PresentOutcome, Presenter},
    profile::DisplayTarget,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[derive(Clone, Copy, Debug)]
enum Phase {
    Visible,
    Minimize,
    Restore,
    Resize,
    MonitorMove,
    MonitorResize,
}
struct Check {
    window: Option<Arc<Window>>,
    presenter: Option<Presenter>,
    phase: Phase,
    since: Instant,
    next_tick: Instant,
    cycles: usize,
    minimized_frames: usize,
    presented: usize,
    skipped: usize,
    rgba: Vec<[u8; 4]>,
    grid: GridParams,
    error: Option<String>,
    exited: bool,
    monitor_mode: bool,
    monitors: Vec<winit::monitor::MonitorHandle>,
}
impl Check {
    fn new(monitor_mode: bool) -> Self {
        let now = Instant::now();
        let mut grid = GridParams::with_strength(0.);
        grid.scale_mode = 1;
        Self {
            window: None,
            presenter: None,
            phase: Phase::Visible,
            since: now,
            next_tick: now,
            cycles: 0,
            minimized_frames: 0,
            presented: 0,
            skipped: 0,
            rgba: (0..240 * 160)
                .map(|i| {
                    if (i % 240 / 16 + i / 240 / 16) % 2 == 0 {
                        [32, 96, 200, 255]
                    } else {
                        [220, 200, 32, 255]
                    }
                })
                .collect(),
            grid,
            error: None,
            exited: false,
            monitor_mode,
            monitors: Vec::new(),
        }
    }
    fn phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.since = Instant::now();
    }
    fn draw(&mut self, window: &Window) -> Result<bool, String> {
        let size = window.inner_size();
        let outcome = self.presenter.as_mut().unwrap().present(
            &self.rgba,
            (240, 160),
            (size.width, size.height),
            &self.grid,
            [0., 0., 240., 160.],
        )?;
        match outcome {
            PresentOutcome::Presented => {
                self.presented += 1;
                Ok(true)
            }
            PresentOutcome::Skipped => {
                self.skipped += 1;
                Ok(false)
            }
        }
    }
    fn step(&mut self, events: &ActiveEventLoop) -> Result<(), String> {
        let window = self.window.as_ref().ok_or("window not resumed")?.clone();
        if self.since.elapsed() > Duration::from_secs(10) {
            return Err(format!(
                "timeout waiting for {:?}; minimized={:?}, size={:?}",
                self.phase,
                window.is_minimized(),
                window.inner_size()
            ));
        }
        let targets = [
            PhysicalSize::new(640, 360),
            PhysicalSize::new(960, 540),
            PhysicalSize::new(852, 480),
        ];
        match self.phase {
            Phase::Visible => {
                if self.draw(&window)? && self.since.elapsed() > Duration::from_millis(500) {
                    window.set_minimized(true);
                    self.phase(Phase::Minimize);
                }
            }
            Phase::Minimize => {
                match window.is_minimized() {
                    Some(true)=>{},Some(false)=>return Ok(()),
                    None=>return Err("minimized state unavailable; this backend cannot prove the lifecycle check".into()),
                }
                // Exercise actual acquisition while the WM holds the window
                // minimized. Either presented or skipped is valid here.
                self.draw(&window)?;
                self.minimized_frames += 1;
                if self.minimized_frames == 3 {
                    println!(
                        "cycle={} observed minimized=true size={:?}",
                        self.cycles + 1,
                        window.inner_size()
                    );
                    window.set_minimized(false);
                    self.phase(Phase::Restore);
                }
            }
            Phase::Restore => {
                if window.is_minimized() == Some(false) && self.draw(&window)? {
                    println!(
                        "cycle={} observed restored and presented size={:?}",
                        self.cycles + 1,
                        window.inner_size()
                    );
                    let _ = window.request_inner_size(targets[self.cycles]);
                    self.phase(Phase::Resize);
                }
            }
            Phase::MonitorMove => {
                let expected = &self.monitors[self.cycles % 2];
                if window.current_monitor().as_ref() == Some(expected) && self.draw(&window)? {
                    let expected = &self.monitors[self.cycles % 2];
                    println!(
                        "placement={} monitor={:?} origin={:?} window={:?} scale={} presented",
                        self.cycles,
                        expected.name(),
                        expected.position(),
                        window.outer_position(),
                        window.scale_factor()
                    );
                    let size = expected.size();
                    let _ = window.request_inner_size(PhysicalSize::new(
                        (size.width / 2).clamp(240, 960),
                        (size.height / 2).clamp(160, 540),
                    ));
                    self.phase(Phase::MonitorResize);
                }
            }
            Phase::MonitorResize => {
                let expected = &self.monitors[self.cycles % 2];
                let size = expected.size();
                let target = PhysicalSize::new(
                    (size.width / 2).clamp(240, 960),
                    (size.height / 2).clamp(160, 540),
                );
                if window.current_monitor().as_ref() == Some(expected)
                    && window.inner_size() == target
                    && self.draw(&window)?
                {
                    for enabled in [true, false] {
                        self.presenter.as_mut().unwrap().set_vsync(enabled)?;
                        if !self.draw(&window)? {
                            return Err("monitor VSync check skipped presentation".into());
                        }
                    }
                    println!(
                        "placement={} resize={:?}, VSync on/off presented",
                        self.cycles, target
                    );
                    self.cycles += 1;
                    if self.cycles == 5 {
                        events.exit();
                    } else {
                        let origin = self.monitors[self.cycles % 2].position();
                        window.set_outer_position(winit::dpi::PhysicalPosition::new(
                            origin.x + 40,
                            origin.y + 40,
                        ));
                        self.phase(Phase::MonitorMove);
                    }
                }
            }
            Phase::Resize => {
                if window.inner_size() == targets[self.cycles] && self.draw(&window)? {
                    for enabled in [true, false] {
                        self.presenter.as_mut().unwrap().set_vsync(enabled)?;
                        if !self.draw(&window)? {
                            return Err("visible VSync check skipped a presentation".into());
                        }
                    }
                    println!(
                        "cycle={} resized={:?}, VSync on/off presented",
                        self.cycles + 1,
                        window.inner_size()
                    );
                    self.cycles += 1;
                    self.minimized_frames = 0;
                    if self.cycles == 3 {
                        events.exit();
                    } else {
                        window.set_minimized(true);
                        self.phase(Phase::Minimize);
                    }
                }
            }
        }
        Ok(())
    }
}
impl ApplicationHandler for Check {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| {
            let window = Arc::new(
                events
                    .create_window(
                        Window::default_attributes()
                            .with_title("FFTA Presenter lifecycle diagnostic")
                            .with_inner_size(PhysicalSize::new(852, 480)),
                    )
                    .map_err(|e| e.to_string())?,
            );
            let presenter = Presenter::new(window.clone(), 240, 160, DisplayTarget::Srgb, false)?;
            self.presenter = Some(presenter);
            self.window = Some(window);
            if self.monitor_mode {
                self.monitors = self.window.as_ref().unwrap().available_monitors().collect();
                self.monitors
                    .sort_by_key(|m| (m.position().x, m.position().y));
                if self.monitors.len() < 2
                    || self.monitors[0] == self.monitors[1]
                    || self.monitors[0].position() == self.monitors[1].position()
                {
                    return Err(
                        "monitor mode requires two separately positioned enumerated monitors"
                            .into(),
                    );
                }
                self.monitors.truncate(2);
                if self
                    .monitors
                    .iter()
                    .any(|m| m.size().width < 1000 || m.size().height < 600)
                {
                    return Err("monitor mode needs at least 1000x600 per monitor for the diagnostic window".into());
                }
                println!(
                    "monitors={:?}",
                    self.monitors
                        .iter()
                        .map(|m| (m.name(), m.position(), m.size(), m.scale_factor()))
                        .collect::<Vec<_>>()
                );
                let origin = self.monitors[0].position();
                self.window.as_ref().unwrap().set_outer_position(
                    winit::dpi::PhysicalPosition::new(origin.x + 40, origin.y + 40),
                );
                self.phase(Phase::MonitorMove);
            } else {
                self.phase(Phase::Visible);
            }
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            self.error = Some(error);
            events.exit();
        }
    }
    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if matches!(event, WindowEvent::CloseRequested) {
            self.error = Some("window closed before lifecycle checks completed".into());
            events.exit();
        }
    }
    fn about_to_wait(&mut self, events: &ActiveEventLoop) {
        if Instant::now() >= self.next_tick && self.error.is_none() {
            if let Err(error) = self.step(events) {
                self.error = Some(error);
                events.exit();
            }
            self.next_tick = Instant::now() + Duration::from_millis(100);
        }
        events.set_control_flow(ControlFlow::WaitUntil(self.next_tick));
    }
    fn exiting(&mut self, _: &ActiveEventLoop) {
        // Same surface-before-window ordering as the desktop runtime.
        drop(self.presenter.take());
        drop(self.window.take());
        self.exited = true;
    }
}
fn main() {
    let events = EventLoop::new().unwrap();
    let args: Vec<_> = std::env::args().skip(1).collect();
    let monitor_mode = match args.as_slice() {
        [] => false,
        [arg] if arg == "--monitors" => true,
        _ => panic!("usage: presenter_window_lifecycle [--monitors]"),
    };
    let mut check = Check::new(monitor_mode);
    events.run_app(&mut check).unwrap();
    if let Some(error) = check.error {
        panic!("lifecycle check failed: {error}");
    }
    assert!(
        check.exited && check.cycles == if monitor_mode { 5 } else { 3 },
        "lifecycle did not finish"
    );
    if monitor_mode {
        println!("PASS 4 observed monitor transitions, 5 placements/resizes, VSync on/off, clean teardown; presented={} skipped={}", check.presented, check.skipped);
    } else {
        println!("PASS 3 observed WM minimize/restore cycles, resize, VSync on/off, clean teardown; presented={} skipped={}",check.presented,check.skipped);
    }
}
