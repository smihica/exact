//! Nonblocking output and the unwritten bytes retained across termination.

use super::{drain_signals, SIGNAL};
use rustix::event::{poll, PollFd, PollFlags, Timespec};
use std::io::{self, ErrorKind};
use std::os::unix::net::UnixStream;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

trait Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize>;
    fn wait(&mut self, timeout: Option<Duration>) -> io::Result<()>;
    fn terminating(&self) -> bool;
    fn now(&self) -> Instant;
}

struct Terminal<'a>(Option<&'a UnixStream>);

impl Output for Terminal<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        rustix::io::write(rustix::stdio::stdout(), bytes).map_err(Into::into)
    }

    fn wait(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        let stdout = rustix::stdio::stdout();
        let mut fds = vec![PollFd::new(&stdout, PollFlags::OUT)];
        if let Some(rx) = self.0 {
            fds.push(PollFd::new(rx, PollFlags::IN));
        }
        let timeout = timeout.map(|duration| Timespec {
            tv_sec: duration.as_secs() as i64,
            tv_nsec: duration.subsec_nanos() as i64,
        });
        match poll(&mut fds, timeout.as_ref()) {
            Ok(_) | Err(rustix::io::Errno::INTR) => {}
            Err(e) => return Err(e.into()),
        }
        if let Some(rx) = self.0 {
            drain_signals(rx);
        }
        Ok(())
    }

    fn terminating(&self) -> bool {
        let signal = SIGNAL.load(Ordering::SeqCst);
        signal != 0 && signal != libc::SIGTSTP
    }

    fn now(&self) -> Instant {
        Instant::now()
    }
}

fn write_pending(bytes: &mut &[u8], output: &mut impl Output, bounded: bool) -> io::Result<()> {
    let deadline = bounded.then(|| output.now() + Duration::from_secs(2));
    while !bytes.is_empty() {
        if !bounded && output.terminating() {
            return Err(ErrorKind::Interrupted.into());
        }
        let timeout = if let Some(deadline) = deadline {
            let remaining = deadline.saturating_duration_since(output.now());
            if remaining.is_zero() {
                return Err(ErrorKind::TimedOut.into());
            }
            Some(remaining)
        } else {
            None
        };
        match output.write(bytes) {
            Ok(0) => return Err(ErrorKind::WriteZero.into()),
            Ok(n) => *bytes = &bytes[n..],
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == ErrorKind::WouldBlock => output.wait(timeout)?,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

pub(super) fn write(mut bytes: &[u8], signal_rx: &UnixStream) -> io::Result<()> {
    // Inline history and the cursor already describe these bytes. Finish the
    // frame before suspending so continuation neither loses nor repeats them.
    write_pending(&mut bytes, &mut Terminal(Some(signal_rx)), false)
}

pub(super) fn restore(
    leave: &[u8],
    signal_rx: Option<&UnixStream>,
    restore_input: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    restore_with(
        leave,
        &mut Terminal(signal_rx),
        signal_rx.is_none(),
        restore_input,
    )
}

fn restore_with(
    mut leave: &[u8],
    output: &mut impl Output,
    panic: bool,
    restore_input: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let result = if panic {
        Err(ErrorKind::Interrupted.into())
    } else {
        write_pending(&mut leave, output, false)
    };
    let restored = restore_input();
    // Termination can arrive during any write or wait, including normal exit.
    // Restore input first, then give only the unwritten suffix a bounded retry.
    let result = if result
        .as_ref()
        .is_err_and(|e| e.kind() == ErrorKind::Interrupted)
    {
        write_pending(&mut leave, output, true)
    } else {
        result
    };
    restored.and(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::collections::VecDeque;

    enum Step {
        Write(usize),
        Block,
        Wait {
            timeout: Option<Duration>,
            advance: Duration,
            terminate: bool,
        },
    }

    struct Writer<'a> {
        steps: VecDeque<Step>,
        now: Instant,
        terminating: bool,
        emitted: Vec<u8>,
        restored: &'a Cell<bool>,
    }

    impl Output for Writer<'_> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            assert_eq!(self.restored.get(), self.terminating);
            match self.steps.pop_front().expect("unexpected write") {
                Step::Write(n) => {
                    let n = n.min(bytes.len());
                    self.emitted.extend_from_slice(&bytes[..n]);
                    Ok(n)
                }
                Step::Block => Err(ErrorKind::WouldBlock.into()),
                Step::Wait { .. } => panic!("expected wait"),
            }
        }

        fn wait(&mut self, timeout: Option<Duration>) -> io::Result<()> {
            let Some(Step::Wait {
                timeout: expected,
                advance,
                terminate,
            }) = self.steps.pop_front()
            else {
                panic!("unexpected wait");
            };
            assert_eq!(timeout, expected);
            self.now += advance;
            self.terminating = terminate;
            Ok(())
        }

        fn terminating(&self) -> bool {
            self.terminating
        }
        fn now(&self) -> Instant {
            self.now
        }
    }

    #[test]
    fn normal_exit_and_suspend_wait_without_a_deadline() {
        for leave in [super::super::LEAVE_FULL, super::super::LEAVE_INLINE] {
            let restored = Cell::new(false);
            let mut writer = Writer {
                steps: VecDeque::from([
                    Step::Block,
                    Step::Wait {
                        timeout: None,
                        advance: Duration::from_secs(3),
                        terminate: false,
                    },
                    Step::Write(usize::MAX),
                ]),
                now: Instant::now(),
                terminating: false,
                emitted: Vec::new(),
                restored: &restored,
            };
            restore_with(leave.as_bytes(), &mut writer, false, || {
                restored.set(true);
                Ok(())
            })
            .unwrap();
            assert_eq!(writer.emitted, leave.as_bytes());
            assert!(restored.get());
            assert!(writer.steps.is_empty());
        }
    }

    #[test]
    fn termination_preserves_the_unwritten_suffix_and_bounds_its_retry() {
        for prefix in [0, 7] {
            for drain in [true, false] {
                let restored = Cell::new(false);
                let mut steps = VecDeque::new();
                if prefix > 0 {
                    steps.push_back(Step::Write(prefix));
                }
                steps.extend([
                    Step::Block,
                    Step::Wait {
                        timeout: None,
                        advance: Duration::from_secs(3),
                        terminate: true,
                    },
                    Step::Block,
                    Step::Wait {
                        timeout: Some(Duration::from_secs(2)),
                        advance: Duration::from_secs(if drain { 1 } else { 2 }),
                        terminate: true,
                    },
                ]);
                if drain {
                    steps.push_back(Step::Write(usize::MAX));
                }
                let mut writer = Writer {
                    steps,
                    now: Instant::now(),
                    terminating: false,
                    emitted: Vec::new(),
                    restored: &restored,
                };
                let leave = super::super::LEAVE_FULL.as_bytes();
                let result = restore_with(leave, &mut writer, false, || {
                    restored.set(true);
                    Ok(())
                });
                if drain {
                    result.unwrap();
                    assert_eq!(writer.emitted, leave);
                } else {
                    assert_eq!(result.unwrap_err().kind(), ErrorKind::TimedOut);
                    assert_eq!(writer.emitted, leave[..prefix]);
                }
                assert!(restored.get());
                assert!(writer.steps.is_empty());
            }
        }
    }
}
