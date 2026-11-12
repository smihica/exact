# On evdev one wheel detent of an ordinary mouse scrolls twice

**Status:** Closed
**Resolution:** Evdev selects high-resolution input per device and axis, without duplicate ticks or lost input from other wheels.
**Systems:** host/linux/src/input.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Input::read` turns both `REL_WHEEL` and `REL_WHEEL_HI_RES` into `InputEvent::Wheel`
(`host/linux/src/input.rs:334-347`) and relies on `coalesce` (`input.rs:419-430`) to
drop the coarse event when a device reports both. LLP 1015 §6 says the wheels are
"coarse and high-resolution, folded".

`coalesce` recognises a high-resolution device only by a vertical delta that is not a
multiple of `LINE` (40 points, one notch):

```rust
let fine = events.iter()
    .any(|e| matches!(e, InputEvent::Wheel(_, dy) if dy.abs() % LINE != 0.0));
if !fine { return events; }
```

Linux reports both codes for every HID wheel, and a wheel with detents reports
exactly 120 high-resolution units a click. The kernel's event-code documentation
(<https://docs.kernel.org/input/event-codes.html>, `REL_WHEEL_HI_RES`): "The
accumulated value 120 represents movement by one detent. For devices that do not
provide high-resolution scrolling, the value is always a multiple of 120", and the
code "will be emitted in addition to REL_WHEEL or REL_HWHEEL". The HID driver sends
the two together (`hidinput_handle_scroll`, `drivers/hid/hid-input.c`, read at v6.12).

120 units and one coarse click both become `Wheel(0, ∓40)`, nothing is "fine", and
both are delivered.

The test also reads only `dy`, so a horizontal high-resolution wheel never counts as
fine and its coarse click is always kept.

## The failure

One report from a USB mouse turning its wheel one detent towards the user:
`REL_WHEEL -1`, `REL_WHEEL_HI_RES -120`, `SYN_REPORT`.

Observed: `[Wheel(0.0, 40.0), Wheel(0.0, 40.0)]`, 80 points. Expected: 40, the one
tick the VNC path sends for the same click (`vnc.rs:322-327`).

A horizontal report `REL_HWHEEL 1`, `REL_HWHEEL_HI_RES 15` gives
`[Wheel(40.0, 0.0), Wheel(5.0, 0.0)]`, 45 points where 5 were meant.

## Verification

`input.rs` compiles only on Linux, and this review ran on a Mac with no Linux runtime,
so the four wheel arms, `LINE` and `coalesce` were copied unchanged into a standalone
program and given those reports. It printed
`one detent, ordinary wheel: [Wheel(0.0, 40.0), Wheel(0.0, 40.0)] -> scrolls (0.0, 80.0)`
and
`horizontal smooth report with its coarse click: [Wheel(40.0, 0.0), Wheel(5.0, 0.0)] -> scrolls (45.0, 0.0)`.
A smooth vertical wheel sending one detent as eight reports of 15 units scrolled 40,
as it should.

Not run: the module itself, a real mouse. LLP 1015 §6 records that evdev has carried
no real event yet, which is why this has not been seen.

## Direction

Decide per device, not per batch: when a device's `supported_relative_axes()` has
`REL_WHEEL_HI_RES` (or `REL_HWHEEL_HI_RES`), ignore its `REL_WHEEL` (or `REL_HWHEEL`),
and delete `coalesce`.

## Related

`QUEUE.md`, "Run the Linux host on KMS" (evdev input reaching the app is still to be
qualified on hardware) does not name the wheel. LLP 1015.002 is the game wheel on
Windows. No issue mentions `coalesce` in `input.rs`.
