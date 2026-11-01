//! Host-only dialogue placement. This is the single adjustment point for
//! future Settings controls; it never changes OAM, VRAM or game coordinates.

#[derive(Clone, Copy, Debug)]
pub struct DialogueStyle {
    /// Relative to the normal screen-space size, independently of world zoom.
    pub scale: f32,
    pub length: f32,
    /// Offsets in native UI pixels; these also compensate world zoom.
    pub offset_x: f32,
    pub offset_y: f32,
}
impl Default for DialogueStyle {
    fn default() -> Self {
        Self {
            scale: 1.,
            length: 1.,
            offset_x: 0.,
            offset_y: 0.,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub scale: f32,
    pub left: f32,
    pub top: f32,
    pub extra_y: i32,
    pub length_pixels: i32,
    pub right: bool,
}
impl Placement {
    pub fn new(right: bool, width: usize, extra_y: usize, style: DialogueStyle) -> Self {
        let zoom = (160 + extra_y * 2) as f32 / 160.;
        let scale = zoom
            * if style.scale.is_finite() {
                style.scale.clamp(0.5, 3.)
            } else {
                1.
            };
        Self {
            scale,
            left: if right {
                width as f32 - 240. * scale
            } else {
                0.
            } + style.offset_x * zoom,
            top: style.offset_y * zoom,
            extra_y: extra_y as i32,
            length_pixels: ((style.length.clamp(1., 2.) - 1.) * 120.).round() as i32,
            right,
        }
    }
    pub fn source_x(self, render_x: i32) -> i32 {
        ((render_x as f32 - self.left) / self.scale).floor() as i32
    }
    pub fn source_y(self, virtual_line: i32) -> i32 {
        ((virtual_line as f32 + self.extra_y as f32 - self.top) / self.scale).floor() as i32
    }
    pub fn destination_x(self, native_x: i32) -> i32 {
        (self.left + native_x as f32 * self.scale).ceil() as i32
    }
}
#[derive(Default)]
pub struct DialogueState {
    pub profile: UiProfile,
    pub reference_dialogue: bool,
    pub reference_frame: bool,
    pub(crate) reuse_world_row: bool,
    pub style: DialogueStyle,
    /// Desktop opt-in: extract native UI instead of drawing it into the world.
    pub separate: bool,
    /// Verified tiled menu backdrop; foreground/UI stay inside their canvas.
    pub menu_pattern: bool,
    pub(crate) title_paper: bool,
    pub(crate) world_paper: bool,
    pub(crate) world_ocean: [u16;3],
    pub(crate) world_fog_key: u64,
    pub(crate) world_fog: Vec<std::sync::atomic::AtomicU16>,
    pub(crate) capture: bool,
    pub(crate) capture_extra: bool,
    pub(crate) capture_backing: bool,
    pub backing: Vec<UiPixel>,
    pub action_layout: bool,
    /// Whole native naming canvas, independent of world zoom.
    pub naming_layout: bool,
    /// 0 hidden, 1 native entering, 2 complete, 3 native exiting.
    pub naming_phase: u8,
    /// A native panel may be gone while its parked portrait is still published.
    pub(crate) suspended_portrait: bool,
    pub(crate) suspended_stats: bool,
    pub action_banner: Option<[i32;4]>,
    pub announcement_bounds: Option<[i32;4]>,
    pub(crate) confirmation_marker:bool,
    pub(crate) action_banner_rotation:usize,
    pub(crate) action_owner_bounds:Option<[i32;4]>,
    pub town_layout: bool,
    pub shop_layout: bool,
    pub(crate) shop_numbers_exiting:bool,
    pub(crate) shop_page_dirty:bool,
    pub(crate) shop_page_complete:Vec<UiPixel>,
    pub site_layout: bool,
    /// Verified service interiors: root MENU left, all remaining UI right.
    pub service_layout: bool,
    pub(crate) pub_window_owners:[Option<usize>;3],
    pub service_speech:Panel,
    pub(crate) service_speech_frame:Option<[i32;4]>,
    pub service_speech_right:bool,
    pub service_list: bool,
    pub funds_bounds:Option<[i32;4]>,
    pub(crate) funds_bg:Option<usize>,
    pub funds_pixels:Vec<UiPixel>,
    pub(crate) capture_funds:bool,
    pub service_list_bounds:Option<[i32;4]>,
    pub attack_name: bool,
    pub(crate) attack_exiting:bool,
    pub attack_rotation:usize,
    pub stats_right:bool,
    pub stats_target:Option<[i32;4]>,
    pub(crate) stats_context:Option<[u16;5]>,
    /// Site menu is on the auxiliary canvas while speech stays edge-attached.
    pub split_speech: bool,
    pub(crate) attachment: Option<(bool, f32)>,
    pub extra: Panel,
    /// Concurrent target stats must never replace an active action/spell list.
    pub battle_stats:Panel,
    pub pixels: Vec<UiPixel>,
    pub(crate) anchor_bounds: Option<(usize, usize)>,
    /// Complete native backing before animated WIN/HOFS clipping.
    pub native_bounds:Option<[i32;4]>,
    /// Exact desktop crop zoom; zero uses render-height fallback for other runners.
    pub view_zoom: f32,
    pub battle_hud_overlay: bool,
    pub view_width: f32,
    pub view_height: f32,
    pub(crate) placement: Option<Placement>,
    pub(crate) wide_pass: bool,
}

pub fn enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("FFTA_DIALOGUE_LAYOUT").as_deref() != Ok("0"))
}

