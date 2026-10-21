//! Original menu pictograms, drawn in a shared optical 24-unit grid.
//! No third-party SVG paths, bitmap assets or icon fonts are embedded.
use super::{Row, canvas::Canvas};
use tiny_skia::{PathBuilder, Transform};

pub(super) fn draw(c: &mut Canvas, row: Row, x: f32, y: f32, scale: f32, ink: [u8; 3]) {
    let transform = Transform::from_row(scale, 0., 0., scale, x - 12. * scale, y - 12. * scale);
    let mut paint = |path: PathBuilder, filled: bool| {
        if let Some(path) = path.finish().and_then(|p| p.transform(transform)) {
            c.path(path, ink, if filled { 0. } else { 1.55 * scale });
        }
    };
    let mut p = PathBuilder::new();
    match row {
        Row::Resume => {
            p.move_to(8., 4.7);
            p.quad_to(6.5, 3.8, 6.5, 5.6);
            p.line_to(6.5, 18.4);
            p.quad_to(6.5, 20.2, 8., 19.3);
            p.line_to(18.5, 13.);
            p.quad_to(20., 12., 18.5, 11.);
            p.close();
            paint(p, true);
        }
        Row::SaveStatesMenu | Row::SaveBackupsMenu => {
            p.move_to(4., 3.); p.line_to(18., 3.); p.line_to(21., 6.);
            p.line_to(21., 21.); p.line_to(3., 21.); p.line_to(3., 3.); p.close();
            p.move_to(7., 3.); p.line_to(7., 10.); p.line_to(17., 10.); p.line_to(17., 3.);
            p.move_to(7., 21.); p.line_to(7., 14.); p.line_to(17., 14.); p.line_to(17., 21.);
            paint(p, false);
        }
        Row::FastForwardMenu => {
            for left in [2., 12.] {
                let mut triangle = PathBuilder::new();
                triangle.move_to(left, 5.);
                triangle.line_to(left + 8.5, 12.);
                triangle.line_to(left, 19.);
                triangle.close();
                paint(triangle, false);
            }
        }
        Row::TextBubblesMenu => {
            p.move_to(6., 4.);
            p.line_to(18., 4.);
            p.quad_to(22., 4., 22., 8.);
            p.line_to(22., 14.);
            p.quad_to(22., 18., 18., 18.);
            p.line_to(11., 18.);
            p.line_to(6., 22.);
            p.line_to(6., 18.);
            p.quad_to(2., 18., 2., 14.);
            p.line_to(2., 8.);
            p.quad_to(2., 4., 6., 4.);
            p.close();
            paint(p, false);
        }
        Row::AudioMenu => {
            // Hollow enclosure and tapered cone share one continuous contour.
            p.move_to(3., 8.5);
            p.line_to(6.8, 8.5);
            p.line_to(12., 4.3);
            p.quad_to(12.7, 3.8, 12.7, 4.8);
            p.line_to(12.7, 19.2);
            p.quad_to(12.7, 20.2, 12., 19.7);
            p.line_to(6.8, 15.5);
            p.line_to(3., 15.5);
            p.quad_to(2., 15.5, 2., 14.5);
            p.line_to(2., 9.5);
            p.quad_to(2., 8.5, 3., 8.5);
            p.close();
            p.move_to(16., 8.);
            p.cubic_to(18.6, 10.1, 18.6, 13.9, 16., 16.);
            p.move_to(19.2, 4.8);
            p.cubic_to(23.6, 8.6, 23.6, 15.4, 19.2, 19.2);
            paint(p, false);
        }
        Row::VideoMenu => {
            // Bezel, lower seam, pedestal and foot are separate readable parts.
            p.move_to(4., 3.5);
            p.line_to(20., 3.5);
            p.quad_to(22., 3.5, 22., 5.5);
            p.line_to(22., 15.);
            p.quad_to(22., 17., 20., 17.);
            p.line_to(4., 17.);
            p.quad_to(2., 17., 2., 15.);
            p.line_to(2., 5.5);
            p.quad_to(2., 3.5, 4., 3.5);
            p.close();
            p.move_to(2.8, 14.);
            p.line_to(21.2, 14.);
            p.move_to(10., 17.5);
            p.line_to(9.2, 20.5);
            p.move_to(14., 17.5);
            p.line_to(14.8, 20.5);
            p.move_to(7., 21.);
            p.line_to(17., 21.);
            paint(p, false);
        }
        Row::InputsMenu => {
            // Sculpted grips with an open centre, instead of a pill-shaped box.
            p.move_to(7., 5.);
            p.cubic_to(4.6, 5., 3.6, 7.1, 2.8, 10.2);
            p.line_to(1.4, 16.4);
            p.cubic_to(0.8, 19.8, 3.3, 21., 5.3, 18.9);
            p.line_to(8.1, 16.);
            p.quad_to(12., 14.6, 15.9, 16.);
            p.line_to(18.7, 18.9);
            p.cubic_to(20.7, 21., 23.2, 19.8, 22.6, 16.4);
            p.line_to(21.2, 10.2);
            p.cubic_to(20.4, 7.1, 19.4, 5., 17., 5.);
            p.quad_to(12., 6.7, 7., 5.);
            p.close();
            p.move_to(4.7, 10.9);
            p.line_to(10., 10.9);
            p.move_to(7.35, 8.25);
            p.line_to(7.35, 13.55);
            paint(p, false);
            // Two small diagonal face buttons, same ink as the D-pad.
            for (bx, by) in [(15.6, 12.2), (18., 9.8)] {
                let dot = PathBuilder::from_circle(bx, by, 1.)
                    .unwrap()
                    .transform(transform)
                    .unwrap();
                c.path(dot, ink, 0.);
            }
        }
        Row::SwitchRomMenu => {
            p.move_to(6., 3.);
            p.line_to(18., 3.);
            p.line_to(21., 6.);
            p.line_to(21., 19.5);
            p.quad_to(21., 21., 19.5, 21.);
            p.line_to(4.5, 21.);
            p.quad_to(3., 21., 3., 19.5);
            p.line_to(3., 6.);
            p.close();
            p.move_to(7., 9.);
            p.line_to(17., 9.);
            p.line_to(14.5, 6.8);
            p.move_to(17., 13.5);
            p.line_to(7., 13.5);
            p.line_to(9.5, 15.7);
            for bx in [8., 12., 16.] {
                p.move_to(bx, 18.8);
                p.line_to(bx, 20.5);
            }
            paint(p, false);
        }
        Row::Quit => {
            // Door in perspective, frame and a clear outward arrow.
            p.move_to(10., 3.);
            p.line_to(3., 4.7);
            p.line_to(3., 19.3);
            p.line_to(10., 21.);
            p.close();
            p.move_to(10., 3.);
            p.line_to(14., 3.);
            p.line_to(14., 6.5);
            p.move_to(14., 17.5);
            p.line_to(14., 21.);
            p.line_to(10., 21.);
            p.move_to(13., 12.);
            p.line_to(22., 12.);
            p.move_to(18.5, 8.5);
            p.line_to(22., 12.);
            p.line_to(18.5, 15.5);
            paint(p, false);
        }
        _ => {}
    }
}
