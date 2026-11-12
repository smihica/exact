# The Windows serial `open` turns DTR off and parity checking off

**Status:** Closed
**Resolution:** `open` writes the DCB with `fParity` set and `fDtrControl = DTR_CONTROL_ENABLE`, as Chromium's Web Serial does, so DTR is raised on open and parity errors are reported.
**Systems:** packages/serial/windows/lib.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `open` in `packages/serial/windows/lib.rs:352-363` reads the port's `DCB` and
then replaces all of its bit fields with one word (`:358`):

```rust
// fBinary, and RTS/CTS handshaking for hardware flow control (fOutxCtsFlow, fRtsControl = 2).
dcb.flags = 1 | if options["flowControl"] == "hardware" { 0x4 | (2 << 12) } else { 1 << 12 };
```

In the `DCB`'s layout
([winbase.h](https://learn.microsoft.com/windows/win32/api/winbase/ns-winbase-dcb)) bit 0 is
`fBinary`, bit 1 `fParity`, bits 4 to 5 `fDtrControl` and bits 12 to 13 `fRtsControl`. The word
is `0x1001`, or `0x2005` with hardware flow control. Both leave two fields zero that the comment
does not mention:

- `fDtrControl = 0` is `DTR_CONTROL_DISABLE`: "Disables the DTR line when the device is opened
  and leaves it disabled." So every open ends with DTR off and, without hardware flow control,
  RTS on. On Linux and macOS the terminal's open raises both, and Chromium's Web Serial on
  Windows writes `config.fDtrControl = DTR_CONTROL_ENABLE` beside the same RTS choice as here
  (`services/device/serial/serial_io_handler_win.cc`). A board that waits for DTR before it sends
  (a CDC-ACM sketch's `while (!Serial)`) says nothing on Windows. On boards wired as Espressif's
  are, RTS asserted with DTR released holds the chip in reset (LLP 1133.037 D9 describes that
  wiring for Bench's Reset button) for as long as the port stays that way.
- `fParity = 0`. The structure's documentation says "If this member is TRUE, parity checking is
  performed and errors are reported", and Chromium sets it `TRUE`. With it clear, by that
  sentence, `ClearCommError` does not report `CE_RXPARITY`, and the `ParityError` the read loop
  maps (`:263`) cannot occur with `parity="even"` or `"odd"`.

## The failure

On Windows, `open(port="COM3", baudRate=115200)` on an ESP32 development board. Expected, as on
the web, macOS and Linux: the board runs and its output arrives. By the `DCB` written: DTR is
off and RTS on, and the board stays in reset until the app sends
`setSignals(dataTerminalReady=true)`.

## Verification

Read, not run: no Windows machine here, and QUEUE "USB, HID and serial (LLP 1133.037): what is
left" records that the Windows crates have not been driven on hardware. The expression at `:358`
was evaluated and decoded field by field in the structure's documented order:

```
flowControl=none: flags=0x1001  fBinary=1 fParity=0 fDtrControl=DTR_CONTROL_DISABLE fRtsControl=RTS_CONTROL_ENABLE
flowControl=hardware: flags=0x2005  fBinary=1 fParity=0 fOutxCtsFlow=1 fDtrControl=DTR_CONTROL_DISABLE fRtsControl=RTS_CONTROL_HANDSHAKE
```

Nothing else in the file raises DTR after `open`; `EscapeCommFunction` is called only by
`setSignals`. Whether the Windows driver withholds `CE_RXPARITY` when `fParity` is clear was not
observed.

The device check: on Windows, open a port whose DTR is looped to DSR (or an ESP32 board) and read
`getSignals`: `dataSetReady` should be true after `open`.

## Direction

Set `fDtrControl` to `DTR_CONTROL_ENABLE` (`1 << 4`) and `fParity` (`1 << 1`), as Chromium does,
keeping the rest as written.

## Related

None found in `issues/`; the QUEUE entry above owes the hardware drive that would show it.
