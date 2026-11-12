//! Kiosk-only frame-rate readout, for comparing builds on the device.
use super::{Painter, Shape};
use tiny_skia::Transform;

// 3×5 digits: the top row first, the left column in the high bit.
const DIGITS: [u16; 10] = [
    0b111_101_101_101_111,
    0b010_110_010_010_111,
    0b111_001_111_100_111,
    0b111_001_111_001_111,
    0b101_101_111_001_001,
    0b111_100_111_001_111,
    0b111_100_111_101_111,
    0b111_001_001_001_001,
    0b111_101_111_101_111,
    0b111_101_111_001_111,
];
const CELL: f32 = 4.;

impl Painter {
    /// `fps` in the top right corner, over everything else.
    pub(super) fn paint_fps(&mut self, viewport: (f32, f32)) {
        let Some(fps) = self.fps else { return };
        let digits = fps.to_string();
        let width = (digits.len() as f32 * 4. + 1.) * CELL;
        let (x, y) = (viewport.0 - width - 2. * CELL, 2. * CELL);
        let mut fill = |rect, radius, color| {
            self.backend
                .fill(&Shape::new(rect, [radius; 4]), color, Transform::identity());
        };
        fill((x, y, width, 7. * CELL), CELL, [0, 0, 0, 200]);
        for (place, digit) in digits.bytes().enumerate() {
            for cell in
                (0..15).filter(|cell| DIGITS[usize::from(digit - b'0')] >> (14 - cell) & 1 == 1)
            {
                let column = (place * 4 + cell % 3 + 1) as f32;
                let row = (cell / 3 + 1) as f32;
                fill(
                    (x + column * CELL, y + row * CELL, CELL, CELL),
                    0.,
                    [255; 4],
                );
            }
        }
    }
}