/// Measured EUR portrait: priority 0, 8-bpp, non-affine, OBJ tiles 0x240..0x29A.
/// Battle bodies use 4-bpp. Apply only inside the verified dialogue scene.
pub(crate) fn portrait(attrs: [u16; 3], x: i32) -> bool {
    let [a0, _, a2] = attrs;
    a0 & 0x2300 == 0x2000
        && (a0 >> 10) & 3 <= 1
        && (a2 >> 10) & 3 == 0
        && (0x200..0x400).contains(&(a2 & 1023))
        && (x < 64 || x >= 176)
}

/// Measured tutorial continuation marker: OBJ 8x16, 4-bpp, palette 2,
/// priority 0, tile 0x122. Its three Y positions are guest animation, not layout.
/// Called only inside the existing exact-ROM/verified-dialogue scene gate.
/// UI atlas cursor and scroll markers. Classify only while a verified UI
/// canvas owns them; ordinary battle/world cursors use different resources.
pub(crate) fn list_marker(a:[u16;3])->bool {
    a[0]&0x2300==0 && a[1]&0xc000==0 && a[2]&0x0c00==0
        && (0x120..=0x12f).contains(&(a[2]&1023))
}
/// Four-bit UI atlas objects share the inventory canvas with its BG labels.
/// World actors use a separate producer/priority and must stay in the world.
pub(crate) fn service_object(a:[u16;3])->bool {
    a[0]&0x2300==0 && (a[0]>>10)&3<=1 && a[2]&0x0c00==0
        && (0x120..0x400).contains(&(a[2]&1023))
}
/// Monster Bank's affine preview pieces and their small overlay icons.
/// Their tile allocation is distinct from stage actors (0x400 and above).
pub(crate) fn bank_preview(a:[u16;3])->bool {
    a[0]&0x2200==0 && (a[0]>>10)&3<=1 && a[2]&0x0c00==0
        && (0x2c0..0x340).contains(&(a[2]&1023))
}
/// White, black-outlined inventory attributes: ten paired 8x8 digit tiles.
/// Prices and Clan Funds use different BG resources.
pub(crate) fn shop_attribute_digit(a:[u16;3])->bool {
    a[0]&0xff00==0 && a[1]&0xfe00==0 && (0x02c0..=0x02d3).contains(&a[2])
}
pub(crate) fn service_counter(a:[u16;3],x:i32,y:i32)->bool {
    service_object(a) && !list_marker(a) && x>=152 && y>=96
}
pub(crate) fn continuation(attrs: [u16; 3]) -> bool {
    let [a0, a1, a2] = attrs;
    a0 & 0xff00 == 0x8000 && a1 & 0xfe00 == 0 && a2 == 0x2122
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shop_attribute_filter_only_matches_the_outlined_digit_atlas(){
        for tile in 0x2c0..=0x2d3 {assert!(shop_attribute_digit([92,104,tile]));}
        for a in [[0x4014,0,0x120],[92,104,0x800],[0x805c,0x1068,0x804],
            [0,0,0x2d4],[0,0,0x2bf],[0,0,0x12c0],[0x4000,0,0x2c0]] {
            assert!(!shop_attribute_digit(a));
        }
    }
    #[test]
    fn anchors_keep_native_size_on_screen_at_every_zoom() {
        for ey in [0, 10, 33, 80, 176] {
            for width in [284, 568, 1136, 1536] {
                let right = Placement::new(true, width, ey, DialogueStyle::default());
                let left = Placement::new(false, width, ey, DialogueStyle::default());
                assert_eq!(right.source_x(width as i32 - 1), 239);
                assert_eq!(left.source_x(0), 0);
                assert_eq!(right.source_y(-(ey as i32)), 0);
                // Display-height normalization keeps portrait/text proportion stable.
                assert!((right.scale / (160 + ey * 2) as f32 - 1. / 160.).abs() < 0.00001);
            }
        }
    }
    #[test]
    fn portrait_filter_excludes_units_cursor_and_affine_objects() {
        assert!(portrait([0xa000, 0xc0d0, 0x240], 208));
        assert!(portrait([0x2030, 0x40c0, 0x280], 192));
        assert!(!portrait([0x808c, 0x90, 0x1820], 144));
        assert!(!portrait([0xa100, 0xc0d0, 0x240], 208));
        assert!(!portrait([0x2000, 0, 0x100], 0));
    }
}

