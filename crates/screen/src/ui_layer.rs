//! Native UI is sampled once in physical window coordinates, after world zoom.
use super::present::letterbox_f;
mod coords;
/// Content token shared by independent UI producers; slots can outlive sources.
pub fn fresh_pixels_revision() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub rect: [f32; 4],
    pub clip: [f32; 4],
}
impl Layout {
    /// A complete modal stays horizontally centered and grows down from its
    /// original top margin, independently of the world's camera and zoom.
    pub fn centered_panel(window:(u32,u32),base_width:f32,size:f32,bounds:[i32;4])->Self {
        let(s,x,y)=letterbox_f(window,base_width,160.);
        let k=s*size.clamp(0.5,2.);
        Self{rect:[(x+base_width*s*0.5-(120.+(bounds[0]+bounds[2])as f32*0.5)*k).round(),
            (y+bounds[1]as f32*(s-k)).round(),480.*k,160.*k],clip:[x,y,base_width*s,160.*s]}
    }
    /// Center the complete host UI canvas inside the game viewport.
    pub fn centered(window:(u32,u32),base_width:f32,size:f32)->Self {
        let(s,x,y)=letterbox_f(window,base_width,160.);
        let k=s*size.clamp(0.5,2.);
        let left=x+base_width*s*0.5-240.*k;
        Self{rect:[left.round(),(y+80.*(s-k)).round(),480.*k,160.*k],clip:[x,y,base_width*s,160.*s]}
    }
    pub fn new(
        window: (u32, u32),
        base_width: f32,
        right: bool,
        size: f32,
        offset: [f32; 2],
    ) -> Self {
        Self::new_with_anchor(window, base_width, right, size, offset, 0.)
    }
    pub fn new_with_anchor(
        window: (u32, u32),
        base_width: f32,
        right: bool,
        size: f32,
        offset: [f32; 2],
        anchor_y: f32,
    ) -> Self {
        let (s, x, y) = letterbox_f(window, base_width, 160.);
        let k = s * size.clamp(0.5, 2.);
        let left = if right {
            x + base_width * s - 240. * k
        } else {
            x
        };
        Self {
            rect: [
                (left - 120. * k + offset[0] * s).round(),
                (y + anchor_y.clamp(0., 160.) * (s - k) + offset[1] * s).round(),
                480. * k,
                160. * k,
            ],
            clip: [x, y, base_width * s, 160. * s],
        }
    }
    /// A compact native panel grows inward from its own content corner.
    pub fn at_corner(
        window: (u32, u32),
        base_width: f32,
        right: bool,
        bottom: bool,
        size: f32,
        length: f32,
        bounds: [i32; 4],
    ) -> Self {
        let (s, x, y) = letterbox_f(window, base_width, 160.);
        let k = s * size.clamp(0.5, 2.);
        // Length changes native backing geometry before upload, never texture scale.
        let _ = length;
        let horizontal = k;
        let left = if right {
            x + base_width * s - bounds[2] as f32 * horizontal
        } else {
            x - bounds[0] as f32 * horizontal
        };
        let top = if bottom {
            y + 160. * s - bounds[3] as f32 * k
        } else {
            y - bounds[1] as f32 * k
        };
        Self {
            rect: [(left - 120. * horizontal).round(), top.round(), 480. * horizontal, 160. * k],
            clip: [x, y, base_width * s, 160. * s],
        }
    }
    pub fn source(&self, x: f32, y: f32) -> Option<usize> {
        let x = coords::ui_axis(x, [self.rect[0], self.rect[2]], [self.clip[0], self.clip[2]], 480)?;
        let y = coords::ui_axis(y, [self.rect[1], self.rect[3]], [self.clip[1], self.clip[3]], 160)?;
        Some(y as usize * 480 + x as usize)
    }
}
#[derive(Clone, Copy)]
pub struct Frame<'a> {
    pub pixels: &'a [[u8; 4]],
    /// A token never reused for different pixels in this layer's source stream.
    /// None preserves unconditional uploads for dynamic/unversioned callers.
    pub pixels_revision: Option<u64>,
    pub raw_world: &'a [u16],
    pub world_dims: (u32, u32),
    pub lut: &'a [[u8; 4]],
    pub layout: Layout,
    pub world_view: [f32; 4],
}
impl Frame<'_> {
    pub fn sample(&self, x: f32, y: f32) -> Option<[u8; 4]> {
        let data = *self.pixels.get(self.layout.source(x, y)?)?;
        if data[2] == 0 && data[3] == 16 {
            return None;
        }
        let c = self.layout.clip;
        let wx = coords::world_axis(x, [c[0], c[2]], [self.world_view[0], self.world_view[2]], self.world_dims.0) as usize;
        let wy = coords::world_axis(y, [c[1], c[3]], [self.world_view[1], self.world_view[3]], self.world_dims.1) as usize;
        let world = *self.raw_world.get(wy * self.world_dims.0 as usize + wx)?;
        let front = u16::from_le_bytes([data[0], data[1]]);
        let mut color = 0;
        for shift in [0, 5, 10] {
            let lane = if shift == 5 { 2 } else { 1 };
            color |= ((((((front >> shift) & 31) * lane * data[2] as u16
                + ((world >> shift) & 31) * lane * data[3] as u16
                + 8)
                >> 4)
                .min(if shift == 5 { 63 } else { 31 }))
                / lane)
                << shift;
        }
        Some(self.lut[color as usize])
    }
}
// Compare only immutable borrows within this update call. This is not an
// address-based content cache: every unique world is uploaded again next call.
fn same_world(a: &Frame<'_>, b: &Frame<'_>) -> bool {
    a.world_dims == b.world_dims && std::ptr::eq(a.raw_world, b.raw_world)
}

