//! CPU-defined pixel-centre mapping, shared by software sampling and GPU tables.
use super::Frame;

pub(super) fn ui_axis(pos: f32, rect: [f32; 2], clip: [f32; 2], size: u32) -> Option<u32> {
    if pos < clip[0] || pos >= clip[0] + clip[1] {
        return None;
    }
    let u = (pos - rect[0]) / rect[1];
    (0. ..1.)
        .contains(&u)
        .then(|| (u * size as f32).floor() as u32)
}

pub(super) fn world_axis(pos: f32, clip: [f32; 2], view: [f32; 2], size: u32) -> u32 {
    (view[0] + (pos - clip[0]) / clip[1] * view[1])
        .floor()
        .clamp(0., size as f32 - 1.) as u32
}

#[derive(PartialEq, Eq)]
struct Key {
    floats: [u32; 12],
    dims: (u32, u32),
    limit: u32,
}

pub(super) struct Map {
    key: Option<Key>,
    pub width: u32,
    pub origin: [u32; 2],
    pub bytes: Vec<u8>,
}

impl Default for Map {
    fn default() -> Self {
        Self {
            key: None,
            width: 1,
            origin: [0; 2],
            bytes: Vec::new(),
        }
    }
}

impl Map {
    pub fn scissor(&self, bounds: [u32; 4]) -> Option<[u32; 4]> {
        let mut ranges = [[0; 2]; 2];
        for axis in 0..2 {
            let row = &self.bytes[axis * self.width as usize * 8..][..self.width as usize * 8];
            let inside = |word: &[u8]| {
                let ui = u32::from_le_bytes(word[..4].try_into().unwrap());
                ui >= bounds[axis] && ui < bounds[axis + 2]
            };
            let first = row.chunks_exact(8).position(inside)? as u32;
            let last = row.chunks_exact(8).rposition(inside)? as u32;
            ranges[axis] = [self.origin[axis] + first, last - first + 1];
        }
        Some([ranges[0][0], ranges[1][0], ranges[0][1], ranges[1][1]])
    }

    pub fn update(&mut self, frame: &Frame<'_>, limit: u32) -> bool {
        let key = Key {
            floats: std::array::from_fn(|i| {
                if i < 4 {
                    frame.layout.rect[i]
                } else if i < 8 {
                    frame.layout.clip[i - 4]
                } else {
                    frame.world_view[i - 8]
                }
                .to_bits()
            }),
            dims: frame.world_dims,
            limit,
        };
        if self.key.as_ref() == Some(&key) {
            return false;
        }
        let clip = frame.layout.clip;
        self.origin = [
            clip[0].max(0.).floor() as u32,
            clip[1].max(0.).floor() as u32,
        ]
        .map(|origin| origin.min(limit));
        let lengths: [u32; 2] = std::array::from_fn(|axis| {
            ((clip[axis] + clip[axis + 2]).max(0.).ceil() as u32).saturating_sub(self.origin[axis])
        });
        // Keep oversized/minimized-window preparation bounded by device limits;
        // surface validation still owns the normal fallback decision.
        self.width = lengths[0].max(lengths[1]).max(1).min(limit.max(1));
        self.bytes.clear();
        self.bytes.reserve_exact(self.width as usize * 16);
        for axis in 0..2 {
            let extent = if axis == 0 { 480 } else { 160 };
            let world_extent = if axis == 0 {
                frame.world_dims.0
            } else {
                frame.world_dims.1
            };
            for i in 0..self.width {
                let pos = (self.origin[axis] + i) as f32 + 0.5;
                let ui = ui_axis(
                    pos,
                    [frame.layout.rect[axis], frame.layout.rect[axis + 2]],
                    [clip[axis], clip[axis + 2]],
                    extent,
                );
                let world = if ui.is_some() {
                    world_axis(
                        pos,
                        [clip[axis], clip[axis + 2]],
                        [frame.world_view[axis], frame.world_view[axis + 2]],
                        world_extent,
                    )
                } else {
                    0
                };
                self.bytes
                    .extend_from_slice(&ui.unwrap_or(u32::MAX).to_le_bytes());
                self.bytes.extend_from_slice(&world.to_le_bytes());
            }
        }
        self.key = Some(key);
        true
    }
}

