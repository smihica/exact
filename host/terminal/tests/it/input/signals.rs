//! Signal interleavings stopped at the child’s system calls, without timing races.

use super::*;
use std::os::unix::process::ExitStatusExt;

pub(super) struct Trace(i32);

impl Trace {
    pub(super) fn attach(pty: &Pty) -> Self {
        let trace = Self(pty.child.id() as i32);
        trace.call(libc::PTRACE_ATTACH, 0);
        assert!(libc::WIFSTOPPED(trace.wait()));
        trace.call(libc::PTRACE_SETOPTIONS, libc::PTRACE_O_TRACESYSGOOD);
        trace
    }

    fn call(&self, request: libc::c_uint, data: i32) {
        assert_eq!(
            unsafe { libc::ptrace(request, self.0, 0usize, data as usize) },
            0
        );
    }

    fn wait(&self) -> i32 {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let mut status = 0;
            let ready =
                unsafe { libc::waitpid(self.0, &mut status, libc::WNOHANG | libc::WUNTRACED) };
            if ready == self.0 {
                return status;
            }
            assert_eq!(ready, 0);
            assert!(Instant::now() < deadline, "child did not change state");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn until(&self, signal: i32, mut matches: impl FnMut(&libc::user_regs_struct) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        self.call(libc::PTRACE_SYSCALL, signal);
        loop {
            let status = self.wait();
            assert!(libc::WIFSTOPPED(status), "child exited: {status}");
            let signal = libc::WSTOPSIG(status);
            if signal == libc::SIGTRAP | 0x80 {
                let mut regs = std::mem::MaybeUninit::<libc::user_regs_struct>::uninit();
                assert_eq!(
                    unsafe {
                        libc::ptrace(libc::PTRACE_GETREGS, self.0, 0usize, regs.as_mut_ptr())
                    },
                    0
                );
                if matches(&unsafe { regs.assume_init() }) {
                    return;
                }
            }
            assert!(
                Instant::now() < deadline,
                "child did not reach the expected syscall"
            );
            self.call(
                libc::PTRACE_SYSCALL,
                if signal == libc::SIGTRAP | 0x80 {
                    0
                } else {
                    signal
                },
            );
        }
    }

    fn word(&self, address: u64) -> u64 {
        unsafe { libc::ptrace(libc::PTRACE_PEEKDATA, self.0, address as usize, 0usize) as u64 }
    }

    fn leave(&self, signal: i32) {
        self.until(signal, |regs| {
            regs.orig_rax == libc::SYS_write as u64
                && regs.rax == (-libc::ENOSYS as i64) as u64
                && regs.rdi == 1
                && self
                    .word(regs.rsi)
                    .to_le_bytes()
                    .starts_with(b"\x1b]8;;\x1b\\")
        });
    }

    pub(super) fn blocked_leave(&self, pty: &Pty) {
        self.leave(0);
        assert_eq!(
            unsafe { libc::tcflow(pty.slave.as_raw_fd(), libc::TCOOFF) },
            0
        );
        self.until(0, |regs| {
            if regs.rax != (-libc::ENOSYS as i64) as u64 {
                return false;
            }
            if regs.orig_rax == libc::SYS_ppoll as u64 {
                assert_eq!(regs.rdx, 0, "restoration must wait without a deadline");
            } else if regs.orig_rax == libc::SYS_poll as u64 {
                assert_eq!(
                    regs.rdx as i32, -1,
                    "restoration must wait without a deadline"
                );
            } else {
                return false;
            }
            let fd = self.word(regs.rdi);
            assert_eq!(fd as i32, 1);
            assert_ne!((fd >> 32) as i16 & libc::POLLOUT, 0);
            true
        });
    }

    pub(super) fn detach(&self) {
        self.call(libc::PTRACE_DETACH, 0);
    }
}

fn termination_status(pty: &mut Pty) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = pty.child.try_wait().unwrap() {
            return status;
        }
        pty.text();
        assert!(
            Instant::now() < deadline,
            "termination did not finish cleanup"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn termination_during_suspend_cleanup_preserves_leave_bytes() {
    let mut pty = Pty::spawn("terminate-leave", SUSPEND_APP);
    pty.until("Ready");
    pty.until_output("\x1b[?2026l");
    pty.output.clear();
    assert_eq!(
        unsafe { libc::tcflow(pty.slave.as_raw_fd(), libc::TCOOFF) },
        0
    );
    let trace = Trace::attach(&pty);
    trace.leave(libc::SIGTSTP);
    trace.until(0, |regs| {
        regs.orig_rax == libc::SYS_write as u64 && regs.rax == (-libc::EAGAIN as i64) as u64
    });
    // Termination interrupts an unwritten leave. Let the host restore input,
    // then make output available before its bounded cleanup can expire.
    pty.signal(libc::SIGTERM);
    trace.until(0, |regs| {
        regs.orig_rax == libc::SYS_ioctl as u64
            && regs.rax == 0
            && pty.termios().c_lflag & libc::ICANON != 0
    });
    assert_eq!(
        unsafe { libc::tcflow(pty.slave.as_raw_fd(), libc::TCOON) },
        0
    );
    trace.detach();
    assert_eq!(termination_status(&mut pty).signal(), Some(libc::SIGTERM));
    pty.text();
    assert!(
        String::from_utf8_lossy(&pty.output).contains("\x1b[?25h\x1b[?1049l"),
        "termination discarded the leave bytes: {:?}",
        pty.output
    );
}

#[test]
fn a_second_suspend_notification_does_not_quit_after_resume() {
    let source = SUSPEND_APP.replace("      input", "      text `value: ${value}`\n      input");
    let mut pty = Pty::spawn("second-suspend", &source);
    pty.until("Ready");
    let trace = Trace::attach(&pty);
    // The socket was drained before stop restores the termination handlers.
    // Deliver the second suspend before stop consumes the pending signal.
    trace.until(libc::SIGTSTP, |regs| {
        regs.orig_rax == libc::SYS_rt_sigaction as u64
            && regs.rdi == libc::SIGQUIT as u64
            && regs.rax == 0
    });
    pty.signal(libc::SIGTSTP);
    trace.detach();
    pty.stopped();
    pty.output.clear();
    pty.signal(libc::SIGCONT);
    pty.until_output("Ready");
    pty.write(b"resumed");
    pty.until("value: resumed");
    pty.write(b"\x03");
    pty.exited();
}

#[test]
fn termination_interrupts_normal_exit_cleanup() {
    let mut pty = Pty::spawn("terminate-normal-leave", SUSPEND_APP);
    pty.until("Ready");
    pty.until_output("\x1b[?2026l");
    assert_eq!(
        unsafe { libc::tcflow(pty.slave.as_raw_fd(), libc::TCOOFF) },
        0
    );
    let trace = Trace::attach(&pty);
    pty.write(b"\x03");
    trace.leave(0);
    pty.signal(libc::SIGTERM);
    trace.detach();
    assert_eq!(termination_status(&mut pty).signal(), Some(libc::SIGTERM));
    let cooked = libc::ICANON | libc::ECHO | libc::ISIG;
    assert_eq!(pty.termios().c_lflag & cooked, cooked);
}

#[test]
fn termination_between_restoration_and_stop_is_not_deferred() {
    let mut pty = Pty::spawn("terminate-before-stop", SUSPEND_APP);
    pty.until("Ready");
    let trace = Trace::attach(&pty);
    // Linux raise(SIGSTOP) reads its thread ID immediately before tgkill.
    // Interrupt that syscall's return, after the last pending-signal check.
    trace.until(libc::SIGTSTP, |regs| {
        regs.orig_rax == libc::SYS_gettid as u64 && regs.rax == pty.child.id() as u64
    });
    pty.signal(libc::SIGTERM);
    trace.detach();
    let status = trace.wait();
    assert!(
        libc::WIFSIGNALED(status),
        "termination was deferred: {status}"
    );
    assert_eq!(libc::WTERMSIG(status), libc::SIGTERM);
}