pub(super) fn update_layers(
    layers: &mut Vec<Option<Gpu>>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    srgb: bool,
    frames: &[Option<Frame<'_>>],
) -> usize {
    if layers.len() < frames.len() {
        layers.resize_with(frames.len(), || None);
    }
    for layer in layers.iter_mut().flatten() {
        layer.active = false;
    }
    let mut uploaded = 0;
    for (i, frame) in frames.iter().enumerate() {
        let Some(frame) = frame else {
            continue;
        };
        assert_eq!(frame.pixels.len(), 480 * 160);
        assert_eq!(frame.lut.len(), 32768);
        assert!(frame.raw_world.len() >= frame.world_dims.0 as usize * frame.world_dims.1 as usize);
        let (before, tail) = layers.split_at_mut(i);
        let slot = &mut tail[0];
        let owner = frames[..i]
            .iter()
            .position(|other| other.as_ref().is_some_and(|other| same_world(frame, other)));
        let world = if let Some(owner) = owner {
            before[owner].as_ref().unwrap().world.clone()
        } else if let Some(current) = slot.as_ref().filter(|current| {
            current.dimensions() == frame.world_dims
                && !before
                    .iter()
                    .flatten()
                    .any(|other| other.active && other.world == current.world)
        }) {
            current.world.clone()
        } else {
            // A previously shared slot may now have a different source. Split
            // before writing, so it cannot overwrite an earlier layer's world.
            world_texture(device, frame.world_dims)
        };
        if owner.is_none() {
            uploaded += upload_world(queue, &world, frame);
        }
        let layer = slot.get_or_insert_with(|| Gpu::with_world(device, format, world.clone()));
        layer.bind_inputs(device, world, frame);
        uploaded += layer.update_ui(queue, frame, srgb);
    }
    uploaded
}

fn world_texture(device: &wgpu::Device, dims: (u32, u32)) -> wgpu::Texture {
    tex(
        device,
        "ui-world-raw",
        dims.0,
        dims.1,
        wgpu::TextureFormat::R16Uint,
    )
}

fn upload_world(queue: &wgpu::Queue, world: &wgpu::Texture, frame: &Frame<'_>) -> usize {
    let (w, h) = frame.world_dims;
    let pixels = &frame.raw_world[..w as usize * h as usize];
    // Same native u16 upload as before; the source borrow is live for this call.
    let raw = unsafe { std::slice::from_raw_parts(pixels.as_ptr().cast(), pixels.len() * 2) };
    write(queue, world, raw, w, h, 2);
    raw.len()
}

fn tex(
    device: &wgpu::Device,
    label: &str,
    w: u32,
    h: u32,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn write(queue: &wgpu::Queue, tex: &wgpu::Texture, data: &[u8], w: u32, h: u32, bpp: u32) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * bpp),
            rows_per_image: Some(h),
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
}

fn bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    ui: &wgpu::Texture,
    world: &wgpu::Texture,
    lut: &wgpu::Texture,
    params: &wgpu::Buffer,
    coordinates: &wgpu::Texture,
) -> wgpu::BindGroup {
    let views = [
        ui.create_view(&Default::default()),
        world.create_view(&Default::default()),
        lut.create_view(&Default::default()),
        coordinates.create_view(&Default::default()),
    ];
    let mut bindings: Vec<_> = views
        .iter()
        .enumerate()
        .map(|(i, v)| wgpu::BindGroupEntry {
            binding: if i == 3 { 4 } else { i as u32 },
            resource: wgpu::BindingResource::TextureView(v),
        })
        .collect();
    bindings.push(wgpu::BindGroupEntry {
        binding: 3,
        resource: params.as_entire_binding(),
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("native-ui"),
        layout,
        entries: &bindings,
    });
    bind
}

