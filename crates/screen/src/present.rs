//! GPU presenter: attaches a wgpu surface to an existing window (anything
//! implementing raw-window-handle 0.6) and draws the post-LUT frame through
//! the band-limited grid/scaling pass.
//!
//! Pacing: emulation speed is owned by the audio clock, never the display.
//! A 60/120/144 Hz panel, ProMotion/G-Sync/FreeSync, or a throttling
//! compositor must not change game speed; the presenter just shows the
//! latest frame whenever it's asked to. Vsync is therefore a pure
//! PRESENTATION choice (tear-free vs lowest latency): off =
//! PresentMode::AutoNoVsync, on = PresentMode::AutoVsync, switchable live
//! via [`Presenter::set_vsync`] — neither setting ever paces the emulator.
//!
//! Color: the surface preferentially is 8-bit UNORM and receives
//! display-encoded bytes (the LUT already encoded for the chosen target).
//! Some GL/Vulkan/compositor combinations only expose *Srgb surface
//! formats; on those the surface view re-encodes whatever we write, so the
//! shader pre-applies the inverse (the exact piecewise sRGB EOTF) to its
//! final color — the hardware re-encode then reproduces the intended
//! bytes (flag in the params buffer, slot 13). On macOS we tag the
//! underlying layer with the matching colorspace so the compositor
//! color-matches our output to the actual panel — that is what makes the
//! simulation hold on wide-gamut (P3) laptop screens without us measuring
//! anything. On platforms without compositor color management the sRGB
//! assumption applies (with a manual wide-gamut override in the config).

use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::profile::DisplayTarget;

#[path = "gpu_timing.rs"]
mod gpu_timing;
pub use gpu_timing::{GpuTimingCounts, GpuTimings};

/// Per-frame grid/scaling parameters; resolved from config + screen kind.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GridParams {
    /// 0 = plain sharp scaling, 1 = full physical grid.
    pub strength: f32,
    /// Published parameterization of the band-limited grid for this panel
    /// class: gain/gamma/tint and aperture half-widths.
    pub gain: f32,
    pub gamma: f32,
    pub tint: f32,
    pub stripe_halfwidth: f32,
    pub row_halfwidth: f32,
    pub bgr: bool,
    /// Base scaler under the grid: 0 = off (exact area average, the
    /// original behaviour), 1 = nearest, 2 = sharp bilinear.
    pub scale_mode: u32,
}

impl GridParams {
    /// The published grid parameter set for this panel family (gain 1.5,
    /// linearization 2.2, stripe self-transmission 0.75, BGR stripe order,
    /// stripe aperture half-width 1.5 subpixels, row half-width 0.63).
    pub fn with_strength(strength: f32) -> GridParams {
        GridParams {
            strength: strength.clamp(0.0, 1.0),
            gain: 1.5,
            gamma: 2.2,
            tint: 0.75,
            stripe_halfwidth: 1.5,
            row_halfwidth: 0.63,
            bgr: true,
            scale_mode: 0,
        }
    }
}

/// Aspect-correct letterbox of a `view_w` x `src_h` image into a window of
/// `window` physical pixels. Returns `(scale, origin_x, origin_y)`. Shared
/// with the play loop so mouse coordinates map with the exact same math.
pub fn letterbox(window: (u32, u32), view_w: u32, src_h: u32) -> (f32, f32, f32) {
    letterbox_f(window, view_w.max(1) as f32, src_h.max(1) as f32)
}

/// Same as [`letterbox`] for a fractional view size (the zoom crop keeps the
/// aspect ratio exactly, so its size is not always a whole number of pixels).
pub fn letterbox_f(window: (u32, u32), vw: f32, vh: f32) -> (f32, f32, f32) {
    let (w, h) = (window.0.max(1) as f32, window.1.max(1) as f32);
    let (vw, vh) = (vw.max(1.0), vh.max(1.0));
    let scale = (w / vw).min(h / vh);
    (scale, (w - vw * scale) * 0.5, (h - vh * scale) * 0.5)
}

