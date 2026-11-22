//! The Linux kiosk shell's display and gesture boundary.
use super::*;

impl<D: DataSource> Presenter<D> {
    /// Current logical bounds and offset for the kiosk gesture owner.
    pub fn kiosk_scroll_state(&self, id: ViewId) -> Option<Scroller> {
        self.scroller(id, &self.collection_scroll_limits())
    }

    /// Move the logical scroll and apply its visual rubber-band displacement.
    pub fn kiosk_scroll_to(&mut self, id: ViewId, y: f32, overscroll: f32) -> bool {
        let Some(s) = self.kiosk_scroll_state(id) else {
            return false;
        };
        if self.scroll_to(id, (s.offset.0, y)).is_none() {
            return false;
        }
        let next = (overscroll != 0.).then_some((id, overscroll));
        if self.kiosk_overscroll != next {
            self.kiosk_overscroll = next;
            self.dirty = true;
        }
        true
    }

    /// Remove the visual displacement when the gesture ends or is cancelled.
    pub fn kiosk_clear_overscroll(&mut self) {
        if self.kiosk_overscroll.take().is_some() {
            self.dirty = true;
        }
    }

    /// The visual displacement in excess of a scroller's logical bounds.
    pub fn kiosk_overscroll(&self) -> Option<(ViewId, f32)> {
        self.kiosk_overscroll
    }

    /// Update the optional kiosk frame-rate overlay.
    pub fn set_kiosk_fps(&mut self, fps: u32) {
        self.brush.fps = Some(fps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kiosk_overscroll_moves_pixels_without_changing_logical_scroll() {
        let plan = crate::test_plan(
            r##"component App
  view
    scroll testId="port" width=80 height=60 background-color="#ffffff"
      column
        box testId="row" width=80 height=40 background-color="#ff0000"
        box width=80 height=200 background-color="#0000ff"
"##,
        );
        let (mut p, error) = Presenter::boot_with_fixture_fonts(
            &plan,
            (),
            (80., 60.),
            1.,
            PathBuf::new(),
            PainterChoice::Cpu,
        )
        .unwrap();
        assert!(error.is_none(), "{error:?}");
        p.frame();
        let port = stage::tests::id(&p, "port");
        let row = stage::tests::id(&p, "row");
        assert!(p.kiosk_scroll_to(port, -12., -12.));
        let displaced = p.frame();
        assert_eq!(p.kiosk_scroll_state(port).unwrap().offset, (0., 0.));
        assert_eq!(p.box_of(row).unwrap().rect.1, 12.);
        assert_eq!(displaced.pixel(20, 6).unwrap().green(), 255);
        assert_eq!(displaced.pixel(20, 16).unwrap().green(), 0);

        p.kiosk_clear_overscroll();
        let restored = p.frame();
        assert_eq!(p.kiosk_overscroll(), None);
        assert_eq!(p.box_of(row).unwrap().rect.1, 0.);
        assert_eq!(restored.pixel(20, 6).unwrap().green(), 0);

        assert!(p.kiosk_scroll_to(port, -12., -12.));
        p.reload(&plan, ()).unwrap();
        assert_eq!(p.kiosk_overscroll(), None);
    }
}