/// Only the shader's exact discard predicate is transparent. Palette values
/// cannot prove transparency: even black or world-only blends can be visible.
pub(super) fn visible_bounds(pixels: &[[u8; 4]]) -> Option<[u32; 4]> {
    let mut bounds = [480, 160, 0, 0];
    for (y, row) in pixels.chunks_exact(480).enumerate() {
        let visible = |p: &[u8; 4]| p[2] != 0 || p[3] != 16;
        if let Some(left) = row.iter().position(visible) {
            let right = row.iter().rposition(visible).unwrap() + 1;
            bounds[0] = bounds[0].min(left as u32);
            bounds[1] = bounds[1].min(y as u32);
            bounds[2] = bounds[2].max(right as u32);
            bounds[3] = y as u32 + 1;
        }
    }
    (bounds[2] > bounds[0]).then_some(bounds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_layer::Layout;

    fn frame() -> Frame<'static> {
        Frame {
            pixels: &[],
            pixels_revision: None,
            raw_world: &[],
            world_dims: (640, 320),
            lut: &[],
            layout: Layout::new((320, 160), 320., true, 1., [0., 0.]),
            world_view: [0., 0., 640. / 3., 320.],
        }
    }

    fn entry(map: &Map, axis: usize, index: usize) -> [u32; 2] {
        let start = (axis * map.width as usize + index) * 8;
        [
            u32::from_le_bytes(map.bytes[start..start + 4].try_into().unwrap()),
            u32::from_le_bytes(map.bytes[start + 4..start + 8].try_into().unwrap()),
        ]
    }

    #[test]
    fn tables_preserve_cpu_boundary_policy_and_cover_clip_and_clamp() {
        let f = frame();
        let mut map = Map::default();
        assert!(map.update(&f, 512));
        // Previously selected differently by WGSL after reassociation.
        assert_eq!(entry(&map, 0, 4)[1], 2);
        assert_eq!(entry(&map, 1, 0)[1], 1);
        for y in 0..160 {
            for x in 0..320 {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let legacy_x = (px / 320. * (640. / 3.)).floor() as u32;
                let legacy_y = (py / 160. * 320.).floor() as u32;
                assert_eq!(
                    (entry(&map, 0, x)[1], entry(&map, 1, y)[1]),
                    (legacy_x, legacy_y)
                );
            }
        }
        let clipped = Frame {
            layout: Layout {
                rect: [-8., -4., 480., 160.],
                clip: [2.25, 3.75, 3.5, 2.],
            },
            world_view: [-100., 999., 1., 1.],
            ..f
        };
        map.update(&clipped, 512);
        assert_eq!(map.origin, [2, 3]);
        assert_eq!(entry(&map, 0, 0)[1], 0);
        assert_eq!(entry(&map, 1, 0)[0], u32::MAX);
        assert_eq!(entry(&map, 1, 1)[1], 319);
        assert_eq!(entry(&map, 1, 3)[0], u32::MAX);
    }

    #[test]
    fn complete_geometry_key_reuses_data_and_bounds_oversized_layouts() {
        let f = frame();
        let mut map = Map::default();
        map.update(&f, 512);
        let bytes = map.bytes.clone();
        let capacity = map.bytes.capacity();
        assert!(!map.update(&f, 512));
        assert_eq!(map.bytes, bytes);
        assert_eq!(map.bytes.capacity(), capacity);
        for i in 0..12 {
            let mut changed = f;
            let value = if i < 4 {
                &mut changed.layout.rect[i]
            } else if i < 8 {
                &mut changed.layout.clip[i - 4]
            } else {
                &mut changed.world_view[i - 8]
            };
            *value += 0.25;
            assert!(map.update(&changed, 512), "key field {i}");
            assert!(map.update(&f, 512));
        }
        assert!(map.update(
            &Frame {
                world_dims: (320, 160),
                ..f
            },
            512
        ));
        let outside = Frame {
            layout: Layout {
                rect: f.layout.rect,
                clip: [f32::MAX; 4],
            },
            ..f
        };
        assert!(map.update(&outside, 512));
        assert_eq!(map.bytes.len(), 512 * 16);
        assert_eq!(map.origin, [512; 2]);
        assert!(
            map.bytes
                .chunks_exact(8)
                .all(|v| u32::from_le_bytes(v[..4].try_into().unwrap()) == u32::MAX)
        );
    }

    #[test]
    fn visible_bounds_and_integer_scissor_never_exclude_a_sampled_pixel() {
        let mut pixels = vec![[255, 255, 0, 16]; 480 * 160];
        assert_eq!(visible_bounds(&pixels), None);
        pixels[40 * 480 + 190] = [0, 0, 0, 15];
        pixels[90 * 480 + 270] = [0, 0, 0, 0];
        let bounds = visible_bounds(&pixels).unwrap();
        assert_eq!(bounds, [190, 40, 271, 91]);
        let f = Frame {
            pixels: &pixels,
            ..frame()
        };
        let mut map = Map::default();
        map.update(&f, 512);
        let rect = map.scissor(bounds).unwrap();
        for y in 0..160 {
            for x in 0..320 {
                if let Some(index) = f.layout.source(x as f32 + 0.5, y as f32 + 0.5) {
                    let p = pixels[index];
                    if p[2] != 0 || p[3] != 16 {
                        assert!((rect[0]..rect[0] + rect[2]).contains(&(x as u32)));
                        assert!((rect[1]..rect[1] + rect[3]).contains(&(y as u32)));
                    }
                }
            }
        }
    }
}