/// Fixed native canvas includes room for length extensions on either side.
pub const LAYER_WIDTH: usize = 480;
pub const LAYER_HEIGHT: usize = 160;
pub const LAYER_INSET: i32 = 120;

/// PPU arithmetic, including its rounding and six-bit green lane.
/// Transparent pixels have eva=0, evb=16. No encoded-color alpha approximation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct UiPixel {
    pub color: u16,
    pub eva: u8,
    pub evb: u8,
}
impl Default for UiPixel {
    fn default() -> Self {
        Self {
            color: 0,
            eva: 0,
            evb: 16,
        }
    }
}
impl UiPixel {
    pub fn opaque(color: u16) -> Self {
        Self {
            color,
            eva: 16,
            evb: 0,
        }
    }
    pub fn resolve(self, world: u16) -> u16 {
        let mut color = 0;
        for shift in [0, 5, 10] {
            let lane = if shift == 5 { 2 } else { 1 };
            let value = (((((self.color >> shift) & 31) * lane * self.eva as u16
                + ((world >> shift) & 31) * lane * self.evb as u16
                + 8)
                >> 4)
                .min(if shift == 5 { 63 } else { 31 }))
                / lane;
            color |= value << shift;
        }
        color
    }
    pub fn visible(self) -> bool {
        self.eva != 0 || self.evb != 16
    }
    pub fn packed(self) -> [u8; 4] {
        [
            self.color as u8,
            (self.color >> 8) as u8,
            self.eva,
            self.evb,
        ]
    }
}
impl DialogueState {
    /// Keep the nearest visible vertical edge fixed when Size changes.
    /// Uses the native UI bounds, so upper/lower dialogue and its portrait
    /// share one anchor; world zoom and transient window sizes are irrelevant.
    pub fn vertical_anchor(&self) -> f32 {
        if let Some((_,anchor))=self.attachment {return anchor;}
        let Some((top, bottom)) = self.anchor_bounds else {
            return 0.;
        };
        // Anchor to the screen edge, retaining space for the arrow's full
        // native animation envelope rather than chasing its bouncing tip.
        if top + bottom >= LAYER_HEIGHT {
            LAYER_HEIGHT as f32
        } else {
            0.
        }
    }
    pub(crate) fn include_anchor_bounds(&mut self, top: usize, bottom: usize) {
        if top >= bottom {
            return;
        }
        self.anchor_bounds = Some(
            self.anchor_bounds
                .map_or((top, bottom), |(a, b)| (a.min(top), b.max(bottom))),
        );
    }

