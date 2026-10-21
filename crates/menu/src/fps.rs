//! Compact bitmap HUD in the existing native UI layer's packed color format.
use crate::font;

pub const WIDTH: usize = 120;
pub const HEIGHT: usize = 26;
pub const STRIDE: usize = 480;

/// Refresh only when the displayed rates or selected mode change.
pub fn render(pixels: &mut Vec<[u8; 4]>, mode: &str, rates: Option<(f64, f64)>) {
    pixels.resize(STRIDE * 160, [0, 0, 0, 16]);
    pixels.fill([0, 0, 0, 16]);
    let lines: &[(&str, usize)] = match mode {
        "internal" => &[("INT", 0)],
        "external" => &[("EXT", 1)],
        "both" => &[("INT", 0), ("EXT", 1)],
        _ => return,
    };
    let line_height = font::H + 2;
    let height = lines.len() * line_height + 6;
    for y in 0..height {
        pixels[y * STRIDE..y * STRIDE + WIDTH].fill([0, 0, 16, 0]);
    }
    for (row, &(label, index)) in lines.iter().enumerate() {
        let value = match rates {
            Some((internal, external)) => format!(
                "{:5.1}",
                if index == 0 { internal } else { external }.min(999.9)
            ),
            None => " --.-".into(),
        };
        let text = format!("FPS {label}:{value}");
        for (column, ch) in text.chars().enumerate() {
            for (y, bits) in font::glyph(ch).iter().enumerate() {
                for x in 0..font::W {
                    let px = 4 + column * font::W + x;
                    if px < WIDTH - 4 && bits & (0x80 >> x) != 0 {
                        pixels[(3 + row * line_height + y) * STRIDE + px] = [255, 127, 16, 0];
                    }
                }
            }
        }
    }
}