/// Optional CPU wall times for one presentation, not GPU execution durations.
/// None means that stage was not reached (for example after acquire failure).
#[derive(Clone, Copy, Default)]
pub struct CpuTimings {
    pub setup_upload: Option<Duration>,
    pub acquire: Option<Duration>,
    pub encode: Option<Duration>,
    pub submit: Option<Duration>,
    pub present: Option<Duration>,
    pub timestamp_readback: Option<Duration>,
}

/// A skipped surface frame is not a successful host presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentOutcome {
    Presented,
    Skipped,
}

// Queue writes allocate staging immediately but only start at submit. Track
// them across layer updates (including clearing the layers) so skipped frames
// cannot retain an unbounded sequence of unsubmitted copies.
#[derive(Default)]
struct PendingUploads(bool);

impl PendingUploads {
    fn mark(&mut self) { self.0 = true; }
    fn submitted(&mut self) { self.0 = false; }
    fn flush(&mut self, submit: impl FnOnce()) -> bool {
        if !self.0 { return false; }
        submit();
        self.submitted();
        true
    }
}

#[derive(Default)]
struct SurfaceRecovery {
    failed_recoveries: u8,
}

impl SurfaceRecovery {
    // At most one reconfigure/retry per call and three failed calls before
    // software fallback. Timeouts do not reconfigure or spin: the normal
    // event-loop pacing schedules the next attempt.
    fn acquire<T>(
        &mut self,
        mut acquire: impl FnMut(bool) -> Result<T, wgpu::SurfaceError>,
    ) -> Result<Option<T>, String> {
        use wgpu::SurfaceError::{Lost, Outdated, Timeout};
        let result = match acquire(false) {
            Err(Lost | Outdated) => acquire(true),
            result => result,
        };
        match result {
            Ok(frame) => {
                self.failed_recoveries = 0;
                Ok(Some(frame))
            }
            Err(Timeout) => Ok(None),
            Err(error @ (Lost | Outdated)) => {
                self.failed_recoveries = self.failed_recoveries.saturating_add(1);
                if self.failed_recoveries >= 3 {
                    Err(format!("surface recovery exhausted: {error}"))
                } else {
                    Ok(None)
                }
            }
            // Memory exhaustion and unknown errors are not resize signals.
            Err(error) => Err(error.to_string()),
        }
    }
}

/// Configuration is infrequent and may fail even after capability discovery.
/// Capture backend errors without replacing the device's uncaptured-error
/// handler. Always pop every scope, including when a nested scope caught one.
/// On the pinned native wgpu backend, pop_error_scope returns a ready future;
/// no GPU polling/readback wait is introduced here.
fn configuration_result(device: &wgpu::Device, configure: impl FnOnce()) -> Result<(), String> {
    for filter in [wgpu::ErrorFilter::Validation, wgpu::ErrorFilter::OutOfMemory, wgpu::ErrorFilter::Internal] {
        device.push_error_scope(filter);
    }
    configure();
    let mut error = None;
    for _ in 0..3 {
        if let Some(next) = pollster::block_on(device.pop_error_scope()) {
            error.get_or_insert(next);
        }
    }
    match error {
        Some(error) => Err(format!("surface configuration failed: {error}")),
        None => Ok(()),
    }
}

fn configure_surface(surface: &wgpu::Surface<'_>, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Result<(), String> {
    configuration_result(device, || surface.configure(device, config))
}

pub struct Presenter {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    failure: Arc<OnceLock<String>>,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    texture: wgpu::Texture,
    uniforms: wgpu::Buffer,
    config: wgpu::SurfaceConfiguration,
    /// True when the surface format is an sRGB view (only *Srgb formats
    /// were available): the shader must pre-decode its display-encoded
    /// output so the hardware re-encode is an identity overall.
    srgb_surface: bool,
    src_size: (u32, u32),
    ui_layer: Vec<Option<crate::ui_layer::Gpu>>,
    cpu_timings: Option<CpuTimings>,
    ui_uploaded_bytes: usize,
    pending_uploads: PendingUploads,
    recovery: SurfaceRecovery,
    gpu_profile: Option<gpu_timing::GpuProfiler>,
    gpu_sampling: bool,
}

