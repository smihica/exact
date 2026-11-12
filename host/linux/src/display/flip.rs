use crate::presenter::SubmittedFrame;
use drm::control::{crtc, Event};
use tiny_skia::Pixmap;

#[cfg(test)]
#[path = "flip/tests.rs"]
mod tests;

// The two buffers and the one submitted owner transition together, only after
// successful syscalls. A failed copy/ioctl cannot change front or claim a flip.
pub(super) struct FlipState {
    front: usize,
    first: bool,
    pending: Option<(usize, SubmittedFrame)>,
    pub(super) sequence: Option<u32>,
    pub(super) at: Option<std::time::Duration>,
}
impl Default for FlipState {
    fn default() -> Self {
        Self {
            front: 0,
            first: true,
            pending: None,
            sequence: None,
            at: None,
        }
    }
}
impl FlipState {
    pub(super) fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub(super) fn submit(
        &mut self,
        frame: SubmittedFrame,
        send: impl FnOnce(usize, bool, &Pixmap) -> Result<(), String>,
    ) -> Result<Option<SubmittedFrame>, String> {
        if self.pending() {
            return Err("display already has a pending flip".into());
        }
        let back = 1 - self.front;
        send(back, self.first, &frame.pixels)?;
        if self.first {
            self.first = false;
            self.front = back;
            Ok(Some(frame))
        } else {
            self.pending = Some((back, frame));
            Ok(None)
        }
    }
    pub(super) fn ready(
        &mut self,
        crtc: crtc::Handle,
        read: impl FnOnce() -> std::io::Result<Vec<Event>>,
    ) -> std::io::Result<Option<SubmittedFrame>> {
        let Some((sequence, at)) = receive_flip(crtc, read)? else {
            return Ok(None);
        };
        let Some((back, frame)) = self.pending.take() else {
            return Ok(None);
        };
        self.front = back;
        self.sequence = Some(sequence);
        self.at = Some(at);
        Ok(Some(frame))
    }
}

// drm::receive_events reads at most 1024 bytes. One bounded read per ready turn
// lets other FDs/timers run. There is one pending flip and one event per flip.
// Its sequence is only a sample: drivers without a vblank counter report zero.
fn receive_flip(
    crtc: crtc::Handle,
    read: impl FnOnce() -> std::io::Result<Vec<Event>>,
) -> std::io::Result<Option<(u32, std::time::Duration)>> {
    let events = match read() {
        Ok(events) => events,
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
            ) =>
        {
            return Ok(None)
        }
        Err(e) => return Err(e),
    };
    Ok(events.into_iter().find_map(|event| match event {
        Event::PageFlip(e) if e.crtc == crtc => {
            // `duration` is the kernel's flip timestamp (tv_sec/tv_usec).
            Some((e.frame, e.duration))
        }
        _ => None,
    }))
}