    /// Battle NOTICE owns the screen during deployment/confirmation. The
    /// guest keeps the selected unit HUD alive behind it; do not expose that
    /// suspended panel or its portrait as an independently relocated canvas.
    pub fn battle_notice(&self)->bool {
        self.separate && self.action_layout && self.extra.kind==PanelKind::Notice && self.extra.active()
    }
    pub fn layer(&self) -> Option<(&[UiPixel], bool)> {
        (self.separate
            && self.placement.is_some()
            && self.pixels.len() == LAYER_WIDTH * LAYER_HEIGHT)
            .then(|| (self.pixels.as_slice(), self.placement.unwrap().right))
    }
}
#[cfg(test)]
mod layer_tests {
    use super::*;
    #[test]
    fn raw_mix_is_exact_and_transparent_is_identity() {
        for world in 0..32768 {
            assert_eq!(UiPixel::default().resolve(world), world);
            let pixel = UiPixel {
                color: 0x3a1f,
                eva: 9,
                evb: 7,
            };
            for shift in [0, 5, 10] {
                assert_eq!((pixel.resolve(world) >> shift) & 31, {
                    let lane = if shift == 5 { 2 } else { 1 };
                    (((((0x3a1f >> shift) & 31) * lane * 9
                        + ((world >> shift) & 31) * lane * 7
                        + 8)
                        >> 4)
                        .min(if shift == 5 { 63 } else { 31 }))
                        / lane
                });
            }
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PanelKind {
    #[default]
    None,
    Stats,
    Actions,
    Notice,
    Speech,
}
#[derive(Default)]
pub struct Panel {
    pub backing: Vec<UiPixel>,
    /// Raw top BG, before effects or OBJ, for native semitransparent OBJ blending.
    pub background: Vec<u16>,
    pub kind: PanelKind,
    pub bounds: Option<[i32; 4]>,
    pub pixels: Vec<UiPixel>,
}
impl Panel {
    pub fn active(&self) -> bool {
        self.kind != PanelKind::None && self.bounds.is_some()
    }
    pub fn contains_stats(&self, attrs: [u16; 3], x: i32, y: i32, w: i32, h: i32) -> bool {
        let Some([l, t, r, b]) = self.bounds else {
            return false;
        };
        self.kind == PanelKind::Stats
            && attrs[0] & 0x2000 == 0 && attrs[0]&0x300!=0x200
            && (attrs[0] >> 10) & 3 <= 1
            && attrs[2] & 0xc00 == 0
            && matches!(attrs[2] >> 12, 11 | 12)
            && (0x1c8..0x240).contains(&(attrs[2] & 1023))
            && x < r
            && x + w > l
            && y < b
            && y + h > t
    }
}
/// Frame geometry comes from the static backing, never its animated cursor.
pub fn panel_bounds(pixels:&[UiPixel],backing:&[UiPixel])->Option<[i32;4]> {
    pixel_bounds(backing).or_else(||pixel_bounds(pixels))
}
pub fn pixel_bounds(pixels: &[UiPixel]) -> Option<[i32; 4]> {
    let mut out = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
    for (i, p) in pixels.iter().enumerate() {
        if p.visible() {
            let x = (i % LAYER_WIDTH) as i32 - LAYER_INSET;
            let y = (i / LAYER_WIDTH) as i32;
            out = [
                out[0].min(x),
                out[1].min(y),
                out[2].max(x + 1),
                out[3].max(y + 1),
            ];
        }
    }
    (out[0] != i32::MAX).then_some(out)
}
pub(crate) fn action_portrait(attrs: [u16; 3], x: i32, y: i32) -> bool {
    attrs[0] & 0x2000 == 0x2000 && attrs[0]&0x300!=0x200
        && (attrs[0] >> 10) & 3 <= 1
        && attrs[2] & 0xc00 <= 0x400
        && (0x200..0x400).contains(&(attrs[2] & 1023))
        && (-256..256).contains(&x)
        // Long-eared portraits include separate pieces above Y=64. They
        // belong to the same verified 8-bpp portrait atlas, not the world.
        && (-64..160).contains(&y)
}
pub(crate) fn notice_cursor(attrs:[u16;3],x:i32,y:i32,bounds:Option<[i32;4]>) -> bool {
    let Some([l,t,r,b])=bounds else{return false;};
    attrs[0]&0xff00==0 && attrs[1]&0xfe00==0
        && (0x124..=0x127).contains(&(attrs[2]&1023)) && attrs[2]&0xfc00==0
        && x>=l && x+8<=r && y>=t && y+8<=b
}

/// Extend only the backing at its outer edge; translate foreground rigidly inward.
/// Coordinates and scale remain native, so glyphs and portraits are never stretched.
pub fn pack_panel(pixels:&[UiPixel],backing:&[UiPixel],length:f32,right:bool,out:&mut Vec<[u8;4]>) {
    pack_panel_with_bounds(pixels,backing,length,right,None,out);
}
/// Use the owner's complete rectangle throughout entry/content/exit. Visible
/// ink is only a reveal mask and cannot change the extension distance or pivot.
pub fn pack_panel_with_bounds(pixels:&[UiPixel],backing:&[UiPixel],length:f32,right:bool,geometry:Option<[i32;4]>,out:&mut Vec<[u8;4]>) {
    out.resize(pixels.len(),[0;4]);
    if length<=1. || !length.is_finite() || backing.len()!=pixels.len() {
        for (dst,p) in out.iter_mut().zip(pixels) {*dst=p.packed();}
        return;
    }
    let Some(bounds)=geometry.or_else(||panel_bounds(pixels,backing)) else {out.fill(UiPixel::default().packed());return;};
    let requested=((bounds[2]-bounds[0]) as f32*(length.clamp(1.,2.)-1.)).round() as i32;
    let content=geometry.unwrap_or_else(||pixel_bounds(pixels).unwrap_or(bounds));
    let room=if right {content[0]+LAYER_INSET}else{LAYER_WIDTH as i32-LAYER_INSET-content[2]};
    let distance=requested.min(room.max(0));
    if distance==0 || backing.len()!=pixels.len() {
        for (dst,p) in out.iter_mut().zip(pixels) {*dst=p.packed();} return;
    }
    out.fill(UiPixel::default().packed());
    let shift=if right {-distance}else{distance};
    let backing_bounds=geometry.unwrap_or_else(||pixel_bounds(backing).unwrap_or(bounds));
    // One flat-side cut for the entire panel. Per-row cuts follow the rounded
    // outline and can change the cap's contour as Length grows.
    let flat=if right {backing_bounds[2]-1}else{backing_bounds[0]}+LAYER_INSET;
    let width=backing_bounds[2]-backing_bounds[0];
    let inset=8.min(width/2);
    let pivot=flat+if right {-inset}else{inset};
    for y in 0..LAYER_HEIGHT {
        let row=&backing[y*LAYER_WIDTH..(y+1)*LAYER_WIDTH];
        for x in 0..LAYER_WIDTH as i32 {
            let sx=if right {if x>pivot {x}else if x<pivot-distance {x+distance}else{pivot}}
                   else {if x<pivot {x}else if x>pivot+distance {x-distance}else{pivot}};
            if (0..LAYER_WIDTH as i32).contains(&sx) {out[y*LAYER_WIDTH+x as usize]=row[sx as usize].packed();}
        }
        for x in 0..LAYER_WIDTH {
            let i=y*LAYER_WIDTH+x;let p=pixels[i];
            if p.visible() && p!=backing[i] {
                let dx=x as i32+shift;
                if (0..LAYER_WIDTH as i32).contains(&dx) {out[y*LAYER_WIDTH+dx as usize]=p.packed();}
            }
        }
    }
}

/// Native panel cache. Blink-free/static frames reuse packed bytes and geometry.
/// Source equality is exact, never a hash: no stale glyphs from collisions.
#[derive(Default)]
pub struct PackedLayer {
    pixels:Vec<UiPixel>,backing:Vec<UiPixel>,key:Option<(u32,bool,Option<[i32;4]>)>,
    pub bounds:Option<[i32;4]>,
}
impl PackedLayer {
    pub fn update(&mut self,pixels:&[UiPixel],backing:&[UiPixel],length:f32,right:bool,out:&mut Vec<[u8;4]>)->bool {
        self.update_with_bounds(pixels,backing,length,right,None,out)
    }
    pub fn update_with_bounds(&mut self,pixels:&[UiPixel],backing:&[UiPixel],length:f32,right:bool,geometry:Option<[i32;4]>,out:&mut Vec<[u8;4]>)->bool {
        let length=if length.is_finite(){length.clamp(1.,2.)}else{1.};
        let key=(length.to_bits(),right,geometry);
        if self.key==Some(key) && self.pixels==pixels && self.backing==backing && out.len()==pixels.len(){return false;}
        pack_panel_with_bounds(pixels,backing,length,right,geometry,out);
        self.bounds=geometry.or_else(||panel_bounds(pixels,backing));
        self.pixels.clear();self.pixels.extend_from_slice(pixels);
        self.backing.clear();self.backing.extend_from_slice(backing);
        self.key=Some(key);true
    }
    pub fn invalidate(&mut self){self.key=None;self.bounds=None;}
}
#[cfg(test)]
mod panel_extension_tests {
    use super::*;
    #[test]
    fn animated_reveal_cannot_change_length_translation_or_cache_geometry() {
        let blue=UiPixel::opaque(0x7c00);let glyph=UiPixel::opaque(31);
        let geometry=Some([152,104,240,152]);
        let mut cache=PackedLayer::default();let mut out=Vec::new();
        for left in [232,208,176,152,176,208,232] {
            let mut backing=vec![UiPixel::default();480*160];
            for y in 104..152 {for x in left..240 {backing[y*480+x+120]=blue;}}
            let mut pixels=backing.clone();pixels[120*480+355]=glyph;
            cache.update_with_bounds(&pixels,&backing,1.5,true,geometry,&mut out);
            assert_eq!(cache.bounds,geometry);
            assert_eq!(out[120*480+355-44],glyph.packed(),"reveal starts at {left}");
            assert_eq!(out.iter().filter(|p|**p==glyph.packed()).count(),1);
        }
        assert!(cache.update_with_bounds(&cache.pixels.clone(),&cache.backing.clone(),1.5,true,Some([144,104,240,152]),&mut out),"owner geometry invalidates an otherwise identical cache");
    }
    #[test]
    fn packed_cache_observes_every_edit_style_change_and_disappearance() {
        let mut pixels=vec![UiPixel::default();480*160];pixels[1000]=UiPixel::opaque(31);
        let mut cache=PackedLayer::default();let mut out=Vec::new();
        assert!(cache.update(&pixels,&[],1.,true,&mut out));
        assert!(!cache.update(&pixels,&[],1.,true,&mut out));
        pixels[1000]=UiPixel::default();pixels[1001]=UiPixel::opaque(1);
        assert!(cache.update(&pixels,&[],1.,true,&mut out));
        assert_eq!(out[1000],UiPixel::default().packed());assert_eq!(out[1001],pixels[1001].packed());
        assert!(cache.update(&pixels,&[],1.5,true,&mut out));
        out.clear();assert!(cache.update(&pixels,&[],1.5,true,&mut out));
        cache.invalidate();assert!(cache.update(&pixels,&[],1.5,true,&mut out));
    }
    #[test]
    #[ignore = "release packing microbenchmark, not gameplay FPS"]
    fn bench_static_panel_packing() {
        let mut pixels=vec![UiPixel::default();480*160];
        for y in 80..152 {for x in 120..360 {pixels[y*480+x]=UiPixel::opaque(0x7c00);}}
        let backing=pixels.clone();
        for cached in [false,true] {
            let mut out=Vec::new();let mut cache=PackedLayer::default();
            cache.update(&pixels,&backing,1.5,true,&mut out);
            let start=std::time::Instant::now();
            for _ in 0..1000 {
                if cached {cache.update(&pixels,&backing,1.5,true,&mut out);}
                else{pack_panel(&pixels,&backing,1.5,true,&mut out);}
                std::hint::black_box(&out);
            }
            println!("UI_PACK_BENCH cached={cached} mean_ms={:.4}",start.elapsed().as_secs_f64());
        }
    }
    #[test]
    fn long_foreground_never_falls_out_of_layer_when_length_increases() {
        let bg=UiPixel::opaque(0x7c00);let ink=UiPixel::opaque(31);
        for right in [false,true] {
            let mut backing=vec![UiPixel::default();LAYER_WIDTH*LAYER_HEIGHT];
            for y in 10..30 {for x in 120..360 {backing[y*480+x]=bg;}}
            let mut pixels=backing.clone();
            let range=if right {1..119}else{361..479};
            for x in range.clone(){pixels[15*480+x]=ink;}
            let mut out=Vec::new();pack_panel(&pixels,&backing,2.,right,&mut out);
            assert_eq!(out.iter().filter(|p|**p==ink.packed()).count(),range.len());
        }
    }
    #[test]
    fn animated_cursor_cannot_change_panel_bounds_or_length_displacement() {
        let blue=UiPixel{color:0x7c00,eva:8,evb:8};
        let word=UiPixel::opaque(0x03e0);let cursor=UiPixel::opaque(31);
        let mut backing=vec![UiPixel::default();LAYER_WIDTH*LAYER_HEIGHT];
        for y in 70..150 {for x in 296..360 {backing[y*LAYER_WIDTH+x]=blue;}}
        let stable=panel_bounds(&backing,&backing).unwrap();
        assert_eq!(stable,[176,70,240,150]);
        let mut old_bounds=std::collections::BTreeSet::new();
        for length in [1.,1.25,1.5,2.] {
            let mut expected=None;
            for cursor_x in [None,Some(291),Some(292),Some(294),Some(296)] {
                let mut pixels=backing.clone();
                for y in 90..98 {for x in 310..326 {pixels[y*LAYER_WIDTH+x]=word;}}
                if let Some(x)=cursor_x {for y in 90..98 {pixels[y*LAYER_WIDTH+x]=cursor;}}
                old_bounds.insert(pixel_bounds(&pixels).unwrap());
                assert_eq!(panel_bounds(&pixels,&backing),Some(stable));
                let mut packed=Vec::new();pack_panel(&pixels,&backing,length,true,&mut packed);
                let positions:Vec<_>=packed.iter().enumerate().filter_map(|(i,p)|(*p==word.packed()).then_some(i)).collect();
                assert_eq!(positions.len(),128);
                if let Some(ref previous)=expected {assert_eq!(&positions,previous);}else{expected=Some(positions);}
            }
        }
        assert!(old_bounds.len()>1,"fixture must exercise the old fluctuating width");
    }
    #[test]
    fn rounded_inner_cap_is_translated_rigidly_on_both_sides() {
        let blue=UiPixel{color:0x7c00,eva:8,evb:8};
        for right in [false,true] {
            let mut backing=vec![UiPixel::default();LAYER_WIDTH*LAYER_HEIGHT];
            for y in 0..48 {
                let dy=(y as f32+0.5)-24.;
                let edge=(24.-(24.*24.-dy*dy).sqrt()).floor()as usize;
                let (l,r)=if right{(240+edge,328)}else{(120,208-edge)};
                for x in l..r {backing[(y+50)*LAYER_WIDTH+x]=blue;}
            }
            let mut out=Vec::new();
            for length in [1.,1.25,1.5,2.] {
                pack_panel(&backing,&backing,length,right,&mut out);
                let d=(88.*(length-1.)).round() as usize;
                for y in 50..98 {
                    let src=&backing[y*LAYER_WIDTH..(y+1)*LAYER_WIDTH];
                    let dst=&out[y*LAYER_WIDTH..(y+1)*LAYER_WIDTH];
                    let l=src.iter().position(|p|p.visible()).unwrap();let r=src.iter().rposition(|p|p.visible()).unwrap();
                    assert_eq!(dst.iter().position(|p|*p==blue.packed()).unwrap(),if right{l-d}else{l});
                    assert_eq!(dst.iter().rposition(|p|*p==blue.packed()).unwrap(),if right{r}else{r+d});
                }
            }
        }
    }
    #[test]
    fn length_translates_foreground_without_resampling_and_keeps_backing_caps() {
        let blue=UiPixel{color:0x7c00,eva:8,evb:8};
        let glyph=UiPixel::opaque(31);let border=UiPixel::opaque(0x03e0);
        let mut backing=vec![UiPixel::default();LAYER_WIDTH*LAYER_HEIGHT];
        for y in 10..30 {for x in 140..180 {backing[y*LAYER_WIDTH+x]=blue;}
            backing[y*LAYER_WIDTH+140]=border;backing[y*LAYER_WIDTH+179]=border;}
        let mut pixels=backing.clone();
        for y in 12..20 {for x in 153..157 {pixels[y*LAYER_WIDTH+x]=glyph;}}
        for right in [false,true] {
            let mut packed=Vec::new();pack_panel(&pixels,&backing,1.,right,&mut packed);
            assert_eq!(packed,pixels.iter().map(|p|p.packed()).collect::<Vec<_>>());
            for length in [1.25,1.5,2.] {
                pack_panel(&pixels,&backing,length,right,&mut packed);
                let distance=(40.*(length-1.))as i32;let shift=if right{-distance}else{distance};
                assert_eq!(packed.iter().filter(|p|**p==glyph.packed()).count(),32);
                for y in 12..20 {for x in 153..157 {
                    assert_eq!(packed[y*LAYER_WIDTH+(x as i32+shift)as usize],glyph.packed());
                }}
                assert_eq!(packed[15*LAYER_WIDTH+if right {179}else{140}],border.packed());
                assert_eq!(packed.iter().filter(|p|**p==blue.packed()).count(),(38+distance as usize)*20-32);
            }
        }
    }
}

#[derive(Default)]
pub struct UiProfile { pub enabled:bool, pub capture_ns:u64, pub setup_ns:u64, pub raster_ns:u64, pub zoom_ns:u64, pub rows:u32 }
