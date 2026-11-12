//! The kernel's notice that the date jumped: a timer on `CLOCK_REALTIME`
//! armed with `TFD_TIMER_CANCEL_ON_SET`, which becomes readable when the
//! clock is set discontinuously (timerfd_create(2)). A resume from suspend
//! counts as one: the kernel notifies such timers on resume, since the
//! monotonic clock did not run through it (`timerfd_resume`,
//! kernel/time/timekeeping.c).
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

pub(super) struct ClockSet(OwnedFd);

impl ClockSet {
    /// `None` where the kernel refuses the timer; the loop's checks remain.
    pub(super) fn open() -> Option<Self> {
        // SAFETY: creates a descriptor this value then owns.
        let fd = unsafe {
            libc::timerfd_create(libc::CLOCK_REALTIME, libc::TFD_NONBLOCK | libc::TFD_CLOEXEC)
        };
        if fd < 0 {
            return None;
        }
        // SAFETY: `fd` is a new descriptor nothing else owns.
        let set = Self(unsafe { OwnedFd::from_raw_fd(fd) });
        // Due at the end of time: the timer never expires, only cancels.
        // SAFETY: an all-zero `itimerspec` is a disarmed timer.
        let mut at: libc::itimerspec = unsafe { std::mem::zeroed() };
        at.it_value.tv_sec = libc::time_t::MAX;
        let flags = libc::TFD_TIMER_ABSTIME | libc::TFD_TIMER_CANCEL_ON_SET;
        // SAFETY: `at` is a valid timer value; no old value is asked for.
        let armed = unsafe { libc::timerfd_settime(fd, flags, &at, std::ptr::null_mut()) };
        (armed == 0).then_some(set)
    }

    pub(super) fn fd(&self) -> i32 {
        self.0.as_raw_fd()
    }

    /// Whether the date jumped since the last call. The read that reports
    /// it (`ECANCELED`) also rearms the notice.
    pub(super) fn take(&self) -> bool {
        let mut count = [0u8; 8];
        // SAFETY: reads at most the buffer's eight bytes from our descriptor.
        let read = unsafe { libc::read(self.fd(), count.as_mut_ptr().cast(), count.len()) };
        read < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ECANCELED)
    }
}
