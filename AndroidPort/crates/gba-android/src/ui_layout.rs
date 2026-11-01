// Screen-space UI coordinates shared with the desktop layout rules.
pub(crate) fn letterbox_f(window:(u32,u32),width:f32,height:f32)->(f32,f32,f32){
    let scale=(window.0 as f32/width).min(window.1 as f32/height);
    (scale,(window.0 as f32-width*scale)/2.,(window.1 as f32-height*scale)/2.)
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
