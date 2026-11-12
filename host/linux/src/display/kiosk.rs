//! Kiosk-only rotation for the direct KMS probe, enabled explicitly.
use crate::input::InputEvent;
use tiny_skia::Pixmap;

// The panel is mounted sideways: logical portrait -> framebuffer clockwise 90°.
pub(super) fn copy_portrait(frame: &Pixmap, dst: &mut [u8], pitch: usize, width: u32, height: u32) {
    assert_eq!((frame.width(), frame.height()), (height, width));
    let src = frame.data();
    for y in 0..height as usize {
        for x in 0..width as usize {
            let source = ((width as usize - 1 - x) * height as usize + y) * 4;
            let target = y * pitch + x * 4;
            dst[target..target + 4].copy_from_slice(&[
                src[source + 2],
                src[source + 1],
                src[source],
                255,
            ]);
        }
    }
}

// Absolute touch follows the panel; relative mouse motion follows the user.
// Partial absolute-axis reports stay partial.
pub(super) fn input(event: InputEvent) -> InputEvent {
    match event {
        InputEvent::Absolute(x, y) => InputEvent::Absolute(y, x.map(|x| 1.0 - x)),
        event => event,
    }
}

/// The rate of the motion on screen (`KIOSK_FPS=1`). Frames start as fast as
/// the display takes them, so starts under a quarter second apart are one
/// motion; its last half second is averaged.
#[derive(Default)]
pub(super) struct Fps(std::collections::VecDeque<std::time::Instant>);

impl Fps {
    /// The rate up to the frame starting `now`, once its motion has two frames.
    pub(super) fn frame(&mut self, now: std::time::Instant) -> Option<u32> {
        let since = |then: &std::time::Instant| now.duration_since(*then).as_secs_f64();
        if self.0.back().is_some_and(|last| since(last) > 0.25) {
            self.0.clear();
        }
        self.0.push_back(now);
        while since(&self.0[0]) > 0.5 {
            self.0.pop_front();
        }
        let span = since(&self.0[0]);
        (span > 0.).then(|| ((self.0.len() - 1) as f64 / span).round() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portrait_copy_rotates_channels_and_respects_padded_rows() {
        let mut frame = Pixmap::new(2, 3).unwrap();
        for (index, pixel) in frame.data_mut().chunks_exact_mut(4).enumerate() {
            pixel.copy_from_slice(&[index as u8 + 1, 20, 30, 255]);
        }
        let mut dst = [99; 32];
        copy_portrait(&frame, &mut dst, 16, 3, 2);
        for (y, expected) in [[5, 3, 1], [6, 4, 2]].iter().enumerate() {
            for (x, red) in expected.iter().enumerate() {
                assert_eq!(&dst[y * 16 + x * 4..][..4], &[30, 20, *red, 255]);
            }
            assert_eq!(&dst[y * 16 + 12..][..4], &[99; 4]);
        }
    }

    #[test]
    fn portrait_input_maps_corners_partial_axes_and_release() {
        for ((x, y), (u, v)) in [
            ((0., 0.), (0., 1.)),
            ((1., 0.), (0., 0.)),
            ((0., 1.), (1., 1.)),
            ((1., 1.), (1., 0.)),
        ] {
            assert_eq!(
                input(InputEvent::Absolute(Some(x), Some(y))),
                InputEvent::Absolute(Some(u), Some(v))
            );
        }
        assert_eq!(
            input(InputEvent::Absolute(Some(0.25), None)),
            InputEvent::Absolute(None, Some(0.75))
        );
        assert_eq!(
            input(InputEvent::Motion(3., 5.)),
            InputEvent::Motion(3., 5.)
        );
        assert_eq!(input(InputEvent::Button(false)), InputEvent::Button(false));
    }
}
