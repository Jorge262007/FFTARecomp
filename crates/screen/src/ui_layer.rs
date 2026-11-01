//! Native UI is sampled once in physical window coordinates, after world zoom.
use super::present::letterbox_f;
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
        if x < self.clip[0]
            || y < self.clip[1]
            || x >= self.clip[0] + self.clip[2]
            || y >= self.clip[1] + self.clip[3]
        {
            return None;
        }
        let u = (x - self.rect[0]) / self.rect[2];
        let v = (y - self.rect[1]) / self.rect[3];
        ((0. ..1.).contains(&u) && (0. ..1.).contains(&v))
            .then(|| (v * 160.).floor() as usize * 480 + (u * 480.).floor() as usize)
    }
}
#[derive(Clone, Copy)]
pub struct Frame<'a> {
    pub pixels: &'a [[u8; 4]],
    pub raw_world: &'a [u16],
    pub world_dims: (u32, u32),
    pub lut: &'a [[u8; 4]],
    pub layout: Layout,
    pub world_view: [f32; 4],
    /// Physical viewport of the world, independent of widget clipping.
    pub world_rect: [f32; 4],
}
impl Frame<'_> {
    pub fn sample(&self, x: f32, y: f32) -> Option<[u8; 4]> {
        let data = *self.pixels.get(self.layout.source(x, y)?)?;
        if data[2] == 0 && data[3] == 16 {
            return None;
        }
        let c = self.world_rect;
        let wx = (self.world_view[0] + (x - c[0]) / c[2] * self.world_view[2])
            .floor()
            .clamp(0., self.world_dims.0 as f32 - 1.) as usize;
        let wy = (self.world_view[1] + (y - c[1]) / c[3] * self.world_view[3])
            .floor()
            .clamp(0., self.world_dims.1 as f32 - 1.) as usize;
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
pub(super) struct Gpu {
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    ui: wgpu::Texture,
    world: wgpu::Texture,
    lut: wgpu::Texture,
    params: wgpu::Buffer,
    dims: (u32, u32),
    last_ui: Vec<[u8;4]>,
    last_world: Vec<u16>,
    last_lut: Vec<[u8;4]>,
    occupied: [u32;4],
    translucent: bool,
    layout: Layout,
    pub active: bool,
}
impl Gpu {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, dims: (u32, u32)) -> Self {
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
        let ui = tex(
            device,
            "native-ui",
            480,
            160,
            wgpu::TextureFormat::Rgba8Uint,
        );
        let world = tex(
            device,
            "ui-world-raw",
            dims.0,
            dims.1,
            wgpu::TextureFormat::R16Uint,
        );
        let lut = tex(device, "ui-lut", 256, 128, wgpu::TextureFormat::Rgba8Unorm);
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui-layout"),
            size: 80,
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
        let views = [
            ui.create_view(&Default::default()),
            world.create_view(&Default::default()),
            lut.create_view(&Default::default()),
        ];
        let mut bindings: Vec<_> = views
            .iter()
            .enumerate()
            .map(|(i, v)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(v),
            })
            .collect();
        bindings.push(wgpu::BindGroupEntry {
            binding: 3,
            resource: params.as_entire_binding(),
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("native-ui"),
            layout: &layout,
            entries: &bindings,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("native-ui"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui_layer.wgsl").into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("native-ui"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let make_pipeline = |blend| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let pipeline=make_pipeline(None);
        Self {
            pipeline,
            bind,
            ui,
            world,
            lut,
            params,
            dims,
            last_ui:Vec::new(),last_world:Vec::new(),last_lut:Vec::new(),occupied:[0;4],translucent:false,
            layout:Layout {rect:[0.;4],clip:[0.;4]},
            active: false,
        }
    }
    pub fn dimensions(&self) -> (u32, u32) {
        self.dims
    }
    pub fn resize_world(&mut self,device:&wgpu::Device,dims:(u32,u32)) {
        if self.dims==dims {return;}
        self.world=device.create_texture(&wgpu::TextureDescriptor {
            label:Some("ui-world-raw"),size:wgpu::Extent3d {width:dims.0,height:dims.1,depth_or_array_layers:1},
            mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,
            format:wgpu::TextureFormat::R16Uint,
            usage:wgpu::TextureUsages::TEXTURE_BINDING|wgpu::TextureUsages::COPY_DST,view_formats:&[],
        });
        let views=[self.ui.create_view(&Default::default()),self.world.create_view(&Default::default()),self.lut.create_view(&Default::default())];
        let mut entries:Vec<_>=views.iter().enumerate().map(|(i,v)|wgpu::BindGroupEntry {
            binding:i as u32,resource:wgpu::BindingResource::TextureView(v)
        }).collect();
        entries.push(wgpu::BindGroupEntry {binding:3,resource:self.params.as_entire_binding()});
        self.bind=device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:Some("ui-bind"),layout:&self.pipeline.get_bind_group_layout(0),entries:&entries,
        });
        self.dims=dims;self.last_world.clear();
    }
    pub fn update(&mut self, queue: &wgpu::Queue, frame: &Frame<'_>, srgb: bool) {
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
        // POD slices: packed UI bytes and native u16 world colors.
        let ui = unsafe {
            std::slice::from_raw_parts(frame.pixels.as_ptr().cast(), frame.pixels.len() * 4)
        };
        let raw = unsafe {
            std::slice::from_raw_parts(
                frame.raw_world.as_ptr().cast(),
                self.dims.0 as usize * self.dims.1 as usize * 2,
            )
        };
        let lut =
            unsafe { std::slice::from_raw_parts(frame.lut.as_ptr().cast(), frame.lut.len() * 4) };
        if self.last_ui!=frame.pixels {
            write(queue, &self.ui, ui, 480, 160, 4);
            self.last_ui.clear();self.last_ui.extend_from_slice(frame.pixels);
            let mut bounds=[480,160,0,0];
            for (i,p) in frame.pixels.iter().enumerate() {
                if p[2]==0 && p[3]==16 {continue;}
                let (x,y)=((i%480) as u32,(i/480) as u32);
                bounds=[bounds[0].min(x),bounds[1].min(y),bounds[2].max(x+1),bounds[3].max(y+1)];
            }
            self.occupied=bounds;
            self.translucent=frame.pixels.iter().any(|p|p[3]>0 && !(p[2]==0 && p[3]==16));
        }
        if self.translucent && self.last_world!=frame.raw_world {
            write(queue, &self.world, raw, self.dims.0, self.dims.1, 2);
            self.last_world.clear();self.last_world.extend_from_slice(frame.raw_world);
        }
        if self.last_lut!=frame.lut {
            write(queue, &self.lut, lut, 256, 128, 4);
            self.last_lut.clear();self.last_lut.extend_from_slice(frame.lut);
        }
        self.layout=frame.layout;
        let mut values = [0f32; 20];
        values[..4].copy_from_slice(&frame.layout.rect);
        values[4..8].copy_from_slice(&frame.layout.clip);
        values[8..12].copy_from_slice(&frame.world_view);
        values[12] = if srgb { 1. } else { 0. };
        values[13] = 0.;
        values[16..20].copy_from_slice(&frame.world_rect);
        let bytes = unsafe { std::slice::from_raw_parts(values.as_ptr().cast(), 80) };
        queue.write_buffer(&self.params, 0, bytes);
        self.active = true;
    }
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>,window:(u32,u32)) {
        if self.active {
            let [l,t,r,b]=self.occupied;
            if r<=l || b<=t {return;}
            let Layout{rect,clip}=self.layout;
            let left=(rect[0]+l as f32/480.*rect[2]).max(clip[0]).floor().max(0.) as u32;
            let top=(rect[1]+t as f32/160.*rect[3]).max(clip[1]).floor().max(0.) as u32;
            let right=((rect[0]+r as f32/480.*rect[2]).min(clip[0]+clip[2]).ceil().max(0.) as u32).min(window.0);
            let bottom=((rect[1]+b as f32/160.*rect[3]).min(clip[1]+clip[3]).ceil().max(0.) as u32).min(window.1);
            if right<=left || bottom<=top {return;}
            pass.set_scissor_rect(left,top,right-left,bottom-top);
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
    fn widget_clip_never_rescales_the_transparent_world(){
        let pixels=vec![[0,0,0,16];480*160];
        let world:Vec<u16>=(0..320*160).map(|i|(i%32768)as u16).collect();
        let lut:Vec<[u8;4]>=(0..32768u16).map(|c|[c as u8,(c>>8)as u8,0,255]).collect();
        let mut pixels=pixels;
        for y in 0..160 {for x in 0..480 {pixels[y*480+x]=[0,0,0,8];}}
        let frame=Frame{pixels:&pixels,raw_world:&world,world_dims:(320,160),lut:&lut,
            layout:Layout{rect:[-80.,0.,480.,160.],clip:[0.,0.,320.,160.]},
            world_view:[0.,0.,320.,160.],world_rect:[0.,0.,320.,160.]};
        let mut clipped=frame;clipped.layout.clip=[240.,48.,80.,96.];
        for y in 48..144 {for x in 240..320 {
            assert_eq!(clipped.sample(x as f32+0.5,y as f32+0.5),frame.sample(x as f32+0.5,y as f32+0.5));
        }}
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
        let mut gpu = Gpu::new(&device, wgpu::TextureFormat::Rgba8Unorm,(320,160));
        for zoom in [1, 2] {
            let dims = (320 * zoom, 160 * zoom);
            let world: Vec<u16> = (0..dims.0 * dims.1).map(|i| (i % 32768) as u16).collect();
            let frame = Frame {
                pixels: &pixels,
                raw_world: &world,
                world_dims: dims,
                lut: &lut,
                layout: Layout{clip:[176.,8.,144.,56.],..Layout::new((320, 160), 320., true, 1., [0., 0.])},
                world_view: [0., 0., dims.0 as f32, dims.1 as f32],
                world_rect: [0.,0.,320.,160.],
            };
            gpu.resize_world(&device,dims);
            gpu.update(&queue, &frame, false);
            gpu.update(&queue, &frame, false);

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("ui-test-target"),
                size: wgpu::Extent3d {
                    width: 320,
                    height: 160,
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
                size: 320 * 160 * 4,
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
                gpu.draw(&mut pass,(320,160));
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
                        bytes_per_row: Some(1280),
                        rows_per_image: Some(160),
                    },
                },
                wgpu::Extent3d {
                    width: 320,
                    height: 160,
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
            for y in 0..160 {
                for x in 0..320 {
                    let expected = frame
                        .sample(x as f32 + 0.5, y as f32 + 0.5)
                        .unwrap_or([0, 0, 0, 255]);
                    let actual=&mapped[(y*320+x)*4..(y*320+x+1)*4];
                    for channel in 0..4 {assert!(actual[channel].abs_diff(expected[channel])==0,
                        "zoom={zoom} x={x} y={y} actual={actual:?} expected={expected:?}");}
                }
            }
            drop(mapped);
            read.unmap();
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