pub(super) struct Gpu {
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    bind_layout: wgpu::BindGroupLayout,
    ui: wgpu::Texture,
    world: wgpu::Texture,
    lut: wgpu::Texture,
    params: wgpu::Buffer,
    coordinates: wgpu::Texture,
    coordinate_map: coords::Map,
    coordinates_dirty: bool,
    visible_bounds: Option<[u32; 4]>,
    scissor: Option<[u32; 4]>,
    dims: (u32, u32),
    pixels_revision: Option<u64>,
    lut_shadow: Vec<[u8; 4]>,
    pub active: bool,
}
impl Gpu {
    #[cfg(test)]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, dims: (u32, u32)) -> Self {
        Self::with_world(device, format, world_texture(device, dims))
    }

    fn with_world(device: &wgpu::Device, format: wgpu::TextureFormat, world: wgpu::Texture) -> Self {
        let dims = (world.width(), world.height());
        let ui = tex(
            device,
            "native-ui",
            480,
            160,
            wgpu::TextureFormat::Rgba8Uint,
        );
        let lut = tex(device, "ui-lut", 256, 128, wgpu::TextureFormat::Rgba8Unorm);
        let coordinates = tex(device, "ui-coordinates", 1, 2, wgpu::TextureFormat::Rg32Uint);
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui-layout"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut entries: Vec<_> = (0..3)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: if binding == 2 {
                        wgpu::TextureSampleType::Float { filterable: false }
                    } else {
                        wgpu::TextureSampleType::Uint
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            })
            .collect();
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Uint,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            }, count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui-layout"),
            entries: &entries,
        });
        let bind = bind_group(device, &layout, &ui, &world, &lut, &params, &coordinates);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("native-ui"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui_layer.wgsl").into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("native-ui"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("native-ui"),
            layout: Some(&pl),
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
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Self {
            pipeline,
            bind,
            bind_layout: layout,
            ui,
            world,
            lut,
            params,
            coordinates,
            coordinate_map: Default::default(),
            coordinates_dirty: true,
            visible_bounds: None,
            scissor: None,
            dims,
            pixels_revision: None,
            lut_shadow: Vec::with_capacity(32768),
            active: false,
        }
    }
    pub fn dimensions(&self) -> (u32, u32) {
        self.dims
    }
    #[cfg(test)]
    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, frame: &Frame<'_>, srgb: bool) -> usize {
        self.bind_inputs(device, self.world.clone(), frame);
        upload_world(queue, &self.world, frame) + self.update_ui(queue, frame, srgb)
    }

    fn bind_inputs(&mut self, device: &wgpu::Device, world: wgpu::Texture, frame: &Frame<'_>) {
        self.coordinates_dirty |= self.coordinate_map.update(frame, device.limits().max_texture_dimension_2d);
        let resized = self.coordinates.width() != self.coordinate_map.width;
        if resized {
            self.coordinates = tex(device, "ui-coordinates", self.coordinate_map.width, 2, wgpu::TextureFormat::Rg32Uint);
        }
        if self.world != world || resized {
            self.bind = bind_group(device, &self.bind_layout, &self.ui, &world, &self.lut, &self.params, &self.coordinates);
            self.dims = (world.width(), world.height());
            self.world = world;
        }
    }

    /// World content is uploaded once per distinct source by update_layers.
    fn update_ui(&mut self, queue: &wgpu::Queue, frame: &Frame<'_>, srgb: bool) -> usize {
        // POD slices: packed UI bytes and native u16 world colors.
        let ui = unsafe {
            std::slice::from_raw_parts(frame.pixels.as_ptr().cast(), frame.pixels.len() * 4)
        };
        let lut =
            unsafe { std::slice::from_raw_parts(frame.lut.as_ptr().cast(), frame.lut.len() * 4) };
        let mut uploaded = 0;
        let mut scissor_dirty = self.coordinates_dirty;
        if self.coordinates_dirty {
            write(queue, &self.coordinates, &self.coordinate_map.bytes, self.coordinate_map.width, 2, 8);
            uploaded += self.coordinate_map.bytes.len();
            self.coordinates_dirty = false;
        }
        if frame.pixels_revision.is_none() || frame.pixels_revision != self.pixels_revision {
            let bounds = coords::visible_bounds(frame.pixels);
            scissor_dirty |= self.visible_bounds != bounds;
            self.visible_bounds = bounds;
            write(queue, &self.ui, ui, 480, 160, 4);
            uploaded += ui.len();
        }
        self.pixels_revision = frame.pixels_revision;
        if scissor_dirty {
            self.scissor = self.visible_bounds.and_then(|bounds| self.coordinate_map.scissor(bounds));
        }
        // Compare actual colors, never addresses: a settings change may mutate
        // the same allocation, and distinct layers may use distinct LUTs.
        if self.lut_shadow.as_slice() != frame.lut {
            write(queue, &self.lut, lut, 256, 128, 4);
            self.lut_shadow.clear();
            self.lut_shadow.extend_from_slice(frame.lut);
            uploaded += lut.len();
        }
        let mut values = [0f32; 16];
        values[..4].copy_from_slice(&frame.layout.rect);
        values[4..8].copy_from_slice(&frame.layout.clip);
        values[8..12].copy_from_slice(&frame.world_view);
        values[12] = if srgb { 1. } else { 0. };
        values[13] = self.coordinate_map.origin[0] as f32;
        values[14] = self.coordinate_map.origin[1] as f32;
        let bytes = unsafe { std::slice::from_raw_parts(values.as_ptr().cast(), 64) };
        queue.write_buffer(&self.params, 0, bytes);
        // Active means this call claimed the world source, even if no pixel
        // draws. The sharing planner must not reuse that destination for a
        // different source; scissor visibility is handled separately in draw.
        self.active = true;
        uploaded + bytes.len()
    }
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, viewport: (u32, u32)) {
        if self.active {
            let Some(rect) = self.scissor else { return; };
            let (x, y) = (rect[0].min(viewport.0), rect[1].min(viewport.1));
            let right = rect[0].saturating_add(rect[2]).min(viewport.0);
            let bottom = rect[1].saturating_add(rect[3]).min(viewport.1);
            if right <= x || bottom <= y { return; }
            pass.set_scissor_rect(x, y, right - x, bottom - y);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modal_panel_is_centered_and_keeps_its_top_margin_when_size_changes() {
        for width in [284.,568.] {for size in [0.5,1.,1.5,2.] {
            let l=Layout::centered_panel((1920,1080),width,size,[16,16,224,64]);
            let k=l.rect[2]/480.;let s=l.clip[3]/160.;
            let center=l.rect[0]+240.*k;
            assert!((center-l.clip[0]-l.clip[2]*0.5).abs()<=0.5);
            assert!((l.rect[1]+16.*k-l.clip[1]-16.*s).abs()<=0.5);
        }}
    }
    #[test]
    fn centered_canvas_keeps_native_mouse_coordinates_in_the_middle() {
        for window in [(1280,720),(2560,720),(1920,1080)] {
            for size in [0.5,1.,1.5,2.] {
                let l=Layout::centered(window,568.,size);
                assert!((l.rect[0]+l.rect[2]*0.5-l.clip[0]-l.clip[2]*0.5).abs()<=0.5);
                assert!((l.rect[1]+l.rect[3]*0.5-l.clip[1]-l.clip[3]*0.5).abs()<=0.5);
            }
            let layout=Layout::centered(window,568.,1.);
            for (x,y) in [(0usize,0usize),(90,24),(239,159)] {
                let px=layout.rect[0]+(120.+x as f32+0.5)*layout.rect[2]/480.;
                let py=layout.rect[1]+(y as f32+0.5)*layout.rect[3]/160.;
                let index=layout.source(px,py).unwrap();
                assert_eq!(((index%480)as i32-120,(index/480)as i32),(x as i32,y as i32));
            }
        }
    }
    #[test]
    fn shader_validates() {
        let module = naga::front::wgsl::parse_str(include_str!("ui_layer.wgsl")).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap();
    }
    #[test]
    fn shared_world_key_requires_the_same_borrow_extent_and_dimensions() {
        let world = vec![7u16; 2];
        let other = vec![7u16; 2];
        let frame = Frame {
            pixels: &[], pixels_revision: None, raw_world: &world,
            world_dims: (1, 1), lut: &[],
            layout: Layout::new((320, 160), 320., true, 1., [0., 0.]),
            world_view: [0., 0., 1., 1.],
        };
        assert!(same_world(&frame, &Frame { world_view: [4., 2., 1., 1.], ..frame }));
        assert!(!same_world(&frame, &Frame { raw_world: &other, ..frame }));
        assert!(!same_world(&frame, &Frame { raw_world: &world[..1], ..frame }));
        assert!(!same_world(&frame, &Frame { world_dims: (2, 1), ..frame }));
    }
    #[test]
    #[ignore = "requires a working graphics adapter; run explicitly"]
    fn shared_world_uploads_preserve_pixels_when_sources_split_merge_and_resize() {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("SHARED_WORLD_GPU adapter={}", adapter.get_info().name);
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let lut: Vec<_> = (0..32768).map(|c| std::array::from_fn(|i| {
            if i == 3 { 255 } else { let v = (c >> (5 * i)) & 31; ((v << 3) | (v >> 2)) as u8 }
        })).collect();
        let pixels: Vec<_> = (0..3).map(|i| {
            let color = (31u16 << (i * 5)).to_le_bytes();
            vec![[color[0], color[1], 8, 8]; 480 * 160]
        }).collect();
        let luts: Vec<_> = (0..3).map(|i| lut.iter().map(|color| {
            let mut color = *color;
            color[i] = 255 - color[i];
            color
        }).collect::<Vec<_>>()).collect();
        let mut layers = Vec::new();
        for dims in [(320, 160), (640, 320), (1, 1)] {
            let mut a: Vec<_> = (0..dims.0 * dims.1).map(|i| ((i * 13) & 32767) as u16).collect();
            let mut b: Vec<_> = (0..dims.0 * dims.1).map(|i| ((i * 7 + 123) & 32767) as u16).collect();
            for (step, sources) in [
                [Some(0), Some(0), Some(1)],
                [Some(0), Some(0), Some(1)],
                [Some(0), Some(1), Some(1)],
                [Some(1), Some(0), Some(0)],
                [None, Some(0), Some(1)],
                [Some(0), Some(0), Some(0)],
                [Some(0), None, Some(1)],
                [Some(0), Some(1), Some(0)],
                [None, None, None],
                [Some(1), Some(0), Some(1)],
                [Some(1), Some(0), Some(1)],
            ].into_iter().enumerate() {
                if step == 1 {
                    for c in a.iter_mut().chain(&mut b) { *c ^= 0x7fff; }
                }
                if step == 10 { layers[0] = None; }
                let frames = std::array::from_fn::<_, 3, _>(|i| sources[i].map(|source| Frame {
                    pixels: &pixels[i], pixels_revision: Some(7),
                    raw_world: if source == 0 { &a } else { &b }, world_dims: dims, lut: &luts[i],
                    layout: Layout::new((320, 160), 320., i != 1, 1., [i as f32 * 4., 0.]),
                    world_view: [0., 0., dims.0 as f32 / (i + 1) as f32, dims.1 as f32],
                }));
                let fresh = frames.iter().enumerate().filter(|(i, frame)| frame.is_some() && layers.get(*i).is_none_or(Option::is_none)).count();
                let worlds = usize::from(sources.contains(&Some(0))) + usize::from(sources.contains(&Some(1)));
                let active = sources.iter().flatten().count();
                let expected = worlds * a.len() * 2 + active * 64 + fresh * (pixels[0].len() * 4 + lut.len() * 4) + if step == 0 { active * 5120 } else { fresh * 5120 };
                let uploaded = update_layers(&mut layers, &device, &queue, wgpu::TextureFormat::Rgba8Unorm, false, &frames);
                assert_eq!(uploaded, expected, "dims={dims:?} step={step}");
                for (i, frame) in frames.iter().enumerate() {
                    if let Some(frame) = frame {
                        let layer = layers[i].as_ref().unwrap();
                        assert!(layer.active);
                        let shared = gpu_layer_pixels(&device, &queue, layer);
                        let mut independent = Gpu::new(&device, wgpu::TextureFormat::Rgba8Unorm, dims);
                        independent.update(&device, &queue, frame, false);
                        let reference = gpu_layer_pixels(&device, &queue, &independent);
                        assert_eq!(shared, reference, "shared versus independent dims={dims:?} step={step} layer={i}");
                        assert_cpu_pixels(&shared, frame);
                    } else if let Some(layer) = &layers[i] { assert!(!layer.active); }
                }
            }
        }
        eprintln!("SHARED_WORLD_CPU_PARITY all_layers=true independent_gpu_matches_shared=true");
    }

    #[test]
    #[ignore = "requires a working graphics adapter; run explicitly"]
    fn runtime_zoom_coordinates_match_cpu() {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("UI_COORDINATES_GPU adapter={}", adapter.get_info().name);
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let lut: Vec<_> = (0..32768).map(|c| std::array::from_fn(|i| {
            if i == 3 { 255 } else { let v = (c >> (5 * i)) & 31; ((v << 3) | (v >> 2)) as u8 }
        })).collect();
        let pixels: Vec<_> = (0..480 * 160).map(|i| {
            let c = ((i * 17) & 32767) as u16;
            [c as u8, (c >> 8) as u8, 8, 8]
        }).collect();
        let mut layers = Vec::new();
        for window in [(852,480), (853,481), (320,160), (1704,480)] {
        for (level, clamp_zoom) in [(0,false),(1,false),(2,false),(3,false),(4,false),(5,false),(6,false),(6,true)] {
            // Same 16:9 starting width and zoom/ceil/centering as sync_view.
            let z = if level == 0 { 1. } else { 1.122_462f32.powi(level) };
            let z = if clamp_zoom { z.min(2.) } else { z };
            let (vw, vh) = (284. * z, 160. * z);
            let dims = (240 + 2 * ((vw - 240.) / 2.).ceil() as u32,
                        160 + 2 * ((vh - 160.) / 2.).ceil() as u32);
            let world: Vec<_> = (0..dims.0 * dims.1).map(|i| ((i * 13 + 123) & 32767) as u16).collect();
            let frame = Frame {
                pixels: &pixels, pixels_revision: None, raw_world: &world,
                world_dims: dims, lut: &lut,
                layout: Layout::new(window, 284., true, if window.0 == 853 { 1.25 } else { 1. }, [0., 0.]),
                world_view: [(dims.0 as f32 - vw) / 2., (dims.1 as f32 - vh) / 2., vw, vh],
            };
            update_layers(&mut layers, &device, &queue, wgpu::TextureFormat::Rgba8Unorm, false, &[Some(frame)]);
            let actual = gpu_layer_pixels_at(&device, &queue, layers[0].as_ref().unwrap(), window);
            let mut differences = 0;
            for y in 0..window.1 as usize { for x in 0..window.0 as usize {
                let expected = frame.sample(x as f32 + 0.5, y as f32 + 0.5).unwrap_or([0, 0, 0, 255]);
                differences += usize::from(actual[(y * window.0 as usize + x) * 4..(y * window.0 as usize + x + 1) * 4] != expected);
            }}
            eprintln!("UI_RUNTIME_COORDINATES window={window:?} zoom={level} clamp={clamp_zoom} compared={} differences={differences}", window.0 * window.1);
            assert_eq!(differences, 0, "runtime zoom={level}");
        }
        }
    }

    fn gpu_layer_pixels(device: &wgpu::Device, queue: &wgpu::Queue, gpu: &Gpu) -> Vec<u8> {
        gpu_layer_pixels_at(device, queue, gpu, (320, 160))
    }

    fn gpu_layer_pixels_at(device: &wgpu::Device, queue: &wgpu::Queue, gpu: &Gpu, size: (u32, u32)) -> Vec<u8> {
        let stride = (size.0 * 4).div_ceil(256) * 256;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ui-test-target"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui-test-readback"),
            size: (stride * size.1) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let view = target.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui-test"),
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
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            gpu.draw(&mut pass, size);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(size.1),
                },
            },
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let slice = read.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::PollType::Wait).unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = slice.get_mapped_range();
        let pixels = mapped.chunks_exact(stride as usize).flat_map(|row| row[..size.0 as usize * 4].iter().copied()).collect();
        drop(mapped);
        read.unmap();
        pixels
    }

    fn assert_cpu_pixels(mapped: &[u8], frame: &Frame<'_>) {
        let dims = frame.world_dims;
        for y in 0..160 {
            for x in 0..320 {
                let expected = frame
                    .sample(x as f32 + 0.5, y as f32 + 0.5)
                    .unwrap_or([0, 0, 0, 255]);
                assert_eq!(
                    &mapped[(y * 320 + x) * 4..(y * 320 + x + 1) * 4],
                    &expected,
                    "dims={dims:?} x={x} y={y}"
                );
            }
        }
    }

    fn assert_gpu_layer(device: &wgpu::Device, queue: &wgpu::Queue, gpu: &Gpu, frame: &Frame<'_>) {
        assert_cpu_pixels(&gpu_layer_pixels(device, queue, gpu), frame);
    }

    #[test]
    #[ignore = "requires a working graphics adapter; run explicitly"]
    fn centered_laws_modal_matches_cpu_across_sizes() {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("LAWS_UI_GPU adapter={}", adapter.get_info().name);
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let lut: Vec<_> = (0..32768).map(|c| std::array::from_fn(|i| {
            if i == 3 { 255 } else { let v = (c >> (5 * i)) & 31; ((v << 3) | (v >> 2)) as u8 }
        })).collect();
        let world: Vec<_> = (0..284 * 160).map(|i| ((i * 13) & 32767) as u16).collect();
        let mut layers = Vec::new();
        for bounds in [[0,72,216,128], [0,16,224,128]] {
            let pixels: Vec<_> = (0..480 * 160).map(|i| {
                let x = (i % 480) as i32 - 120;
                let y = (i / 480) as i32;
                if x < bounds[0] || x >= bounds[2] || y < bounds[1] || y >= bounds[3] || (x + y) % 7 == 0 {
                    [0,0,0,16]
                } else {
                    let c = ((i * 17) & 32767) as u16;
                    [c as u8,(c >> 8) as u8,8,8]
                }
            }).collect();
            for (window, base_width) in [((720,480),240.), ((852,480),284.), ((1704,480),568.), ((853,481),284.)] {
                for size in [0.5,1.,2.] {
                    let frame = Frame {
                        pixels: &pixels, pixels_revision: None, raw_world: &world,
                        world_dims: (284,160), lut: &lut,
                        layout: Layout::centered_panel(window,base_width,size,bounds),
                        world_view: [0.,0.,284.,160.],
                    };
                    update_layers(&mut layers,&device,&queue,wgpu::TextureFormat::Rgba8Unorm,false,&[Some(frame)]);
                    let actual = gpu_layer_pixels_at(&device,&queue,layers[0].as_ref().unwrap(),window);
                    for y in 0..window.1 as usize { for x in 0..window.0 as usize {
                        let expected = frame.sample(x as f32+0.5,y as f32+0.5).unwrap_or([0,0,0,255]);
                        assert_eq!(actual[(y*window.0 as usize+x)*4..(y*window.0 as usize+x+1)*4],expected,
                            "bounds={bounds:?} window={window:?} size={size} x={x} y={y}");
                    }}
                }
            }
        }
        eprintln!("LAWS_UI_CPU_GPU_PARITY cases=24 differences=0");
    }
    #[test]
    #[ignore = "requires a working graphics adapter; run explicitly"]
    fn gpu_matches_cpu_native_layer_without_window() {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("GPU adapter");
        eprintln!("UI_LAYER_GPU adapter={}", adapter.get_info().name);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let lut: Vec<_> = (0..32768)
            .map(|c| {
                std::array::from_fn(|i| {
                    if i == 3 {
                        255
                    } else {
                        let v = (c >> (5 * i)) & 31;
                        ((v << 3) | (v >> 2)) as u8
                    }
                })
            })
            .collect();
        let mut pixels = vec![[0, 0, 0, 16]; 480 * 160];
        for y in 8..64 {
            for x in 120 + 96..120 + 240 {
                let color = if y < 32 { 0x7c1fu16 } else { 0x03e0 };
                pixels[y * 480 + x] = [
                    color as u8,
                    (color >> 8) as u8,
                    if x % 2 == 0 { 8 } else { 16 },
                    if x % 2 == 0 { 8 } else { 0 },
                ];
            }
        }
        // The FPS HUD uses a one-pixel backdrop independent of the game view.
        for dims in [(320, 160), (640, 320), (1, 1)] {
            let mut world: Vec<u16> = (0..dims.0 * dims.1).map(|i| (i % 32768) as u16).collect();
            let mut local_pixels = pixels.clone();
            let mut local_lut = lut.clone();
            let mut gpu = Gpu::new(&device, wgpu::TextureFormat::Rgba8Unorm, dims);
            for step in 0..10 {
                // Mutation keeps the same allocation: pointer-based LUT caching
                // would miss it. Recreated resources must also upload everything.
                if step == 2 {
                    for color in &mut local_lut {
                        for c in &mut color[..3] {
                            *c = 255 - *c;
                        }
                    }
                }
                if step == 3 {
                    for pixel in &mut local_pixels {
                        pixel[0] ^= 31;
                    }
                }
                if step == 3 { local_pixels[120] = [1, 0, 8, 8]; }
                if step == 9 { local_pixels.fill([0, 0, 0, 16]); }
                if step == 7 {
                    for color in &mut world {
                        *color ^= 0x7fff;
                    }
                }
                if step == 8 {
                    gpu = Gpu::new(&device, wgpu::TextureFormat::Rgba8Unorm, dims);
                }
                let frame = Frame {
                    pixels: &local_pixels,
                    pixels_revision: match step {
                        0..=2 => Some(7),
                        4..=5 => None,
                        9 => Some(9),
                        _ => Some(8),
                    },
                    raw_world: &world,
                    world_dims: dims,
                    lut: &local_lut,
                    layout: Layout::new(
                        (320, 160),
                        320.,
                        true,
                        1.,
                        if step == 1 { [8., 4.] } else { [0., 0.] },
                    ),
                    world_view: [0., 0., dims.0 as f32, dims.1 as f32],
                };
                let uploaded = gpu.update(&device, &queue, &frame, false);
                let all = step == 0 || step == 8;
                let expected = world.len() * 2
                    + 64
                    + if matches!(step, 0..=2 | 8) { 5120 } else { 0 }
                    + if all || matches!(step, 3..=6 | 9) {
                        local_pixels.len() * 4
                    } else {
                        0
                    }
                    + if all || step == 2 {
                        local_lut.len() * 4
                    } else {
                        0
                    };
                assert_eq!(uploaded, expected, "upload bytes dims={dims:?} step={step}");
                assert_gpu_layer(&device, &queue, &gpu, &frame);
            }
        }
    }
    #[test]
    fn increasing_size_grows_toward_center_and_keeps_vertical_edge_fixed() {
        for right in [false, true] {
            for anchor in [0., 160.] {
                for size in [0.5, 1., 1.1, 1.5, 2.] {
                    let layout =
                        Layout::new_with_anchor((1704, 480), 568., right, size, [0., 0.], anchor);
                    let k = layout.rect[3] / 160.;
                    let native = layout.clip[3] / 160.;
                    assert!(
                        (layout.rect[1] + anchor * k - (layout.clip[1] + anchor * native)).abs()
                            <= 0.51
                    );
                    let (top, bottom) = if anchor == 0. { (0., 64.) } else { (96., 160.) };
                    assert!(layout.rect[1] + top * k >= layout.clip[1] - 0.51);
                    assert!(layout.rect[1] + bottom * k <= layout.clip[1] + layout.clip[3] + 0.51);
                    assert!((layout.rect[2] / 480. - k).abs() < 0.00001);
                }
            }
        }
    }
    #[test]
    fn independent_panels_remain_on_their_corners_at_every_size() {
        for (right, b) in [
            (false, [0, 88, 136, 157]),
            (true, [175, 70, 240, 149]),
            (true, [152, 104, 240, 152]),
            (false, [0, 104, 88, 152]),
        ] {
            for size in [0.5, 1., 1.1, 1.5, 2.] {
                let l = Layout::at_corner((1704, 480), 568., right, true, size, 1., b);
                let k = l.rect[3] / 160.;
                let edge = l.rect[0] + (120 + b[if right { 2 } else { 0 }]) as f32 * k;
                assert!((edge - if right { 1704. } else { 0. }).abs() <= 0.51);
                assert!((l.rect[1] + b[3] as f32 * k - 480.).abs() <= 0.51);
                assert!(l.rect[1] + b[1] as f32 * k >= -0.51);
                assert!((l.rect[2] / 480. - k).abs() < 0.00001);
            }
        }
    }
    #[test]
    fn panel_length_never_stretches_texture_or_moves_corner() {
        for right in [false,true] {for bottom in [false,true] {
            let b=[152,104,240,152];
            for size in [0.5,1.,2.] {
                let base=Layout::at_corner((1704,480),568.,right,bottom,size,1.,b);
                for length in [1.,1.25,1.5,2.] {
                    let l=Layout::at_corner((1704,480),568.,right,bottom,size,length,b);
                    assert_eq!(l.rect[1],base.rect[1]);assert_eq!(l.rect[3],base.rect[3]);
                    assert_eq!(l.rect,base.rect);
                    let edge=l.rect[0]+(120+b[if right {2}else{0}])as f32*l.rect[2]/480.;
                    assert!((edge-if right {1704.}else{0.}).abs()<=0.51);
                }
            }
        }}
    }
    #[test]
    fn layout_is_independent_of_world_zoom() {
        for right in [false, true] {
            for size in [0.5, 1., 1.5] {
                let layout = Layout::new((1920, 1080), 568., right, size, [0., 0.]);
                for zoom in 0..13 {
                    // Zoom is intentionally absent from the UI layout inputs.
                    let _world_zoom = 1.122462f32.powi(zoom);
                    assert_eq!(
                        layout.rect,
                        Layout::new((1920, 1080), 568., right, size, [0., 0.]).rect
                    );
                }
            }
        }
    }
}
