/// The rate of the motion on screen (`KIOSK_FPS=1`). Frames start as fast as
/// the display takes them, so starts under a quarter second apart are one
/// motion; its last half second is averaged.
#[derive(Default)]
pub(crate) struct Fps(std::collections::VecDeque<std::time::Instant>);

impl Fps {
    /// The rate up to the frame starting `now`, once its motion has two frames.
    pub(crate) fn frame(&mut self, now: std::time::Instant) -> Option<u32> {
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
