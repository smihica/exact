# The Linux serial `close` never closes the port's descriptor, so the port cannot be opened again

**Status:** Closed
**Resolution:** `close` ends exclusive mode (`TIOCNXCL`) and drops the connection's handle, so the reader thread's is the last and the port opens again.
**Systems:** packages/serial/linux/lib.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd`, `Serial::close` in `packages/serial/linux/lib.rs:430-440` marks the connection
closed and stops its reader thread, but leaves the `Connection` in `self.connections`:

```rust
c.open = false;
c.stop.store(true, Ordering::Relaxed);
c.stream.1.notify_all();
```

`Connection.file` is an `Arc<File>` (`:144`), and nothing removes the entry or drops the file
before the session ends, so the descriptor stays open after `close`, `forget` and a port's
`done`. `open` put the terminal in exclusive mode (`TIOCEXCL`, `:385`), under which Linux
refuses every later `open(2)` of that terminal, from this process too: "No further open(2)
operations on the terminal are permitted. (They fail with EBUSY, except for a process with the
CAP_SYS_ADMIN capability.)"
([TIOCEXCL(2const)](https://man7.org/linux/man-pages/man2/TIOCEXCL.2const.html)). `open` maps
`EBUSY` to `InvalidStateError` (`:380`).

So on the Linux host a person who disconnects and connects again is refused for the rest of
the session. The device also never sees the port close (DTR stays asserted), and another
program cannot take the port while the app runs. The Windows implementation closes its handle
in `close` (`packages/serial/windows/lib.rs:392-407`).

**The failure.**

```
send c = open(port="/dev/ttyUSB0", baudRate=115200)
send x = close(connection=c.connection)
send again = open(port="/dev/ttyUSB0", baudRate=115200)
```

Expected: `again` is a new connection. On Linux: `InvalidStateError`, "Device or resource
busy". In Bench, Disconnect then Connect shows "In use: another program or window has this
port open."

**Verification.** Bench on the Linux host run headless on macOS (`cargo build --profile host-dev
-p bench-linux`), the Rust implementation bound, its port a pseudo-terminal named through
`EXACT_DEVICE_FIXTURE`, launched as `packages/serial/test/pty.mjs linux` launches it, with these
operations:

```
"clock settle" "type port pty" "clock settle" "tap connect" "clock settle" "clock +2500 real"
"tap disconnect" "clock settle" "state connection" "clock +2500 real"
"tap connect" "clock settle" "tap disconnect" "clock settle" "state connection" "clock +2500 real"
```

`lsof <the pty>`, polled beside the drive, listed for `bench-linux`:

```
after Connect:                            1 descriptor (fd 11u)
after Disconnect (connection reads ""):   1
after Connect and Disconnect again:       2 (fd 10u, 11u)
```

Not run: Linux. macOS's pseudo-terminals do not enforce `TIOCEXCL`, so the second open
succeeded there; the refusal is the manual page's.

**Direction.** Keep the file as an `Option` the connection takes at `close` (the reader thread's
clone goes when it ends), or remove the entry once its stream has answered `done`.

**Related.** Closed `20261102-serial-blocked-write-prevents-close` is the Apple implementation's
write queue; closed `20261102-listing-serial-ports-opens-them` is listing. Neither covers this.
`packages/serial/test/pty.mjs` opens once and never closes.