impl Presenter {
    /// Create a presenter on `window`'s surface.
    ///
    /// `vsync` selects the present mode only (false = AutoNoVsync, true =
    /// AutoVsync): presentation pacing, never emulation pacing — game
    /// speed is owned by the audio clock (see module docs).
    ///
    /// The surface takes an owning `Arc` clone of `window`, so the window is
    /// guaranteed to outlive the surface — no `unsafe`, no scope contract.
    pub fn new<W>(
        window: Arc<W>,
        src_w: u32,
        src_h: u32,
        target: DisplayTarget,
        vsync: bool,
    ) -> Result<Presenter, String>
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        Self::new_profiled(window, src_w, src_h, target, vsync, false)
    }

    /// As `new`, optionally requesting render-pass timestamps when supported.
    /// Unsupported timestamps never prevent normal presenter creation.
    pub fn new_profiled<W>(
        window: Arc<W>, src_w: u32, src_h: u32, target: DisplayTarget,
        vsync: bool, gpu_timestamps: bool,
    ) -> Result<Presenter, String>
    where W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| format!("surface: {e}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: if cfg!(target_os = "linux") {
                wgpu::PowerPreference::HighPerformance
            } else {
                wgpu::PowerPreference::LowPower
            },
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .map_err(|e| format!("no graphics adapter: {e}"))?;
        let adapter_info = adapter.get_info();
        eprintln!(
            "video: adapter={} backend={:?} type={:?} driver={} ({})",
            adapter_info.name,
            adapter_info.backend,
            adapter_info.device_type,
            adapter_info.driver,
            adapter_info.driver_info,
        );
        let mut timestamp_features = gpu_timing::requested_features(gpu_timestamps, adapter.features());
        let requested_device = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("screen-presenter"), required_features: timestamp_features,
            ..Default::default()
        }));
        let (device, queue) = match requested_device {
            Err(error) if !timestamp_features.is_empty() => {
                eprintln!("video: timestamp device unavailable ({error}); retrying without optional features");
                timestamp_features = wgpu::Features::empty();
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    label: Some("screen-presenter"), ..Default::default()
                }))
            }
            result => result,
        }.map_err(|error| format!("graphics device: {error}"))?;
        // This Presenter owns the Device; retain its first fatal error instead
        // of letting uncaptured validation errors panic during resource creation
        // after device loss. Error scopes still handle local configuration errors.
        let failure = Arc::new(OnceLock::new());
        let errors = failure.clone();
        device.on_uncaptured_error(Box::new(move |error| {
            errors.get_or_init(|| format!("graphics error: {error}"));
        }));
        let lost = failure.clone();
        device.set_device_lost_callback(move |reason, message| {
            lost.get_or_init(|| format!("graphics device lost ({reason:?}): {message}"));
        });
        if gpu_timestamps {
            eprintln!("video: GPU render-pass timestamps {}", if timestamp_features.is_empty() { "unavailable; CPU profiling remains available" } else { "enabled (deferred readback, 3 slots, 18000 samples)" });
        }

        let caps = surface.get_capabilities(&adapter);
        // Display-encoded bytes in, display-encoded bytes out: prefer a
        // UNORM (non-sRGB-view) format so nothing re-encodes behind our
        // back. Some GL/Vulkan/compositor combos only expose *Srgb
        // formats — accept those and have the shader pre-decode its final
        // color (piecewise sRGB EOTF) so the hardware re-encode reproduces
        // the intended bytes.
        let format = [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ]
        .into_iter()
        .find(|f| caps.formats.contains(f))
        .ok_or_else(|| format!("no 8-bit surface format in {:?}", caps.formats))?;
        let srgb_surface = format.is_srgb();
        // Opaque when supported (we always write alpha 1 and want no
        // compositor blending); otherwise whatever the platform prefers.
        let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            caps.alpha_modes[0]
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            // Guard against a zero-sized (e.g. minimized) start; present()
            // resizes to the real window size on the first frame.
            width: (src_w * 3).max(1),
            height: (src_h * 3).max(1),
            present_mode: if vsync {
                wgpu::PresentMode::AutoVsync
            } else {
                wgpu::PresentMode::AutoNoVsync
            },
            desired_maximum_frame_latency: 1,
            alpha_mode,
            view_formats: vec![],
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screen-src"),
            size: wgpu::Extent3d {
                width: src_w,
                height: src_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen-params"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("screen-bind"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("screen-bind"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniforms.as_entire_binding(),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("screen-grid"),
            source: wgpu::ShaderSource::Wgsl(include_str!("grid.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("screen-pipeline"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("screen-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let gpu_profile = (!timestamp_features.is_empty()).then(|| gpu_timing::GpuProfiler::new(&device, &queue));
        let p = Presenter {
            surface,
            device,
            queue,
            failure,
            pipeline,
            bind_group,
            texture,
            uniforms,
            config,
            srgb_surface,
            src_size: (src_w, src_h),
            ui_layer: Vec::new(),
            cpu_timings: None,
            pending_uploads: PendingUploads::default(),
            recovery: SurfaceRecovery::default(),
            ui_uploaded_bytes: 0,
            gpu_profile,
            gpu_sampling: false,
        };
        configure_surface(&p.surface, &p.device, &p.config)?;
        // Color-match the layer to the panel (macOS only); the surface +
        // CAMetalLayer now exist on the window's view.
        #[cfg(target_os = "macos")]
        macos::tag_layer_colorspace(macos::ns_view_ptr(window.as_ref()), target);
        // macOS consumes `target` above. Elsewhere the LUT was already encoded
        // for the resolved primaries (see `display::resolve`) and we declare
        // nothing to the compositor — Wayland treats untagged surfaces as sRGB.
        let _ = target;
        p.device_result()?;
        Ok(p)
    }

    fn device_result(&self) -> Result<(), String> {
        match self.failure.get() {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    /// Collect CPU wall times and sample this frame if GPU timestamps were
    /// requested at creation. Disabling sampling still drains earlier readbacks.
    pub fn enable_cpu_profiling(&mut self, enabled: bool) {
        self.cpu_timings = enabled.then(CpuTimings::default);
        self.gpu_sampling = enabled;
    }

    pub fn cpu_timings(&self) -> Option<CpuTimings> { self.cpu_timings }

    /// Drain once after each present, including skipped/error presentations.
    /// Results refer to earlier sampled passes and may arrive while paused.
    pub fn take_gpu_timings(&mut self) -> Option<GpuTimings> {
        self.gpu_profile.as_mut().map(|profile| profile.take())
    }

    pub fn ui_uploaded_bytes(&self) -> usize { self.ui_uploaded_bytes }

    /// Independent UI passes in stable layer order; absent slots clear immediately.
    pub fn set_ui_layers(&mut self,frames:&[Option<crate::ui_layer::Frame<'_>>]) {
        if self.failure.get().is_some() { self.ui_uploaded_bytes = 0; return; }
        self.ui_uploaded_bytes = crate::ui_layer::update_layers(
            &mut self.ui_layer, &self.device, &self.queue,
            self.config.format, self.srgb_surface, frames,
        );
        if self.failure.get().is_some() { self.ui_uploaded_bytes = 0; }
        if self.ui_uploaded_bytes != 0 { self.pending_uploads.mark(); }
    }

    pub fn set_ui_layer(&mut self,frame:Option<&crate::ui_layer::Frame<'_>>) {
        self.set_ui_layers(&[frame.copied()]);
    }

    fn flush_uploads(&mut self) {
        // Submit transfers only: no draw, surface present, timestamp reservation,
        // or GPU wait. Revision caches remain valid on the next visible frame.
        let start = self.cpu_timings.as_ref().map(|_| Instant::now());
        if self.pending_uploads.flush(|| { self.queue.submit([]); }) {
            if let (Some(times), Some(start)) = (self.cpu_timings.as_mut(), start) {
                times.submit = Some(start.elapsed());
            }
        }
    }

    /// Switch presentation mode live (false = AutoNoVsync, true =
    /// AutoVsync). Pacing principle: emulation speed is owned by the audio
    /// clock; vsync only affects presentation (tearing vs latency), never
    /// game speed — see the module header.
    pub fn set_vsync(&mut self, on: bool) -> Result<(), String> {
        self.device_result()?;
        let mode = if on {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        if self.config.present_mode != mode {
            self.config.present_mode = mode;
            self.config.width = self.config.width.max(1);
            self.config.height = self.config.height.max(1);
            configure_surface(&self.surface, &self.device, &self.config)?;
        }
        Ok(())
    }

    /// Upload the frame and draw it at the window's current size.
    /// Transient acquisition failures return `Skipped`; retain the presenter
    /// and try again on the next normally paced frame. Errors request fallback.
    /// `window_physical` is the surface size in physical pixels (winit's
    /// `inner_size()` reports this directly and cross-platform).
    ///
    /// `frame_dims` = (width, height) of `rgba`, which changes with the zoom
    /// (the core only renders what the zoom shows). The texture is allocated
    /// for the largest frame and only this rectangle is uploaded.
    ///
    /// `view` = [x0, y0, w, h] in frame pixels: the zoom crop, fractional so
    /// it can keep the window's aspect ratio exactly.
    pub fn present(
        &mut self,
        rgba: &[[u8; 4]],
        frame_dims: (u32, u32),
        window_physical: (u32, u32),
        grid: &GridParams,
        view: [f32; 4],
    ) -> Result<PresentOutcome, String> {
        if let Some(times) = self.cpu_timings.as_mut() { *times = CpuTimings::default(); }
        self.device_result()?;
        if let Some(profile) = self.gpu_profile.as_mut() {
            let start = self.cpu_timings.as_ref().map(|_| Instant::now());
            profile.poll(&self.device);
            if let (Some(times), Some(start)) = (self.cpu_timings.as_mut(), start) {
                times.timestamp_readback = Some(start.elapsed());
            }
        }
        if window_physical.0 == 0 || window_physical.1 == 0 {
            self.flush_uploads();
            return Ok(PresentOutcome::Skipped);
        }
        let upload_start = self.cpu_timings.as_ref().map(|_| Instant::now());
        let (w, h) = (window_physical.0.max(1), window_physical.1.max(1));
        if (w, h) != (self.config.width, self.config.height) {
            self.config.width = w;
            self.config.height = h;
            configure_surface(&self.surface, &self.device, &self.config)?;
        }

        // Menus have their own high-resolution canvas; grow once when needed.
        // Gameplay frame dimensions and filtering remain unchanged.
        if frame_dims.0 > self.src_size.0 || frame_dims.1 > self.src_size.1 {
            let width = frame_dims.0.max(self.src_size.0);
            let height = frame_dims.1.max(self.src_size.1);
            if width > self.device.limits().max_texture_dimension_2d
                || height > self.device.limits().max_texture_dimension_2d
            {
                return Err("UI texture exceeds device limits".into());
            }
            self.texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("screen-src"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("screen-bind"),
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &self
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.uniforms.as_entire_binding(),
                    },
                ],
            });
            self.src_size = (width, height);
            self.device_result()?;
        }
        let (tw, th) = self.src_size;
        let (sw, sh) = (frame_dims.0.clamp(1, tw), frame_dims.1.clamp(1, th));
        debug_assert_eq!(rgba.len(), (sw * sh) as usize);
        self.pending_uploads.mark();
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck_cast(rgba),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(sw * 4),
                rows_per_image: Some(sh),
            },
            wgpu::Extent3d {
                width: sw,
                height: sh,
                depth_or_array_layers: 1,
            },
        );

        // Zoom crop: a rectangle of the rendered frame.
        let vw = view[2].clamp(1.0, sw as f32);
        let vh = view[3].clamp(1.0, sh as f32);
        let vx0 = view[0].clamp(0.0, sw as f32 - vw);
        let vy0 = view[1].clamp(0.0, sh as f32 - vh);
        // Aspect-correct letterbox of the visible view, centered.
        let (scale, ox, oy) = letterbox_f((w, h), vw, vh);
        let dst = (vw * scale, vh * scale);
        let origin = (ox, oy);
        let params: [f32; 20] = [
            vw,
            vh,
            origin.0,
            origin.1,
            dst.0,
            dst.1,
            grid.strength,
            grid.gain,
            grid.gamma,
            grid.tint,
            grid.stripe_halfwidth,
            grid.row_halfwidth,
            if grid.bgr { 1.0 } else { 0.0 },
            if self.srgb_surface { 1.0 } else { 0.0 },
            vx0,
            vy0,
            sw as f32,
            sh as f32,
            grid.scale_mode as f32,
            0.0,
        ];
        self.queue
            .write_buffer(&self.uniforms, 0, as_bytes(&params));
        if let (Some(times), Some(start)) = (self.cpu_timings.as_mut(), upload_start) {
            times.setup_upload = Some(start.elapsed());
        }

        let acquire_start = self.cpu_timings.as_ref().map(|_| Instant::now());
        let mut configuration_error = None;
        let frame = self.recovery.acquire(|reconfigure| {
            if reconfigure {
                if let Err(error) = configure_surface(&self.surface, &self.device, &self.config) {
                    configuration_error = Some(error);
                    return Err(wgpu::SurfaceError::Other);
                }
            }
            self.surface.get_current_texture()
        });
        let frame = configuration_error.map_or(frame, Err);
        if let (Some(times), Some(start)) = (self.cpu_timings.as_mut(), acquire_start) {
            times.acquire = Some(start.elapsed());
        }
        self.device_result()?;
        let Some(frame) = frame? else {
            self.flush_uploads();
            return Ok(PresentOutcome::Skipped);
        };
        let encode_start = self.cpu_timings.as_ref().map(|_| Instant::now());
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let sample = if self.gpu_sampling {
            self.gpu_profile.as_mut().and_then(|profile| profile.reserve())
        } else { None };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("screen-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: sample.map(|index| self.gpu_profile.as_ref().unwrap().writes(index)),
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
            for ui in self.ui_layer.iter().flatten() {ui.draw(&mut pass, (self.config.width, self.config.height));}
        }
        if let Some(index) = sample { self.gpu_profile.as_ref().unwrap().resolve(&mut encoder, index); }
        let command = encoder.finish();
        self.device_result()?;
        if let (Some(times), Some(start)) = (self.cpu_timings.as_mut(), encode_start) {
            times.encode = Some(start.elapsed());
        }
        let submit_start = self.cpu_timings.as_ref().map(|_| Instant::now());
        self.queue.submit([command]);
        self.pending_uploads.submitted();
        self.device_result()?;
        if let Some(index) = sample { self.gpu_profile.as_ref().unwrap().map(index); }
        if let (Some(times), Some(start)) = (self.cpu_timings.as_mut(), submit_start) {
            times.submit = Some(start.elapsed());
        }
        let present_start = self.cpu_timings.as_ref().map(|_| Instant::now());
        frame.present();
        if let (Some(times), Some(start)) = (self.cpu_timings.as_mut(), present_start) {
            times.present = Some(start.elapsed());
        }
        Ok(PresentOutcome::Presented)
    }
}

