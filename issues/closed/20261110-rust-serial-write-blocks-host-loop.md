# On the Linux and Windows hosts a serial `write` holds the host's loop until the device takes every byte

**Status:** Closed
**Resolution:** Each connection's writer thread writes the calls' bytes in order and answers each write once the device has taken it, so the loop never waits on the device, and `close` fails the writes not yet taken with `AbortError`.
**Systems:** packages/serial/linux/lib.rs, packages/serial/windows/lib.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

A package's `later` runs on the host's loop thread, between turns
(`host/linux/src/package.rs:13-15`). At `dd6f5dcd` both Rust implementations of `write` finish
the whole write inside it:

- `packages/serial/linux/lib.rs:478-489` loops until every byte is written, sleeping a
  millisecond on `EAGAIN`:

  ```rust
  while sent < bytes.len() {
      match (&*c.file).write(&bytes[sent..]) {
          Ok(n) => sent += n,
          Err(e) if e.kind() == ErrorKind::WouldBlock || ... => std::thread::sleep(Duration::from_millis(1)),
  ```

- `packages/serial/windows/lib.rs:459` calls a synchronous `WriteFile`, and `open` set no write
  timeout (`write_multiplier: 0, write_constant: 0`, `:363`), which Win32 reads as "total
  time-outs are not used for write operations"
  ([COMMTIMEOUTS](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-commtimeouts)).

A device that stops taking bytes (hardware flow control with CTS low, a board in reset)
therefore stops the loop: nothing paints, no input is taken, and `close` or the session's end
cannot run, because they are calls on the same thread. By the same loop a write larger than the
driver's buffer holds the thread for as long as the line takes to send it: at 115,200 baud
(11,520 bytes a second), more than five seconds for 64 KiB. The Apple implementation had this on its own queue and was
fixed by queueing the output (`20261102-serial-blocked-write-prevents-close`).

**The failure.** `open(port=…, baudRate=115200)`, then `write(connection=…, data=<64 KiB>)`
while the device reads nothing.

**Verification.** Bench on the Linux host run headless on macOS (`cargo build --profile host-dev
-p bench-linux`), the Rust implementation bound, launched as `packages/serial/test/pty.mjs linux`
launches it. Its port was a pseudo-terminal (`EXACT_DEVICE_FIXTURE`) whose other end read
nothing for 8 s after the first byte arrived. The operations:

```
"clock settle" "type port pty" "clock settle" "tap connect" "clock settle"
"type send <64 KiB of x>" "type send key Enter" "state connection" "tree send" "tap clear"
```

With the other end reading at once, every operation had answered 0.4 s after launch. With it
silent, `type send key Enter` and each operation after it answered at 8.3 s, when the reading
began. `sample <pid> 1`, taken during the wait, had all 584 samples of the main thread under
`Presenter::package_turn`, `Packages::dispatch`, `Serial::later`: 501 in `thread::sleep`, 83 in
`write`.

Not run: Linux itself, a UART, or Windows (by reading: `WriteFile` with zero write timeouts on
a handle opened without `FILE_FLAG_OVERLAPPED`).

**Direction.** Hand the bytes to a writer thread per connection (the reader already has one)
that answers the `Reply` when they are out and stops at `close`; `Reply` is `Send` for this.

**Related.** Closed `20261102-serial-blocked-write-prevents-close` (Apple only). QUEUE "A bound
Linux package's data-code call stalls 20 s" is a poll during `clock +N real`, a different stall.
`issues/20261110-linux-serial-close-keeps-descriptor.md` is `close` itself.
