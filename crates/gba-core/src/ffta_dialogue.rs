//! Host-only dialogue placement. This is the single adjustment point for
//! future Settings controls; it never changes OAM, VRAM or game coordinates.

#[derive(Clone, Copy, Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
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
#[derive(serde::Serialize, serde::Deserialize)]
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
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DialogueState {
    pub style: DialogueStyle,
    /// Desktop opt-in: extract native UI instead of drawing it into the world.
    pub separate: bool,
    pub(crate) capture: bool,
    pub(crate) capture_extra: bool,
    pub(crate) capture_backing: bool,
    pub backing: Vec<UiPixel>,
    pub action_layout: bool,
    pub(crate) action_banner: Option<[i32;4]>,
    pub town_layout: bool,
    pub shop_layout: bool,
    pub site_layout: bool,
    /// Host-only subtype of the centered Notice layer; recomputed each frame.
    /// Skipped so existing snapshot layouts stay compatible.
    #[serde(skip)]
    pub laws_layout: bool,
    pub extra: Panel,
    pub pixels: Vec<UiPixel>,
    pub(crate) anchor_bounds: Option<(usize, usize)>,
    /// Exact desktop crop zoom; zero uses render-height fallback for other runners.
    pub view_zoom: f32,
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
pub(crate) fn continuation(attrs: [u16; 3]) -> bool {
    let [a0, a1, a2] = attrs;
    a0 & 0xff00 == 0x8000 && a1 & 0xfe00 == 0 && a2 == 0x2122
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn laws_host_subtype_does_not_change_snapshot_bytes() {
        let mut machine = crate::Machine::new(vec![0; 192]);
        let original = machine.save_state().unwrap();
        machine.bus.ffta_dialogue.laws_layout = true;
        assert_eq!(machine.save_state().unwrap(), original);
        machine.load_state(&original).unwrap();
        assert!(!machine.bus.ffta_dialogue.laws_layout);
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
#[derive(serde::Serialize, serde::Deserialize)]
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
#[derive(serde::Serialize, serde::Deserialize)]
pub enum PanelKind {
    #[default]
    None,
    Stats,
    Actions,
    Notice,
}
#[derive(Default)]
#[derive(serde::Serialize, serde::Deserialize)]
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
            && attrs[0] & 0x2300 == 0
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
    attrs[0] & 0x2300 == 0x2000
        && (attrs[0] >> 10) & 3 <= 1
        && attrs[2] & 0xc00 <= 0x400
        && (0x200..0x400).contains(&(attrs[2] & 1023))
        && (-64..64).contains(&x)
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

/// Verified laws selector (8x16) and HELP continuation marker. These belong
/// to the centered modal, not the portrait or the world beneath it.
pub(crate) fn laws_cursor(attrs: [u16; 3], x: i32, y: i32, bounds: Option<[i32; 4]>) -> bool {
    let Some([l, t, r, b]) = bounds else { return false; };
    attrs[1] & 0xfe00 == 0 && x >= l && x + 8 <= r && y >= t && y + 16 <= b
        && ((attrs[0] & 0xff00 == 0x8000 && attrs[2] == 0x02dc) || continuation(attrs))
}

/// Extend only the backing at its outer edge; translate foreground rigidly inward.
/// Coordinates and scale remain native, so glyphs and portraits are never stretched.
pub fn pack_panel(pixels:&[UiPixel],backing:&[UiPixel],length:f32,right:bool,out:&mut Vec<[u8;4]>) {
    out.resize(pixels.len(),[0;4]);
    let Some(bounds)=panel_bounds(pixels,backing) else {out.fill(UiPixel::default().packed());return;};
    let distance=((bounds[2]-bounds[0]) as f32*(length.clamp(1.,2.)-1.)).round() as i32;
    if distance==0 || backing.len()!=pixels.len() {
        for (dst,p) in out.iter_mut().zip(pixels) {*dst=p.packed();} return;
    }
    out.fill(UiPixel::default().packed());
    let shift=if right {-distance}else{distance};
    let backing_bounds=pixel_bounds(backing).unwrap_or(bounds);
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

#[cfg(test)]
mod panel_extension_tests {
    use super::*;
    fn row_endpoint_pixel_bounds(pixels: &[UiPixel]) -> Option<[i32; 4]> {
        let mut out = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
        // Only row endpoints can enlarge the bounds. Avoid visiting a dense
        // panel's interior, and derive coordinates once per nonempty row.
        // chunks() also preserves the previous behavior for partial final rows.
        for (y, row) in pixels.chunks(LAYER_WIDTH).enumerate() {
            if let Some(left) = row.iter().position(|p| p.visible()) {
                let right = left + row[left..].iter().rposition(|p| p.visible()).unwrap();
                out = [
                    out[0].min(left as i32 - LAYER_INSET),
                    out[1].min(y as i32),
                    out[2].max(right as i32 + 1 - LAYER_INSET),
                    out[3].max(y as i32 + 1),
                ];
            }
        }
        (out[0] != i32::MAX).then_some(out)
    }
    #[test]
    fn row_endpoint_bounds_match_full_scan_for_sparse_dense_and_partial_layers() {
        for len in [0, 1, 479, 480, 481, 480 * 160, 480 * 161 + 7] {
            let mut pixels = vec![UiPixel::default(); len];
            assert_eq!(row_endpoint_pixel_bounds(&pixels), pixel_bounds(&pixels));
            for i in 0..len {
                // Include varying colors/blend weights, isolated endpoints,
                // transparent holes, and pixels outside the usual canvas.
                let bits = (i as u32).wrapping_mul(1664525).wrapping_add(1013904223);
                pixels[i] = UiPixel {
                    color: bits as u16,
                    eva: (bits >> 16) as u8 % 17,
                    evb: (bits >> 24) as u8 % 17,
                };
            }
            assert_eq!(row_endpoint_pixel_bounds(&pixels), pixel_bounds(&pixels));
            pixels.fill(UiPixel::opaque(31));
            assert_eq!(row_endpoint_pixel_bounds(&pixels), pixel_bounds(&pixels));
            pixels.fill(UiPixel::default());
            for i in [0, 479, 480, len.saturating_sub(1)] {
                if i < len {
                    pixels[i] = UiPixel::opaque(0);
                    assert_eq!(row_endpoint_pixel_bounds(&pixels), pixel_bounds(&pixels));
                    pixels[i] = UiPixel::default();
                }
            }
        }
    }

    #[test]
    #[ignore = "diagnostic microbenchmark; run release and serially without other workloads"]
    fn benchmark_panel_bounds() {
        use std::{hint::black_box, time::Instant};
        for pattern in ["empty", "dense", "sparse", "panel"] {
            let mut pixels = vec![UiPixel::default(); LAYER_WIDTH * LAYER_HEIGHT];
            for y in 0..LAYER_HEIGHT {
                for x in 0..LAYER_WIDTH {
                    if match pattern {
                        "dense" => true,
                        "sparse" => (x + y * 7) % 113 == 0,
                        "panel" => (240..360).contains(&x) && (70..150).contains(&y),
                        _ => false,
                    } {
                        pixels[y * LAYER_WIDTH + x] = UiPixel::opaque(31);
                    }
                }
            }
            assert_eq!(row_endpoint_pixel_bounds(&pixels), pixel_bounds(&pixels));
            for run in 0..5 {
                let mut elapsed = [0.; 2];
                for which in if run % 2 == 0 { [0, 1] } else { [1, 0] } {
                    let start = Instant::now();
                    for _ in 0..2000 {
                        black_box(if which == 0 {
                            pixel_bounds(black_box(&pixels))
                        } else {
                            row_endpoint_pixel_bounds(black_box(&pixels))
                        });
                    }
                    elapsed[which] = start.elapsed().as_secs_f64() * 1e6 / 2000.;
                }
                eprintln!("PANEL_BOUNDS pattern={pattern} run={run} reference_us={:.3} candidate_us={:.3}", elapsed[0], elapsed[1]);
            }
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