fn bytemuck_cast(rgba: &[[u8; 4]]) -> &[u8] {
    // [[u8;4]] is layout-identical to a flat byte slice.
    unsafe { std::slice::from_raw_parts(rgba.as_ptr().cast(), rgba.len() * 4) }
}

fn as_bytes(v: &[f32; 20]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast(), 80) }
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
#[path = "device_loss_tests.rs"]
mod device_loss_tests;

#[cfg(test)]
mod tests {
    use super::letterbox;

    #[test]
    #[ignore = "requires a working graphics adapter; run explicitly"]
    fn configuration_validation_is_returned_and_error_scopes_remain_balanced() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
        let (device, _queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let invalid = || {
            let _texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("invalid-configuration-test"),
                size: wgpu::Extent3d { width: 0, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
            });
        };
        assert!(super::configuration_result(&device, invalid).is_err());
        // Retain an outer caller-owned scope across successful configuration.
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        assert!(super::configuration_result(&device, || {
            let _buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None, size: 4, usage: wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
            });
        }).is_ok());
        invalid();
        assert!(pollster::block_on(device.pop_error_scope()).is_some());
        assert!(super::configuration_result(&device, || {}).is_ok());
    }

    #[test]
    fn skipped_upload_batches_are_flushed_once_and_normal_submit_consumes_them() {
        let mut pending = super::PendingUploads::default();
        let mut submits = 0;
        // Empty/zero-size frames need no submission. Multiple layer updates or
        // hiding layers must not lose an earlier queued write.
        pending.flush(|| submits += 1);
        assert_eq!(submits, 0);
        for _ in 0..120 {
            pending.mark(); // UI
            pending.mark(); // framebuffer/uniforms, same batch
            pending.flush(|| submits += 1); // timeout: transfers, no draw
            pending.flush(|| panic!("same batch submitted twice"));
        }
        assert_eq!(submits, 120);
        pending.mark(); // last UI update before a zero-size skip
        // An absent-layer update queues no write and leaves this batch intact.
        pending.flush(|| submits += 1);
        assert_eq!(submits, 121);
        pending.mark(); // restored window: regular draw submission
        pending.submitted();
        pending.flush(|| panic!("normal presentation left pending uploads"));
        pending.mark(); // next skipped frame starts a fresh batch
        pending.flush(|| submits += 1);
        assert_eq!(submits, 122);
    }

    #[test]
    fn surface_timeout_skips_without_reconfiguration_then_recovers() {
        let mut recovery = super::SurfaceRecovery::default();
        for _ in 0..120 {
            assert_eq!(recovery.acquire::<()>(|retry| {
                assert!(!retry);
                Err(wgpu::SurfaceError::Timeout)
            }), Ok(None));
        }
        assert_eq!(recovery.acquire(|retry| {
            assert!(!retry);
            Ok(42)
        }), Ok(Some(42)));
    }

    #[test]
    fn lost_surface_reconfigures_once_and_success_resets_failure_budget() {
        let mut recovery = super::SurfaceRecovery::default();
        for _ in 0..2 {
            let mut calls = Vec::new();
            assert_eq!(recovery.acquire::<()>(|retry| {
                calls.push(retry);
                Err(wgpu::SurfaceError::Lost)
            }), Ok(None));
            assert_eq!(calls, [false, true]);
        }
        assert_eq!(recovery.acquire(|retry| {
            if retry { Ok(7) } else { Err(wgpu::SurfaceError::Outdated) }
        }), Ok(Some(7)));
        assert_eq!(recovery.failed_recoveries, 0);
        assert_eq!(recovery.acquire::<()>(|_| Err(wgpu::SurfaceError::Lost)), Ok(None));
    }

    #[test]
    fn persistent_surface_loss_has_bounded_retries() {
        let mut recovery = super::SurfaceRecovery::default();
        let mut calls = 0;
        for attempt in 0..3 {
            let result = recovery.acquire::<()>(|_| {
                calls += 1;
                Err(wgpu::SurfaceError::Outdated)
            });
            assert_eq!(result.is_err(), attempt == 2);
        }
        assert_eq!(calls, 6);
    }

    #[test]
    fn fatal_surface_errors_do_not_reconfigure() {
        for error in [wgpu::SurfaceError::OutOfMemory, wgpu::SurfaceError::Other] {
            let mut recovery = super::SurfaceRecovery::default();
            let mut calls = 0;
            assert!(recovery.acquire::<()>(|retry| {
                calls += 1;
                assert!(!retry);
                Err(error.clone())
            }).is_err());
            assert_eq!(calls, 1);
        }
    }

    #[test]
    fn grid_shader_parses_and_validates() {
        let module = naga::front::wgsl::parse_str(include_str!("grid.wgsl"))
            .unwrap_or_else(|e| panic!("WGSL parse: {e}"));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("WGSL validate: {e:?}"));
    }

    #[test]
    fn uniform_struct_matches_written_buffer() {
        // present() writes a [f32; 20] (80-byte buffer); the WGSL Params
        // struct is 3 vec2f + 13 scalars = 19 floats (padded to 80 bytes),
        // and field ORDER is what the Rust array encodes — this guards the
        // count so a drifted struct fails loudly instead of misreading
        // params. Slot 13 is srgb_surface; the last slot (18) is scale_mode.
        let wgsl = include_str!("grid.wgsl");
        let fields = wgsl
            .split("struct Params {")
            .nth(1)
            .unwrap()
            .split('}')
            .next()
            .unwrap();
        let vec2s = fields.matches("vec2f").count();
        let scalars = fields.matches("f32").count();
        assert_eq!(vec2s, 3, "Params vec2 count drifted");
        assert_eq!(vec2s * 2 + scalars, 19, "Params float count drifted");
        let names: Vec<&str> = fields
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                l.split_once(':')
                    .filter(|(name, _)| !name.contains("//"))
                    .map(|(name, _)| name.trim())
            })
            .collect();
        assert_eq!(names[10], "srgb_surface", "slot 13");
        assert_eq!(names[names.len() - 1], "scale_mode", "slot 18");
    }

    #[test]
    fn letterbox_centers_and_scales() {
        // 240x160 into 720x480: exact 3x, no bars.
        let (s, ox, oy) = letterbox((720, 480), 240, 160);
        assert_eq!((s, ox, oy), (3.0, 0.0, 0.0));
        // Wider window: pillarboxed, centered.
        let (s, ox, oy) = letterbox((1000, 480), 240, 160);
        assert_eq!(s, 3.0);
        assert_eq!(oy, 0.0);
        assert!((ox - 140.0).abs() < 1e-3);
        // Zoomed-out view (372 cols) in the same window letterboxes top/bottom.
        let (s, _, oy) = letterbox((1116, 720), 372, 160);
        assert_eq!(s, 3.0);
        assert!((oy - 120.0).abs() < 1e-3);
    }
}

