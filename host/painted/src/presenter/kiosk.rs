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

    /// Hand a DRM connector to the Ink painter for direct presentation.
    ///
    /// # Safety
    /// The caller owns DRM master and keeps `fd` open while this presenter lives.
    /// It must stop submitting and reading display events on that descriptor.
    #[cfg(all(feature = "scanout", target_os = "linux"))]
    #[allow(unsafe_code)]
    pub unsafe fn kiosk_attach_scanout(
        &mut self,
        fd: i32,
        connector: u32,
        size: [u32; 2],
        refresh: u32,
        turned: bool,
    ) -> Result<(), String> {
        // SAFETY: the shell upholds the display ownership contract above.
        unsafe {
            self.brush
                .attach_scanout(fd, connector, size, refresh, turned)
        }
    }

    /// Whether the painter still owns direct presentation after a frame.
    #[cfg(all(feature = "scanout", target_os = "linux"))]
    pub fn kiosk_presents(&self) -> bool {
        self.brush.presents()
    }
}