#[cfg(target_os = "macos")]
mod macos {
    //! Colorspace tagging: a bare metal layer is NOT color-matched by the
    //! compositor — its bytes go to the panel raw, which on a P3 laptop
    //! screen oversaturates everything (the classic "vivid emulator"
    //! artifact). Tagging the layer with the space our LUT encodes for
    //! makes the compositor color-match per-monitor, which is exactly the
    //! per-user-display calibration this feature is about.

    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_core_foundation::CFString;
    use objc2_core_graphics::CGColorSpace;
    use objc2_quartz_core::CAMetalLayer;

    use crate::profile::DisplayTarget;

#[path = "gpu_timing.rs"]
mod gpu_timing;
pub use gpu_timing::{GpuTimingCounts, GpuTimings};

    pub fn ns_view_ptr(window: &impl raw_window_handle::HasWindowHandle) -> *mut std::ffi::c_void {
        match window.window_handle().map(|h| h.as_raw()) {
            Ok(raw_window_handle::RawWindowHandle::AppKit(h)) => h.ns_view.as_ptr(),
            _ => std::ptr::null_mut(),
        }
    }

    pub fn tag_layer_colorspace(ns_view: *mut std::ffi::c_void, target: DisplayTarget) {
        if ns_view.is_null() {
            return;
        }
        unsafe {
            let view = ns_view as *mut AnyObject;
            let layer: Option<Retained<AnyObject>> = msg_send![&*view, layer];
            let Some(layer) = layer else { return };
            let Ok(layer) = layer.downcast::<CAMetalLayer>() else {
                return;
            };
            let name = match target {
                DisplayTarget::Auto | DisplayTarget::Srgb => {
                    CFString::from_static_str("kCGColorSpaceSRGB")
                }
                DisplayTarget::DisplayP3 => CFString::from_static_str("kCGColorSpaceDisplayP3"),
            };
            if let Some(cs) = CGColorSpace::with_name(Some(&name)) {
                layer.setColorspace(Some(&cs));
            }
        }
    }
}
